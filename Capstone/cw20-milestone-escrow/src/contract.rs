#[cfg(not(feature = "library"))]
use cosmwasm_std::entry_point;
use cosmwasm_std::{
    from_binary, to_binary, Addr, BankMsg, Binary, Deps, DepsMut, Env, MessageInfo, Response,
    StdError, StdResult, SubMsg, WasmMsg,
};

use cw2::set_contract_version;
use cw20::{Balance, Cw20Coin, Cw20CoinVerified, Cw20ExecuteMsg, Cw20ReceiveMsg};

use crate::error::ContractError;
use crate::msg::{
    CreateMilestoneMsg, CreateMsg, EscrowDetailsResponse, ExecuteMsg, InstantiateMsg,
    ListEscrowsResponse, ListMilestonesResponse, QueryMsg, ReceiveMsg,
};
use crate::state::{all_escrow_ids, get_escrow_by_id, Escrow, GenericBalance, Milestone, ESCROWS};

// version info for migration info
const CONTRACT_NAME: &str = "crates.io:cw20-escrow-milestones";
const CONTRACT_VERSION: &str = env!("CARGO_PKG_VERSION");

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn instantiate(
    deps: DepsMut,
    _env: Env,
    _info: MessageInfo,
    _msg: InstantiateMsg,
) -> StdResult<Response> {
    set_contract_version(deps.storage, CONTRACT_NAME, CONTRACT_VERSION)?;
    // No setup required aside from contract version
    Ok(Response::default())
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn execute(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    msg: ExecuteMsg,
) -> Result<Response, ContractError> {
    match msg {
        ExecuteMsg::Create(msg) => {
            execute_create(deps, msg, info.clone(), Balance::from(info.funds))
        }
        ExecuteMsg::CreateMilestone(msg) => {
            execute_create_milestone(deps, msg, info.clone(), Balance::from(info.funds))
        }
        ExecuteMsg::SetRecipient { id, recipient } => {
            execute_set_recipient(deps, env, info, id, recipient)
        }
        ExecuteMsg::ApproveMilestone { id, milestone_id } => {
            execute_approve_milestone(deps, env, info, id, milestone_id)
        }
        ExecuteMsg::ExtendMilestone {
            id,
            milestone_id,
            end_height,
            end_time,
        } => execute_extend_milestone(deps, env, info, id, milestone_id, end_height, end_time),
        ExecuteMsg::Refund { id } => execute_refund(deps, env, info, id),
        ExecuteMsg::Receive(msg) => execute_receive(deps, info, msg),
    }
}

pub fn execute_receive(
    deps: DepsMut,
    info: MessageInfo,
    wrapper: Cw20ReceiveMsg,
) -> Result<Response, ContractError> {
    let msg: ReceiveMsg = from_binary(&wrapper.msg)?;
    let api = deps.api;
    let validated_sender = api.addr_validate(&wrapper.sender)?;
    let balance = Balance::Cw20(Cw20CoinVerified {
        address: validated_sender,
        amount: wrapper.amount,
    });
    match msg {
        ReceiveMsg::Create(msg) => execute_create(deps, msg, info, balance),
        ReceiveMsg::CreateMilestone(msg) => execute_create_milestone(deps, msg, info, balance),
    }
}

pub fn execute_create(
    deps: DepsMut,
    msg: CreateMsg,
    info: MessageInfo,
    balance: Balance,
) -> Result<Response, ContractError> {
    // check to make sure at least one milestone exists
    if msg.milestones.is_empty() {
        return Err(ContractError::EmptyMilestones {});
    }

    // check to make sure at least one milestone contains a balance
    if msg.is_total_balance_empty() {
        return Err(ContractError::EmptyBalance {});
    }

    // check to make sure the total balance of all milestones is equal to the funds sent
    // only checks the first token for each type
    if !msg.is_deposit_equal_to_milestones_balance(balance.clone()) {
        return Err(ContractError::FundsMismatch {});
    }

    // setup escrow properties
    let arbiter: Addr = deps.as_ref().api.addr_validate(&msg.arbiter)?;
    let recipient: Option<Addr> = msg
        .clone()
        .recipient
        .and_then(|addr| deps.api.addr_validate(&addr).ok());
    let mut cw20_whitelist = msg.addr_whitelist(deps.api)?;
    let balance = match balance {
        Balance::Native(balance) => GenericBalance {
            native: balance.0,
            cw20: vec![],
        },
        Balance::Cw20(token) => {
            // make sure the token sent is on the whitelist by default
            if !cw20_whitelist.iter().any(|t| t == &token.address) {
                cw20_whitelist.push(token.address.clone())
            }
            GenericBalance {
                native: vec![],
                cw20: vec![token],
            }
        }
    };
    let end_time = msg.get_end_time();
    let end_height = msg.get_end_height();

    // create the escrow
    let mut escrow = Escrow {
        arbiter,
        recipient,
        source: info.sender.clone(),
        title: msg.title,
        description: msg.description,
        end_height,
        end_time,
        balance,
        cw20_whitelist,
        milestones: vec![],
    };

    // add the milestones to the escrow
    for milestone in msg.milestones {
        escrow.create_milestone(milestone);
    }

    // try to store the escrow, fail if the id was already in use
    ESCROWS.update(deps.storage, &msg.id, |existing| match existing {
        None => Ok(escrow),
        Some(_) => Err(ContractError::AlreadyInUse {}),
    })?;

    let res = Response::new().add_attributes(vec![("action", "create"), ("id", msg.id.as_str())]);
    Ok(res)
}

pub fn execute_create_milestone(
    deps: DepsMut,
    msg: CreateMilestoneMsg,
    info: MessageInfo,
    amount: Balance,
) -> Result<Response, ContractError> {
    let mut escrow = get_escrow_by_id(&deps.as_ref(), &msg.escrow_id)?;

    // Ensure sender is authorized
    if info.sender.clone() != escrow.arbiter {
        return Err(ContractError::Unauthorized {});
    }
    // Ensure milestone balance is not empty
    if msg.amount.native.is_empty() && msg.amount.cw20.is_empty() {
        return Err(ContractError::EmptyBalance {});
    }

    let mut cw20_whitelist = escrow.cw20_whitelist;
    let _amount = match amount {
        Balance::Native(token) => GenericBalance {
            native: token.0,
            cw20: vec![],
        },
        Balance::Cw20(token) => {
            // make sure the token sent is on the whitelist, otherwise throw an error
            if !cw20_whitelist.iter().any(|t| t == &token.address) {
                cw20_whitelist.push(token.clone().address)
            }
            GenericBalance {
                native: vec![],
                cw20: vec![token],
            }
        }
    };
    escrow.cw20_whitelist = cw20_whitelist;

    // Create new milestone and add to escrow
    escrow.create_milestone(msg.clone());
    let next_id: String = escrow.milestones.len().to_string();

    // Update escrow balance and expiration
    escrow.update_calculated_properties();

    // Save changes to escrow
    ESCROWS.save(deps.storage, &msg.escrow_id, &escrow)?;

    Ok(Response::new().add_attributes(vec![
        ("action", "create_milestone"),
        ("escrow_id", msg.escrow_id.as_str()),
        ("milestone_id", &next_id),
    ]))
}

pub fn execute_set_recipient(
    deps: DepsMut,
    _env: Env,
    info: MessageInfo,
    id: String,
    recipient: String,
) -> Result<Response, ContractError> {
    let mut escrow = get_escrow_by_id(&deps.as_ref(), &id)?;

    if info.sender != escrow.arbiter {
        return Err(ContractError::Unauthorized {});
    }

    let validated_recipient = validate_recipient(&deps, &recipient)?;
    escrow.recipient = Some(validated_recipient.clone());

    ESCROWS.save(deps.storage, &id, &escrow)?;

    Ok(Response::new().add_attributes(vec![
        ("action", "set_recipient"),
        ("id", id.as_str()),
        ("recipient", validated_recipient.as_str()),
    ]))
}

fn validate_recipient(deps: &DepsMut, recipient: &String) -> Result<Addr, ContractError> {
    match deps.api.addr_validate(recipient.as_str()) {
        Ok(addr) => Ok(addr),
        Err(_) => return Err(ContractError::InvalidAddress {}),
    }
}

pub fn execute_approve_milestone(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    id: String,
    milestone_id: String,
) -> Result<Response, ContractError> {
    // fails if escrow doesn't exist
    let mut escrow = get_escrow_by_id(&deps.as_ref(), &id)?;

    if info.sender != escrow.arbiter {
        return Err(ContractError::Unauthorized {});
    }
    if escrow.is_expired(&env) {
        return Err(ContractError::Expired {});
    }

    let milestone = escrow
        .milestones
        .iter_mut()
        .find(|m| m.id == milestone_id)
        .ok_or(ContractError::MilestoneNotFound {})?;

    if milestone.is_expired(&env) {
        return Err(ContractError::MilestoneExpired {});
    }

    milestone.is_completed = true;

    // send milestone amount to recipient in a submessage
    let recipient = escrow
        .recipient
        .as_ref()
        .ok_or(ContractError::RecipientNotSet {})?;
    let messages: Vec<SubMsg> = send_tokens(&recipient, &milestone.amount)?;

    // if last milestone, send escrow balance to recipient and delete escrow using the approve function
    // otherwise, just save the escrow
    if escrow.is_complete() {
        let approve_messages = execute_approve(deps, env, info, id.clone())?;

        println!("\n approve_res: {:?}\n", approve_messages);

        Ok(Response::new()
            .add_attribute("action", "approve_milestone")
            .add_attribute("id", id.as_str())
            .add_attribute("is_escrow_complete", "true")
            .add_submessages(approve_messages))
    } else {
        escrow.update_calculated_properties();

        ESCROWS.save(deps.storage, &id, &escrow)?;

        Ok(Response::new()
            .add_attributes(vec![
                ("action", "approve_milestone"),
                ("id", id.as_str()),
                ("milestone_id", milestone_id.as_str()),
            ])
            .add_submessages(messages))
    }
}

pub fn execute_extend_milestone(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    id: String,
    milestone_id: String,
    end_height: Option<u64>,
    end_time: Option<u64>,
) -> Result<Response, ContractError> {
    // fails if escrow doesn't exist
    let mut escrow = get_escrow_by_id(&deps.as_ref(), &id)?;

    if info.sender != escrow.arbiter {
        return Err(ContractError::Unauthorized {});
    }

    let milestone = escrow
        .milestones
        .iter_mut()
        .find(|m| m.id == milestone_id)
        .ok_or(ContractError::MilestoneNotFound {})?;

    if milestone.is_expired(&env) {
        return Err(ContractError::MilestoneExpired {});
    }

    if let Some(end_height) = end_height {
        milestone.end_height = Some(end_height);
    }
    if let Some(end_time) = end_time {
        milestone.end_time = Some(end_time);
    }

    // Update escrow balance and expiration
    escrow.update_calculated_properties();

    ESCROWS.save(deps.storage, &id, &escrow)?;

    Ok(Response::new().add_attributes(vec![
        ("action", "extend_milestone"),
        ("id", id.as_str()),
        ("milestone_id", milestone_id.as_str()),
    ]))
}

pub fn execute_refund(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    id: String,
) -> Result<Response, ContractError> {
    // this fails is no escrow there
    let escrow = get_escrow_by_id(&deps.as_ref(), &id)?;

    // the arbiter can send anytime OR anyone can send after expiration
    if !escrow.is_expired(&env) && info.sender != escrow.arbiter {
        Err(ContractError::Unauthorized {})
    } else {
        // we delete the escrow
        ESCROWS.remove(deps.storage, &id);

        // send all tokens out
        let messages = send_tokens(&escrow.source, &escrow.get_remaining_balance())?;

        Ok(Response::new()
            .add_attribute("action", "refund")
            .add_attribute("id", id)
            .add_attribute("to", escrow.source)
            .add_submessages(messages))
    }
}

fn execute_approve(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    id: String,
) -> Result<Vec<SubMsg>, ContractError> {
    // fails if escrow doesn't exist
    let escrow = get_escrow_by_id(&deps.as_ref(), &id)?;

    if info.sender != escrow.arbiter {
        return Err(ContractError::Unauthorized {});
    }
    if escrow.is_expired(&env) {
        return Err(ContractError::Expired {});
    }

    let recipient = escrow
        .clone()
        .recipient
        .ok_or(ContractError::RecipientNotSet {})?;

    // we delete the escrow
    ESCROWS.remove(deps.storage, &id);

    // send all tokens out
    let messages: Vec<SubMsg> = send_tokens(&recipient, &escrow.get_remaining_balance())?;

    Ok(messages)
}

fn send_tokens(to: &Addr, balance: &GenericBalance) -> StdResult<Vec<SubMsg>> {
    let native_balance = &balance.native;
    let mut msgs: Vec<SubMsg> = if native_balance.is_empty() {
        vec![]
    } else {
        vec![SubMsg::new(BankMsg::Send {
            to_address: to.into(),
            amount: native_balance.to_vec(),
        })]
    };

    let cw20_balance = &balance.cw20;
    let cw20_msgs: StdResult<Vec<_>> = cw20_balance
        .iter()
        .map(|c| {
            let msg = Cw20ExecuteMsg::Transfer {
                recipient: to.into(),
                amount: c.amount,
            };
            let exec = SubMsg::new(WasmMsg::Execute {
                contract_addr: c.address.to_string(),
                msg: to_binary(&msg)?,
                funds: vec![],
            });
            Ok(exec)
        })
        .collect();
    msgs.append(&mut cw20_msgs?);
    Ok(msgs)
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn query(deps: Deps, _env: Env, msg: QueryMsg) -> StdResult<Binary> {
    match msg {
        QueryMsg::List {} => to_binary(&query_list(deps)?),
        QueryMsg::EscrowDetails { id } => to_binary(&query_escrow_details(deps, id)?),
        QueryMsg::MilestoneDetails { id, milestone_id } => {
            to_binary(&query_milestone_details(deps, id, milestone_id)?)
        }
        QueryMsg::ListMilestones { id } => to_binary(&query_list_milestones(deps, id)?),
    }
}

pub fn query_escrow_details(deps: Deps, id: String) -> StdResult<EscrowDetailsResponse> {
    let escrow = ESCROWS.load(deps.storage, &id)?;

    let cw20_whitelist = escrow.human_whitelist();

    // transform tokens
    let native_balance = escrow.balance.native;

    let cw20_balance: StdResult<Vec<_>> = escrow
        .balance
        .cw20
        .into_iter()
        .map(|token| {
            Ok(Cw20Coin {
                address: token.address.into(),
                amount: token.amount,
            })
        })
        .collect();

    let recipient = escrow.recipient.map(|addr| addr.into_string());

    let details = EscrowDetailsResponse {
        id,
        arbiter: escrow.arbiter.into(),
        recipient,
        source: escrow.source.into(),
        title: escrow.title,
        description: escrow.description,
        end_height: escrow.end_height,
        end_time: escrow.end_time,
        native_balance,
        cw20_balance: cw20_balance?,
        cw20_whitelist,
        milestones: escrow.milestones,
    };
    Ok(details)
}

pub fn query_milestone_details(
    deps: Deps,
    id: String,
    milestone_id: String,
) -> StdResult<Milestone> {
    let escrow = ESCROWS.load(deps.storage, &id)?;
    let milestone = escrow
        .get_milestone_by_id(&milestone_id)
        .ok_or_else(|| StdError::generic_err("Milestone not found"))?;
    Ok(milestone.to_owned())
}

pub fn query_list(deps: Deps) -> StdResult<ListEscrowsResponse> {
    Ok(ListEscrowsResponse {
        escrows: all_escrow_ids(deps.storage)?,
    })
}

pub fn query_list_milestones(deps: Deps, id: String) -> StdResult<ListMilestonesResponse> {
    let escrow = get_escrow_by_id(&deps, &id)
        .map_err(|err| StdError::generic_err(format!("Error: {:?}", err)))?;
    Ok(ListMilestonesResponse {
        milestones: escrow.milestones.iter().map(|m| m.id.clone()).collect(),
    })
}
