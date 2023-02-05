use cosmwasm_schema::{cw_serde, QueryResponses};
use cosmwasm_std::Uint128;

#[cw_serde]
pub struct InstantiateMsg {
    pub count: i32,
}

#[cw_serde]
pub enum ExecuteMsg {
    Increment {},
    Decrement {},
    Reset { count: i32 },
    IncremementBy { count: i32 },
    DecrementBy { count: i32 },
    ReflectFunds { amount: Uint128 },
}

#[cw_serde]
#[derive(QueryResponses)]
pub enum QueryMsg {
    // GetCount returns the current count as a json-encoded number
    #[returns(GetCountResponse)]
    GetCount {},
}

// We define a custom struct for each query response
#[cw_serde]
pub struct GetCountResponse {
    pub count: i32,
}
