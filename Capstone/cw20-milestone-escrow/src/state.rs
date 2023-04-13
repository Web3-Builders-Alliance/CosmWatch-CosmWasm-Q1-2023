use cosmwasm_schema::cw_serde;
use cosmwasm_std::{Addr, Coin, Deps, Env, Order, StdResult, Storage, Timestamp};
use cw20::{Balance, Cw20CoinVerified};
use cw_storage_plus::Map;
use cw_utils::NativeBalance;

use crate::{msg::CreateMilestoneMsg, ContractError};

pub const ESCROWS: Map<&str, Escrow> = Map::new("escrow");

macro_rules! is_expired {
    ($self:ident, $env:ident) => {{
        (if let Some(end_height) = $self.end_height {
            $env.block.height > end_height
        } else {
            false
        }) || (if let Some(end_time) = $self.end_time {
            $env.block.time > Timestamp::from_seconds(end_time)
        } else {
            false
        })
    }};
}

#[cw_serde]
pub struct Milestone {
    pub id: String,
    pub title: String,
    pub description: String,
    pub amount: GenericBalance,
    pub end_height: Option<u64>,
    pub end_time: Option<u64>,
    pub is_completed: bool,
}

impl HasAmount for Milestone {
    fn get_amount(&self) -> GenericBalance {
        self.amount.clone()
    }
}

impl HasEnd for Milestone {
    fn get_end_time(&self) -> Option<u64> {
        self.end_time
    }
    fn get_end_height(&self) -> Option<u64> {
        self.end_height
    }
}

impl Milestone {
    pub fn is_empty(&self) -> bool {
        match &self.amount {
            balance => balance.native.is_empty() && balance.cw20.is_empty(),
        }
    }

    pub fn is_expired(&self, env: &Env) -> bool {
        is_expired!(self, env)
    }

    pub fn extend_expiration(&mut self, end_height: Option<u64>, end_time: Option<u64>) {
        // Check if new time is in the past
        if end_height < self.end_height || end_time < self.end_time {
            return;
        }

        if let Some(height) = end_height {
            self.end_height = Some(height);
        }
        if let Some(time) = end_time {
            self.end_time = Some(time);
        }
    }
}

#[cw_serde]
#[derive(Default)]
pub struct GenericBalance {
    pub native: Vec<Coin>,
    pub cw20: Vec<Cw20CoinVerified>,
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
        is_expired!(self, env)
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

    pub fn create_milestone(&mut self, milestone: CreateMilestoneMsg) {
        let id = (self.milestones.len() + 1).to_string();
        self.milestones.push(Milestone {
            id,
            title: milestone.title,
            description: milestone.description,
            amount: milestone.amount,
            is_completed: false,
            end_height: milestone.end_height,
            end_time: milestone.end_time,
        });
    }

    pub fn get_milestone_by_id(&self, id: &str) -> Option<&Milestone> {
        self.milestones.iter().find(|m| m.id == id)
    }

    pub fn get_total_balance(&self) -> GenericBalance {
        get_total_balance_from(self.clone().milestones).unwrap()
    }

    pub fn get_end_height(&self) -> Option<u64> {
        get_end_height(self.clone().milestones)
    }

    pub fn get_end_time(&self) -> Option<u64> {
        get_end_time(self.clone().milestones)
    }

    pub fn update_calculated_properties(&mut self) {
        self.balance = self.get_total_balance();
        self.end_height = self.get_end_height();
        self.end_time = self.get_end_time();
    }
}

pub trait HasAmount {
    fn get_amount(&self) -> GenericBalance;
}

pub trait HasEnd {
    fn get_end_height(&self) -> Option<u64>;
    fn get_end_time(&self) -> Option<u64>;
}

// Helper functions
pub fn get_total_balance_from<T: HasAmount>(milestones: Vec<T>) -> StdResult<GenericBalance> {
    let mut total_balance = GenericBalance::default();
    for milestone in milestones.iter() {
        let amount = milestone.get_amount();
        total_balance.add_tokens(Balance::Native(NativeBalance(amount.native)));
        for token in &amount.cw20 {
            total_balance.add_tokens(Balance::Cw20(token.clone()));
        }
    }
    Ok(total_balance)
}

pub fn get_end_height<T: HasEnd>(milestones: Vec<T>) -> Option<u64> {
    milestones.iter().filter_map(|m| m.get_end_height()).max()
}

pub fn get_end_time<T: HasEnd>(milestones: Vec<T>) -> Option<u64> {
    milestones.iter().filter_map(|m| m.get_end_time()).max()
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
    fn test_no_escrow_ids() {
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
    fn test_all_escrow_ids_in_order() {
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
