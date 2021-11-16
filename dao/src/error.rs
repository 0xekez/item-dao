use cosmwasm_std::{StdError, Uint128};
use thiserror::Error;

#[derive(Error, Debug)]
pub enum ContractError {
    #[error("{0}")]
    Std(#[from] StdError),

    #[error("Unauthorized")]
    Unauthorized {},

    #[error("Invalid quorum: quorum must be greater than zero")]
    InvalidQuorum,

    #[error("Insufficent funds for proposal. Needed ({needed}), got ({got})")]
    InsufficentProposalFunds { needed: Uint128, got: Uint128 },
}
