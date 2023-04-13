#[cfg(test)]
mod tests {
    use cosmwasm_std::testing::{mock_dependencies, mock_env, mock_info};
    use cosmwasm_std::{coin, coins, from_binary, BankMsg, Coin, CosmosMsg, SubMsg};
    use cw20::Cw20Coin;

    use crate::contract::{execute, instantiate, query, query_escrow_details};
    use crate::msg::{
        CreateMilestoneMsg, CreateMsg, EscrowDetailsResponse, ExecuteMsg, InstantiateMsg, QueryMsg,
    };
    use crate::state::{GenericBalance, Milestone};
    use crate::ContractError;

    const ARBITER: &str = "arbiter";
    const RECIPIENT: &str = "recipient";
    const RECIPIENT2: &str = "recipient2";

    fn empty_strings() -> Vec<String> {
        vec![]
    }

    fn empty_cw20_coins() -> Vec<Cw20Coin> {
        vec![]
    }

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
    fn test_create_and_approve() {
        let mut deps = mock_dependencies();

        // instantiate an empty contract
        let instantiate_msg = InstantiateMsg {};
        let info = mock_info(&ARBITER, &[]);
        let res = instantiate(deps.as_mut(), mock_env(), info, instantiate_msg).unwrap();
        assert_eq!(0, res.messages.len());

        // create one milestone
        let milestones = vec![CreateMilestoneMsg {
            escrow_id: "escrow_1".to_string(),
            title: "milestone_1_title".to_string(),
            description: "milestone_1_description".to_string(),
            amount: GenericBalance {
                native: vec![coin(100, "tokens")],
                cw20: vec![],
            },
            end_height: None,
            end_time: None,
        }];

        // create an escrow
        let create_msg = CreateMsg {
            id: "escrow_1".to_string(),
            arbiter: ARBITER.to_string(),
            recipient: Some(RECIPIENT.to_string()),
            title: "escrow_1_title".to_string(),
            cw20_whitelist: None,
            description: "escrow_1_description".to_string(),
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
        let details = query_escrow_details(deps.as_ref(), "escrow_1".to_string()).unwrap();
        assert_eq!(
            details,
            EscrowDetailsResponse {
                id: "escrow_1".to_string(),
                arbiter: ARBITER.to_string(),
                recipient: Some(RECIPIENT.to_string()),
                source: ARBITER.to_string(),
                title: "escrow_1_title".to_string(),
                description: "escrow_1_description".to_string(),
                end_height: None,
                end_time: None,
                native_balance: balance.clone(),
                cw20_balance: vec![],
                cw20_whitelist: vec![],
                milestones: vec![Milestone {
                    id: String::from("1"),
                    title: "milestone_1_title".to_string(),
                    description: "milestone_1_description".to_string(),
                    amount: GenericBalance {
                        native: vec![coin(100, "tokens")],
                        cw20: vec![],
                    },
                    end_height: None,
                    end_time: None,
                    is_completed: false,
                }],
            }
        );

        // approve it
        let id = create_msg.id.clone();
        let milestone_id = String::from("1");
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
        let milestone_id = String::from("1");
        let info = mock_info(&create_msg.arbiter, &[]);
        let err = execute(
            deps.as_mut(),
            mock_env(),
            info,
            ExecuteMsg::ApproveMilestone { id, milestone_id },
        )
        .unwrap_err();
        assert!(matches!(err, ContractError::NotFound {}));
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
        let info = mock_info(ARBITER, &coins(200, "tokens"));

        let msg = ExecuteMsg::Create(CreateMsg {
            id: "escrow_1".to_string(),
            arbiter: ARBITER.to_string(),
            recipient: Some(RECIPIENT.to_string()),
            title: "escrow_1_title".to_string(),
            description: "escrow_1_description".to_string(),
            cw20_whitelist: None,
            milestones: vec![
                CreateMilestoneMsg {
                    escrow_id: "escrow_1".to_string(),
                    title: "milestone_1_title".to_string(),
                    description: "milestone_1_description".to_string(),
                    amount: GenericBalance {
                        native: vec![coin(100, "tokens")],
                        cw20: vec![],
                    },
                    end_height: None,
                    end_time: None,
                },
                CreateMilestoneMsg {
                    escrow_id: "escrow_1".to_string(),
                    title: "milestone_2_title".to_string(),
                    description: "milestone_2_description".to_string(),
                    amount: GenericBalance {
                        native: vec![coin(100, "tokens")],
                        cw20: vec![],
                    },
                    end_height: None,
                    end_time: None,
                },
            ],
        });

        let res = execute(deps.as_mut(), env.clone(), info.clone(), msg).unwrap();
        assert_eq!(0, res.messages.len());
    }

    #[test]
    fn test_set_receipient() {
        let mut deps = mock_dependencies();
        let env = mock_env();
        let info = mock_info(ARBITER, &coins(100, "tokens"));

        // Create a new escrow
        let create_msg = CreateMsg {
            id: "escrow_1".to_string(),
            arbiter: ARBITER.to_string(),
            recipient: Some(RECIPIENT.to_string()),
            title: "escrow_1_title".to_string(),
            description: "escrow_1_description".to_string(),
            cw20_whitelist: None,
            milestones: vec![CreateMilestoneMsg {
                escrow_id: "escrow_1".to_string(),
                title: "milestone_1_title".to_string(),
                description: "milestone_1_description".to_string(),
                amount: GenericBalance {
                    native: vec![coin(100, "tokens")],
                    cw20: vec![],
                },
                end_height: None,
                end_time: None,
            }],
        };
        let msg = ExecuteMsg::Create(create_msg.clone());
        let res = execute(deps.as_mut(), env.clone(), info.clone(), msg).unwrap();
        assert_eq!(0, res.messages.len());

        // Set recipient
        let id = create_msg.id.clone();
        let info = mock_info(&create_msg.arbiter, &[]);
        let msg = ExecuteMsg::SetRecipient {
            id,
            recipient: RECIPIENT2.to_string(),
        };
        let res = execute(deps.as_mut(), env.clone(), info.clone(), msg).unwrap();
        assert_eq!(0, res.messages.len());

        // Attempt setting empty recipient and assert failure
        let id = create_msg.id.clone();
        let info = mock_info(&create_msg.arbiter, &[]);
        let msg = ExecuteMsg::SetRecipient {
            id,
            recipient: String::new(),
        };
        let err = execute(deps.as_mut(), env.clone(), info.clone(), msg).unwrap_err();
        assert!(matches!(err, ContractError::InvalidAddress {}));
    }

    #[test]
    fn test_query_escrow() {
        let mut deps = mock_dependencies();
        let env = mock_env();
        let info = mock_info(ARBITER, &coins(200, "tokens"));

        // Create a new escrow
        let msg = ExecuteMsg::Create(CreateMsg {
            id: "escrow_1".to_string(),
            arbiter: ARBITER.to_string(),
            recipient: Some(RECIPIENT.to_string()),
            title: "escrow_1_title".to_string(),
            description: "escrow_1_description".to_string(),
            cw20_whitelist: None,
            milestones: vec![
                CreateMilestoneMsg {
                    escrow_id: "escrow_1".to_string(),
                    title: "milestone_1_title".to_string(),
                    description: "milestone_1_description".to_string(),
                    amount: GenericBalance {
                        native: vec![coin(100, "tokens")],
                        cw20: vec![],
                    },
                    end_height: None,
                    end_time: None,
                },
                CreateMilestoneMsg {
                    escrow_id: "escrow_1".to_string(),
                    title: "milestone_2_title".to_string(),
                    description: "milestone_2_description".to_string(),
                    amount: GenericBalance {
                        native: vec![coin(100, "tokens")],
                        cw20: vec![],
                    },
                    end_height: None,
                    end_time: None,
                },
            ],
        });

        let _res = execute(deps.as_mut(), env.clone(), info.clone(), msg).unwrap();

        // Query the created escrow
        let query_msg = QueryMsg::EscrowDetails {
            id: "escrow_1".to_string(),
        };
        let query_res = query(deps.as_ref(), env.clone(), query_msg).unwrap();
        let escrow: EscrowDetailsResponse = from_binary(&query_res).unwrap();
        assert_eq!("escrow_1", escrow.id);
        assert_eq!("escrow_1_title", escrow.title);
        assert_eq!("escrow_1_description", escrow.description);
        assert_eq!(2, escrow.milestones.len());
        assert_eq!(ARBITER, escrow.arbiter);
        assert_eq!(ARBITER, escrow.source);
        assert_eq!(RECIPIENT, escrow.recipient.unwrap());
        assert_eq!(None, escrow.end_height);
        assert_eq!(None, escrow.end_time);
        assert_eq!(empty_strings(), escrow.cw20_whitelist);
        assert_eq!(vec![Coin::new(200, "tokens")], escrow.native_balance);
        assert_eq!(empty_cw20_coins(), escrow.cw20_balance);
    }

    #[test]
    fn test_extend_escrow_milestone_time() {
        let mut deps = mock_dependencies();

        // instantiate an empty contract
        let instantiate_msg = InstantiateMsg {};
        let info = mock_info(&ARBITER, &[]);
        let res = instantiate(deps.as_mut(), mock_env(), info, instantiate_msg).unwrap();
        assert_eq!(0, res.messages.len());

        // create one milestone with an expired end_time
        let timestamp = 1_681_516_799u64;
        let milestones = vec![CreateMilestoneMsg {
            escrow_id: "escrow_1".to_string(),
            title: "milestone_1_title".to_string(),
            description: "milestone_1_description".to_string(),
            amount: GenericBalance {
                native: vec![coin(100, "tokens")],
                cw20: vec![],
            },
            end_height: None,
            end_time: Some(timestamp),
        }];

        // create an escrow
        let create_msg = CreateMsg {
            id: "escrow_1".to_string(),
            arbiter: ARBITER.to_string(),
            recipient: Some(RECIPIENT.to_string()),
            title: "escrow_1_title".to_string(),
            cw20_whitelist: None,
            description: "escrow_1_description".to_string(),
            milestones,
        };
        let sender = ARBITER.to_string();
        let balance = coins(100, "tokens");
        let info = mock_info(&sender, &balance);
        let msg = ExecuteMsg::Create(create_msg.clone());
        let res = execute(deps.as_mut(), mock_env(), info, msg).unwrap();
        assert_eq!(0, res.messages.len());
        assert_eq!(("action", "create"), res.attributes[0]);

        // extend the escrow
        let extended_timestamp = 1_681_603_199u64;
        let id = create_msg.id.clone();
        let info = mock_info(&create_msg.arbiter, &[]);
        let msg = ExecuteMsg::ExtendMilestone {
            id,
            milestone_id: String::from("1"),
            end_height: None,
            end_time: Some(extended_timestamp),
        };
        let res = execute(deps.as_mut(), mock_env(), info, msg).unwrap();
        assert_eq!(0, res.messages.len());
        assert_eq!(("action", "extend_milestone"), res.attributes[0]);

        // query the extended escrow
        let query_msg = QueryMsg::EscrowDetails {
            id: "escrow_1".to_string(),
        };
        let query_res = query(deps.as_ref(), mock_env(), query_msg).unwrap();
        let escrow: EscrowDetailsResponse = from_binary(&query_res).unwrap();

        // check the milestone end_time
        assert!(extended_timestamp > timestamp);
        assert_eq!(extended_timestamp, escrow.milestones[0].end_time.unwrap());
    }

    #[test]
    fn test_extend_escrow_milestone_block() {
        let mut deps = mock_dependencies();

        // instantiate an empty contract
        let instantiate_msg = InstantiateMsg {};
        let info = mock_info(&ARBITER, &[]);
        let res = instantiate(deps.as_mut(), mock_env(), info, instantiate_msg).unwrap();
        assert_eq!(0, res.messages.len());

        // create one milestone with an expired end_time
        let height = 7_807_000u64;
        let milestones = vec![CreateMilestoneMsg {
            escrow_id: "escrow_1".to_string(),
            title: "milestone_1_title".to_string(),
            description: "milestone_1_description".to_string(),
            amount: GenericBalance {
                native: vec![coin(100, "tokens")],
                cw20: vec![],
            },
            end_height: Some(height),
            end_time: None,
        }];

        // create an escrow
        let create_msg = CreateMsg {
            id: "escrow_1".to_string(),
            arbiter: ARBITER.to_string(),
            recipient: Some(RECIPIENT.to_string()),
            title: "escrow_1_title".to_string(),
            cw20_whitelist: None,
            description: "escrow_1_description".to_string(),
            milestones,
        };
        let sender = ARBITER.to_string();
        let balance = coins(100, "tokens");
        let info = mock_info(&sender, &balance);
        let msg = ExecuteMsg::Create(create_msg.clone());
        let res = execute(deps.as_mut(), mock_env(), info, msg).unwrap();
        assert_eq!(0, res.messages.len());
        assert_eq!(("action", "create"), res.attributes[0]);

        // extend the escrow
        let extended_height = 7_810_000u64;
        let id = create_msg.id.clone();
        let info = mock_info(&create_msg.arbiter, &[]);
        let msg = ExecuteMsg::ExtendMilestone {
            id,
            milestone_id: String::from("1"),
            end_height: Some(extended_height),
            end_time: None,
        };
        let res = execute(deps.as_mut(), mock_env(), info, msg).unwrap();
        assert_eq!(0, res.messages.len());
        assert_eq!(("action", "extend_milestone"), res.attributes[0]);

        // query the extended escrow
        let query_msg = QueryMsg::EscrowDetails {
            id: "escrow_1".to_string(),
        };
        let query_res = query(deps.as_ref(), mock_env(), query_msg).unwrap();
        let escrow: EscrowDetailsResponse = from_binary(&query_res).unwrap();

        // check the milestone end_time
        assert!(extended_height > height);
        assert_eq!(extended_height, escrow.milestones[0].end_height.unwrap());
    }
}
