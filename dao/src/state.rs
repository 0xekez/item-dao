use cosmwasm_std::{Addr, Uint128};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use cw_storage_plus::{Item, Map};

use crate::msg::{ProposeAction, ProposeMsg};

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct State {
    pub quorum: Uint128,
    pub proposal_cost: Uint128,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub enum ProposalStatus {
    /// The quorum requirement was reached for this proposal and it
    /// passed.
    Passed,
    /// The quorum requirement was reached for this proposal and it
    /// failed.
    Failed,
    /// The quorum requirement for this proposal has yet to be
    /// reached.
    Pending,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct Proposal {
    pub title: String,
    pub body: String,
    pub action: ProposeAction,

    pub status: ProposalStatus,
    pub yes: Uint128,
    pub no: Uint128,
    pub abstain: Uint128,
}

#[derive(Serialize, Deserialize, Clone, PartialEq, JsonSchema, Debug)]
pub struct TokenInfo {
    pub name: String,
    pub symbol: String,
    pub decimals: u8,
    pub total_supply: Uint128,
}

pub const STATE: Item<State> = Item::new("state");
pub const PROPOSALS: Item<Vec<Proposal>> = Item::new("proposals");
pub const BALANCES: Map<&Addr, Uint128> = Map::new("balances");
pub const TOKEN_INFO: Item<TokenInfo> = Item::new("token_info");

impl From<ProposeMsg> for Proposal {
    fn from(msg: ProposeMsg) -> Self {
        Self {
            title: msg.title,
            body: msg.body,
            action: msg.action,

            status: ProposalStatus::Pending,
            yes: Uint128::zero(),
            no: Uint128::zero(),
            abstain: Uint128::zero(),
        }
    }
}
