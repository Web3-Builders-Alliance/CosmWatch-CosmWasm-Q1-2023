use std::env::current_dir;
use std::fs::create_dir;

use cosmwasm_schema::{export_schema_with_title, remove_schemas, schema_for};

use cosm_wasm_zero2_hero::config::{Ballot, Config, Poll};
use cosm_wasm_zero2_hero::msg::{
    AllPollsResponse, ConfigResponse, ExecuteMsg, InstantiateMsg, PollResponse, QueryMsg,
    VoteResponse,
};

fn main() {
    // Define the path where the schema will be saved
    let mut out_dir = current_dir().unwrap();
    out_dir.push("schema");
    create_dir(&out_dir).unwrap();
    remove_schemas(&out_dir).unwrap();

    // Export schema for messages
    export_schema_with_title(&schema_for!(ExecuteMsg), &out_dir, "ExecuteMsg");
    export_schema_with_title(&schema_for!(InstantiateMsg), &out_dir, "InstantiateMsg");
    export_schema_with_title(&schema_for!(QueryMsg), &out_dir, "QueryMsg");

    // Export schema for message responses
    export_schema_with_title(&schema_for!(AllPollsResponse), &out_dir, "AllPollsResponse");
    export_schema_with_title(&schema_for!(PollResponse), &out_dir, "PollResponse");
    export_schema_with_title(&schema_for!(VoteResponse), &out_dir, "VoteResponse");
    export_schema_with_title(&schema_for!(ConfigResponse), &out_dir, "ConfigResponse");

    // Export schema for Config, Ballot, and Poll
    export_schema_with_title(&schema_for!(Config), &out_dir, "Config");
    export_schema_with_title(&schema_for!(Ballot), &out_dir, "Ballot");
    export_schema_with_title(&schema_for!(Poll), &out_dir, "Poll");
}
