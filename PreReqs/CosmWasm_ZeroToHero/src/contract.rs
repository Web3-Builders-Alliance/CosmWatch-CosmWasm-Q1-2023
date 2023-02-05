#[cfg(not(feature = "library"))]
use cosmwasm_std::entry_point;
use cosmwasm_std::{
    to_binary, Binary, Deps, DepsMut, Env, MessageInfo, Order, Response, StdResult,
};
use cw2::set_contract_version;

use crate::config::{Ballot, Config, Poll, BALLOTS, CONFIG, POLLS};
use crate::error::ContractError;
use crate::msg::{
    AllPollsResponse, ConfigResponse, ExecuteMsg, InstantiateMsg, PollResponse, QueryMsg,
    VoteResponse,
};

// version info for migration info
const CONTRACT_NAME: &str = "crates.io:cosm-wasm-zero2-hero";
const CONTRACT_VERSION: &str = env!("CARGO_PKG_VERSION");

/*
** INSTANTIATE
*/
#[cfg_attr(not(feature = "library"), entry_point)]
pub fn instantiate(
    deps: DepsMut,
    _env: Env,
    info: MessageInfo,
    msg: InstantiateMsg,
) -> Result<Response, ContractError> {
    set_contract_version(deps.storage, CONTRACT_NAME, CONTRACT_VERSION)?;
    let admin = msg.admin.unwrap_or(info.sender.to_string());
    let validated_admin = deps.api.addr_validate(&admin)?;
    let config = Config {
        admin: validated_admin.clone(),
    };
    CONFIG.save(deps.storage, &config)?;
    Ok(Response::new()
        .add_attribute("action", "instantiate")
        .add_attribute("admin", validated_admin.to_string()))
}

/*
** EXECUTE
*/
#[cfg_attr(not(feature = "library"), entry_point)]
pub fn execute(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    msg: ExecuteMsg,
) -> Result<Response, ContractError> {
    match msg {
        ExecuteMsg::CreatePoll {
            poll_id,
            question,
            options,
        } => execute_create_poll(deps, env, info, poll_id, question, options),
        ExecuteMsg::Vote { poll_id, vote } => execute_vote(deps, env, info, poll_id, vote),
    }
}

fn execute_create_poll(
    deps: DepsMut,
    _env: Env,
    info: MessageInfo,
    poll_id: String,
    question: String,
    options: Vec<String>,
) -> Result<Response, ContractError> {
    // Ensure there are no more than 10 options
    if options.len() > 10 {
        return Err(ContractError::TooManyOptions {});
    }

    // Loop over options and add to options vector
    let mut opts: Vec<(String, u64)> = vec![];
    for opt in options {
        opts.push((opt, 0));
    }

    // Create poll and save it to config (aka state)
    let poll = Poll {
        creator: info.sender,
        question,
        options: opts,
    };
    POLLS.save(deps.storage, poll_id, &poll)?;

    Ok(Response::new()
        .add_attribute("action", "create_poll")
        .add_attribute("creator", &poll.creator)
        .add_attribute("question", &poll.question)
        .add_attribute(
            "options",
            poll.options
                .iter()
                .map(|(s, _)| s.to_string())
                .collect::<Vec<String>>()
                .join(", "),
        ))
}

fn execute_vote(
    deps: DepsMut,
    _env: Env,
    info: MessageInfo,
    poll_id: String,
    vote: String,
) -> Result<Response, ContractError> {
    // Get Poll or None from state
    let poll = POLLS.may_load(deps.storage, poll_id.clone())?;

    // Check for poll or None
    match poll {
        // If poll found, update ballot with vote
        Some(mut poll) => {
            BALLOTS.update(
                deps.storage,
                (info.sender, poll_id.clone()),
                |ballot| -> StdResult<Ballot> {
                    match ballot {
                        Some(ballot) => {
                            let position_of_old_vote = poll
                                .options
                                .iter()
                                .position(|option| option.0 == ballot.option)
                                .unwrap();
                            poll.options[position_of_old_vote].1 -= 1;
                            Ok(Ballot {
                                option: vote.clone(),
                            })
                        }
                        None => Ok(Ballot {
                            option: vote.clone(),
                        }),
                    }
                },
            )?;

            let position = poll.options.iter().position(|option| option.0 == vote);

            if position.is_none() {
                return Err(ContractError::Unauthorized {});
            }
            let position = position.unwrap();
            poll.options[position].1 += 1;

            // Save to state
            POLLS.save(deps.storage, poll_id, &poll)?;
            Ok(Response::new()
                .add_attribute("action", "vote")
                .add_attribute("poll", poll.question)
                .add_attribute("vote", vote))
        }
        // If poll not found, return a PollNotFound error
        None => Err(ContractError::PollNotFound {}),
    }
}

/*
** QUERY
*/
#[cfg_attr(not(feature = "library"), entry_point)]
pub fn query(deps: Deps, _env: Env, msg: QueryMsg) -> StdResult<Binary> {
    match msg {
        QueryMsg::Config => to_binary(&query_config(deps)?),
        QueryMsg::AllPolls => to_binary(&query_all_polls(deps)?),
        QueryMsg::Poll { poll_id } => to_binary(&query_poll(deps, poll_id)?),
        QueryMsg::Vote { poll_id, address } => to_binary(&query_vote(deps, poll_id, address)?),
    }
}

pub fn query_config(deps: Deps) -> StdResult<ConfigResponse> {
    let config = CONFIG.load(deps.storage)?;

    Ok(ConfigResponse { config })
}

pub fn query_all_polls(deps: Deps) -> StdResult<AllPollsResponse> {
    let polls = POLLS
        .range(deps.storage, None, None, Order::Ascending)
        .map(|p| Ok(p?.1))
        .collect::<StdResult<Vec<_>>>()?;

    Ok(AllPollsResponse { polls })
}

pub fn query_poll(deps: Deps, poll_id: String) -> StdResult<PollResponse> {
    let poll = POLLS.may_load(deps.storage, poll_id)?;

    Ok(PollResponse { poll })
}

pub fn query_vote(deps: Deps, poll_id: String, address: String) -> StdResult<VoteResponse> {
    let validated_address = deps.api.addr_validate(&address)?;
    let vote = BALLOTS.may_load(deps.storage, (validated_address, poll_id))?;

    Ok(VoteResponse { vote })
}

/*
** TESTS
*/
#[cfg(test)]
mod tests {
    use crate::contract::{execute, instantiate};
    use crate::msg::{
        AllPollsResponse, ConfigResponse, ExecuteMsg, InstantiateMsg, PollResponse, QueryMsg,
        VoteResponse,
    };
    use crate::ContractError;
    use cosmwasm_std::testing::{mock_dependencies, mock_env, mock_info};
    use cosmwasm_std::{attr, from_binary};

    use super::query;

    pub const ADDR1: &str = "addr1";
    pub const ADDR2: &str = "addr2";

    #[test]
    fn test_instantiate() {
        // Define mock dependencies, env, and info
        let mut deps = mock_dependencies();
        let env = mock_env();
        let info = mock_info(ADDR1, &[]);

        // Define message to instantiate contract and call instantiate
        let msg = InstantiateMsg { admin: None };
        let res = instantiate(deps.as_mut(), env, info, msg).unwrap();

        // Check response for success
        assert_eq!(
            res.attributes,
            vec![attr("action", "instantiate"), attr("admin", ADDR1)]
        )
    }

    #[test]
    fn test_instantiate_with_admin() {
        // Define mock dependencies, env, and info
        let mut deps = mock_dependencies();
        let env = mock_env();
        let info = mock_info(ADDR1, &[]);

        // Define message to instantiate contract (with admin this time) and call instantiate
        let msg = InstantiateMsg {
            admin: Some(ADDR2.to_string()),
        };
        let res = instantiate(deps.as_mut(), env, info, msg).unwrap();

        // Check response for success
        assert_eq!(
            res.attributes,
            vec![attr("action", "instantiate"), attr("admin", ADDR2)]
        )
    }

    #[test]
    fn test_execute_create_poll_valid() {
        // Define mock dependencies, env, and info
        let mut deps = mock_dependencies();
        let env = mock_env();
        let info = mock_info(ADDR1, &[]);
        let question = "What is your favorite Cosmos coin?".to_string();

        // Define message to instantiate contract and call instantiate
        let msg = InstantiateMsg { admin: None };
        let _res = instantiate(deps.as_mut(), env.clone(), info.clone(), msg).unwrap();

        let msg = ExecuteMsg::CreatePoll {
            poll_id: "some_id".to_string(),
            question: question.clone(),
            options: vec![
                "Cosmos Hub".to_string(),
                "Juno".to_string(),
                "Osmosis".to_string(),
            ],
        };

        let res = execute(deps.as_mut(), env, info, msg).unwrap();

        for attr in res.attributes.iter() {
            match attr.key.as_str() {
                "action" => assert_eq!(attr.value, "create_poll".to_string()),
                "creator" => assert_eq!(attr.value, ADDR1.to_string()),
                "question" => assert_eq!(attr.value, question.clone()),
                &_ => (),
            }
        }
    }

    #[test]
    fn test_execute_create_poll_invalid() {
        // Define mock dependencies, env, and info
        let mut deps = mock_dependencies();
        let env = mock_env();
        let info = mock_info(ADDR1, &[]);

        // Define message to instantiate contract and call instantiate
        let msg = InstantiateMsg { admin: None };
        let _res = instantiate(deps.as_mut(), env.clone(), info.clone(), msg).unwrap();

        let msg = ExecuteMsg::CreatePoll {
            poll_id: "some_id".to_string(),
            question: "What is your favorite number?".to_string(),
            options: vec![
                "1".to_string(),
                "2".to_string(),
                "3".to_string(),
                "4".to_string(),
                "5".to_string(),
                "6".to_string(),
                "7".to_string(),
                "8".to_string(),
                "9".to_string(),
                "10".to_string(),
                "11".to_string(),
            ],
        };

        let err = execute(deps.as_mut(), env, info, msg).unwrap_err();

        match err {
            ContractError::TooManyOptions {} => {}
            ContractError::Std(_err) => {}
            _ => {}
        }
    }

    #[test]
    fn test_execute_vote_valid() {
        // Define mock dependencies, env, and info
        let mut deps = mock_dependencies();
        let env = mock_env();
        let info = mock_info(ADDR1, &[]);
        let question = "What is your favorite Cosmos coin?".to_string();

        // Define message to instantiate contract and call instantiate
        let msg = InstantiateMsg { admin: None };
        let _res = instantiate(deps.as_mut(), env.clone(), info.clone(), msg).unwrap();

        // Create a poll with valid options
        let msg = ExecuteMsg::CreatePoll {
            poll_id: "some_id".to_string(),
            question: question.clone(),
            options: vec![
                "Cosmos Hub".to_string(),
                "Juno".to_string(),
                "Osmosis".to_string(),
            ],
        };
        let _res = execute(deps.as_mut(), env.clone(), info.clone(), msg).unwrap();

        // Vote on the poll created and expect success
        let msg = ExecuteMsg::Vote {
            poll_id: "some_id".to_string(),
            vote: "Juno".to_string(),
        };
        let _res = execute(deps.as_mut(), env.clone(), info.clone(), msg).unwrap();

        // Update existing vote on the poll and expect success
        let vote = "Cosmos Hub".to_string();
        let msg = ExecuteMsg::Vote {
            poll_id: "some_id".to_string(),
            vote: vote.clone(),
        };
        let res = execute(deps.as_mut(), env, info, msg).unwrap();

        for attr in res.attributes.iter() {
            match attr.key.as_str() {
                "action" => assert_eq!(attr.value, "vote".to_string()),
                "poll" => assert_eq!(attr.value, question.clone()),
                "vote" => assert_eq!(attr.value, vote.clone()),
                &_ => (),
            }
        }
    }

    #[test]
    fn test_execute_vote_invalid() {
        // Define mock dependencies, env, and info
        let mut deps = mock_dependencies();
        let env = mock_env();
        let info = mock_info(ADDR1, &[]);

        // Define message to instantiate contract and call instantiate
        let msg = InstantiateMsg { admin: None };
        let _res = instantiate(deps.as_mut(), env.clone(), info.clone(), msg).unwrap();

        // Create vote without creating a valid poll
        let msg = ExecuteMsg::Vote {
            poll_id: "some_id".to_string(),
            vote: "Juno".to_string(),
        };
        // Unwrap and expect error to assert success
        let _err = execute(deps.as_mut(), env.clone(), info.clone(), msg).unwrap_err();

        // Create a poll
        let msg = ExecuteMsg::CreatePoll {
            poll_id: "some_id".to_string(),
            question: "What's your favourite Cosmos coin?".to_string(),
            options: vec![
                "Cosmos Hub".to_string(),
                "Juno".to_string(),
                "Osmosis".to_string(),
            ],
        };
        let _res = execute(deps.as_mut(), env.clone(), info.clone(), msg).unwrap();

        // Vote on valid poll with an invalid option and expect error to assert success
        let msg = ExecuteMsg::Vote {
            poll_id: "some_id".to_string(),
            vote: "Akash".to_string(),
        };
        let err = execute(deps.as_mut(), env, info, msg).unwrap_err();

        match err {
            ContractError::TooManyOptions {} => {}
            ContractError::Std(_err) => {}
            _ => {}
        }
    }

    #[test]
    fn test_query_all_polls() {
        // Define mock dependencies, env, and info
        let mut deps = mock_dependencies();
        let env = mock_env();
        let info = mock_info(ADDR1, &[]);

        // Define message to instantiate contract and call instantiate
        let msg = InstantiateMsg { admin: None };
        let _res = instantiate(deps.as_mut(), env.clone(), info.clone(), msg).unwrap();

        // Query for polls when no polls have been created
        let msg = QueryMsg::AllPolls;
        let bin = query(deps.as_ref(), env.clone(), msg).unwrap();
        let res: AllPollsResponse = from_binary(&bin).unwrap();
        assert_eq!(res.polls.len(), 0);

        // Create a poll
        let msg = ExecuteMsg::CreatePoll {
            poll_id: "some_id_1".to_string(),
            question: "What's your favourite Cosmos coin?".to_string(),
            options: vec![
                "Cosmos Hub".to_string(),
                "Juno".to_string(),
                "Osmosis".to_string(),
            ],
        };
        let _res = execute(deps.as_mut(), env.clone(), info.clone(), msg).unwrap();

        // Create a second poll
        let msg = ExecuteMsg::CreatePoll {
            poll_id: "some_id_2".to_string(),
            question: "What's your colour?".to_string(),
            options: vec!["Red".to_string(), "Green".to_string(), "Blue".to_string()],
        };
        let _res = execute(deps.as_mut(), env.clone(), info, msg).unwrap();

        let msg = QueryMsg::AllPolls {};
        let bin = query(deps.as_ref(), env, msg).unwrap();
        let res: AllPollsResponse = from_binary(&bin).unwrap();

        assert_eq!(res.polls.len(), 2);
    }

    #[test]
    fn test_query_poll() {
        // Define mock dependencies, env, and info
        let mut deps = mock_dependencies();
        let env = mock_env();
        let info = mock_info(ADDR1, &[]);

        // Define message to instantiate contract and call instantiate
        let msg = InstantiateMsg { admin: None };
        let _res = instantiate(deps.as_mut(), env.clone(), info.clone(), msg).unwrap();

        // Create a poll
        let msg = ExecuteMsg::CreatePoll {
            poll_id: "some_id_1".to_string(),
            question: "What's your favourite Cosmos coin?".to_string(),
            options: vec![
                "Cosmos Hub".to_string(),
                "Juno".to_string(),
                "Osmosis".to_string(),
            ],
        };
        let _res = execute(deps.as_mut(), env.clone(), info, msg).unwrap();

        // Query a valid poll
        let msg = QueryMsg::Poll {
            poll_id: "some_id_1".to_string(),
        };
        let bin = query(deps.as_ref(), env.clone(), msg).unwrap();
        let res: PollResponse = from_binary(&bin).unwrap();
        assert!(res.poll.is_some());

        // Query an invalid poll
        let msg = QueryMsg::Poll {
            poll_id: "some_invalid_id".to_string(),
        };
        let bin = query(deps.as_ref(), env, msg).unwrap();
        let res: PollResponse = from_binary(&bin).unwrap();
        assert!(res.poll.is_none());
    }

    #[test]
    fn test_query_vote() {
        // Define mock dependencies, env, and info
        let mut deps = mock_dependencies();
        let env = mock_env();
        let info = mock_info(ADDR1, &[]);

        // Define message to instantiate contract and call instantiate
        let msg = InstantiateMsg { admin: None };
        let _res = instantiate(deps.as_mut(), env.clone(), info.clone(), msg).unwrap();

        // Create a poll
        let msg = ExecuteMsg::CreatePoll {
            poll_id: "some_id_1".to_string(),
            question: "What's your favourite Cosmos coin?".to_string(),
            options: vec![
                "Cosmos Hub".to_string(),
                "Juno".to_string(),
                "Osmosis".to_string(),
            ],
        };
        let _res = execute(deps.as_mut(), env.clone(), info.clone(), msg).unwrap();

        // Create a vote
        let msg = ExecuteMsg::Vote {
            poll_id: "some_id_1".to_string(),
            vote: "Juno".to_string(),
        };
        let _res = execute(deps.as_mut(), env.clone(), info, msg).unwrap();

        // Query an existing vote and assert its existence
        let msg = QueryMsg::Vote {
            poll_id: "some_id_1".to_string(),
            address: ADDR1.to_string(),
        };
        let bin = query(deps.as_ref(), env.clone(), msg).unwrap();
        let res: VoteResponse = from_binary(&bin).unwrap();
        assert!(res.vote.is_some());

        // Query a non-existent vote and assert its non-existence
        let msg = QueryMsg::Vote {
            poll_id: "some_id_2".to_string(),
            address: ADDR2.to_string(),
        };
        let bin = query(deps.as_ref(), env, msg).unwrap();
        let res: VoteResponse = from_binary(&bin).unwrap();
        assert!(res.vote.is_none());
    }

    #[test]
    fn test_query_config() {
        // Define mock dependencies, env, and info
        let mut deps = mock_dependencies();
        let env = mock_env();
        let info = mock_info(ADDR1, &[]);

        // Define message to instantiate contract and call instantiate
        let msg = InstantiateMsg { admin: None };
        let _res = instantiate(deps.as_mut(), env.clone(), info, msg).unwrap();

        // Query config and assert admin
        let msg = QueryMsg::Config;
        let bin = query(deps.as_ref(), env, msg).unwrap();
        let res: ConfigResponse = from_binary(&bin).unwrap();

        assert_eq!(res.config.admin.to_string(), ADDR1.to_string());
    }
}
