use crate::msg::{self, ProposeMsg, VoteMsg, VotePosition, WithdrawVoteMsg};
use crate::state::{Proposal, ProposalStatus, ITEMS, PROPOSALS, STATE};
use crate::tokens;
use crate::ContractError;
use cosmwasm_std::{DepsMut, Env, MessageInfo, Response, StdError, Uint128};
use msg::ProposeAction;
use std::cmp::Ordering;

pub(crate) fn handle_propose(
    mut deps: DepsMut,
    env: Env,
    info: MessageInfo,
    proposal: ProposeMsg,
) -> Result<Response, ContractError> {
    let state = STATE.load(deps.storage)?;
    let cost = state.proposal_cost;

    let contract_addr = env.contract.address.to_string();
    // Transfer the proposal cost to this contract. If this fails the
    // program will bail out.
    tokens::execute_transfer(deps.branch(), env, info.clone(), contract_addr, cost)?;

    PROPOSALS.update(deps.storage, |mut proposals| -> Result<_, ContractError> {
        proposals.push(Proposal::new(proposal.clone(), info.sender, cost));
        Ok(proposals)
    })?;

    Ok(Response::new()
        .add_attribute("method", "propose")
        .add_attribute("title", proposal.title)
        .add_attribute("body", proposal.body)
        .add_attribute("action", format!("{:?}", proposal.action)))
}

pub(crate) fn handle_vote(
    mut deps: DepsMut,
    env: Env,
    info: MessageInfo,
    vote: VoteMsg,
) -> Result<Response, ContractError> {
    let state = STATE.load(deps.storage)?;

    let contract_addr = env.contract.address.as_str().to_string();
    // Transfer the vote stake amount to this contract. If this fails the
    // program will bail out. This will fail if amount is zero.
    tokens::execute_transfer(
        deps.branch(),
        env.clone(),
        info.clone(),
        contract_addr,
        vote.amount,
    )?;

    // Need to do this because the compiler doesn't want to capture
    // vote by reference in the closure below for some reason.
    let proposal_id = vote.proposal_id;
    let amount = vote.amount;

    let proposals =
        PROPOSALS.update(deps.storage, |mut proposals| -> Result<_, ContractError> {
            match proposals.get_mut(vote.proposal_id) {
                Some(proposal) => {
                    if proposal.status != ProposalStatus::Pending {
                        return Err(ContractError::VoteOnCompletedProposal);
                    }

                    proposal.add_vote(&info.sender, vote.position, vote.amount);

                    let staked = proposal.get_total_votes();
                    if staked >= state.quorum {
                        match proposal
                            .get_votes(VotePosition::Yes)
                            .cmp(&proposal.get_votes(VotePosition::No))
                        {
                            Ordering::Less | Ordering::Equal => {
                                proposal.status = ProposalStatus::Failed
                            }
                            Ordering::Greater => proposal.status = ProposalStatus::Passed,
                        };
                    }
                }
                None => {
                    return Err(ContractError::Std(StdError::NotFound {
                        kind: format!("no such proposal ID ({})", vote.proposal_id),
                    }))
                }
            }
            Ok(proposals)
        })?;

    if proposals[proposal_id].status != ProposalStatus::Pending {
        handle_proposal_completion(deps, env, info, &proposals[proposal_id])?;
    }

    Ok(Response::new()
        .add_attribute("method", "vote")
        .add_attribute("proposal", proposal_id.to_string())
        .add_attribute("tokens", amount))
}

/// On proposal completion the submitter of the proposal and all of
/// the voters ought to have their tokens returned.
fn handle_proposal_completion(
    mut deps: DepsMut,
    env: Env,
    _info: MessageInfo,
    proposal: &Proposal,
) -> Result<(), ContractError> {
    assert!(proposal.status != ProposalStatus::Pending);

    match &proposal.action {
        ProposeAction::ChangeQuorum { new_quorum } => {
            STATE.update(deps.storage, |mut state| -> Result<_, ContractError> {
                state.quorum = *new_quorum;
                Ok(state)
            })?;
        }
        ProposeAction::ChangeProposalCost { new_proposal_cost } => {
            STATE.update(deps.storage, |mut state| -> Result<_, ContractError> {
                state.proposal_cost = *new_proposal_cost;
                Ok(state)
            })?;
        }
        ProposeAction::AddItem(item) => {
            ITEMS.update(deps.storage, |mut items| -> Result<_, ContractError> {
                items.push(item.clone());
                Ok(items)
            })?;
        }
        ProposeAction::RemoveItem { id } => {
            ITEMS.update(deps.storage, |mut items| -> Result<_, ContractError> {
                items.remove(*id);
                Ok(items)
            })?;
        }
    }

    // Refund the proposer.
    tokens::execute_transfer(
        deps.branch(),
        env.clone(),
        MessageInfo {
            sender: env.contract.address.clone(),
            funds: vec![],
        },
        proposal.proposer.to_string(),
        proposal.proposal_cost,
    )?;

    // Refund the voters.
    for (addr, amount) in proposal
        .yes
        .iter()
        .chain(proposal.no.iter().chain(proposal.abstain.iter()))
    {
        tokens::execute_transfer(
            deps.branch(),
            env.clone(),
            MessageInfo {
                sender: env.contract.address.clone(),
                funds: vec![],
            },
            addr.to_string(),
            *amount,
        )?;
    }

    Ok(())
}

pub(crate) fn handle_withdrawal(
    mut deps: DepsMut,
    env: Env,
    info: MessageInfo,
    msg: WithdrawVoteMsg,
) -> Result<Response, ContractError> {
    let mut withdrawn = Uint128::zero();
    PROPOSALS.update(deps.storage, |mut proposals| -> Result<_, ContractError> {
        match proposals.get_mut(msg.proposal_id) {
            Some(proposal) => {
                if proposal.status != ProposalStatus::Pending {
                    return Err(ContractError::VoteOnCompletedProposal);
                }

                for (addr, amount) in proposal
                    .yes
                    .iter_mut()
                    .chain(proposal.no.iter_mut().chain(proposal.abstain.iter_mut()))
                {
                    if *addr == info.sender {
                        withdrawn += *amount;
                        *amount = Uint128::zero();
                    }
                }
                Ok(proposals)
            }
            None => Err(ContractError::Std(StdError::NotFound {
                kind: format!("no such proposal ID ({})", msg.proposal_id),
            })),
        }
    })?;
    tokens::execute_transfer(
        deps.branch(),
        env.clone(),
        MessageInfo {
            sender: env.contract.address.clone(),
            funds: vec![],
        },
        info.sender.to_string(),
        withdrawn,
    )
}
