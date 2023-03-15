use cosmwasm_schema::cw_serde;
use cosmwasm_std::{Addr, Coin, Deps, Env, Order, StdResult, Storage, Timestamp};
use cw20::{Balance, Cw20CoinVerified};
use cw_storage_plus::Map;

use crate::ContractError;

pub const ESCROWS: Map<&str, Escrow> = Map::new("escrow");

#[cw_serde]
#[derive(Default)]
pub struct GenericBalance {
    pub native: Vec<Coin>,
    pub cw20: Vec<Cw20CoinVerified>,
}

#[cw_serde]
pub struct Milestone {
    pub id: String,
    pub title: String,
    pub description: String,
    pub amount: GenericBalance,
    pub is_completed: bool,
}

impl GenericBalance {
    pub fn add_tokens(&mut self, add: Balance) {
        match add {
            Balance::Native(balance) => {
                for token in balance.0 {
                    let index = self.native.iter().enumerate().find_map(|(i, exist)| {
                        if exist.denom == token.denom {
                            Some(i)
                        } else {
                            None
                        }
                    });
                    match index {
                        Some(idx) => self.native[idx].amount += token.amount,
                        None => self.native.push(token),
                    }
                }
            }
            Balance::Cw20(token) => {
                let index = self.cw20.iter().enumerate().find_map(|(i, exist)| {
                    if exist.address == token.address {
                        Some(i)
                    } else {
                        None
                    }
                });
                match index {
                    Some(idx) => self.cw20[idx].amount += token.amount,
                    None => self.cw20.push(token),
                }
            }
        };
    }
}

#[cw_serde]
pub struct Escrow {
    /// arbiter can decide to approve or refund the escrow
    pub arbiter: Addr,
    /// if approved, funds go to the recipient, cannot approve if recipient is none
    pub recipient: Option<Addr>,
    /// if refunded, funds go to the source
    pub source: Addr,
    /// Title of the escrow, for example for a bug bounty "Fix issue in contract.rs"
    pub title: String,
    /// Description of the escrow, a more in depth description of how to meet the escrow condition
    pub description: String,
    /// When end height set and block height exceeds this value, the escrow is expired.
    /// Once an escrow is expired, it can be returned to the original funder (via "refund").
    pub end_height: Option<u64>,
    /// When end time (in seconds since epoch 00:00:00 UTC on 1 January 1970) is set and
    /// block time exceeds this value, the escrow is expired.
    /// Once an escrow is expired, it can be returned to the original funder (via "refund").
    pub end_time: Option<u64>,
    /// Balance in Native and Cw20 tokens
    pub balance: GenericBalance,
    /// All possible contracts that we accept tokens from
    pub cw20_whitelist: Vec<Addr>,
    // Milestones to be met
    pub milestones: Vec<Milestone>,
}

impl Escrow {
    pub fn is_expired(&self, env: &Env) -> bool {
        if let Some(end_height) = self.end_height {
            if env.block.height > end_height {
                return true;
            }
        }

        if let Some(end_time) = self.end_time {
            if env.block.time > Timestamp::from_seconds(end_time) {
                return true;
            }
        }

        false
    }

    pub fn is_complete(&self) -> bool {
        self.milestones.iter().all(|m| m.is_completed)
    }

    pub fn human_whitelist(&self) -> Vec<String> {
        self.cw20_whitelist.iter().map(|a| a.to_string()).collect()
    }

    pub fn human_milestones(&self) -> Vec<String> {
        self.milestones
            .iter()
            .map(|m| {
                format!(
                    "id: {}\ntitle: {}\ndescription: {}\ncomplete: {}",
                    m.id, m.title, m.description, m.is_completed
                )
            })
            .collect()
    }

    pub fn create_milestone(
        &mut self,
        id: String,
        title: String,
        description: String,
        amount: GenericBalance,
    ) {
        self.milestones.push(Milestone {
            id,
            title,
            description,
            amount,
            is_completed: false,
        });
    }

    pub fn get_milestone_by_id(&self, id: &str) -> Option<&Milestone> {
        self.milestones.iter().find(|m| m.id == id)
    }

    pub fn get_total_balance(&self, id: &str) -> GenericBalance {
        get_total_balance_from(self.clone().milestones).unwrap()
    }
}

pub fn get_total_balance_from(milestones: Vec<Milestone>) -> StdResult<GenericBalance> {
    let mut total_balance = GenericBalance::default();
    for milestone in milestones.iter() {
        match &milestone.amount {
            native => total_balance.add_tokens(Balance::from(milestone.amount.native.clone())),
            cw20 => {
                for token in milestone.amount.cw20.clone() {
                    total_balance.add_tokens(Balance::from(token));
                }
            }
        }
    }
    Ok(total_balance)
}

pub fn get_escrow_by_id(deps: &Deps, id: &String) -> Result<Escrow, ContractError> {
    match ESCROWS.may_load(deps.storage, &id)? {
        Some(escrow) => Ok(escrow),
        None => Err(ContractError::NotFound {}),
    }
}

/// This returns the list of ids for all registered escrows
pub fn all_escrow_ids(storage: &dyn Storage) -> StdResult<Vec<String>> {
    ESCROWS
        .keys(storage, None, None, Order::Ascending)
        .collect()
}
// This returns the list of ids for all milestones for a given escrow
pub fn all_escrow_milestone_ids(storage: &dyn Storage, escrow_id: &str) -> StdResult<Vec<String>> {
    let escrow = ESCROWS.load(storage, escrow_id)?;
    Ok(escrow.milestones.iter().map(|m| m.id.clone()).collect())
}

#[cfg(test)]
mod tests {
    use super::*;

    use cosmwasm_std::testing::MockStorage;

    #[test]
    fn no_escrow_ids() {
        let storage = MockStorage::new();
        let ids = all_escrow_ids(&storage).unwrap();
        assert_eq!(0, ids.len());
    }

    fn dummy_escrow() -> Escrow {
        Escrow {
            arbiter: Addr::unchecked("arb"),
            recipient: Some(Addr::unchecked("recip")),
            source: Addr::unchecked("source"),
            title: "some_escrow".to_string(),
            description: "some escrow desc".to_string(),
            end_height: None,
            end_time: None,
            balance: Default::default(),
            cw20_whitelist: vec![],
            milestones: vec![],
        }
    }

    #[test]
    fn all_escrow_ids_in_order() {
        let mut storage = MockStorage::new();
        ESCROWS.save(&mut storage, "lazy", &dummy_escrow()).unwrap();
        ESCROWS
            .save(&mut storage, "assign", &dummy_escrow())
            .unwrap();
        ESCROWS.save(&mut storage, "zen", &dummy_escrow()).unwrap();

        let ids = all_escrow_ids(&storage).unwrap();
        assert_eq!(3, ids.len());
        assert_eq!(
            vec!["assign".to_string(), "lazy".to_string(), "zen".to_string()],
            ids
        )
    }
}
