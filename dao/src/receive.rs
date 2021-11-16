use crate::msg::{TokenMsg, VotePosition};
use crate::state::{Proposal, ProposalStatus, PROPOSALS, STATE};
use crate::ContractError;
use cosmwasm_std::{from_binary, DepsMut, Response, StdError};
use cw20::Cw20ReceiveMsg;
use std::cmp::Ordering;

// TODO(zeke):
//
// 1. Handle voting for a proposal for which voting has already
//    concluded.
// 2. Refund tokens to voters once proposal voting concludes.
pub(crate) fn handle_receive(
    deps: DepsMut,
    msg: Cw20ReceiveMsg,
) -> Result<Response, ContractError> {
    let amount = msg.amount;
    let msg: TokenMsg = from_binary(&msg.msg)?;

    match msg {
        TokenMsg::Propose(p) => {
            let state = STATE.load(deps.storage)?;
            if amount < state.proposal_cost {
                Err(ContractError::InsufficentProposalFunds {
                    needed: state.proposal_cost,
                    got: amount,
                })
            } else {
                PROPOSALS.update(deps.storage, |mut proposals| -> Result<_, ContractError> {
                    proposals.push(Proposal::from(p.clone()));
                    Ok(proposals)
                })?;
                Ok(Response::new()
                    .add_attribute("method", "propose")
                    .add_attribute("title", p.title)
                    .add_attribute("body", p.body)
                    .add_attribute("action", format!("{:?}", p.action)))
            }
        }
        TokenMsg::Vote(vote) => {
            let state = STATE.load(deps.storage)?;

            PROPOSALS.update(deps.storage, |mut proposals| -> Result<_, ContractError> {
                match proposals.get_mut(vote.proposal_id as usize) {
                    Some(proposal) => {
                        match vote.position {
                            VotePosition::Yes => proposal.yes += amount,
                            VotePosition::No => proposal.no += amount,
                            VotePosition::Abstain => proposal.abstain += amount,
                        }

                        let staked = proposal.yes + proposal.no + proposal.abstain;
                        if staked >= state.quorum {
                            match proposal.yes.cmp(&proposal.no) {
                                Ordering::Less | Ordering::Equal => {
                                    proposal.status = ProposalStatus::Failed
                                }
                                Ordering::Greater => proposal.status = ProposalStatus::Passed,
                            }
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
            Ok(Response::new()
                .add_attribute("method", "vote")
                .add_attribute("proposal", vote.proposal_id.to_string())
                .add_attribute("tokens", amount))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use cosmwasm_std::testing::mock_dependencies;

    #[test]
    #[should_panic]
    fn handle_receive_bad_msg() {
        let mut deps = mock_dependencies(&[]);

        let msg = Cw20ReceiveMsg {
            sender: "🦄".to_string(),
            amount: cosmwasm_std::Uint128::new(1),
            msg: cosmwasm_std::Binary::from(&[]),
        };

        handle_receive(deps.as_mut(), msg).unwrap();
    }
}
