/* 
** Excerpt from cw-plus/cw1-whitelist contract: https://github.com/CosmWasm/cw-plus/blob/main/contracts/cw1-whitelist/src
*/

// Import JSON schema to convert Rust data structures to JSON
use schemars::JsonSchema;
// Import fmt from Rust standard library for string interpolation
use std::fmt;

// Import entry point macro and other utilities from the CosmWasm standard library
#[cfg(not(feature = "library"))]
use cosmwasm_std::entry_point;
use cosmwasm_std::{
    to_binary, Addr, Api, Binary, CosmosMsg, Deps, DepsMut, Empty, Env, MessageInfo, Response,
    StdResult,
};

// Import Response struct from CW1 library
use cw1::CanExecuteResponse;
// Import function from CW2 to set contract version
use cw2::set_contract_version;

// Import contract error, messages, and state-related data structures
use crate::error::ContractError;
use crate::msg::{AdminListResponse, ExecuteMsg, InstantiateMsg, QueryMsg};
use crate::state::{AdminList, ADMIN_LIST};

// Define constants for contract name and version to be used later
const CONTRACT_NAME: &str = "crates.io:cw1-whitelist";
const CONTRACT_VERSION: &str = env!("CARGO_PKG_VERSION");

// Mark instantiate function as an entry point in the wasm application
// Instantiate is the method in which we 'activate' the contract on-chain.
// For the cw1 contract in particular, this is an opportunity to initialize the contract
// with a list of admin addresses and determine whether or not the admin list should be mutable 
// after instantiation. The instantiate function uses the map_validate function to ensure the 
// addresses provided upon instantiation are valid.
#[cfg_attr(not(feature = "library"), entry_point)]
pub fn instantiate(
    deps: DepsMut,
    _env: Env,
    _info: MessageInfo,
    msg: InstantiateMsg,
) -> StdResult<Response> {
    // Set contract name and version. Passing in contract storage, contract name and version as
    // seen above
    set_contract_version(deps.storage, CONTRACT_NAME, CONTRACT_VERSION)?;
    // Define initial admin list and save to contract state
    let cfg = AdminList {
        admins: map_validate(deps.api, &msg.admins)?,
        mutable: msg.mutable,
    };
    ADMIN_LIST.save(deps.storage, &cfg)?;
    // Respond with default response. What does this look like from the client/request-side and
    //  when is this sort of response best used?
    Ok(Response::default())
}

// Define function to take in a vector of addresses, loop over each address in the vector and
// validate it using the Api argument, then collect all of the validated addresses into a new
// vector to be returned
pub fn map_validate(api: &dyn Api, admins: &[String]) -> StdResult<Vec<Addr>> {
    admins.iter().map(|addr| api.addr_validate(addr)).collect()
}

// Mark execute function as an entry point in the wasm application
#[cfg_attr(not(feature = "library"), entry_point)]
pub fn execute(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    // Note: implement this function with different type to add support for custom messages
    // and then import the rest of this contract code.
    msg: ExecuteMsg<Empty>,
) -> Result<Response<Empty>, ContractError> {
    // Match the incoming message and route to the appropriate handler function
    match msg {
        // When a message is matched, invoke the respective function and 
        // pass along dependencies, environment, message info, in addition to other custom
        // parameters (if any). In this cw1 contract, Freeze is the only execute message that
        // doesn't accept a custom parameter, only deps, env, and message info. In contrast, 
        // Execute and UpdateAdmins both accept custom parameters such as a vector of messages
        // and admins respectively. These custom parameters are provided by the user.
        ExecuteMsg::Execute { msgs } => execute_execute(deps, env, info, msgs),
        ExecuteMsg::Freeze {} => execute_freeze(deps, env, info),
        ExecuteMsg::UpdateAdmins { admins } => execute_update_admins(deps, env, info, admins),
    }
}

// Execute function to execute messages received by authorized addresses (as determined by the
// ADMIN_LIST Item).
pub fn execute_execute<T>(
    deps: DepsMut,
    _env: Env,
    info: MessageInfo,
    msgs: Vec<CosmosMsg<T>>,
) -> Result<Response<T>, ContractError>
where
    T: Clone + fmt::Debug + PartialEq + JsonSchema,
{
    // If the sending address IS NOT authorized to execute messages on behalf of the contract, we'll
    // return an unauthorized error to the user.
    if !can_execute(deps.as_ref(), info.sender.as_ref())? {
        Err(ContractError::Unauthorized {})
    } else {
    // If the sending address IS authorized, we'll send a successful response back with the messages
    // executed and an "action" attribute with a value of "execute".
        let res = Response::new()
            .add_messages(msgs)
            .add_attribute("action", "execute");
        Ok(res)
    }
}

// Freeze function that disables admin list modifications
pub fn execute_freeze(
    deps: DepsMut,
    _env: Env,
    info: MessageInfo,
) -> Result<Response, ContractError> {
    // Load admin list from storage
    let mut cfg = ADMIN_LIST.load(deps.storage)?;
    // If sending address IS NOT authorized to modify the admin list, we'll return an unauthorized
    // error to the user
    if !cfg.can_modify(info.sender.as_ref()) {
        Err(ContractError::Unauthorized {})
    // If sending address IS authorized, disable changes to the admin list then save it to contract state
    } else {
        cfg.mutable = false;
        ADMIN_LIST.save(deps.storage, &cfg)?;

        // Return a successful response with the "action" attribute set to "freeze"
        let res = Response::new().add_attribute("action", "freeze");
        Ok(res)
    }
}

// Define function to overwrite existing admin list with the provided addresses, but only if the sender is  
// an existing admin, the admin list is mutable, AND all the provided addresses are valid
pub fn execute_update_admins(
    deps: DepsMut,
    _env: Env,
    info: MessageInfo,
    admins: Vec<String>,
) -> Result<Response, ContractError> {
    // Load the admin list from storage
    let mut cfg = ADMIN_LIST.load(deps.storage)?;
    // If sending address IS NOT authorized to modify the admin list, we'll return an unauthorized
    if !cfg.can_modify(info.sender.as_ref()) {
        Err(ContractError::Unauthorized {})
    // If sending address IS authorized, validate incoming addresses and overwrite existing admins vector
    // with the new addresses, then save the new admins to contract state
    } else {
        cfg.admins = map_validate(deps.api, &admins)?;
        ADMIN_LIST.save(deps.storage, &cfg)?;

        // Return a successful response with the "update_admins" action
        let res = Response::new().add_attribute("action", "update_admins");
        Ok(res)
    }
}

// Can execute function takes in a sender address and returns a boolean. The function will return true if
// the sending address is an admin and will otherwise return false.
fn can_execute(deps: Deps, sender: &str) -> StdResult<bool> {
    // Load admin list from storage
    let cfg = ADMIN_LIST.load(deps.storage)?;
    // Check if sending address is an admin
    let can = cfg.is_admin(&sender);
    Ok(can)
}

// Mark query function as an entry point in the wasm application.
// Query is how we retrieve information about the contract's current state. We convert the response
// to binary before sending it back to the user
#[cfg_attr(not(feature = "library"), entry_point)]
pub fn query(deps: Deps, _env: Env, msg: QueryMsg) -> StdResult<Binary> {
    // Similar to execute messages, incoming messages will be matched and their respective functions will
    // be invoked and converted to binary before responding to the user.
    match msg {
        QueryMsg::AdminList {} => to_binary(&query_admin_list(deps)?),
        QueryMsg::CanExecute { sender, msg } => to_binary(&query_can_execute(deps, sender, msg)?),
    }
}

// The query_admin_list function returns a list of admins from the contract's current state
pub fn query_admin_list(deps: Deps) -> StdResult<AdminListResponse> {
    // load the admin list from storage
    let cfg = ADMIN_LIST.load(deps.storage)?;
    // Return an admin list response, containing the current list of admins and whether or not the admin
    // list is mutable
    Ok(AdminListResponse {
        admins: cfg.admins.into_iter().map(|a| a.into()).collect(),
        mutable: cfg.mutable,
    })
}

// The query_can_execute function will check if the provided sender address is an admin, then responds with
// true if the sender address is an admin (and can in turn execute messages on behalf of the contract)
// and false if the user isn't an admin
pub fn query_can_execute(
    deps: Deps,
    sender: String,
    _msg: CosmosMsg,
) -> StdResult<CanExecuteResponse> {
    Ok(CanExecuteResponse {
        can_execute: can_execute(deps, &sender)?,
    })
}

// Let Rust know this is where the tests are defined with a macro and module
#[cfg(test)]
mod tests {
    // Invoke superclass constructor
    use super::*;
    // Import mock depenencies, environment, and message info from cosmwasm standard testing library
    use cosmwasm_std::testing::{mock_dependencies, mock_env, mock_info};
    // Import data structures from cosmwasm standard library
    use cosmwasm_std::{coin, coins, BankMsg, StakingMsg, SubMsg, WasmMsg};

    // Defines a test that instantiates a new contract with mock data and ensures the following functionality
    // is working as expected:
    // - Instantiate contract with valid config
    // - Ensure only admins can modify and freeze the contract
    #[test]
    fn instantiate_and_modify_config() {
        // Define mock dependencies
        let mut deps = mock_dependencies();

        // Create fictious addresses
        let alice = "alice";
        let bob = "bob";
        let carl = "carl";

        let anyone = "anyone";

        // instantiate the contract
        let instantiate_msg = InstantiateMsg {
            admins: vec![alice.to_string(), bob.to_string(), carl.to_string()],
            mutable: true,
        };
        // Create message info with the "anyone" address and instantiate the contract with it
        let info = mock_info(anyone, &[]);
        instantiate(deps.as_mut(), mock_env(), info, instantiate_msg).unwrap();

        // ensure expected config
        // Expect alice, bob, and carl as admins and the admin list is open for changes
        let expected = AdminListResponse {
            admins: vec![alice.to_string(), bob.to_string(), carl.to_string()],
            mutable: true,
        };
        assert_eq!(query_admin_list(deps.as_ref()).unwrap(), expected);

        // anyone cannot modify the contract
        // Create new UpdateAdmins execute message with anyone as the sole address,
        // then try to execute it with anyone as the sending address.
        let msg = ExecuteMsg::UpdateAdmins {
            admins: vec![anyone.to_string()],
        };
        // Execute the message from anyone
        let info = mock_info(anyone, &[]);
        let err = execute(deps.as_mut(), mock_env(), info, msg).unwrap_err();
        // Assert the response of execution was an unauthorized contract error
        assert_eq!(err, ContractError::Unauthorized {});

        // but alice can kick out carl
        // Create a new update admins message with alice and bob as admins, removing carl
        let msg = ExecuteMsg::UpdateAdmins {
            admins: vec![alice.to_string(), bob.to_string()],
        };
        // Execute the update message as alice
        let info = mock_info(alice, &[]);
        execute(deps.as_mut(), mock_env(), info, msg).unwrap();

        // ensure expected config
        // Expect success because alice is an admin and is therefor authorized to update the admins list
        let expected = AdminListResponse {
            admins: vec![alice.to_string(), bob.to_string()],
            mutable: true,
        };
        assert_eq!(query_admin_list(deps.as_ref()).unwrap(), expected);

        // Carl cannot freeze the admins list because he is not currently an admin
        let info = mock_info(carl, &[]);
        let err = execute(deps.as_mut(), mock_env(), info, ExecuteMsg::Freeze {}).unwrap_err();
        assert_eq!(err, ContractError::Unauthorized {});

        // but bob can
        let info = mock_info(bob, &[]);
        execute(deps.as_mut(), mock_env(), info, ExecuteMsg::Freeze {}).unwrap();
        let expected = AdminListResponse {
            admins: vec![alice.to_string(), bob.to_string()],
            mutable: false,
        };
        assert_eq!(query_admin_list(deps.as_ref()).unwrap(), expected);

        // and now alice cannot change it again
        let msg = ExecuteMsg::UpdateAdmins {
            admins: vec![alice.to_string()],
        };
        let info = mock_info(alice, &[]);
        let err = execute(deps.as_mut(), mock_env(), info, msg).unwrap_err();
        assert_eq!(err, ContractError::Unauthorized {});
    }

    // Defines a test that instantiates a new contract with mock data and ensures the following functionality
    // is working as expected:
    // - Ensure only admins can execute messages on behalf of the contract
    #[test]
    fn execute_messages_has_proper_permissions() {
        let mut deps = mock_dependencies();

        let alice = "alice";
        let bob = "bob";
        let carl = "carl";

        // instantiate the contract
        let instantiate_msg = InstantiateMsg {
            admins: vec![alice.to_string(), carl.to_string()],
            mutable: false,
        };
        let info = mock_info(bob, &[]);
        instantiate(deps.as_mut(), mock_env(), info, instantiate_msg).unwrap();

        // Create freeze, send, and execute messages. Assign freeze to a variable and assign send and execute
        // messages to a vector
        let freeze: ExecuteMsg<Empty> = ExecuteMsg::Freeze {};
        let msgs = vec![
            BankMsg::Send {
                to_address: bob.to_string(),
                amount: coins(10000, "DAI"),
            }
            .into(),
            WasmMsg::Execute {
                contract_addr: "some contract".into(),
                msg: to_binary(&freeze).unwrap(),
                funds: vec![],
            }
            .into(),
        ];

        // make some nice message
        let execute_msg = ExecuteMsg::Execute { msgs: msgs.clone() };

        // bob cannot execute them
        let info = mock_info(bob, &[]);
        let err = execute(deps.as_mut(), mock_env(), info, execute_msg.clone()).unwrap_err();
        assert_eq!(err, ContractError::Unauthorized {});

        // but carl can
        let info = mock_info(carl, &[]);
        let res = execute(deps.as_mut(), mock_env(), info, execute_msg).unwrap();
        assert_eq!(
            res.messages,
            msgs.into_iter().map(SubMsg::new).collect::<Vec<_>>()
        );
        assert_eq!(res.attributes, [("action", "execute")]);
    }

    // Defines a test that instantiates a new contract with mock data and ensures the following functionality
    // is working as expected:
    // - Respond whether a given address can execute messages on behalf of a contract or not
    #[test]
    fn can_execute_query_works() {
        let mut deps = mock_dependencies();

        let alice = "alice";
        let bob = "bob";

        let anyone = "anyone";

        // instantiate the contract
        let instantiate_msg = InstantiateMsg {
            admins: vec![alice.to_string(), bob.to_string()],
            mutable: false,
        };
        let info = mock_info(anyone, &[]);
        instantiate(deps.as_mut(), mock_env(), info, instantiate_msg).unwrap();

        // let us make some queries... different msg types by owner and by other
        let send_msg = CosmosMsg::Bank(BankMsg::Send {
            to_address: anyone.to_string(),
            amount: coins(12345, "ushell"),
        });
        let staking_msg = CosmosMsg::Staking(StakingMsg::Delegate {
            validator: anyone.to_string(),
            amount: coin(70000, "ureef"),
        });

        // owner can send
        let res = query_can_execute(deps.as_ref(), alice.to_string(), send_msg.clone()).unwrap();
        assert!(res.can_execute);

        // owner can stake
        let res = query_can_execute(deps.as_ref(), bob.to_string(), staking_msg.clone()).unwrap();
        assert!(res.can_execute);

        // anyone cannot send
        let res = query_can_execute(deps.as_ref(), anyone.to_string(), send_msg).unwrap();
        assert!(!res.can_execute);

        // anyone cannot stake
        let res = query_can_execute(deps.as_ref(), anyone.to_string(), staking_msg).unwrap();
        assert!(!res.can_execute);
    }
}