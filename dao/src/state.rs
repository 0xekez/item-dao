use cosmwasm_std::{Addr, Uint128};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use cw_storage_plus::{Item, Map};

use crate::msg::{DaoItem, ProposeAction, ProposeMsg, VotePosition};

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
    pub yes: Vec<(Addr, Uint128)>,
    pub no: Vec<(Addr, Uint128)>,
    pub abstain: Vec<(Addr, Uint128)>,

    pub proposer: Addr,
    pub proposal_cost: Uint128,
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
pub const ITEMS: Item<Vec<DaoItem>> = Item::new("dao_items");

impl Proposal {
    pub fn new(msg: ProposeMsg, proposer: Addr, proposal_cost: Uint128) -> Self {
        Self {
            title: msg.title,
            body: msg.body,
            action: msg.action,
            status: ProposalStatus::Pending,
            yes: vec![],
            no: vec![],
            abstain: vec![],
            proposer,
            proposal_cost,
        }
    }

    pub fn add_vote(&mut self, addr: &Addr, position: VotePosition, staked: Uint128) {
        match position {
            VotePosition::Yes => self.yes.push((addr.clone(), staked)),
            VotePosition::No => self.no.push((addr.clone(), staked)),
            VotePosition::Abstain => self.abstain.push((addr.clone(), staked)),
        }
    }

    fn pos_sum(items: &[(Addr, Uint128)]) -> Uint128 {
        items.iter().map(|i| i.1).sum()
    }

    pub fn get_votes(&self, position: VotePosition) -> Uint128 {
        Self::pos_sum(match position {
            VotePosition::Yes => self.yes.as_slice(),
            VotePosition::No => self.no.as_slice(),
            VotePosition::Abstain => self.abstain.as_slice(),
        })
    }

    pub fn get_total_votes(&self) -> Uint128 {
        self.get_votes(VotePosition::Yes)
            + self.get_votes(VotePosition::Abstain)
            + self.get_votes(VotePosition::No)
    }
}
