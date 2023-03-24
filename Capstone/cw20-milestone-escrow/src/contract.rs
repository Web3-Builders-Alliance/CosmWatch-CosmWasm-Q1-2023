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
            execute_create(deps, msg, &info.sender, Balance::from(info.funds))
        }
        ExecuteMsg::CreateMilestone(msg) => {
            execute_create_milestone(deps, msg, &info.sender, Balance::from(info.funds))
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
        ExecuteMsg::Receive(msg) => execute_receive(deps, env, info, msg),
    }
}

pub fn execute_receive(
    deps: DepsMut,
    _env: Env,
    info: MessageInfo,
    wrapper: Cw20ReceiveMsg,
) -> Result<Response, ContractError> {
    let msg: ReceiveMsg = from_binary(&wrapper.msg)?;
    let api = deps.api;
    let validated_sender = api.addr_validate(&wrapper.sender)?;
    let cw20_balance = Balance::Cw20(Cw20CoinVerified {
        address: info.sender,
        amount: wrapper.amount,
    });
    match msg {
        ReceiveMsg::Create(msg) => execute_create(deps, msg, &validated_sender, cw20_balance),
        ReceiveMsg::CreateMilestone(msg) => {
            execute_create_milestone(deps, msg, &validated_sender, cw20_balance)
        }
    }
}

pub fn execute_create(
    deps: DepsMut,
    msg: CreateMsg,
    sender: &Addr,
    amount: Balance,
) -> Result<Response, ContractError> {
    // check to make sure at least one milestone exists
    if msg.milestones.is_empty() {
        return Err(ContractError::EmptyMilestones {});
    }

    // check to make sure at least one milestone contains a balance
    if msg.total_balance_is_empty() {
        return Err(ContractError::EmptyBalance {});
    }

    // check to make sure the total balance is equal to the amount sent
    if msg.total_balance_is_valid(amount) {
        return Err(ContractError::FundsMismatch {});
    }

    // setup escrow properties
    let arbiter: Addr = deps.api.addr_validate(&msg.arbiter)?;
    let recipient: Option<Addr> = msg
        .clone()
        .recipient
        .and_then(|addr| deps.api.addr_validate(&addr).ok());
    let balance = msg.total_balance_from_milestones();
    let end_time = msg.get_total_end_time();
    let end_height = msg.get_total_end_height();
    let cw20_whitelist = msg.addr_whitelist(deps.api)?;

    // create the escrow
    let escrow = Escrow {
        arbiter,
        recipient,
        source: sender.clone(),
        title: msg.title,
        description: msg.description,
        end_height,
        end_time,
        balance,
        cw20_whitelist,
        milestones: msg.milestones,
    };

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
    sender: &Addr,
    amount: Balance,
) -> Result<Response, ContractError> {
    let mut escrow = get_escrow_by_id(&deps.as_ref(), &msg.escrow_id)?;

    // Ensure sender is authorized
    if sender.clone() != escrow.arbiter {
        return Err(ContractError::Unauthorized {});
    }
    // Ensure milestone balance is not empty
    if msg.amount.native.is_empty() && msg.amount.cw20.is_empty() {
        return Err(ContractError::EmptyBalance {});
    }

    let mut cw20_whitelist = escrow.cw20_whitelist;
    let balance = match amount {
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
    escrow.cw20_whitelist = cw20_whitelist;

    // Create new milestone
    let next_id = escrow.milestones.len() + 1;

    // Add milestone to escrow
    escrow.create_milestone(
        next_id.to_string(),
        msg.title,
        msg.description,
        balance,
        msg.end_height,
        msg.end_time,
    );

    // Update escrow balance and expiration
    escrow.update_calculated_properties();

    // Save changes to escrow
    ESCROWS.save(deps.storage, &msg.escrow_id, &escrow)?;

    Ok(Response::new().add_attributes(vec![
        ("action", "create_milestone"),
        ("escrow_id", msg.escrow_id.as_str()),
        ("milestone_id", &next_id.to_string()),
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
        Ok(execute_approve(deps, env, info, id.clone())?)
    } else {
        ESCROWS.save(deps.storage, &id, &escrow)?;

        Ok(Response::new()
            .add_submessages(messages)
            .add_attributes(vec![
                ("action", "approve_milestone"),
                ("id", id.as_str()),
                ("milestone_id", milestone_id.as_str()),
            ]))
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
        let messages = send_tokens(&escrow.source, &escrow.balance)?;

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
) -> Result<Response, ContractError> {
    // fails if escrow doesn't exist
    let escrow = get_escrow_by_id(&deps.as_ref(), &id)?;

    if info.sender != escrow.arbiter {
        return Err(ContractError::Unauthorized {});
    }
    if escrow.is_expired(&env) {
        return Err(ContractError::Expired {});
    }

    let recipient = escrow.recipient.ok_or(ContractError::RecipientNotSet {})?;

    // we delete the escrow
    ESCROWS.remove(deps.storage, &id);

    // send all tokens out
    let messages: Vec<SubMsg> = send_tokens(&recipient, &escrow.balance)?;

    Ok(Response::new()
        .add_attribute("action", "approve")
        .add_attribute("id", id)
        .add_attribute("to", recipient)
        .add_submessages(messages))
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

fn query_escrow_details(deps: Deps, id: String) -> StdResult<EscrowDetailsResponse> {
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

fn query_milestone_details(deps: Deps, id: String, milestone_id: String) -> StdResult<Milestone> {
    let escrow = ESCROWS.load(deps.storage, &id)?;
    let milestone = escrow
        .get_milestone_by_id(&milestone_id)
        .ok_or_else(|| StdError::generic_err("Milestone not found"))?;
    Ok(milestone.to_owned())
}

fn query_list(deps: Deps) -> StdResult<ListEscrowsResponse> {
    Ok(ListEscrowsResponse {
        escrows: all_escrow_ids(deps.storage)?,
    })
}

fn query_list_milestones(deps: Deps, id: String) -> StdResult<ListMilestonesResponse> {
    let escrow = get_escrow_by_id(&deps, &id)
        .map_err(|err| StdError::generic_err(format!("Error: {:?}", err)))?;
    Ok(ListMilestonesResponse {
        milestones: escrow.milestones.iter().map(|m| m.id.clone()).collect(),
    })
}

#[cfg(test)]
mod tests {
    use cosmwasm_std::testing::{mock_dependencies, mock_env, mock_info};
    use cosmwasm_std::{attr, coin, coins, CosmosMsg, StdError, Uint128};

    use super::*;

    const ANYONE: &str = "anyone";
    const ARBITER: &str = "arbiter";
    // const SOURCE: &str = "source";
    const RECIPIENT: &str = "recipient";
    // const ADDR1: &str = "addr0001";
    // const ADDR2: &str = "addr0002";
    // const ADDR3: &str = "addr0003";

    #[test]
    fn test_instantiate() {
        let mut deps = mock_dependencies();
        let env = mock_env();
        let info = mock_info("creator", &coins(1000, "native"));

        let res = instantiate(deps.as_mut(), env, info, InstantiateMsg {}).unwrap();
        assert_eq!(0, res.messages.len());
    }

    /**
     * Test create escrow with one milestone
     * - Native tokens
     * - No expiration
     */
    #[test]
    fn test_create() {
        let mut deps = mock_dependencies();

        // instantiate an empty contract
        let instantiate_msg = InstantiateMsg {};
        let info = mock_info(&ANYONE, &[]);
        let res = instantiate(deps.as_mut(), mock_env(), info, instantiate_msg).unwrap();
        assert_eq!(0, res.messages.len());

        // create milestones
        let milestones = vec![Milestone {
            id: "milestone1".to_string(),
            title: "some_title".to_string(),
            description: "some_description".to_string(),
            amount: GenericBalance {
                native: vec![coin(100, "tokens")],
                cw20: vec![],
            },
            end_height: None,
            end_time: None,
            is_completed: false,
        }];

        // create an escrow
        let create_msg = CreateMsg {
            id: "foobar".to_string(),
            arbiter: ARBITER.to_string(),
            recipient: Some(RECIPIENT.to_string()),
            title: "some_title".to_string(),
            end_time: None,
            end_height: None,
            cw20_whitelist: None,
            description: "some_description".to_string(),
            milestones,
        };
        let sender = ARBITER.to_string();
        let balance = coins(100, "tokens");
        let info = mock_info(&sender, &balance);
        let msg = ExecuteMsg::Create(create_msg.clone());
        let res = execute(deps.as_mut(), mock_env(), info, msg).unwrap();
        assert_eq!(0, res.messages.len());
        assert_eq!(("action", "create"), res.attributes[0]);

        // ensure the details is what we expect
        let details = query_escrow_details(deps.as_ref(), "foobar".to_string()).unwrap();
        assert_eq!(
            details,
            EscrowDetailsResponse {
                id: "foobar".to_string(),
                arbiter: String::from("arbitrate"),
                recipient: Some(String::from("recd")),
                source: String::from("source"),
                title: "some_title".to_string(),
                description: "some_description".to_string(),
                end_height: Some(123456),
                end_time: None,
                native_balance: balance.clone(),
                cw20_balance: vec![],
                cw20_whitelist: vec![],
                milestones: vec![],
            }
        );

        // approve it
        let id = create_msg.id.clone();
        let milestone_id = create_msg.milestones[0].id.clone();
        let info = mock_info(&create_msg.arbiter, &[]);
        let res = execute(
            deps.as_mut(),
            mock_env(),
            info,
            ExecuteMsg::ApproveMilestone { id, milestone_id },
        )
        .unwrap();
        assert_eq!(1, res.messages.len());
        assert_eq!(("action", "approve"), res.attributes[0]);
        assert_eq!(
            res.messages[0],
            SubMsg::new(CosmosMsg::Bank(BankMsg::Send {
                to_address: create_msg.recipient.unwrap(),
                amount: balance,
            }))
        );

        // second attempt fails (not found)
        let id = create_msg.id.clone();
        let milestone_id = &create_msg.milestones[0].id;
        let info = mock_info(&create_msg.arbiter, &[]);
        let err = execute(
            deps.as_mut(),
            mock_env(),
            info,
            ExecuteMsg::ApproveMilestone {
                id,
                milestone_id: milestone_id.into(),
            },
        )
        .unwrap_err();
        assert!(matches!(err, ContractError::Std(StdError::NotFound { .. })));
    }

    /**
     * Test empty milestones error
     */
    #[test]
    fn test_create_empty_milestones() {
        let mut deps = mock_dependencies();
        let env = mock_env();
        let info = mock_info("creator", &coins(2, "token"));

        let msg = ExecuteMsg::Create(CreateMsg {
            id: "escrow1".to_string(),
            arbiter: "arbiter".to_string(),
            recipient: Some("recipient".to_string()),
            title: "Title".to_string(),
            description: "Description".to_string(),
            end_height: None,
            end_time: None,
            cw20_whitelist: None,
            milestones: vec![],
        });

        let res = execute(deps.as_mut(), env.clone(), info.clone(), msg);
        assert!(matches!(res, Err(ContractError::EmptyMilestones {})));
    }

    /**
     * Test create escrow with multiple milestones
     * - Native tokens
     * - No expiration
     */
    #[test]
    fn test_create_valid_milestones() {
        let mut deps = mock_dependencies();
        let env = mock_env();
        let info = mock_info("creator", &coins(2, "token"));

        let msg = ExecuteMsg::Create(CreateMsg {
            id: "escrow1".to_string(),
            arbiter: ARBITER.to_string(),
            recipient: Some(RECIPIENT.to_string()),
            title: "Title".to_string(),
            description: "Description".to_string(),
            end_height: None,
            end_time: None,
            cw20_whitelist: None,
            milestones: vec![
                Milestone {
                    id: "1".to_string(),
                    amount: GenericBalance {
                        native: coins(1, "token"),
                        cw20: vec![],
                    },
                    title: "title1".to_string(),
                    description: "description1".to_string(),
                    end_height: None,
                    end_time: None,
                    is_completed: false,
                },
                Milestone {
                    id: "2".to_string(),
                    amount: GenericBalance {
                        native: coins(1, "token"),
                        cw20: vec![],
                    },
                    title: "title2".to_string(),
                    description: "description2".to_string(),
                    end_height: None,
                    end_time: None,
                    is_completed: false,
                },
            ],
        });

        let res = execute(deps.as_mut(), env.clone(), info.clone(), msg).unwrap();
        assert_eq!(1, res.messages.len());
    }

    #[test]
    fn test_query_escrow() {
        let mut deps = mock_dependencies(&[]);
        let env = mock_env();
        let info = mock_info("creator", &coins(2, "token"));

        // Create a new escrow
        let msg = ExecuteMsg::Create(CreateMsg {
            id: "escrow1".to_string(),
            arbiter: ARBITER.to_string(),
            recipient: Some(RECIPIENT.to_string()),
            title: "Title".to_string(),
            description: "Description".to_string(),
            cw20_whitelist: None,
            milestones: vec![
                Milestone {
                    id: "1".to_string(),
                    amount: GenericBalance {
                        native: coins(1, "token"),
                        cw20: vec![],
                    },
                    title: "title1".to_string(),
                    description: "description1".to_string(),
                    end_height: None,
                    end_time: None,
                    is_completed: false,
                },
                Milestone {
                    id: "2".to_string(),
                    amount: GenericBalance {
                        native: coins(1, "token"),
                        cw20: vec![],
                    },
                    title: "title2".to_string(),
                    description: "description2".to_string(),
                    end_height: None,
                    end_time: None,
                    is_completed: false,
                },
            ],
        });

        let _res = execute(deps.as_mut(), env.clone(), info.clone(), msg).unwrap();

        // Query the created escrow
        let query_msg = QueryMsg::EscrowDetails {
            id: "escrow1".to_string(),
        };
        let query_res = query(deps.as_ref(), env.clone(), query_msg).unwrap();
        let escrow: EscrowDetailsResponse = from_binary(&query_res).unwrap();
        assert_eq!("escrow1", escrow.id);
        assert_eq!("Title", escrow.title);
        assert_eq!("Description", escrow.description);
        assert_eq!(2, escrow.milestones.len());
        assert_eq!(ARBITER, escrow.arbiter)
    }

    /**
     * OLD TESTS
     */
    #[test]
    fn happy_path_cw20() {
        let mut deps = mock_dependencies();

        // instantiate an empty contract
        let instantiate_msg = InstantiateMsg {};
        let info = mock_info(&ANYONE, &[]);
        let res = instantiate(deps.as_mut(), mock_env(), info, instantiate_msg).unwrap();
        assert_eq!(0, res.messages.len());

        // create an escrow
        let create = CreateMsg {
            id: "foobar".to_string(),
            arbiter: String::from("arbitrate"),
            recipient: Some(String::from("recd")),
            title: "some_title".to_string(),
            end_time: None,
            end_height: None,
            cw20_whitelist: Some(vec![String::from("other-token")]),
            description: "some_description".to_string(),
            milestones: vec![],
        };
        let receive = Cw20ReceiveMsg {
            sender: String::from("source"),
            amount: Uint128::new(100),
            msg: to_binary(&ExecuteMsg::Create(create.clone())).unwrap(),
        };
        let token_contract = String::from("my-cw20-token");
        let info = mock_info(&token_contract, &[]);
        let msg = ExecuteMsg::Receive(receive.clone());
        let res = execute(deps.as_mut(), mock_env(), info, msg).unwrap();
        assert_eq!(0, res.messages.len());
        assert_eq!(("action", "create"), res.attributes[0]);

        // ensure the whitelist is what we expect
        let details = query_escrow_details(deps.as_ref(), "foobar".to_string()).unwrap();
        assert_eq!(
            details,
            EscrowDetailsResponse {
                id: "foobar".to_string(),
                arbiter: String::from("arbitrate"),
                recipient: Some(String::from("recd")),
                source: String::from("source"),
                title: "some_title".to_string(),
                description: "some_description".to_string(),
                end_height: None,
                end_time: None,
                native_balance: vec![],
                cw20_balance: vec![Cw20Coin {
                    address: String::from("my-cw20-token"),
                    amount: Uint128::new(100),
                }],
                cw20_whitelist: vec![String::from("other-token"), String::from("my-cw20-token")],
                milestones: vec![],
            }
        );

        // approve it
        let id = create.id.clone();
        let milestone_id = create.milestones[0].id.clone();
        let info = mock_info(&create.arbiter, &[]);
        let res = execute(
            deps.as_mut(),
            mock_env(),
            info,
            ExecuteMsg::ApproveMilestone {
                id,
                milestone_id: milestone_id.clone(),
            },
        )
        .unwrap();
        assert_eq!(1, res.messages.len());
        assert_eq!(("action", "approve"), res.attributes[0]);
        let send_msg = Cw20ExecuteMsg::Transfer {
            recipient: create.recipient.unwrap(),
            amount: receive.amount,
        };
        assert_eq!(
            res.messages[0],
            SubMsg::new(CosmosMsg::Wasm(WasmMsg::Execute {
                contract_addr: token_contract,
                msg: to_binary(&send_msg).unwrap(),
                funds: vec![]
            }))
        );

        // second attempt fails (not found)
        let id = create.id.clone();
        let info = mock_info(&create.arbiter, &[]);
        let err = execute(
            deps.as_mut(),
            mock_env(),
            info,
            ExecuteMsg::ApproveMilestone { id, milestone_id },
        )
        .unwrap_err();
        assert!(matches!(err, ContractError::Std(StdError::NotFound { .. })));
    }

    #[test]
    fn set_recipient_after_creation() {
        let mut deps = mock_dependencies();

        // instantiate an empty contract
        let instantiate_msg = InstantiateMsg {};
        let info = mock_info(&ANYONE, &[]);
        let res = instantiate(deps.as_mut(), mock_env(), info, instantiate_msg).unwrap();
        assert_eq!(0, res.messages.len());

        // create an escrow
        let create = CreateMsg {
            id: "foobar".to_string(),
            arbiter: String::from("arbitrate"),
            recipient: None,
            title: "some_title".to_string(),
            end_time: None,
            end_height: Some(123456),
            cw20_whitelist: None,
            description: "some_description".to_string(),
            milestones: vec![],
        };
        let sender = String::from("source");
        let balance = coins(100, "tokens");
        let info = mock_info(&sender, &balance);
        let msg = ExecuteMsg::Create(create.clone());
        let res = execute(deps.as_mut(), mock_env(), info, msg).unwrap();
        assert_eq!(0, res.messages.len());
        assert_eq!(("action", "create"), res.attributes[0]);

        // ensure the details is what we expect
        let details = query_escrow_details(deps.as_ref(), "foobar".to_string()).unwrap();
        assert_eq!(
            details,
            EscrowDetailsResponse {
                id: "foobar".to_string(),
                arbiter: String::from("arbitrate"),
                recipient: None,
                source: String::from("source"),
                title: "some_title".to_string(),
                description: "some_description".to_string(),
                end_height: Some(123456),
                end_time: None,
                native_balance: balance.clone(),
                cw20_balance: vec![],
                cw20_whitelist: vec![],
                milestones: vec![],
            }
        );

        // approve it, should fail as we have not set recipient
        let id = create.id.clone();
        let milestone_id = create.milestones[0].id.clone();
        let info = mock_info(&create.arbiter, &[]);
        let res = execute(
            deps.as_mut(),
            mock_env(),
            info,
            ExecuteMsg::ApproveMilestone { id, milestone_id },
        );
        match res {
            Err(ContractError::RecipientNotSet {}) => {}
            _ => panic!("Expect recipient not set error"),
        }

        // test setting recipient not arbiter
        let msg = ExecuteMsg::SetRecipient {
            id: create.id.clone(),
            recipient: "recp".to_string(),
        };
        let info = mock_info("someoneelse", &[]);
        let res = execute(deps.as_mut(), mock_env(), info, msg);
        match res {
            Err(ContractError::Unauthorized {}) => {}
            _ => panic!("Expect unauthorized error"),
        }

        // test setting recipient valid
        let msg = ExecuteMsg::SetRecipient {
            id: create.id.clone(),
            recipient: "recp".to_string(),
        };
        let info = mock_info(&create.arbiter, &[]);
        let res = execute(deps.as_mut(), mock_env(), info, msg).unwrap();
        assert_eq!(res.messages.len(), 0);
        assert_eq!(
            res.attributes,
            vec![
                attr("action", "set_recipient"),
                attr("id", create.id.as_str()),
                attr("recipient", "recp")
            ]
        );

        // approve it, should now work with recp
        let id = create.id.clone();
        let milestone_id = create.milestones[0].id.clone();
        let info = mock_info(&create.arbiter, &[]);
        let res = execute(
            deps.as_mut(),
            mock_env(),
            info,
            ExecuteMsg::ApproveMilestone { id, milestone_id },
        )
        .unwrap();
        assert_eq!(1, res.messages.len());
        assert_eq!(("action", "approve"), res.attributes[0]);
        assert_eq!(
            res.messages[0],
            SubMsg::new(CosmosMsg::Bank(BankMsg::Send {
                to_address: "recp".to_string(),
                amount: balance,
            }))
        );
    }

    #[test]
    fn add_tokens_proper() {
        let mut tokens = GenericBalance::default();
        tokens.add_tokens(Balance::from(vec![coin(123, "atom"), coin(789, "eth")]));
        tokens.add_tokens(Balance::from(vec![coin(456, "atom"), coin(12, "btc")]));
        assert_eq!(
            tokens.native,
            vec![coin(579, "atom"), coin(789, "eth"), coin(12, "btc")]
        );
    }

    #[test]
    fn add_cw_tokens_proper() {
        let mut tokens = GenericBalance::default();
        let bar_token = Addr::unchecked("bar_token");
        let foo_token = Addr::unchecked("foo_token");
        tokens.add_tokens(Balance::Cw20(Cw20CoinVerified {
            address: foo_token.clone(),
            amount: Uint128::new(12345),
        }));
        tokens.add_tokens(Balance::Cw20(Cw20CoinVerified {
            address: bar_token.clone(),
            amount: Uint128::new(777),
        }));
        tokens.add_tokens(Balance::Cw20(Cw20CoinVerified {
            address: foo_token.clone(),
            amount: Uint128::new(23400),
        }));
        assert_eq!(
            tokens.cw20,
            vec![
                Cw20CoinVerified {
                    address: foo_token,
                    amount: Uint128::new(35745),
                },
                Cw20CoinVerified {
                    address: bar_token,
                    amount: Uint128::new(777),
                }
            ]
        );
    }
}
