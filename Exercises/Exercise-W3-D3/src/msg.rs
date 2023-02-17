use cosmwasm_schema::{cw_serde, QueryResponses};
use cosmwasm_std::Uint128;

#[cw_serde]
pub struct InstantiateMsg {}

#[cw_serde]
pub enum ExecuteMsg {
    ForwardTokens {
        forward_to_addr: String,
        amount: Uint128,
    },
}

#[cw_serde]
#[derive(QueryResponses)]
pub enum QueryMsg {
    #[returns(QueryTotalForwardedResponse)]
    QueryTotalForwarded {},
}

#[cw_serde]
pub struct QueryTotalForwardedResponse {
    pub amount: Uint128,
}
