#[cfg(not(feature = "library"))]
use cosmwasm_std::entry_point;
use cosmwasm_std::{
    BankMsg, Binary, Coin, CosmosMsg, Deps, DepsMut, Env, MessageInfo, Response, StdResult, Uint128,
};
// use cw2::set_contract_version;

use crate::error::ContractError;
use crate::msg::{ExecuteMsg, InstantiateMsg, QueryMsg};

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
        ExecuteMsg::ForwardTokens { forward_to_addr } => {
            forward_tokens(deps, env, info, forward_to_addr)
        }
    }
}

fn forward_tokens(
    deps: DepsMut,
    _env: Env,
    info: MessageInfo,
    forward_to_addr: String,
) -> Result<Response, ContractError> {
    let validated_addr = deps.api.addr_validate(&forward_to_addr)?.to_string();

    // If funds are zero, throw an error to the sender - technically unecessary, since the chain will not let you send a
    // message w/zero funds
    if info.funds[0].amount == Uint128::new(0) {
        ContractError::ZeroFunds {};
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
pub fn query(_deps: Deps, _env: Env, _msg: QueryMsg) -> StdResult<Binary> {
    unimplemented!()
}

#[cfg(test)]
mod tests {}
