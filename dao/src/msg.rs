use cosmwasm_std::{Binary, Uint128};
use cw20::Cw20Coin;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct InstantiateMsg {
    /// The number of webdao tokens that must participate in a vote in
    /// order for it to complete.
    pub quorum: Uint128,
    /// The number of webdao tokens that must be locked in order to
    /// create a new proposal.
    pub proposal_cost: Uint128,

    /// Information about the voting tokens that the DAO will use.
    pub token_info: TokenInstantiateInfo,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct TokenInstantiateInfo {
    /// The name of the token.
    pub name: String,
    /// The symbol for the token.
    pub symbol: String,
    /// The number of decimals that frontends should display when
    /// showing token balances. For example, if an address has 100,000
    /// tokens and the decimal number is 3 then the displayed balance
    /// will be 100.000.
    pub decimals: u8,
    /// The initial token balances. This determins the number of
    /// tokens that will initially be in circulation.
    pub initial_balances: Vec<Cw20Coin>,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct WithdrawVoteMsg {
    /// The id of the propsal that the vote ought to be withdrawn for.
    pub proposal_id: usize,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct DaoItem {
    /// The name of the webpage. Frontends are likely to make the
    /// webpage accessible at `/name`.
    pub name: String,
    /// The contents of the webpage. Webdao doesn't have prefered
    /// markdown format. Frontends can figure that out.
    pub contents: String,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum ProposeAction {
    /// Proposes that the quorum be changed to a new value.
    ChangeQuorum { new_quorum: Uint128 },
    /// Proposes that the cost of creating a new proposal be changed
    /// to a new value.
    ChangeProposalCost { new_proposal_cost: Uint128 },

    /// Proposes that a new webpage be added.
    AddItem(DaoItem),
    /// Proposes that an existinig webpage be removed.
    RemoveItem { id: usize },
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct ProposeMsg {
    /// The title of the proposal.
    pub title: String,
    /// The body of the proposal.
    pub body: String,
    /// The action that will be executed should the proposal pass.
    pub action: ProposeAction,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum VotePosition {
    /// I would like to execute the proposal.
    Yes,
    /// I would not like to execute the proposal.
    No,
    /// I do not care one way or the other. My vote to abstain will
    /// count towards the quorum requirements and I trust that those
    /// with positions will decide on a reasonable outcome.
    Abstain,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct VoteMsg {
    /// The ID of the proposal that the sender would like to lock
    /// their tokens on.
    pub proposal_id: usize,
    /// What position that sender would like to lock their tokens to.
    pub position: VotePosition,
    /// The number of tokens that should be staked to this vote.
    pub amount: Uint128,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum ExecuteMsg {
    /// Provides a means via which token holders can unlock tokens
    /// that have been comitted to a proposal.
    Withdraw(WithdrawVoteMsg),

    /// Create a new proposal
    Propose(ProposeMsg),
    /// Vote on an existing proposal
    Vote(VoteMsg),

    /// Move tokens to another account without triggering actions
    Transfer { recipient: String, amount: Uint128 },
    /// Destroy tokens forever
    Burn { amount: Uint128 },
    /// Transfer tokens to a contract and trigger an action on the
    /// receiving contract.
    Send {
        contract: String,
        amount: Uint128,
        msg: Binary,
    },
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum QueryMsg {
    /// Paginated listing of proposals.
    ListProposals,
    /// Get title, body, and action information for a proposal given
    /// it's proposal ID.
    GetProposal { proposal_id: usize },

    /// List all of the items that have been added to the DAO.
    ListItems,
    /// Get all of the items that have been added to the DAO.
    GetItem { item_id: usize },

    /// Get information about what the current quorum is.
    GetQuorum,
    /// Get information about what the current proposal cost is.
    GetProposalCost,

    /// Ask the contract how many tokens a particular address
    /// controls.
    Balance { address: String },
    /// Get info about the token. Returns a TokenInfoResponse
    /// containing {name, ticker, decimal, total_supply}.
    TokenInfo,
}
