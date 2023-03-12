use cosmwasm_schema::{cw_serde, QueryResponses};
use cosmwasm_std::Uint128;

#[cw_serde]
pub struct InstantiateMsg {
    pub token_denom: String,
    pub code_id: u64,
    pub owner: String,
}

#[cw_serde]
pub enum ExecuteMsg {
    Deposit { amount: Uint128 },
}

#[cw_serde]
#[derive(QueryResponses)]
pub enum QueryMsg {
    // QueryTotalTokens
    // QueryAllTokens -> forward to CW20 contract
}

#[cw_serde]
pub struct MigrateMsg {}
