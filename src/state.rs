use cosmwasm_schema::cw_serde;
use cosmwasm_std::{Addr, Coin};
use cw_storage_plus::{Item, Map};

#[cw_serde]
pub struct Collection {
    pub royalty_percentage: Option<u64>,
    pub royalty_payment_address: Option<String>,
    pub is_paused: bool,
}

#[cw_serde]
pub struct Sale {
    pub owner_address: Addr,
    pub price: Coin,
}

pub struct Offer {
    pub offeror_address: Addr,
    pub price: Coin,
}

pub type TokenId = String;

pub const TAKERFEE: Item<u64> = Item::new("taker_fee");
pub const COLLECTIONS: Map<Addr, Collection> = Map::new("collections");
pub const SALES: Map<(Addr, TokenId), Sale> = Map::new("sales");
pub const COLLECTION_OFFERS: Map<(Addr, Addr), Offer> = Map::new("collection_offers");
