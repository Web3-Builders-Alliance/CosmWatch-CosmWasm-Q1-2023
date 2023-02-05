#[cfg(not(feature = "library"))]
use cosmwasm_std::entry_point;
use cosmwasm_std::{
    to_binary, Binary, Deps, DepsMut, Env, MessageInfo, Response, StdResult, Uint128,
};
use cw2::set_contract_version;

use crate::error::ContractError;
use crate::msg::{ExecuteMsg, GetCountResponse, InstantiateMsg, QueryMsg};
use crate::state::{State, STATE};

// version info for migration info
const CONTRACT_NAME: &str = "crates.io:coding-session-1";
const CONTRACT_VERSION: &str = env!("CARGO_PKG_VERSION");

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn instantiate(
    deps: DepsMut,
    _env: Env,
    info: MessageInfo,
    msg: InstantiateMsg,
) -> Result<Response, ContractError> {
    let state = State {
        count: msg.count,
        owner: info.sender.clone(),
    };
    set_contract_version(deps.storage, CONTRACT_NAME, CONTRACT_VERSION)?;
    STATE.save(deps.storage, &state)?;

    Ok(Response::new()
        .add_attribute("method", "instantiate")
        .add_attribute("owner", info.sender)
        .add_attribute("count", msg.count.to_string()))
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn execute(
    deps: DepsMut,
    _env: Env,
    info: MessageInfo,
    msg: ExecuteMsg,
) -> Result<Response, ContractError> {
    match msg {
        ExecuteMsg::Increment {} => execute::increment(deps),
        ExecuteMsg::Decrement {} => execute::decrement(deps, info),
        ExecuteMsg::Reset { count } => execute::reset(deps, info, count),
        ExecuteMsg::IncremementBy { count } => execute::incremement_by(deps, info, count),
        ExecuteMsg::DecrementBy { count } => execute::decrement_by(deps, info, count),
        ExecuteMsg::ReflectFunds { amount } => execute::reflect_funds(deps, info, amount),
    }
}

pub mod execute {
    use cosmwasm_std::{BankMsg, CosmosMsg};

    use super::*;

    pub fn increment(deps: DepsMut) -> Result<Response, ContractError> {
        STATE.update(deps.storage, |mut state| -> Result<_, ContractError> {
            state.count += 1;
            Ok(state)
        })?;

        Ok(Response::new().add_attribute("action", "increment"))
    }

    pub fn reset(deps: DepsMut, info: MessageInfo, count: i32) -> Result<Response, ContractError> {
        STATE.update(deps.storage, |mut state| -> Result<_, ContractError> {
            if info.sender != state.owner {
                return Err(ContractError::Unauthorized {});
            }
            state.count = count;
            Ok(state)
        })?;
        Ok(Response::new().add_attribute("action", "reset"))
    }

    pub fn decrement(deps: DepsMut, _info: MessageInfo) -> Result<Response, ContractError> {
        STATE.update(deps.storage, |mut state| -> Result<_, ContractError> {
            state.count -= 1;
            Ok(state)
        })?;

        Ok(Response::new().add_attribute("action", "decrement"))
    }

    pub fn incremement_by(
        deps: DepsMut,
        __info: MessageInfo,
        count: i32,
    ) -> Result<Response, ContractError> {
        STATE.update(deps.storage, |mut state| -> Result<_, ContractError> {
            state.count += count;
            Ok(state)
        })?;

        Ok(Response::new()
            .add_attribute("action", "increment_by")
            .add_attribute("incremented_by", count.to_string()))
    }

    pub fn decrement_by(
        deps: DepsMut,
        _info: MessageInfo,
        count: i32,
    ) -> Result<Response, ContractError> {
        STATE.update(deps.storage, |mut state| -> Result<_, ContractError> {
            state.count -= count;
            Ok(state)
        })?;

        Ok(Response::new()
            .add_attribute("action", "decrement_by")
            .add_attribute("decremented_by", count.to_string()))
    }
    pub fn reflect_funds(
        _deps: DepsMut,
        info: MessageInfo,
        amount: Uint128,
    ) -> Result<Response, ContractError> {
        let sender = info.sender;

        // If funds are empty, throw appropriate error to let user know
        if info.funds.is_empty() {
            return Err(ContractError::NoFunds {});
        }

        // Check if amount param matches amount sent
        if info.funds[0].amount != amount {
            return Err(ContractError::FundsMismatch {});
        }

        // If funds are not empty, go ahead and send the funds back to sender
        let msg = BankMsg::Send {
            to_address: sender.into_string(),
            amount: info.funds,
        };

        // Craft msg and send funds back to sender
        let _cosmos_msg: CosmosMsg<BankMsg> = CosmosMsg::Bank(msg);

        Ok(Response::new()
            .add_attribute("action", "reflect_funds")
            .add_attribute("amount", amount.to_string()))
    }
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn query(deps: Deps, _env: Env, msg: QueryMsg) -> StdResult<Binary> {
    match msg {
        QueryMsg::GetCount {} => to_binary(&query::count(deps)?),
    }
}

pub mod query {
    use super::*;

    pub fn count(deps: Deps) -> StdResult<GetCountResponse> {
        let state = STATE.load(deps.storage)?;
        Ok(GetCountResponse { count: state.count })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use cosmwasm_std::testing::{mock_dependencies, mock_env, mock_info};
    use cosmwasm_std::{coins, from_binary};

    #[test]
    fn proper_initialization() {
        let mut deps = mock_dependencies();

        let msg = InstantiateMsg { count: 17 };
        let info = mock_info("creator", &coins(1000, "earth"));

        // we can just call .unwrap() to assert this was a success
        let res = instantiate(deps.as_mut(), mock_env(), info, msg).unwrap();
        assert_eq!(0, res.messages.len());

        // it worked, let's query the state
        let res = query(deps.as_ref(), mock_env(), QueryMsg::GetCount {}).unwrap();
        let value: GetCountResponse = from_binary(&res).unwrap();
        assert_eq!(17, value.count);
    }

    #[test]
    fn increment() {
        let mut deps = mock_dependencies();

        let msg = InstantiateMsg { count: 17 };
        let info = mock_info("creator", &coins(2, "token"));
        let _res = instantiate(deps.as_mut(), mock_env(), info, msg).unwrap();

        // beneficiary can release it
        let info = mock_info("anyone", &coins(2, "token"));
        let msg = ExecuteMsg::Increment {};
        let _res = execute(deps.as_mut(), mock_env(), info, msg).unwrap();

        // should increase counter by 1
        let res = query(deps.as_ref(), mock_env(), QueryMsg::GetCount {}).unwrap();
        let value: GetCountResponse = from_binary(&res).unwrap();
        assert_eq!(18, value.count);
    }

    #[test]
    fn decrement() {
        let mut deps = mock_dependencies();

        let msg = InstantiateMsg { count: 17 };
        let info = mock_info("creator", &coins(2, "token"));
        let _res = instantiate(deps.as_mut(), mock_env(), info, msg).unwrap();

        let info = mock_info("creator", &coins(2, "token"));
        let msg = ExecuteMsg::Decrement {};
        let _res = execute(deps.as_mut(), mock_env(), info, msg).unwrap();

        let res = query(deps.as_ref(), mock_env(), QueryMsg::GetCount {}).unwrap();
        let value: GetCountResponse = from_binary(&res).unwrap();
        assert_eq!(16, value.count);
    }

    #[test]
    fn reset() {
        let mut deps = mock_dependencies();

        let msg = InstantiateMsg { count: 17 };
        let info = mock_info("creator", &coins(2, "token"));
        let _res = instantiate(deps.as_mut(), mock_env(), info, msg).unwrap();

        // beneficiary can release it
        let unauth_info = mock_info("anyone", &coins(2, "token"));
        let msg = ExecuteMsg::Reset { count: 5 };
        let res = execute(deps.as_mut(), mock_env(), unauth_info, msg);
        match res {
            Err(ContractError::Unauthorized {}) => {}
            _ => panic!("Must return unauthorized error"),
        }

        // only the original creator can reset the counter
        let auth_info = mock_info("creator", &coins(2, "token"));
        let msg = ExecuteMsg::Reset { count: 5 };
        let _res = execute(deps.as_mut(), mock_env(), auth_info, msg).unwrap();

        // should now be 5
        let res = query(deps.as_ref(), mock_env(), QueryMsg::GetCount {}).unwrap();
        let value: GetCountResponse = from_binary(&res).unwrap();
        assert_eq!(5, value.count);
    }

    #[test]
    fn increment_by() {
        let mut deps = mock_dependencies();

        // Instantiate with 17
        let msg = InstantiateMsg { count: 17 };
        let info = mock_info("creator", &coins(2, "token"));
        let _res = instantiate(deps.as_mut(), mock_env(), info, msg).unwrap();

        // Increment by 5
        let info = mock_info("anyone", &coins(2, "token"));
        let msg = ExecuteMsg::IncremementBy { count: 5 };
        let _res = execute(deps.as_mut(), mock_env(), info, msg).unwrap();

        // Check that state is 22
        let res = query(deps.as_ref(), mock_env(), QueryMsg::GetCount {}).unwrap();
        let value: GetCountResponse = from_binary(&res).unwrap();
        assert_eq!(22, value.count);
    }

    #[test]
    fn decrement_by() {
        let mut deps = mock_dependencies();

        // Instantiate with 17
        let msg = InstantiateMsg { count: 17 };
        let info = mock_info("creator", &coins(1, "token"));
        let _res = instantiate(deps.as_mut(), mock_env(), info, msg).unwrap();

        // Increment by 7
        let info = mock_info("anyone", &coins(1, "token"));
        let msg = ExecuteMsg::DecrementBy { count: 7 };
        let _res = execute(deps.as_mut(), mock_env(), info, msg);

        // Check that state is 10
        let res = query(deps.as_ref(), mock_env(), QueryMsg::GetCount {}).unwrap();
        let value: GetCountResponse = from_binary(&res).unwrap();
        assert_eq!(10, value.count);
    }
}
