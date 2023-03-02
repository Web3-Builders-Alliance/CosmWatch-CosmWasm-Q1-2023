#[cfg(not(feature = "library"))]
use cosmwasm_std::entry_point;
use cosmwasm_std::{
    to_binary, Addr, Binary, Coin, Deps, DepsMut, Env, MessageInfo, Reply, Response, StdResult,
    SubMsg, Uint128, WasmMsg,
};
use cw2::set_contract_version;
use cw20::{Cw20ExecuteMsg, MinterResponse};
use cw20_base::msg::InstantiateMsg as CW20InstantiateMsg;
use cw_utils::parse_reply_instantiate_data;

use crate::error::ContractError;
use crate::msg::{ExecuteMsg, InstantiateMsg, MigrateMsg, QueryMsg};
use crate::state::{Config, CONFIG, LP_TOKEN};

// version info for migration info and a label for cw20 contract
const CONTRACT_NAME: &str = "crates.io:simple-actor";
const CONTRACT_VERSION: &str = env!("CARGO_PKG_VERSION");
const CONTRACT_LABEL: &str = "CosmWatch - W3BA LP TOKEN Contract";

const REPLY_INIT_CW20_ID: u64 = 0;

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn instantiate(
    deps: DepsMut,
    env: Env,
    _info: MessageInfo,
    msg: InstantiateMsg,
) -> Result<Response, ContractError> {
    // Contract migration name and version
    set_contract_version(deps.storage, CONTRACT_NAME, CONTRACT_VERSION)?;

    // Initialize CONFIG state
    let validated_owner: Addr = deps.api.addr_validate(&msg.owner)?;
    let config = Config {
        cw20_address: None,
        owner: validated_owner,
    };
    CONFIG.save(deps.storage, &config)?;

    // Initialize LP_TOKEN state
    let token = Coin {
        amount: Uint128::zero(),
        denom: msg.token_denom.clone(),
    };
    LP_TOKEN.save(deps.storage, &token)?;

    // Construct instantiate CW20 message
    let contract_address = env.contract.address;
    let cw_msg = CW20InstantiateMsg {
        name: CONTRACT_LABEL.clone().to_string(),
        symbol: "W3BALP".to_string(),
        decimals: 6,
        initial_balances: vec![],
        mint: Some(MinterResponse {
            minter: contract_address.clone().to_string(),
            cap: None,
        }),
        marketing: None,
    };
    let bin_msg: Binary = to_binary(&cw_msg)?;

    // Construct Wasm Instantiate message, using our cw20 instantiate message from above
    let instantiate_msg = WasmMsg::Instantiate {
        admin: Some(msg.owner),
        code_id: msg.code_id,
        msg: bin_msg,
        funds: vec![],
        label: CONTRACT_LABEL.clone().to_string(),
    };
    let submsg = SubMsg::reply_on_success(instantiate_msg, REPLY_INIT_CW20_ID);

    // If the code above executed successfully, respond with the cw20 instantiate message
    Ok(Response::new().add_submessage(submsg))
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn execute(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    msg: ExecuteMsg,
) -> Result<Response, ContractError> {
    match msg {
        ExecuteMsg::Deposit { amount } => execute_deposit(deps, env, info, amount),
        // ExecuteMsg::Withdraw { amount: Uint128 } => execute_withdraw(deps, env, info, amount),
    }
}

fn execute_deposit(
    deps: DepsMut,
    _env: Env,
    info: MessageInfo,
    amount: Uint128,
) -> Result<Response, ContractError> {
    // Check if sender sent funds
    if info.funds.len() == 0 {
        return Err(ContractError::NoFundsSent {});
    }

    // Verify amount is more than 0
    if amount.is_zero() {
        return Err(ContractError::InvalidAmount {});
    }

    // Check for funds mismatch
    if info.funds[0].amount != amount {
        return Err(ContractError::AmountMismatch {});
    }

    // MINT THE AMOUNT OF TOKENS DEPOSITED
    // Construct mint msg
    let recipient: String = info.sender.to_string();
    let msg: Cw20ExecuteMsg = cw20_base::msg::ExecuteMsg::Mint { recipient, amount };
    let bin_msg: Binary = to_binary(&msg)?;

    // Construct wasm execute msg and respond with it
    let contract_addr = CONFIG.load(deps.storage)?.cw20_address.into_string();
    let execute_msg = WasmMsg::Execute {
        contract_addr,
        msg: bin_msg,
        funds: info.funds,
    };

    Ok(Response::new().add_message(execute_msg))
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn query(_deps: Deps, _env: Env, _msg: QueryMsg) -> StdResult<Binary> {
    unimplemented!()
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn reply(deps: DepsMut, env: Env, msg: Reply) -> Result<Response, ContractError> {
    match msg.id {
        REPLY_INIT_CW20_ID => reply_init_cw20(deps, env, msg),
        _ => Err(ContractError::InvalidReply {}),
    }
}

fn reply_init_cw20(deps: DepsMut, _env: Env, msg: Reply) -> Result<Response, ContractError> {
    // Get the contract address from the reply
    let res = parse_reply_instantiate_data(msg)?;

    // Save the contract address to the state
    let mut config = CONFIG.load(deps.storage)?;
    config.cw20_address = deps.api.addr_validate(&res.contract_address)?;
    CONFIG.save(deps.storage, &config)?;

    Ok(Response::new())
}

// Handle migration
#[cfg_attr(not(feature = "library"), entry_point)]
pub fn migrate(deps: DepsMut, _env: Env, _msg: MigrateMsg) -> Result<Response, ContractError> {
    set_contract_version(deps.storage, CONTRACT_NAME, CONTRACT_VERSION)?;
    Ok(Response::new()
        .add_attribute("action", "migrate")
        .add_attribute("new_name", CONTRACT_NAME)
        .add_attribute("new_version", CONTRACT_VERSION))
}

#[cfg(test)]
mod tests {}
