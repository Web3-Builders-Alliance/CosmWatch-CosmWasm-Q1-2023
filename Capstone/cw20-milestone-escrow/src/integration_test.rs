#![cfg(test)]

use cosmwasm_std::{coins, from_binary, to_binary, Addr, Coin, Empty, Uint128};
use cw20::{Balance, Cw20Coin, Cw20CoinVerified, Cw20Contract, Cw20ExecuteMsg, Cw20ReceiveMsg};
use cw_multi_test::{App, Contract, ContractWrapper, Executor};

use crate::{
    msg::{
        CreateMilestoneMsg, CreateMsg, EscrowDetailsResponse, ExecuteMsg, InstantiateMsg, QueryMsg,
        ReceiveMsg,
    },
    state::GenericBalance,
};

pub fn contract_escrow_milestones() -> Box<dyn Contract<Empty>> {
    let contract = ContractWrapper::new(
        crate::contract::execute,
        crate::contract::instantiate,
        crate::contract::query,
    );
    Box::new(contract)
}

pub fn contract_cw20() -> Box<dyn Contract<Empty>> {
    let contract = ContractWrapper::new(
        cw20_base::contract::execute,
        cw20_base::contract::instantiate,
        cw20_base::contract::query,
    );
    Box::new(contract)
}

fn create_cw20(
    router: &mut App,
    owner: &Addr,
    name: String,
    symbol: String,
    balance: Uint128,
) -> Cw20Contract {
    // set up cw20 contract with some tokens
    let cw20_id = router.store_code(contract_cw20());
    let msg = cw20_base::msg::InstantiateMsg {
        name,
        symbol,
        decimals: 6,
        initial_balances: vec![Cw20Coin {
            address: owner.to_string(),
            amount: balance,
        }],
        mint: None,
        marketing: None,
    };
    let addr = router
        .instantiate_contract(cw20_id, owner.clone(), &msg, &[], "CASH", None)
        .unwrap();
    Cw20Contract(addr)
}

fn create_escrow(router: &mut App, owner: &Addr, label: &str, admin: Option<String>) -> Addr {
    let escrow_id = router.store_code(contract_escrow_milestones());
    let escrow_addr = router
        .instantiate_contract(
            escrow_id,
            owner.clone(),
            &InstantiateMsg {},
            &[],
            label,
            admin,
        )
        .unwrap();
    escrow_addr
}

fn get_escrow_native_balance(router: &App, escrow_addr: Addr, escrow_id: &str) -> Vec<Coin> {
    let escrow_details = router
        .wrap()
        .query_wasm_smart(
            escrow_addr.clone(),
            &QueryMsg::EscrowDetails {
                id: escrow_id.to_string(),
            },
        )
        .unwrap();
    let escrow: EscrowDetailsResponse = from_binary(&escrow_details).unwrap();
    escrow.native_balance
}

fn get_escrow_cw20_balance(router: &App, escrow_addr: Addr, escrow_id: &str) -> Vec<Cw20Coin> {
    let escrow_details = router
        .wrap()
        .query_wasm_smart(
            escrow_addr.clone(),
            &QueryMsg::EscrowDetails {
                id: escrow_id.to_string(),
            },
        )
        .unwrap();
    let escrow: EscrowDetailsResponse = from_binary(&escrow_details).unwrap();
    escrow.cw20_balance
}

#[test]
fn test_instantiate_and_deposit_cw20() {
    const NATIVE_TOKEN_DENOM: &str = "juno";
    let owner = Addr::unchecked("owner");

    // Declare app and owner native balance
    let mut router = App::new(|router, _, storage| {
        router
            .bank
            .init_balance(storage, &owner, coins(5000, NATIVE_TOKEN_DENOM))
            .unwrap();
    });

    // Instantiate cw20 token and escrow milestone contracts
    let cw20 = create_cw20(
        &mut router,
        &owner,
        "CW Token".to_string(),
        "CWTOKEN".to_string(),
        Uint128::new(5000),
    );
    let escrow_addr = create_escrow(&mut router, &owner, "Escrow Milestones Contract", None);
    assert_ne!(cw20.addr(), escrow_addr.clone());

    // Check owner balance
    let owner_balance = cw20.balance::<_, _, Empty>(&router, owner.clone()).unwrap();
    assert_eq!(owner_balance, Uint128::new(5000));

    // Check escrow balance
    let escrow_balance = get_escrow_cw20_balance(&router, escrow_addr.clone(), "escrow_1");

    assert_eq!(escrow_balance[0].amount, Uint128::zero());

    // Create new escrow and deposit cw20 tokens
    let amount = GenericBalance {
        native: vec![],
        cw20: vec![Cw20CoinVerified {
            address: cw20.addr(),
            amount: Uint128::new(5000),
        }],
    };
    let milestone = CreateMilestoneMsg {
        escrow_id: "escrow_1".to_string(),
        title: "milestone_1".to_string(),
        description: "This is the first milestone".to_string(),
        amount,
        end_height: None,
        end_time: None,
    };
    let msg = ExecuteMsg::Receive(Cw20ReceiveMsg {
        sender: escrow_addr.clone().to_string(),
        amount: Uint128::new(5000),
        msg: to_binary(&ReceiveMsg::CreateMilestone(milestone)).unwrap(),
    });

    let res = router.execute_contract(Addr::unchecked("owner"), cw20.addr(), &msg, &[]);
    println!("{:?}", res.as_ref().unwrap());

    assert!(res.is_ok());

    let owner_balance = cw20
        .balance::<_, _, Empty>(&router, escrow_addr.clone())
        .unwrap();
    assert_eq!(owner_balance, Uint128::zero());

    let escrow_balance = cw20
        .balance::<_, _, Empty>(&router, escrow_addr.clone())
        .unwrap();
    assert_eq!(escrow_balance, Uint128::new(5000));
}

#[test]
fn test_insufficient_sender_balance() {}

#[test]
fn test_invalid_recipient_address() {}

#[test]
fn test_create_escrow_native() {}

#[test]
fn test_create_escrow_cw20() {}

#[test]
fn test_create_escrow_cw20_invalid() {}

#[test]
fn test_create_escrow_native_invalid() {}

#[test]
fn test_refund() {}

#[test]
fn test_release_on_complete() {}

#[test]
fn test_release_on_complete_cw20() {}

#[test]
fn test_release_on_complete_mixed() {}

#[test]
// receive cw20 tokens and release upon approval
fn escrow_happy_path_cw20_tokens() {
    // set personal balance
    let owner = Addr::unchecked("owner");
    let init_funds = coins(2000, "juno");

    let mut router = App::new(|router, _, storage| {
        router
            .bank
            .init_balance(storage, &owner, init_funds)
            .unwrap();
    });

    // set up cw20 contract with some tokens
    let cw20_id = router.store_code(contract_cw20());
    let msg = cw20_base::msg::InstantiateMsg {
        name: "Cash Money".to_string(),
        symbol: "CASH".to_string(),
        decimals: 2,
        initial_balances: vec![Cw20Coin {
            address: owner.to_string(),
            amount: Uint128::new(5000),
        }],
        mint: None,
        marketing: None,
    };
    let cash_addr = router
        .instantiate_contract(cw20_id, owner.clone(), &msg, &[], "CASH", None)
        .unwrap();

    // set up reflect contract
    let escrow_id = router.store_code(contract_escrow_milestones());
    let escrow_addr = router
        .instantiate_contract(
            escrow_id,
            owner.clone(),
            &InstantiateMsg {},
            &[],
            "Escrow",
            None,
        )
        .unwrap();

    // they are different
    assert_ne!(cash_addr, escrow_addr);

    // set up cw20 helpers
    let cash = Cw20Contract(cash_addr.clone());

    // ensure our balances
    let owner_balance = cash.balance::<_, _, Empty>(&router, owner.clone()).unwrap();
    assert_eq!(owner_balance, Uint128::new(5000));
    let escrow_balance = cash
        .balance::<_, _, Empty>(&router, escrow_addr.clone())
        .unwrap();
    assert_eq!(escrow_balance, Uint128::zero());

    // send some tokens to create an escrow
    let arb = Addr::unchecked("arbiter");
    let ben = Addr::unchecked("beneficiary");
    let amount = GenericBalance {
        native: vec![],
        cw20: vec![Cw20CoinVerified {
            address: cash.addr(),
            amount: Uint128::new(1200),
        }],
    };
    let id = "demo";
    let milestones = vec![CreateMilestoneMsg {
        escrow_id: id.to_string(),
        title: "milestone_1".to_string(),
        description: "milestone_description_1".to_string(),
        amount,
        end_height: None,
        end_time: None,
    }];
    let create_msg = ReceiveMsg::Create(CreateMsg {
        id: id.to_string(),
        arbiter: arb.to_string(),
        recipient: Some(ben.to_string()),
        title: "some_title".to_string(),
        description: "some_description".to_string(),
        cw20_whitelist: None,
        milestones,
    });
    let send_msg = Cw20ExecuteMsg::Send {
        contract: escrow_addr.to_string(),
        amount: Uint128::new(1200),
        msg: to_binary(&create_msg).unwrap(),
    };
    let res = router
        .execute_contract(owner.clone(), cash_addr.clone(), &send_msg, &[])
        .unwrap();
    assert_eq!(4, res.events.len());
    println!("{:?}", res.events);

    assert_eq!(res.events[0].ty.as_str(), "execute");
    let cw20_attr = res.custom_attrs(1);
    println!("{:?}", cw20_attr);
    assert_eq!(4, cw20_attr.len());

    assert_eq!(res.events[2].ty.as_str(), "execute");
    let escrow_attr = res.custom_attrs(3);
    println!("{:?}", escrow_attr);
    assert_eq!(2, escrow_attr.len());

    // ensure balances updated
    let owner_balance = cash.balance::<_, _, Empty>(&router, owner.clone()).unwrap();
    assert_eq!(owner_balance, Uint128::new(3800));
    let escrow_balance = cash
        .balance::<_, _, Empty>(&router, escrow_addr.clone())
        .unwrap();
    assert_eq!(escrow_balance, Uint128::new(1200));

    // ensure escrow properly created
    let details: EscrowDetailsResponse = router
        .wrap()
        .query_wasm_smart(
            &escrow_addr,
            &QueryMsg::EscrowDetails { id: id.to_string() },
        )
        .unwrap();
    assert_eq!(id, details.id);
    assert_eq!(arb, details.arbiter);
    assert_eq!(Some(ben.to_string()), details.recipient);
    assert_eq!(
        vec![Cw20Coin {
            address: cash_addr.to_string(),
            amount: Uint128::new(1200)
        }],
        details.cw20_balance
    );

    // release escrow
    let approve_msg = ExecuteMsg::ApproveMilestone {
        id: id.to_string(),
        milestone_id: String::from("1"),
    };
    let _ = router
        .execute_contract(arb, escrow_addr.clone(), &approve_msg, &[])
        .unwrap();

    // ensure balances updated - release to ben
    let owner_balance = cash.balance::<_, _, Empty>(&router, owner).unwrap();
    assert_eq!(owner_balance, Uint128::new(3800));
    let escrow_balance = cash.balance::<_, _, Empty>(&router, escrow_addr).unwrap();
    assert_eq!(escrow_balance, Uint128::zero());
    let ben_balance = cash.balance::<_, _, Empty>(&router, ben).unwrap();
    assert_eq!(ben_balance, Uint128::new(1200));
}
