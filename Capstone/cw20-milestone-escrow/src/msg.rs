use cosmwasm_schema::{cw_serde, QueryResponses};

use cosmwasm_std::{Addr, Api, Coin, StdResult};

use cw20::{Balance, Cw20Coin, Cw20ReceiveMsg};

use crate::state::{
    get_end_height, get_end_time, get_total_balance_from, GenericBalance, HasAmount, HasEnd,
    Milestone,
};

#[cw_serde]
pub struct InstantiateMsg {}

#[cw_serde]
pub enum ExecuteMsg {
    /// Creates a new escrow with the given details
    Create(CreateMsg),
    /// Creates a new milestone for a given escrow
    CreateMilestone(CreateMilestoneMsg),
    /// Set the recipient of the given escrow
    SetRecipient { id: String, recipient: String },
    /// Approve sends all tokens to the recipient for a given milestone.
    /// Only the arbiter can do this
    ApproveMilestone {
        /// id is a human-readable name for the escrow from create
        id: String,
        milestone_id: String,
    },
    // Extend the escrow by the given time
    ExtendMilestone {
        /// id is a human-readable name for the escrow from create
        id: String,
        // The milestone to extend
        milestone_id: String,
        /// When end height set and block height exceeds this value, the escrow is expired.
        /// Once an escrow is expired, it can be returned to the original funder (via "refund").
        end_height: Option<u64>,
        /// When end time (in seconds since epoch 00:00:00 UTC on 1 January 1970) is set and
        /// block time exceeds this value, the escrow is expired.
        /// Once an escrow is expired, it can be returned to the original funder (via "refund").
        end_time: Option<u64>,
    },
    /// Refund returns all remaining tokens to the original sender,
    /// The arbiter can do this any time, or anyone can do this after a timeout
    Refund {
        /// id is a human-readable name for the escrow from create
        id: String,
    },
    /// This accepts a properly-encoded ReceiveMsg from a cw20 contract
    Receive(Cw20ReceiveMsg),
}

#[cw_serde]
pub enum ReceiveMsg {
    Create(CreateMsg),
    CreateMilestone(CreateMilestoneMsg),
}

#[cw_serde]
pub struct CreateMsg {
    /// id is a human-readable name for the escrow to use later
    /// 3-20 bytes of utf-8 text
    pub id: String,
    // arbiter can decide to approve or refund the escrow
    pub arbiter: String,
    /// if approved, funds go to the recipient
    pub recipient: Option<String>,
    /// Title of the escrow
    pub title: String,
    /// Longer description of the escrow, e.g. what conditions should be met
    pub description: String,
    /// When end height set and block height exceeds this value, the escrow is expired.
    /// Once an escrow is expired, it can be returned to the original funder (via "refund").
    pub cw20_whitelist: Option<Vec<String>>,
    /// List of milestones
    /// Each milestone has a title, description, amount, and whether it has been completed or not
    pub milestones: Vec<CreateMilestoneMsg>,
}

impl CreateMsg {
    pub fn addr_whitelist(&self, api: &dyn Api) -> StdResult<Vec<Addr>> {
        match self.cw20_whitelist.as_ref() {
            Some(v) => v.iter().map(|h| api.addr_validate(h)).collect(),
            None => Ok(vec![]),
        }
    }

    pub fn total_balance_from_milestones(&self) -> GenericBalance {
        get_total_balance_from(self.milestones.clone()).unwrap()
    }

    pub fn is_total_balance_empty(&self) -> bool {
        match self.total_balance_from_milestones() {
            balance => balance.native.is_empty() && balance.cw20.is_empty(),
        }
    }

    // Check sent balance against total milestones balance
    // Only checks first token for each type
    pub fn is_deposit_equal_to_milestones_balance(&self, deposit: Balance) -> bool {
        let total_balance_from_milestones = self.total_balance_from_milestones();
        match deposit {
            Balance::Native(balance) => {
                let total_balance = total_balance_from_milestones.native[0].amount;
                balance.0[0].amount == total_balance
            }
            Balance::Cw20(balance) => {
                let total_balance = total_balance_from_milestones.cw20[0].amount;
                balance.amount == total_balance
            }
        }
    }

    pub fn get_end_time(&self) -> Option<u64> {
        get_end_time(self.clone().milestones)
    }

    pub fn get_end_height(&self) -> Option<u64> {
        get_end_height(self.clone().milestones)
    }
}

#[cw_serde]
pub struct CreateMilestoneMsg {
    /// id is a human-readable name for the escrow to use later
    pub escrow_id: String,
    /// Title of the milestone
    pub title: String,
    /// Longer description of the milestone, e.g. what conditions should be met
    pub description: String,
    /// Amount of tokens to be released when the milestone is completed
    pub amount: GenericBalance,
    /// When end height set and block height exceeds this value, the escrow is expired.
    pub end_height: Option<u64>,
    /// When end time (in seconds since epoch 00:00:00 UTC on 1 January 1970) is set and
    /// block time exceeds this value, the escrow is expired.
    pub end_time: Option<u64>,
}

impl HasAmount for CreateMilestoneMsg {
    fn get_amount(&self) -> GenericBalance {
        self.amount.clone()
    }
}

impl HasEnd for CreateMilestoneMsg {
    fn get_end_time(&self) -> Option<u64> {
        self.end_time
    }
    fn get_end_height(&self) -> Option<u64> {
        self.end_height
    }
}

pub fn is_valid_name(name: &str) -> bool {
    let bytes = name.as_bytes();
    if bytes.len() < 3 || bytes.len() > 20 {
        return false;
    }
    true
}

#[cw_serde]
#[derive(QueryResponses)]
pub enum QueryMsg {
    /// Show all open escrows. Return type is ListResponse.
    #[returns(ListEscrowsResponse)]
    List {},

    /// Returns the details of the named escrow, error if not created
    /// Return type: DetailsResponse.
    #[returns(EscrowDetailsResponse)]
    EscrowDetails { id: String },

    // Returns the details for a milestone
    #[returns(Milestone)]
    MilestoneDetails { id: String, milestone_id: String },

    /// Returns the details of all milestones for a given escrow
    #[returns(ListMilestonesResponse)]
    ListMilestones { id: String },
}

#[cw_serde]
pub struct ListEscrowsResponse {
    /// list all registered ids
    pub escrows: Vec<String>,
}

#[cw_serde]
pub struct ListMilestonesResponse {
    /// list all registered milestone ids
    pub milestones: Vec<String>,
}

#[cw_serde]
pub struct EscrowDetailsResponse {
    /// id of this escrow
    pub id: String,
    /// arbiter can decide to approve or refund the escrow
    pub arbiter: String,
    /// if approved, funds go to the recipient
    pub recipient: Option<String>,
    /// if refunded, funds go to the source
    pub source: String,
    /// Title of the escrow
    pub title: String,
    /// Longer description of the escrow, e.g. what conditions should be met
    pub description: String,
    /// When end height set and block height exceeds this value, the escrow is expired.
    /// Once an escrow is expired, it can be returned to the original funder (via "refund").
    pub end_height: Option<u64>,
    /// When end time (in seconds since epoch 00:00:00 UTC on 1 January 1970) is set and
    /// block time exceeds this value, the escrow is expired.
    /// Once an escrow is expired, it can be returned to the original funder (via "refund").
    pub end_time: Option<u64>,
    /// Balance in native tokens
    pub native_balance: Vec<Coin>,
    /// Balance in cw20 tokens
    pub cw20_balance: Vec<Cw20Coin>,
    /// Whitelisted cw20 tokens
    pub cw20_whitelist: Vec<String>,
    /// List of milestones
    pub milestones: Vec<Milestone>,
}
