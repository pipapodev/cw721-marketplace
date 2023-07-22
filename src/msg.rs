use cosmwasm_schema::{cw_serde, QueryResponses};
use cosmwasm_std::{Coin, Uint64};
use cw_ownable::cw_ownable_execute;

use crate::state::{Collection, Sale, TokenId};

#[cw_serde]
pub struct InstantiateMsg {
    pub taker_fee: Uint64,
}

#[cw_ownable_execute]
#[cw_serde]
pub enum ExecuteMsg {
    // Admin function
    RegisterCollection {
        contract_address: String,
        royalty_percentage: Option<u64>,
        royalty_payment_address: Option<String>,
    },
    UpdateCollection {
        contract_address: String,
        royalty_percentage: Option<u64>,
        royalty_payment_address: Option<String>,
        is_paused: bool,
    },
    AdminRemoveSales {
        contract_address: String,
        token_id: TokenId,
    },
    UpdateTakerFee {
        taker_fee: Uint64,
    },
    // Seller / token owner functions
    CreateSale {
        contract_address: String,
        token_id: TokenId,
        price: Coin,
    },
    UpdateSale {
        contract_address: String,
        token_id: TokenId,
        price: Coin,
    },
    RemoveSale {
        contract_address: String,
        token_id: TokenId,
    },
    AcceptCollectionOffer {
        contract_address: String,
        token_id: TokenId,
    },
    // Buyer functions
    Buy {
        contract_address: String,
        token_id: TokenId,
    },
    CreateCollectionOffer {
        contract_address: String,
        price: Coin,
    },
    RemoveCollectionOffer {
        contract_address: String,
    },
}

#[cw_serde]
#[derive(QueryResponses)]
pub enum QueryMsg {
    #[returns(SalesInfo)]
    GetSale {
        contract_address: String,
        token_id: TokenId,
    },
    #[returns(SalesInfo)]
    GetSales { start: u64, limit: u64 },
    #[returns(CollectionsInfo)]
    GetCollection { contract_address: String },
    #[returns(CollectionsInfo)]
    GetCollections { start: u64, limit: u64 },
    #[returns(TakerFeeInfo)]
    GetTakerFee {},
}

#[cw_serde]
pub struct SalesInfo {
    sales: Vec<Sale>,
}

#[cw_serde]
pub struct CollectionsInfo {
    collections: Vec<Collection>,
}

#[cw_serde]
pub struct TakerFeeInfo {
    taker_fee: Uint64,
}
