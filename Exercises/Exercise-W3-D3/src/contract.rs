#[cfg(not(feature = "library"))]
use cosmwasm_std::entry_point;
use cosmwasm_std::{
    to_binary, BankMsg, Binary, CosmosMsg, Deps, DepsMut, Env, MessageInfo, Response, StdResult,
    Uint128,
};
// use cw2::set_contract_version;

use crate::error::ContractError;
use crate::msg::{ExecuteMsg, InstantiateMsg, QueryMsg, QueryTotalForwardedResponse};
use crate::state::TOKENS_SENT;

/*
// version info for migration info
const CONTRACT_NAME: &str = "crates.io:sender-receiver-code-challenge";
const CONTRACT_VERSION: &str = env!("CARGO_PKG_VERSION");
*/

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn instantiate(
    _deps: DepsMut,
    _env: Env,
    _info: MessageInfo,
    _msg: InstantiateMsg,
) -> Result<Response, ContractError> {
    Ok(Response::new().add_attribute("action", "instantiate"))
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn execute(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    msg: ExecuteMsg,
) -> Result<Response, ContractError> {
    match msg {
        ExecuteMsg::ForwardTokens {
            forward_to_addr,
            amount,
        } => forward_tokens(deps, env, info, forward_to_addr, amount),
    }
}

fn forward_tokens(
    deps: DepsMut,
    _env: Env,
    info: MessageInfo,
    forward_to_addr: String,
    amount: Uint128,
) -> Result<Response, ContractError> {
    let validated_addr = deps.api.addr_validate(&forward_to_addr)?.to_string();

    // Check if funds are empty before we access
    if info.funds.is_empty() {
        ContractError::ZeroFunds {};
    }

    // If funds are zero, throw an error to the sender - technically unecessary, since the chain will not let you send a
    // message w/zero funds
    if info.funds[0].amount == Uint128::zero() {
        ContractError::ZeroFunds {};
    }

    // Compare provided amount with funds amount
    if info.funds[0].amount != amount {
        ContractError::AmountMismatch {};
    }

    // Ensure we are not sending tokens other than uluna
    if info.funds[0].denom != "uluna" {
        ContractError::DenomMismatch {};
    }

    // Ensure only 1 type of token is being sent
    if info.funds.len() > 1 {
        ContractError::MoreThanOneToken {};
    }

    // Create send msg using validated forward_to address and funds included in the request
    let msg = BankMsg::Send {
        to_address: validated_addr,
        amount: info.funds,
    };

    // New response with action and
    Ok(Response::new()
        .add_attribute("action", "forward_tokens")
        .add_message(CosmosMsg::Bank(msg)))
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn query(deps: Deps, _env: Env, msg: QueryMsg) -> StdResult<Binary> {
    match msg {
        QueryMsg::QueryTotalForwarded {} => to_binary(&query_total_forwarded(deps)?),
    }
}

fn query_total_forwarded(deps: Deps) -> StdResult<QueryTotalForwardedResponse> {
    let tokens_sent = TOKENS_SENT.load(deps.storage)?;

    Ok(QueryTotalForwardedResponse {
        amount: tokens_sent.amount,
    })
}

#[cfg(test)]
mod tests {}
