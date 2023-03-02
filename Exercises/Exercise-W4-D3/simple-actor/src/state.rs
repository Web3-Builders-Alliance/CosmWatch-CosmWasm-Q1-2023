use cosmwasm_schema::cw_serde;
use cosmwasm_std::{Addr, Coin};
use cw_storage_plus::Item;

pub const LP_TOKEN: Item<Coin> = Item::new("lp_token");
pub const CONFIG: Item<Config> = Item::new("config");

#[cw_serde]
pub struct Config {
    pub cw20_address: Option<Addr>,
    pub owner: Addr,
}
