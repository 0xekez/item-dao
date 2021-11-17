use cosmwasm_std::{StdError, Uint128};
use thiserror::Error;

#[derive(Error, Debug, PartialEq)]
pub enum ContractError {
    #[error("{0}")]
    Std(#[from] StdError),

    #[error("Unauthorized")]
    Unauthorized {},

    #[error("Quorum must be greater than zero and not greater than total token supply")]
    InvalidQuorum,

    #[error("Insufficent funds for proposal. Needed ({needed}), got ({got})")]
    InsufficentProposalFunds { needed: Uint128, got: Uint128 },

    #[error("Can not transfer or send zero tokens")]
    InvalidZeroAmount,
}
