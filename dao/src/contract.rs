#[cfg(not(feature = "library"))]
use cosmwasm_std::entry_point;
use cosmwasm_std::{
    to_binary, Binary, Deps, DepsMut, Env, MessageInfo, Response, StdError, StdResult,
};
use cw2::set_contract_version;

use crate::actions;
use crate::error::ContractError;
use crate::msg::{ExecuteMsg, InstantiateMsg, QueryMsg};
use crate::state::{State, TokenInfo, ITEMS, PROPOSALS, STATE, TOKEN_INFO};
use crate::tokens::{self, create_accounts};

// version info for migration info
const CONTRACT_NAME: &str = "webdao";
const CONTRACT_VERSION: &str = env!("CARGO_PKG_VERSION");

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn instantiate(
    mut deps: DepsMut,
    _env: Env,
    _info: MessageInfo,
    msg: InstantiateMsg,
) -> Result<Response, ContractError> {
    set_contract_version(deps.storage, CONTRACT_NAME, CONTRACT_VERSION)?;

    // Validate the token info and then set up initial balances. We
    // infer total supply from the initial balances.
    msg.token_info.validate()?;
    let total_supply = create_accounts(&mut deps, &msg.token_info.initial_balances)?;

    // Assert that the quorum is not zero and that it is less than the
    // total token supply.
    if msg.quorum.is_zero() || msg.quorum > total_supply {
        return Err(ContractError::InvalidQuorum);
    }

    // Store information about the token for later queries.
    let token_info = TokenInfo {
        name: msg.token_info.name,
        symbol: msg.token_info.symbol,
        decimals: msg.token_info.decimals,
        total_supply,
    };
    TOKEN_INFO.save(deps.storage, &token_info)?;

    // Set up the DAO state.
    let state = State {
        quorum: msg.quorum,
        proposal_cost: msg.proposal_cost,
    };
    STATE.save(deps.storage, &state)?;

    // Set up proposal state.
    let proposals = vec![];
    PROPOSALS.save(deps.storage, &proposals)?;
    let items = vec![];
    ITEMS.save(deps.storage, &items)?;

    Ok(Response::new()
        .add_attribute("method", "instantiate")
        .add_attribute("quorum", msg.quorum.to_string())
        .add_attribute("proposal_cost", msg.proposal_cost.to_string())
        .add_attribute("token_supply", total_supply))
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn execute(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    msg: ExecuteMsg,
) -> Result<Response, ContractError> {
    match msg {
        ExecuteMsg::Withdraw(w) => actions::handle_withdrawal(deps, env, info, w),
        ExecuteMsg::Transfer { recipient, amount } => {
            tokens::execute_transfer(deps, env, info, recipient, amount)
        }
        ExecuteMsg::Burn { amount } => tokens::execute_burn(deps, env, info, amount),
        ExecuteMsg::Send {
            contract,
            amount,
            msg,
        } => tokens::execute_send(deps, env, info, contract, amount, msg),
        ExecuteMsg::Propose(p) => actions::handle_propose(deps, env, info, p),
        ExecuteMsg::Vote(v) => actions::handle_vote(deps, env, info, v),
    }
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn query(deps: Deps, _env: Env, msg: QueryMsg) -> StdResult<Binary> {
    match msg {
        QueryMsg::ListProposals => Ok(to_binary(&PROPOSALS.load(deps.storage)?)?),
        QueryMsg::GetProposal { proposal_id } => {
            let proposals = PROPOSALS.load(deps.storage)?;
            Ok(to_binary(proposals.get(proposal_id as usize).ok_or(
                StdError::NotFound {
                    kind: format!("no such proposal ID ({})", proposal_id),
                },
            )?)?)
        }
        QueryMsg::ListItems => Ok(to_binary(&ITEMS.load(deps.storage)?)?),
        QueryMsg::GetItem { item_id } => {
            let items = ITEMS.load(deps.storage)?;
            Ok(to_binary(items.get(item_id as usize).ok_or(
                StdError::NotFound {
                    kind: format!("no such item ID ({})", item_id),
                },
            )?)?)
        }
        QueryMsg::GetQuorum => {
            let state = STATE.load(deps.storage)?;
            Ok(to_binary(&state.quorum)?)
        }
        QueryMsg::GetProposalCost => {
            let state = STATE.load(deps.storage)?;
            Ok(to_binary(&state.proposal_cost)?)
        }
        QueryMsg::Balance { address } => to_binary(&tokens::query_balance(deps, address)?),
        QueryMsg::TokenInfo {} => to_binary(&tokens::query_token_info(deps)?),
    }
}

#[cfg(test)]
mod tests {
    use crate::msg::{
        DaoItem, ProposeAction, ProposeMsg, TokenInstantiateInfo, VoteMsg, WithdrawVoteMsg,
    };
    use crate::state::{Proposal, ProposalStatus};

    use super::*;
    use cosmwasm_std::testing::{mock_dependencies, mock_env, mock_info, MOCK_CONTRACT_ADDR};
    use cosmwasm_std::{from_binary, CosmosMsg, SubMsg, Uint128, WasmMsg};
    use cw20::{BalanceResponse, Cw20Coin, Cw20ReceiveMsg, TokenInfoResponse};

    #[test]
    fn valid_initialization() {
        let mut deps = mock_dependencies(&[]);

        let msg = InstantiateMsg {
            quorum: Uint128::from(30u128),
            proposal_cost: Uint128::from(1u128),
            token_info: TokenInstantiateInfo {
                name: "item-dao".to_string(),
                symbol: "IDAO".to_string(),
                decimals: 3,
                initial_balances: vec![Cw20Coin {
                    address: "awallet".to_string(),
                    amount: Uint128::from(100000u128),
                }],
            },
        };
        let info = mock_info("creator", &[]);

        let res = instantiate(deps.as_mut(), mock_env(), info, msg).unwrap();
        assert_eq!(0, res.messages.len());

        let res = query(deps.as_ref(), mock_env(), QueryMsg::GetQuorum {}).unwrap();
        let value: Uint128 = from_binary(&res).unwrap();
        assert_eq!(Uint128::from(30u128), value);

        let res = query(deps.as_ref(), mock_env(), QueryMsg::GetProposalCost {}).unwrap();
        let value: Uint128 = from_binary(&res).unwrap();
        assert_eq!(Uint128::from(1u128), value);

        let res = query(deps.as_ref(), mock_env(), QueryMsg::TokenInfo {}).unwrap();
        let value: TokenInfoResponse = from_binary(&res).unwrap();
        assert_eq!(
            TokenInfoResponse {
                name: "item-dao".to_string(),
                symbol: "IDAO".to_string(),
                decimals: 3,
                total_supply: Uint128::from(100000u128),
            },
            value
        );

        let res = query(
            deps.as_ref(),
            mock_env(),
            QueryMsg::Balance {
                address: "awallet".to_string(),
            },
        )
        .unwrap();
        let value: BalanceResponse = from_binary(&res).unwrap();
        assert_eq!(Uint128::from(100000u128), value.balance);

        let res = query(deps.as_ref(), mock_env(), QueryMsg::ListProposals).unwrap();
        let value: Vec<Proposal> = from_binary(&res).unwrap();
        assert_eq!(Vec::<Proposal>::new(), value);
    }

    #[test]
    #[should_panic(expected = "called `Result::unwrap()` on an `Err` value: InvalidQuorum")]
    fn invalid_initialization() {
        let mut deps = mock_dependencies(&[]);

        let msg = InstantiateMsg {
            // Doesn't make sense to require that > 100% of tokens are
            // required for a vote to pass.
            quorum: Uint128::zero(),
            proposal_cost: Uint128::from(1u128),
            token_info: TokenInstantiateInfo {
                name: "item-dao".to_string(),
                symbol: "IDAO".to_string(),
                decimals: 3,
                initial_balances: vec![Cw20Coin {
                    address: "awallet".to_string(),
                    amount: Uint128::from(100000u128),
                }],
            },
        };
        let info = mock_info("creator", &[]);

        instantiate(deps.as_mut(), mock_env(), info, msg).unwrap();
    }

    #[test]
    fn make_proposal() {
        let mut deps = mock_dependencies(&[]);

        let msg = InstantiateMsg {
            quorum: Uint128::from(98u128),
            proposal_cost: Uint128::from(1u128),
            token_info: TokenInstantiateInfo {
                name: "item-dao".to_string(),
                symbol: "IDAO".to_string(),
                decimals: 3,
                initial_balances: vec![Cw20Coin {
                    address: "🦄".to_string(),
                    amount: Uint128::from(100000u128),
                }],
            },
        };
        let info = mock_info("🦄", &[]);

        instantiate(deps.as_mut(), mock_env(), info.clone(), msg).unwrap();

        let proposal = ProposeMsg {
            title: "🦄!".to_string(),
            body: "everyone should use a unicorn emoji for their twitter profile!".to_string(),
            action: ProposeAction::AddItem(DaoItem {
                name: "unicorn emojis must be used for all profile photos".to_string(),
                contents: "unicorn emoji shall be defined as being 🦄".to_string(),
            }),
        };
        execute(
            deps.as_mut(),
            mock_env(),
            info.clone(),
            ExecuteMsg::Propose(proposal.clone()),
        )
        .unwrap();

        let res = query(
            deps.as_ref(),
            mock_env(),
            QueryMsg::GetProposal { proposal_id: 0 },
        )
        .unwrap();

        // This working despire the response to the query being a
        // state::Proposal type and not a ProposalMsg is an
        // interesting quirk.. Good to keep in mind.
        let value: ProposeMsg = from_binary(&res).unwrap();
        assert_eq!(proposal, value);

        let res = query(
            deps.as_ref(),
            mock_env(),
            QueryMsg::Balance {
                address: "🦄".to_string(),
            },
        )
        .unwrap();
        let value: BalanceResponse = from_binary(&res).unwrap();
        assert_eq!(value.balance, Uint128::from(99999u128))
    }

    #[test]
    #[should_panic(
        expected = "called `Result::unwrap()` on an `Err` value: NotFound { kind: \"no such proposal ID (0)\" }"
    )]
    fn invalid_proposal_lookup() {
        let mut deps = mock_dependencies(&[]);

        let msg = InstantiateMsg {
            quorum: Uint128::from(99u128),
            proposal_cost: Uint128::from(1u128),
            token_info: TokenInstantiateInfo {
                name: "item-dao".to_string(),
                symbol: "IDAO".to_string(),
                decimals: 3,
                initial_balances: vec![Cw20Coin {
                    address: "awallet".to_string(),
                    amount: Uint128::from(100000u128),
                }],
            },
        };
        let info = mock_info("🦄", &[]);

        instantiate(deps.as_mut(), mock_env(), info.clone(), msg).unwrap();

        query(
            deps.as_ref(),
            mock_env(),
            QueryMsg::GetProposal { proposal_id: 0 },
        )
        .unwrap();
    }

    #[test]
    #[should_panic(
        expected = "called `Result::unwrap()` on an `Err` value: Std(Overflow { source: OverflowError { operation: Sub, operand1: \"0\", operand2: \"100\" } })"
    )]
    fn insufficent_token_funds() {
        let mut deps = mock_dependencies(&[]);

        let msg = InstantiateMsg {
            // Doesn't make sense to require that > 100% of tokens are
            // required for a vote to pass.
            quorum: Uint128::from(99u128),
            proposal_cost: Uint128::from(100u128),
            token_info: TokenInstantiateInfo {
                name: "item-dao".to_string(),
                symbol: "IDAO".to_string(),
                decimals: 3,
                initial_balances: vec![Cw20Coin {
                    // note that the unicorn wallet doesn't have any
                    // funds.
                    address: "awallet".to_string(),
                    amount: Uint128::from(100000u128),
                }],
            },
        };
        let info = mock_info("🦄", &[]);

        instantiate(deps.as_mut(), mock_env(), info.clone(), msg).unwrap();

        let proposal = ProposeMsg {
            title: "🦄!".to_string(),
            body: "everyone should use a unicorn emoji for their twitter profile!".to_string(),
            action: ProposeAction::AddItem(DaoItem {
                name: "unicorn emojis must be used for all profile photos".to_string(),
                contents: "unicorn emoji shall be defined as being 🦄".to_string(),
            }),
        };

        execute(
            deps.as_mut(),
            mock_env(),
            info.clone(),
            ExecuteMsg::Propose(proposal),
        )
        .unwrap();
    }

    fn setup_near_pass(
        // BIG type
        deps: &mut cosmwasm_std::OwnedDeps<
            cosmwasm_std::testing::MockStorage,
            cosmwasm_std::testing::MockApi,
            cosmwasm_std::testing::MockQuerier,
            cosmwasm_std::Empty,
        >,
        info: MessageInfo,
    ) {
        let msg = InstantiateMsg {
            quorum: Uint128::from(98u128),
            proposal_cost: Uint128::from(1u128),
            token_info: TokenInstantiateInfo {
                name: "item-dao".to_string(),
                symbol: "IDAO".to_string(),
                decimals: 3,
                initial_balances: vec![Cw20Coin {
                    address: "🦄".to_string(),
                    amount: Uint128::from(100000u128),
                }],
            },
        };
        instantiate(deps.as_mut(), mock_env(), info.clone(), msg).unwrap();

        let proposal = ProposeMsg {
            title: "🦄!".to_string(),
            body: "everyone should use a unicorn emoji for their twitter profile!".to_string(),
            action: ProposeAction::AddItem(DaoItem {
                name: "unicorn emojis must be used for all profile photos".to_string(),
                contents: "unicorn emoji shall be defined as being 🦄".to_string(),
            }),
        };
        execute(
            deps.as_mut(),
            mock_env(),
            info.clone(),
            ExecuteMsg::Propose(proposal),
        )
        .unwrap();

        // Send a vote which will not cause the proposal to pass.
        let vote = VoteMsg {
            proposal_id: 0,
            position: crate::msg::VotePosition::Yes,
            amount: Uint128::from(97u128),
        };
        execute(
            deps.as_mut(),
            mock_env(),
            info.clone(),
            ExecuteMsg::Vote(vote),
        )
        .unwrap();

        // Get the proposal and verify that its status is still pending
        let res = query(
            deps.as_ref(),
            mock_env(),
            QueryMsg::GetProposal { proposal_id: 0 },
        )
        .unwrap();
        let prop: Proposal = from_binary(&res).unwrap();
        assert_eq!(prop.status, ProposalStatus::Pending);
        assert_eq!(
            prop.get_votes(crate::msg::VotePosition::Yes),
            Uint128::from(97u128)
        );

        // Also assert that funds have been deducted from the proposer
        // voter's accounts. 1 token for the proposal and 97 for the
        // vote.
        let res = query(
            deps.as_ref(),
            mock_env(),
            QueryMsg::Balance {
                address: "🦄".to_string(),
            },
        )
        .unwrap();
        let value: BalanceResponse = from_binary(&res).unwrap();
        assert_eq!(value.balance, Uint128::from(99902u128));

        // Make sure that the tokens have been correctly sent to the contract.
        let res = query(
            deps.as_ref(),
            mock_env(),
            QueryMsg::Balance {
                address: MOCK_CONTRACT_ADDR.to_string(),
            },
        )
        .unwrap();
        let value: BalanceResponse = from_binary(&res).unwrap();
        assert_eq!(value.balance, Uint128::from(98u128))
    }

    #[test]
    fn vote_yes_yes() {
        let mut deps = mock_dependencies(&[]);
        let info = mock_info("🦄", &[]);

        setup_near_pass(&mut deps, info.clone());

        // Send a yes vote which will cause the proposal to pass.
        let vote = VoteMsg {
            proposal_id: 0,
            position: crate::msg::VotePosition::Yes,
            amount: Uint128::from(1u128),
        };
        execute(
            deps.as_mut(),
            mock_env(),
            info.clone(),
            ExecuteMsg::Vote(vote),
        )
        .unwrap();

        // Get the proposal and verify that its status is passed
        let res = query(
            deps.as_ref(),
            mock_env(),
            QueryMsg::GetProposal { proposal_id: 0 },
        )
        .unwrap();
        let prop: Proposal = from_binary(&res).unwrap();
        assert_eq!(prop.status, ProposalStatus::Passed);
        assert_eq!(
            prop.get_votes(crate::msg::VotePosition::Yes),
            Uint128::from(98u128)
        );

        // Check that tokens have been returned correctly.
        let res = query(
            deps.as_ref(),
            mock_env(),
            QueryMsg::Balance {
                address: "🦄".to_string(),
            },
        )
        .unwrap();
        let balance: BalanceResponse = from_binary(&res).unwrap();
        assert_eq!(Uint128::from(100000u128), balance.balance);
    }

    #[test]
    fn vote_yes_no_pass() {
        let mut deps = mock_dependencies(&[]);
        let info = mock_info("🦄", &[]);

        setup_near_pass(&mut deps, info.clone());

        // Send a no vote which will cause the proposal to pass.
        let vote = VoteMsg {
            proposal_id: 0,
            position: crate::msg::VotePosition::No,
            amount: Uint128::from(1u128),
        };
        execute(
            deps.as_mut(),
            mock_env(),
            info.clone(),
            ExecuteMsg::Vote(vote),
        )
        .unwrap();

        let res = query(
            deps.as_ref(),
            mock_env(),
            QueryMsg::GetProposal { proposal_id: 0 },
        )
        .unwrap();
        let prop: Proposal = from_binary(&res).unwrap();
        assert_eq!(prop.status, ProposalStatus::Passed);
        assert_eq!(
            prop.get_votes(crate::msg::VotePosition::Yes),
            Uint128::from(97u128)
        );

        // Check that tokens have been returned correctly.
        let res = query(
            deps.as_ref(),
            mock_env(),
            QueryMsg::Balance {
                address: "🦄".to_string(),
            },
        )
        .unwrap();
        let balance: BalanceResponse = from_binary(&res).unwrap();
        assert_eq!(Uint128::from(100000u128), balance.balance);
    }

    #[test]
    fn vote_yes_no_fail() {
        let mut deps = mock_dependencies(&[]);
        let info = mock_info("🦄", &[]);

        setup_near_pass(&mut deps, info.clone());

        let vote = VoteMsg {
            proposal_id: 0,
            position: crate::msg::VotePosition::No,
            amount: Uint128::from(97u128),
        };
        execute(
            deps.as_mut(),
            mock_env(),
            info.clone(),
            ExecuteMsg::Vote(vote),
        )
        .unwrap();

        let res = query(
            deps.as_ref(),
            mock_env(),
            QueryMsg::GetProposal { proposal_id: 0 },
        )
        .unwrap();
        let prop: Proposal = from_binary(&res).unwrap();
        assert_eq!(prop.status, ProposalStatus::Failed);
        assert_eq!(
            prop.get_votes(crate::msg::VotePosition::Yes),
            Uint128::from(97u128)
        );
        assert_eq!(
            prop.get_votes(crate::msg::VotePosition::No),
            Uint128::from(97u128)
        );

        // Check that tokens have been returned correctly.
        let res = query(
            deps.as_ref(),
            mock_env(),
            QueryMsg::Balance {
                address: "🦄".to_string(),
            },
        )
        .unwrap();
        let balance: BalanceResponse = from_binary(&res).unwrap();
        assert_eq!(Uint128::from(100000u128), balance.balance);
    }

    #[test]
    fn vote_yes_abstain_pass() {
        let mut deps = mock_dependencies(&[]);
        let info = mock_info("🦄", &[]);

        setup_near_pass(&mut deps, info.clone());

        // Send a no vote which will cause the proposal to pass.
        let vote = VoteMsg {
            proposal_id: 0,
            position: crate::msg::VotePosition::Abstain,
            amount: Uint128::from(1u128),
        };
        execute(
            deps.as_mut(),
            mock_env(),
            info.clone(),
            ExecuteMsg::Vote(vote),
        )
        .unwrap();

        // Get the proposal and verify that its status is passed
        let res = query(
            deps.as_ref(),
            mock_env(),
            QueryMsg::GetProposal { proposal_id: 0 },
        )
        .unwrap();
        let prop: Proposal = from_binary(&res).unwrap();

        assert_eq!(prop.status, ProposalStatus::Passed);
        assert_eq!(
            prop.get_votes(crate::msg::VotePosition::Yes),
            Uint128::from(97u128)
        );
        assert_eq!(
            prop.get_votes(crate::msg::VotePosition::Abstain),
            Uint128::from(1u128)
        );

        // Check that tokens have been returned correctly.
        let res = query(
            deps.as_ref(),
            mock_env(),
            QueryMsg::Balance {
                address: "🦄".to_string(),
            },
        )
        .unwrap();
        let balance: BalanceResponse = from_binary(&res).unwrap();
        assert_eq!(Uint128::from(100000u128), balance.balance);
    }

    #[test]
    fn vote_yes_withdraw() {
        let mut deps = mock_dependencies(&[]);
        let info = mock_info("🦄", &[]);

        setup_near_pass(&mut deps, info.clone());

        // Withdraw all votes.
        let msg = WithdrawVoteMsg { proposal_id: 0 };
        execute(
            deps.as_mut(),
            mock_env(),
            info.clone(),
            ExecuteMsg::Withdraw(msg),
        )
        .unwrap();

        // Check that tokens have been returned correctly.
        let res = query(
            deps.as_ref(),
            mock_env(),
            QueryMsg::Balance {
                address: "🦄".to_string(),
            },
        )
        .unwrap();
        let balance: BalanceResponse = from_binary(&res).unwrap();
        assert_eq!(Uint128::from(99999u128), balance.balance);

        // Get the proposal and verify that its status is pending
        let res = query(
            deps.as_ref(),
            mock_env(),
            QueryMsg::GetProposal { proposal_id: 0 },
        )
        .unwrap();
        let prop: Proposal = from_binary(&res).unwrap();
        assert_eq!(prop.status, ProposalStatus::Pending);
        assert_eq!(
            prop.get_votes(crate::msg::VotePosition::Yes),
            Uint128::zero()
        );
    }

    #[test]
    #[should_panic(
        expected = "called `Result::unwrap()` on an `Err` value: VoteOnCompletedProposal"
    )]
    fn vote_yes_no_pass_withdraw() {
        let mut deps = mock_dependencies(&[]);
        let info = mock_info("🦄", &[]);

        setup_near_pass(&mut deps, info.clone());

        // Send a no vote which will cause the proposal to pass.
        let vote = VoteMsg {
            proposal_id: 0,
            position: crate::msg::VotePosition::No,
            amount: Uint128::from(1u128),
        };
        execute(
            deps.as_mut(),
            mock_env(),
            info.clone(),
            ExecuteMsg::Vote(vote),
        )
        .unwrap();

        let res = query(
            deps.as_ref(),
            mock_env(),
            QueryMsg::GetProposal { proposal_id: 0 },
        )
        .unwrap();
        let prop: Proposal = from_binary(&res).unwrap();
        assert_eq!(prop.status, ProposalStatus::Passed);
        assert_eq!(
            prop.get_votes(crate::msg::VotePosition::Yes),
            Uint128::from(97u128)
        );

        // Check that tokens have been returned correctly.
        let res = query(
            deps.as_ref(),
            mock_env(),
            QueryMsg::Balance {
                address: "🦄".to_string(),
            },
        )
        .unwrap();
        let balance: BalanceResponse = from_binary(&res).unwrap();
        assert_eq!(Uint128::from(100000u128), balance.balance);

        // Withdraw all votes.
        let msg = WithdrawVoteMsg { proposal_id: 0 };
        execute(
            deps.as_mut(),
            mock_env(),
            info.clone(),
            ExecuteMsg::Withdraw(msg),
        )
        .unwrap();
    }

    #[test]
    fn list_proposals() {
        let mut deps = mock_dependencies(&[]);

        let msg = InstantiateMsg {
            // Doesn't make sense to require that > 100% of tokens are
            // required for a vote to pass.
            quorum: Uint128::from(99u128),
            proposal_cost: Uint128::from(100u128),
            token_info: TokenInstantiateInfo {
                name: "item-dao".to_string(),
                symbol: "IDAO".to_string(),
                decimals: 3,
                initial_balances: vec![Cw20Coin {
                    // note that the unicorn wallet doesn't have any
                    // funds.
                    address: "🦄".to_string(),
                    amount: Uint128::from(100000u128),
                }],
            },
        };
        let info = mock_info("🦄", &[]);

        instantiate(deps.as_mut(), mock_env(), info.clone(), msg).unwrap();

        let proposal = ProposeMsg {
            title: "🦄!".to_string(),
            body: "everyone should use a unicorn emoji for their twitter profile!".to_string(),
            action: ProposeAction::AddItem(DaoItem {
                name: "unicorn emojis must be used for all profile photos".to_string(),
                contents: "unicorn emoji shall be defined as being 🦄".to_string(),
            }),
        };

        execute(
            deps.as_mut(),
            mock_env(),
            info.clone(),
            ExecuteMsg::Propose(proposal),
        )
        .unwrap();

        let proposal = ProposeMsg {
            title: "🦄!".to_string(),
            body: "everyone should use a unicorn emoji for their twitter profile!".to_string(),
            action: ProposeAction::AddItem(DaoItem {
                name: "unicorn emojis must be used for all profile photos".to_string(),
                contents: "unicorn emoji shall be defined as being 🦄".to_string(),
            }),
        };

        execute(
            deps.as_mut(),
            mock_env(),
            info.clone(),
            ExecuteMsg::Propose(proposal),
        )
        .unwrap();
        let proposal = ProposeMsg {
            title: "🦄!".to_string(),
            body: "everyone should use a unicorn emoji for their twitter profile!".to_string(),
            action: ProposeAction::AddItem(DaoItem {
                name: "unicorn emojis must be used for all profile photos".to_string(),
                contents: "unicorn emoji shall be defined as being 🦄".to_string(),
            }),
        };

        execute(
            deps.as_mut(),
            mock_env(),
            info.clone(),
            ExecuteMsg::Propose(proposal),
        )
        .unwrap();

        let proposal = ProposeMsg {
            title: "🦄!".to_string(),
            body: "everyone should use a unicorn emoji for their twitter profile!".to_string(),
            action: ProposeAction::AddItem(DaoItem {
                name: "unicorn emojis must be used for all profile photos".to_string(),
                contents: "unicorn emoji shall be defined as being 🦄".to_string(),
            }),
        };

        execute(
            deps.as_mut(),
            mock_env(),
            info.clone(),
            ExecuteMsg::Propose(proposal),
        )
        .unwrap();

        let res = query(deps.as_ref(), mock_env(), QueryMsg::ListProposals).unwrap();
        let value: Vec<Proposal> = from_binary(&res).unwrap();
        assert_eq!(4, value.len());
    }

    #[test]
    fn proposal_actions() {
        let mut deps = mock_dependencies(&[]);
        let info = mock_info("🦄", &[]);
        let msg = InstantiateMsg {
            quorum: Uint128::from(98u128),
            proposal_cost: Uint128::from(1u128),
            token_info: TokenInstantiateInfo {
                name: "item-dao".to_string(),
                symbol: "IDAO".to_string(),
                decimals: 3,
                initial_balances: vec![Cw20Coin {
                    address: "🦄".to_string(),
                    amount: Uint128::from(100000u128),
                }],
            },
        };
        instantiate(deps.as_mut(), mock_env(), info.clone(), msg).unwrap();

        let proposal = ProposeMsg {
            title: "🦄!".to_string(),
            body: "everyone should use a unicorn emoji for their twitter profile!".to_string(),
            action: ProposeAction::AddItem(DaoItem {
                name: "unicorn emojis must be used for all profile photos".to_string(),
                contents: "unicorn emoji shall be defined as being 🦄".to_string(),
            }),
        };
        execute(
            deps.as_mut(),
            mock_env(),
            info.clone(),
            ExecuteMsg::Propose(proposal),
        )
        .unwrap();
        // Send a yes vote which will cause the proposal to pass.
        let vote = VoteMsg {
            proposal_id: 0,
            position: crate::msg::VotePosition::Yes,
            amount: Uint128::from(100u128),
        };
        execute(
            deps.as_mut(),
            mock_env(),
            info.clone(),
            ExecuteMsg::Vote(vote),
        )
        .unwrap();

        let proposal = ProposeMsg {
            title: "🐮!".to_string(),
            body: "everyone should use a cow emoji for their twitter profile!".to_string(),
            action: ProposeAction::AddItem(DaoItem {
                name: "cow emojis must be used for all profile photos".to_string(),
                contents: "cow emoji shall be defined as being 🐮".to_string(),
            }),
        };
        execute(
            deps.as_mut(),
            mock_env(),
            info.clone(),
            ExecuteMsg::Propose(proposal),
        )
        .unwrap();
        // Send a yes vote which will cause the proposal to pass.
        let vote = VoteMsg {
            proposal_id: 1,
            position: crate::msg::VotePosition::Yes,
            amount: Uint128::from(100u128),
        };
        execute(
            deps.as_mut(),
            mock_env(),
            info.clone(),
            ExecuteMsg::Vote(vote),
        )
        .unwrap();

        let items = query(deps.as_ref(), mock_env(), QueryMsg::ListItems).unwrap();
        let items: Vec<DaoItem> = from_binary(&items).unwrap();
        assert_eq!(items.len(), 2);
        assert_eq!(
            items[0].name,
            "unicorn emojis must be used for all profile photos".to_string()
        );
        assert_eq!(
            items[1].contents,
            "cow emoji shall be defined as being 🐮".to_string()
        );

        let unicorn = query(deps.as_ref(), mock_env(), QueryMsg::GetItem { item_id: 0 }).unwrap();
        let unicorn: DaoItem = from_binary(&unicorn).unwrap();
        assert_eq!(
            unicorn.name,
            "unicorn emojis must be used for all profile photos"
        );
        assert_eq!(
            unicorn.contents,
            "unicorn emoji shall be defined as being 🦄".to_string()
        );

        let proposal = ProposeMsg {
            title: "🦄!".to_string(),
            body: "everyone should not use a unicorn emoji for their twitter profile!".to_string(),
            action: ProposeAction::RemoveItem { id: 0 },
        };
        execute(
            deps.as_mut(),
            mock_env(),
            info.clone(),
            ExecuteMsg::Propose(proposal),
        )
        .unwrap();
        // Send a yes vote which will cause the proposal to pass.
        let vote = VoteMsg {
            proposal_id: 2,
            position: crate::msg::VotePosition::Yes,
            amount: Uint128::from(100u128),
        };
        execute(
            deps.as_mut(),
            mock_env(),
            info.clone(),
            ExecuteMsg::Vote(vote),
        )
        .unwrap();

        let items = query(deps.as_ref(), mock_env(), QueryMsg::ListItems).unwrap();
        let items: Vec<DaoItem> = from_binary(&items).unwrap();
        assert_eq!(items.len(), 1);
        assert_eq!(
            items[0].name,
            "cow emojis must be used for all profile photos".to_string()
        );
        assert_eq!(
            items[0].contents,
            "cow emoji shall be defined as being 🐮".to_string()
        );

        let cow = query(deps.as_ref(), mock_env(), QueryMsg::GetItem { item_id: 0 }).unwrap();
        let cow: DaoItem = from_binary(&cow).unwrap();
        assert_eq!(cow.name, "cow emojis must be used for all profile photos");
        assert_eq!(
            cow.contents,
            "cow emoji shall be defined as being 🐮".to_string()
        );

        let proposal = ProposeMsg {
            title: "change the quorum to 1000".to_string(),
            body: "this will mean more people have to vote which is good".to_string(),
            action: ProposeAction::ChangeQuorum {
                new_quorum: Uint128::from(1000u128),
            },
        };
        execute(
            deps.as_mut(),
            mock_env(),
            info.clone(),
            ExecuteMsg::Propose(proposal),
        )
        .unwrap();
        // Send a yes vote which will cause the proposal to pass.
        let vote = VoteMsg {
            proposal_id: 3,
            position: crate::msg::VotePosition::Yes,
            amount: Uint128::from(100u128),
        };
        execute(
            deps.as_mut(),
            mock_env(),
            info.clone(),
            ExecuteMsg::Vote(vote),
        )
        .unwrap();

        let res = query(deps.as_ref(), mock_env(), QueryMsg::GetQuorum {}).unwrap();
        let value: Uint128 = from_binary(&res).unwrap();
        assert_eq!(Uint128::from(1000u128), value);

        let proposal = ProposeMsg {
            title: "change the proposal cost to 1000".to_string(),
            body: "fewer people can submit proposals which is good".to_string(),
            action: ProposeAction::ChangeProposalCost {
                new_proposal_cost: Uint128::from(1000u128),
            },
        };
        execute(
            deps.as_mut(),
            mock_env(),
            info.clone(),
            ExecuteMsg::Propose(proposal),
        )
        .unwrap();
        // Send a yes vote which will cause the proposal to pass.
        let vote = VoteMsg {
            proposal_id: 4,
            position: crate::msg::VotePosition::Yes,
            // Need to vote more because the quorum has changed!
            amount: Uint128::from(1000u128),
        };
        execute(
            deps.as_mut(),
            mock_env(),
            info.clone(),
            ExecuteMsg::Vote(vote),
        )
        .unwrap();

        let res = query(deps.as_ref(), mock_env(), QueryMsg::GetProposalCost {}).unwrap();
        let value: Uint128 = from_binary(&res).unwrap();
        assert_eq!(Uint128::from(1000u128), value);
    }

    #[test]
    fn token_queries() {
        let mut deps = mock_dependencies(&[]);
        let info = mock_info("🦄", &[]);

        let msg = InstantiateMsg {
            quorum: Uint128::from(98u128),
            proposal_cost: Uint128::from(1u128),
            token_info: TokenInstantiateInfo {
                name: "item-dao".to_string(),
                symbol: "IDAO".to_string(),
                decimals: 3,
                initial_balances: vec![Cw20Coin {
                    address: "awallet".to_string(),
                    amount: Uint128::from(100000u128),
                }],
            },
        };
        instantiate(deps.as_mut(), mock_env(), info.clone(), msg).unwrap();

        // Check that the correct balance has been assigned
        let res = query(
            deps.as_ref(),
            mock_env(),
            QueryMsg::Balance {
                address: "awallet".to_string(),
            },
        )
        .unwrap();
        let balance: BalanceResponse = from_binary(&res).unwrap();
        assert_eq!(Uint128::from(100000u128), balance.balance);

        // Check that a query for an address with no tokens returns 0
        let res = query(
            deps.as_ref(),
            mock_env(),
            QueryMsg::Balance {
                address: "notawallet".to_string(),
            },
        )
        .unwrap();
        let balance: BalanceResponse = from_binary(&res).unwrap();
        assert_eq!(Uint128::zero(), balance.balance);
    }

    fn get_balance<T: Into<String>>(deps: Deps, address: T) -> Uint128 {
        tokens::query_balance(deps, address.into()).unwrap().balance
    }

    #[test]
    fn token_transfer() {
        let addr1 = String::from("addr0001");
        let addr2 = String::from("addr0002");
        let amount1 = Uint128::from(12340000u128);
        let transfer = Uint128::from(76543u128);
        let too_much = Uint128::from(12340321u128);

        let mut deps = mock_dependencies(&[]);
        let info = mock_info("🦄", &[]);

        let msg = InstantiateMsg {
            quorum: Uint128::from(98u128),
            proposal_cost: Uint128::from(1u128),
            token_info: TokenInstantiateInfo {
                name: "item-dao".to_string(),
                symbol: "IDAO".to_string(),
                decimals: 3,
                initial_balances: vec![Cw20Coin {
                    address: addr1.clone(),
                    amount: amount1,
                }],
            },
        };
        instantiate(deps.as_mut(), mock_env(), info.clone(), msg).unwrap();

        // cannot transfer nothing
        let info = mock_info(addr1.as_ref(), &[]);
        let env = mock_env();
        let msg = ExecuteMsg::Transfer {
            recipient: addr2.clone(),
            amount: Uint128::zero(),
        };
        let err = execute(deps.as_mut(), env, info, msg).unwrap_err();
        assert_eq!(err, ContractError::InvalidZeroAmount {});

        // cannot send more than we have
        let info = mock_info(addr1.as_ref(), &[]);
        let env = mock_env();
        let msg = ExecuteMsg::Transfer {
            recipient: addr2.clone(),
            amount: too_much,
        };
        let err = execute(deps.as_mut(), env, info, msg).unwrap_err();
        assert!(matches!(err, ContractError::Std(StdError::Overflow { .. })));

        // cannot send from empty account
        let info = mock_info(addr2.as_ref(), &[]);
        let env = mock_env();
        let msg = ExecuteMsg::Transfer {
            recipient: addr1.clone(),
            amount: transfer,
        };
        let err = execute(deps.as_mut(), env, info, msg).unwrap_err();
        assert!(matches!(err, ContractError::Std(StdError::Overflow { .. })));

        // valid transfer
        let info = mock_info(addr1.as_ref(), &[]);
        let env = mock_env();
        let msg = ExecuteMsg::Transfer {
            recipient: addr2.clone(),
            amount: transfer,
        };
        let res = execute(deps.as_mut(), env, info, msg).unwrap();
        assert_eq!(res.messages.len(), 0);

        let remainder = amount1.checked_sub(transfer).unwrap();
        assert_eq!(get_balance(deps.as_ref(), addr1), remainder);
        assert_eq!(get_balance(deps.as_ref(), addr2), transfer);
        assert_eq!(
            tokens::query_token_info(deps.as_ref())
                .unwrap()
                .total_supply,
            amount1
        );
    }

    #[test]
    fn token_send() {
        let addr1 = String::from("addr0001");
        let contract = String::from("addr0002");
        let amount1 = Uint128::from(12340000u128);
        let transfer = Uint128::from(76543u128);
        let too_much = Uint128::from(12340321u128);
        let send_msg = Binary::from(r#"{"some":123}"#.as_bytes());

        let mut deps = mock_dependencies(&[]);
        let info = mock_info("🦄", &[]);

        let msg = InstantiateMsg {
            quorum: Uint128::from(98u128),
            proposal_cost: Uint128::from(1u128),
            token_info: TokenInstantiateInfo {
                name: "item-dao".to_string(),
                symbol: "IDAO".to_string(),
                decimals: 3,
                initial_balances: vec![Cw20Coin {
                    address: addr1.clone(),
                    amount: amount1,
                }],
            },
        };
        instantiate(deps.as_mut(), mock_env(), info.clone(), msg).unwrap();

        // cannot send nothing
        let info = mock_info(addr1.as_ref(), &[]);
        let env = mock_env();
        let msg = ExecuteMsg::Send {
            contract: contract.clone(),
            amount: Uint128::zero(),
            msg: send_msg.clone(),
        };
        let err = execute(deps.as_mut(), env, info, msg).unwrap_err();
        assert_eq!(err, ContractError::InvalidZeroAmount {});

        // cannot send more than we have
        let info = mock_info(addr1.as_ref(), &[]);
        let env = mock_env();
        let msg = ExecuteMsg::Send {
            contract: contract.clone(),
            amount: too_much,
            msg: send_msg.clone(),
        };
        let err = execute(deps.as_mut(), env, info, msg).unwrap_err();
        assert!(matches!(err, ContractError::Std(StdError::Overflow { .. })));

        // valid transfer
        let info = mock_info(addr1.as_ref(), &[]);
        let env = mock_env();
        let msg = ExecuteMsg::Send {
            contract: contract.clone(),
            amount: transfer,
            msg: send_msg.clone(),
        };
        let res = execute(deps.as_mut(), env, info, msg).unwrap();
        assert_eq!(res.messages.len(), 1);

        // ensure proper send message sent
        // this is the message we want delivered to the other side
        let binary_msg = Cw20ReceiveMsg {
            sender: addr1.clone(),
            amount: transfer,
            msg: send_msg,
        }
        .into_binary()
        .unwrap();
        // and this is how it must be wrapped for the vm to process it
        assert_eq!(
            res.messages[0],
            SubMsg::new(CosmosMsg::Wasm(WasmMsg::Execute {
                contract_addr: contract.clone(),
                msg: binary_msg,
                funds: vec![],
            }))
        );

        // ensure balance is properly transferred
        let remainder = amount1.checked_sub(transfer).unwrap();
        assert_eq!(get_balance(deps.as_ref(), addr1), remainder);
        assert_eq!(get_balance(deps.as_ref(), contract), transfer);
        assert_eq!(
            tokens::query_token_info(deps.as_ref())
                .unwrap()
                .total_supply,
            amount1
        );
    }

    #[test]
    fn token_burn() {
        let addr1 = String::from("addr0001");
        let amount1 = Uint128::from(12340000u128);
        let burn = Uint128::from(76543u128);
        let too_much = Uint128::from(12340321u128);

        let mut deps = mock_dependencies(&[]);
        let info = mock_info("🦄", &[]);

        let msg = InstantiateMsg {
            quorum: Uint128::from(98u128),
            proposal_cost: Uint128::from(1u128),
            token_info: TokenInstantiateInfo {
                name: "item-dao".to_string(),
                symbol: "IDAO".to_string(),
                decimals: 3,
                initial_balances: vec![Cw20Coin {
                    address: addr1.clone(),
                    amount: amount1,
                }],
            },
        };
        instantiate(deps.as_mut(), mock_env(), info.clone(), msg).unwrap();

        // cannot burn nothing
        let info = mock_info(addr1.as_ref(), &[]);
        let env = mock_env();
        let msg = ExecuteMsg::Burn {
            amount: Uint128::zero(),
        };
        let err = execute(deps.as_mut(), env, info, msg).unwrap_err();
        assert_eq!(err, ContractError::InvalidZeroAmount {});
        assert_eq!(
            tokens::query_token_info(deps.as_ref())
                .unwrap()
                .total_supply,
            amount1
        );

        // cannot burn more than we have
        let info = mock_info(addr1.as_ref(), &[]);
        let env = mock_env();
        let msg = ExecuteMsg::Burn { amount: too_much };
        let err = execute(deps.as_mut(), env, info, msg).unwrap_err();
        assert!(matches!(err, ContractError::Std(StdError::Overflow { .. })));
        assert_eq!(
            tokens::query_token_info(deps.as_ref())
                .unwrap()
                .total_supply,
            amount1
        );

        // valid burn reduces total supply
        let info = mock_info(addr1.as_ref(), &[]);
        let env = mock_env();
        let msg = ExecuteMsg::Burn { amount: burn };
        let res = execute(deps.as_mut(), env, info, msg).unwrap();
        assert_eq!(res.messages.len(), 0);

        let remainder = amount1.checked_sub(burn).unwrap();
        assert_eq!(get_balance(deps.as_ref(), addr1), remainder);
        assert_eq!(
            tokens::query_token_info(deps.as_ref())
                .unwrap()
                .total_supply,
            remainder
        );
    }
}
