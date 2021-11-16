#[cfg(not(feature = "library"))]
use cosmwasm_std::entry_point;
use cosmwasm_std::{Binary, Deps, DepsMut, Env, MessageInfo, Response, StdError, StdResult};
use cw2::set_contract_version;

use crate::error::ContractError;
use crate::msg::{ExecuteMsg, InstantiateMsg, QueryMsg};
use crate::state::{State, PROPOSALS, STATE};

// version info for migration info
const CONTRACT_NAME: &str = "webdao";
const CONTRACT_VERSION: &str = env!("CARGO_PKG_VERSION");

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn instantiate(
    deps: DepsMut,
    _env: Env,
    _info: MessageInfo,
    msg: InstantiateMsg,
) -> Result<Response, ContractError> {
    if msg.quorum.is_zero() {
        return Err(ContractError::InvalidQuorum);
    }

    set_contract_version(deps.storage, CONTRACT_NAME, CONTRACT_VERSION)?;

    let state = State {
        quorum: msg.quorum,
        proposal_cost: msg.proposal_cost,
    };
    STATE.save(deps.storage, &state)?;

    let proposals = vec![];
    PROPOSALS.save(deps.storage, &proposals)?;

    Ok(Response::new()
        .add_attribute("method", "instantiate")
        .add_attribute("quorum", msg.quorum.to_string())
        .add_attribute("proposal_cost", msg.proposal_cost.to_string()))
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn execute(
    deps: DepsMut,
    _env: Env,
    _info: MessageInfo,
    msg: ExecuteMsg,
) -> Result<Response, ContractError> {
    match msg {
        ExecuteMsg::Withdraw(_w) => todo!(),
        ExecuteMsg::Receive(r) => crate::receive::handle_receive(deps, r),
    }
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn query(deps: Deps, _env: Env, msg: QueryMsg) -> StdResult<Binary> {
    match msg {
        QueryMsg::ListProposals(_) => todo!(),
        QueryMsg::GetProposal { proposal_id } => {
            let proposals = PROPOSALS.load(deps.storage)?;
            Ok(cosmwasm_std::to_binary(
                proposals
                    .get(proposal_id as usize)
                    .ok_or(StdError::NotFound {
                        kind: format!("no such proposal ID ({})", proposal_id),
                    })?,
            )?)
        }
        QueryMsg::GetQuorum => {
            let state = STATE.load(deps.storage)?;
            Ok(cosmwasm_std::to_binary(&state.quorum)?)
        }
        QueryMsg::GetProposalCost => {
            let state = STATE.load(deps.storage)?;
            Ok(cosmwasm_std::to_binary(&state.proposal_cost)?)
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::msg::{ProposeAction, ProposeMsg, TokenMsg, VoteMsg, WebItem};
    use crate::state::{Proposal, ProposalStatus};

    use super::*;
    use cosmwasm_std::testing::{mock_dependencies, mock_env, mock_info};
    use cosmwasm_std::{from_binary, to_binary, Uint128};
    use cw20::Cw20ReceiveMsg;

    #[test]
    fn valid_initialization() {
        let mut deps = mock_dependencies(&[]);

        let msg = InstantiateMsg {
            quorum: Uint128::from(30u128),
            proposal_cost: Uint128::from(1u128),
        };
        let info = mock_info("creator", &[]);

        // we can just call .unwrap() to assert this was a success
        let res = instantiate(deps.as_mut(), mock_env(), info, msg).unwrap();
        assert_eq!(0, res.messages.len());

        // it worked, let's query the state
        let res = query(deps.as_ref(), mock_env(), QueryMsg::GetQuorum {}).unwrap();
        let value: Uint128 = from_binary(&res).unwrap();
        assert_eq!(Uint128::from(30u128), value);

        let res = query(deps.as_ref(), mock_env(), QueryMsg::GetProposalCost {}).unwrap();
        let value: Uint128 = from_binary(&res).unwrap();
        assert_eq!(Uint128::from(1u128), value);
    }

    #[test]
    #[should_panic]
    fn invalid_initialization() {
        let mut deps = mock_dependencies(&[]);

        let msg = InstantiateMsg {
            // Doesn't make sense to require that > 100% of tokens are
            // required for a vote to pass.
            quorum: Uint128::zero(),
            proposal_cost: Uint128::from(1u128),
        };
        let info = mock_info("creator", &[]);

        instantiate(deps.as_mut(), mock_env(), info, msg).unwrap();
    }

    #[test]
    fn cw20_receive() {
        let mut deps = mock_dependencies(&[]);

        let msg = InstantiateMsg {
            quorum: Uint128::from(98u128),
            proposal_cost: Uint128::from(1u128),
        };
        let info = mock_info("🦄", &[]);

        instantiate(deps.as_mut(), mock_env(), info.clone(), msg).unwrap();

        let proposal = ProposeMsg {
            title: "🦄!".to_string(),
            body: "everyone should use a unicorn emoji for their twitter profile!".to_string(),
            action: ProposeAction::AddItem(WebItem {
                name: "unicorn emojis must be used for all profile photos".to_string(),
                contents: "unicorn emoji shall be defined as being 🦄".to_string(),
            }),
        };

        let msg = to_binary(&TokenMsg::Propose(proposal.clone())).unwrap();

        execute(
            deps.as_mut(),
            mock_env(),
            info.clone(),
            ExecuteMsg::Receive(Cw20ReceiveMsg {
                sender: "🦄".to_string(),
                amount: cosmwasm_std::Uint128::new(1),
                msg,
            }),
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
    }

    #[test]
    #[should_panic]
    fn invalid_proposal_lookup() {
        let mut deps = mock_dependencies(&[]);

        let msg = InstantiateMsg {
            quorum: Uint128::from(99u128),
            proposal_cost: Uint128::from(1u128),
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
    #[should_panic]
    fn cw20_receive_insufficent_funds() {
        let mut deps = mock_dependencies(&[]);

        let msg = InstantiateMsg {
            // Doesn't make sense to require that > 100% of tokens are
            // required for a vote to pass.
            quorum: Uint128::from(99u128),
            proposal_cost: Uint128::from(100u128),
        };
        let info = mock_info("🦄", &[]);

        instantiate(deps.as_mut(), mock_env(), info.clone(), msg).unwrap();

        let proposal = ProposeMsg {
            title: "🦄!".to_string(),
            body: "everyone should use a unicorn emoji for their twitter profile!".to_string(),
            action: ProposeAction::AddItem(WebItem {
                name: "unicorn emojis must be used for all profile photos".to_string(),
                contents: "unicorn emoji shall be defined as being 🦄".to_string(),
            }),
        };

        let msg = to_binary(&TokenMsg::Propose(proposal.clone())).unwrap();

        execute(
            deps.as_mut(),
            mock_env(),
            info.clone(),
            ExecuteMsg::Receive(Cw20ReceiveMsg {
                sender: "🦄".to_string(),
                amount: cosmwasm_std::Uint128::new(99),
                msg,
            }),
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
        };
        instantiate(deps.as_mut(), mock_env(), info.clone(), msg).unwrap();

        let proposal = ProposeMsg {
            title: "🦄!".to_string(),
            body: "everyone should use a unicorn emoji for their twitter profile!".to_string(),
            action: ProposeAction::AddItem(WebItem {
                name: "unicorn emojis must be used for all profile photos".to_string(),
                contents: "unicorn emoji shall be defined as being 🦄".to_string(),
            }),
        };
        let msg = to_binary(&TokenMsg::Propose(proposal)).unwrap();
        execute(
            deps.as_mut(),
            mock_env(),
            info.clone(),
            ExecuteMsg::Receive(Cw20ReceiveMsg {
                sender: "🦄".to_string(),
                amount: cosmwasm_std::Uint128::new(1),
                msg,
            }),
        )
        .unwrap();

        // Send a vote which will not cause the proposal to pass.
        let vote = VoteMsg {
            proposal_id: 0,
            position: crate::msg::VotePosition::Yes,
        };
        let msg = to_binary(&TokenMsg::Vote(vote)).unwrap();
        execute(
            deps.as_mut(),
            mock_env(),
            info.clone(),
            ExecuteMsg::Receive(Cw20ReceiveMsg {
                sender: "🦄".to_string(),
                amount: Uint128::from(97u128), // one away from quorum requirement
                msg,
            }),
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
        assert_eq!(prop.yes, Uint128::from(97u128));
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
        };
        let msg = to_binary(&TokenMsg::Vote(vote)).unwrap();
        execute(
            deps.as_mut(),
            mock_env(),
            info.clone(),
            ExecuteMsg::Receive(Cw20ReceiveMsg {
                sender: "🦄".to_string(),
                amount: Uint128::from(1u128), // meets quorum requirement exactly
                msg,
            }),
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
        assert_eq!(prop.yes, Uint128::from(98u128));
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
        };
        let msg = to_binary(&TokenMsg::Vote(vote)).unwrap();
        execute(
            deps.as_mut(),
            mock_env(),
            info.clone(),
            ExecuteMsg::Receive(Cw20ReceiveMsg {
                sender: "🦄".to_string(),
                amount: Uint128::from(1u128), // meets quorum requirement exactly
                msg,
            }),
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
        assert_eq!(prop.yes, Uint128::from(97u128));
    }

    #[test]
    fn vote_yes_no_fail() {
        let mut deps = mock_dependencies(&[]);
        let info = mock_info("🦄", &[]);

        setup_near_pass(&mut deps, info.clone());

        let vote = VoteMsg {
            proposal_id: 0,
            position: crate::msg::VotePosition::No,
        };
        let msg = to_binary(&TokenMsg::Vote(vote)).unwrap();
        execute(
            deps.as_mut(),
            mock_env(),
            info.clone(),
            ExecuteMsg::Receive(Cw20ReceiveMsg {
                sender: "🦄".to_string(),
                // ties yes votes which causes the proposal to fail.
                amount: Uint128::from(97u128),
                msg,
            }),
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
        assert_eq!(prop.yes, Uint128::from(97u128));
        assert_eq!(prop.no, Uint128::from(97u128));
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
        };
        let msg = to_binary(&TokenMsg::Vote(vote)).unwrap();
        execute(
            deps.as_mut(),
            mock_env(),
            info.clone(),
            ExecuteMsg::Receive(Cw20ReceiveMsg {
                sender: "🦄".to_string(),
                // ties yes votes which causes the proposal to fail.
                amount: Uint128::from(1u128),
                msg,
            }),
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
        assert_eq!(prop.yes, Uint128::from(97u128));
        assert_eq!(prop.abstain, Uint128::from(1u128));
    }
}
