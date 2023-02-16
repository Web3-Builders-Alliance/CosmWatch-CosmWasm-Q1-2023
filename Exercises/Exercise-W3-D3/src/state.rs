use cosmwasm_std::Coin;
use cw_storage_plus::Item;

pub const TOKENS_SENT: Item<Coin> = Item::new("tokens_sent");
