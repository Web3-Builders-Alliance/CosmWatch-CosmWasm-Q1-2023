use cosmwasm_schema::cw_serde;

use crate::config::{Ballot, Config, Poll};

#[cw_serde]
pub struct InstantiateMsg {
    pub admin: Option<String>,
}

#[cw_serde]
pub enum ExecuteMsg {
    CreatePoll {
        poll_id: String,
        question: String,
        options: Vec<String>,
    },
    Vote {
        poll_id: String,
        vote: String,
    },
}

#[cw_serde]
pub enum QueryMsg {
    AllPolls,
    Poll { poll_id: String },
    Vote { poll_id: String, address: String },
    Config,
}

#[cw_serde]
pub struct AllPollsResponse {
    pub polls: Vec<Poll>,
}

#[cw_serde]
pub struct PollResponse {
    pub poll: Option<Poll>,
}

#[cw_serde]
pub struct VoteResponse {
    pub vote: Option<Ballot>,
}

#[cw_serde]
pub struct ConfigResponse {
    pub config: Config,
}
