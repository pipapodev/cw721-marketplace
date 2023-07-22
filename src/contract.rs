#[cfg(not(feature = "library"))]
use cosmwasm_std::entry_point;
use cosmwasm_std::{to_binary, Binary, Deps, DepsMut, Env, MessageInfo, Response, StdResult};
use cw2::set_contract_version;

use crate::error::ContractError;
use crate::msg::{ExecuteMsg, InstantiateMsg, MigrateMsg, QueryMsg};
use crate::state::{NATIVE_DENOM, TAKERADDRESS, TAKERFEE};

use self::execute::{
    admin_remove_sale, buy, register_collection, remove_sale, update_collection, update_ownership,
    update_sale, update_taker_fee,
};
use self::query::{get_collection, get_sale, get_taker_fee};

// version info for migration info
const CONTRACT_NAME: &str = "crates.io:cw721-marketplace";
const CONTRACT_VERSION: &str = env!("CARGO_PKG_VERSION");

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn instantiate(
    deps: DepsMut,
    _env: Env,
    info: MessageInfo,
    msg: InstantiateMsg,
) -> Result<Response, ContractError> {
    cw_ownable::initialize_owner(deps.storage, deps.api, Some(&info.sender.to_string()))?;
    TAKERFEE.save(deps.storage, &msg.taker_fee.u64())?;
    NATIVE_DENOM.save(deps.storage, &msg.native_denom)?;
    TAKERADDRESS.save(
        deps.storage,
        &deps.api.addr_validate(&msg.taker_address).unwrap(),
    )?;
    set_contract_version(deps.storage, CONTRACT_NAME, CONTRACT_VERSION)?;

    Ok(Response::new()
        .add_attribute("method", "instantiate")
        .add_attribute("owner", info.sender))
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn execute(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    msg: ExecuteMsg,
) -> Result<Response, ContractError> {
    match msg {
        ExecuteMsg::RegisterCollection {
            contract_address,
            royalty_percentage,
            royalty_payment_address,
        } => register_collection(
            deps,
            info,
            contract_address,
            royalty_percentage,
            royalty_payment_address,
        ),
        ExecuteMsg::UpdateCollection {
            contract_address,
            royalty_percentage,
            royalty_payment_address,
            is_paused,
        } => update_collection(
            deps,
            info,
            contract_address,
            royalty_percentage,
            royalty_payment_address,
            is_paused,
        ),
        ExecuteMsg::AdminRemoveSales {
            contract_address,
            token_id,
        } => admin_remove_sale(deps, info, contract_address, token_id),
        ExecuteMsg::UpdateTakerFee { taker_fee } => update_taker_fee(deps, info, taker_fee),
        ExecuteMsg::UpdateSale {
            contract_address,
            token_id,
            price,
        } => update_sale(deps, env, info, contract_address, token_id, price),
        ExecuteMsg::RemoveSale {
            contract_address,
            token_id,
        } => remove_sale(deps, info, contract_address, token_id),
        ExecuteMsg::AcceptCollectionOffer {
            contract_address: _,
            token_id: _,
        } => todo!(),
        ExecuteMsg::Buy {
            contract_address,
            token_id,
        } => buy(deps, info, contract_address, token_id),
        ExecuteMsg::CreateCollectionOffer {
            contract_address: _,
            price: _,
        } => todo!(),
        ExecuteMsg::RemoveCollectionOffer {
            contract_address: _,
        } => todo!(),
        ExecuteMsg::UpdateOwnership(action) => update_ownership(deps, env, info, action),
    }
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn query(deps: Deps, _env: Env, msg: QueryMsg) -> StdResult<Binary> {
    match msg {
        QueryMsg::GetSale {
            contract_address,
            token_id,
        } => to_binary(&get_sale(deps, contract_address, token_id)?),
        QueryMsg::GetSales { start, limit } => todo!(),
        QueryMsg::GetCollection { contract_address } => {
            to_binary(&get_collection(deps, contract_address)?)
        }
        QueryMsg::GetCollections { start, limit } => todo!(),
        QueryMsg::GetTakerFee {} => to_binary(&get_taker_fee(deps)?),
    }
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn migrate(deps: DepsMut, _env: Env, _msg: MigrateMsg) -> Result<Response, ContractError> {
    set_contract_version(deps.storage, CONTRACT_NAME, CONTRACT_VERSION)?;

    Ok(Response::default())
}

pub mod execute {
    use std::marker::PhantomData;

    use cosmwasm_std::{
        coins, BankMsg, Coin, CosmosMsg, Decimal, DepsMut, Empty, Env, Event, MessageInfo,
        Response, Uint128, Uint64,
    };
    use cw721_rewards::{helpers::Cw721Contract, ExecuteMsg};

    use crate::{
        state::{Collection, Sale, COLLECTIONS, NATIVE_DENOM, SALES, TAKERADDRESS, TAKERFEE},
        ContractError,
    };

    pub fn register_collection(
        deps: DepsMut,
        info: MessageInfo,
        contract_address: String,
        royalty_percentage: Option<u64>,
        royalty_payment_address: Option<String>,
    ) -> Result<Response, ContractError> {
        cw_ownable::assert_owner(deps.storage, &info.sender)?;

        let contract_address = deps.api.addr_validate(&contract_address)?;

        let is_exist = COLLECTIONS.load(deps.storage, contract_address.clone());

        if is_exist.is_ok() {
            return Err(ContractError::CollectionAlreadyRegistered {});
        }

        COLLECTIONS.save(
            deps.storage,
            contract_address.clone(),
            &Collection {
                royalty_percentage,
                royalty_payment_address: if let Some(royalty_payment_address) =
                    royalty_payment_address.clone()
                {
                    Some(deps.api.addr_validate(&royalty_payment_address).unwrap())
                } else {
                    None
                },
                is_paused: false,
            },
        )?;

        Ok(Response::new().add_event(
            Event::new("register_collection")
                .add_attribute("contract_address", contract_address)
                .add_attribute(
                    "royalty_percentage",
                    match royalty_percentage {
                        Some(royalty_percentage) => Uint64::from(royalty_percentage).to_string(),
                        None => "null".to_string(),
                    },
                )
                .add_attribute(
                    "royalty_payment_address",
                    match royalty_payment_address {
                        Some(royalty_payment_address) => royalty_payment_address,
                        None => "null".to_string(),
                    },
                ),
        ))
    }

    pub fn update_taker_fee(
        deps: DepsMut,
        info: MessageInfo,
        taker_fee: Uint64,
    ) -> Result<Response, ContractError> {
        cw_ownable::assert_owner(deps.storage, &info.sender)?;

        TAKERFEE.save(deps.storage, &taker_fee.u64())?;

        Ok(Response::new().add_event(
            Event::new("update_taker_fee")
                .add_attribute("taker_fee", Uint64::from(taker_fee).to_string()),
        ))
    }

    pub fn admin_remove_sale(
        deps: DepsMut,
        info: MessageInfo,
        contract_address: String,
        token_id: String,
    ) -> Result<Response, ContractError> {
        cw_ownable::assert_owner(deps.storage, &info.sender)?;

        let contract_address = deps.api.addr_validate(&contract_address)?;
        let sale = SALES.load(deps.storage, (contract_address.clone(), token_id.clone()));

        if sale.is_err() {
            return Err(ContractError::SaleDoesNotExist {});
        }

        SALES.remove(deps.storage, (contract_address.clone(), token_id.clone()));

        Ok(Response::new().add_event(
            Event::new("remove_sale")
                .add_attribute("contract_address", contract_address.to_string())
                .add_attribute("token_id", token_id),
        ))
    }

    pub fn update_collection(
        deps: DepsMut,
        info: MessageInfo,
        contract_address: String,
        royalty_percentage: Option<u64>,
        royalty_payment_address: Option<String>,
        is_paused: bool,
    ) -> Result<Response, ContractError> {
        cw_ownable::assert_owner(deps.storage, &info.sender)?;

        let contract_address = deps.api.addr_validate(&contract_address)?;

        let collection = COLLECTIONS.load(deps.storage, contract_address.clone());

        if collection.is_err() {
            return Err(ContractError::CollectionAlreadyRegistered {});
        }

        COLLECTIONS.save(
            deps.storage,
            contract_address.clone(),
            &Collection {
                royalty_percentage: royalty_percentage,
                royalty_payment_address: if let Some(royalty_payment_address) =
                    royalty_payment_address.clone()
                {
                    Some(deps.api.addr_validate(&royalty_payment_address).unwrap())
                } else {
                    None
                },
                is_paused,
            },
        )?;

        Ok(Response::new().add_event(
            Event::new("update_collection")
                .add_attribute("contract_address", contract_address)
                .add_attribute(
                    "royalty_percentage",
                    match royalty_percentage {
                        Some(royalty_percentage) => Uint64::from(royalty_percentage).to_string(),
                        None => "null".to_string(),
                    },
                )
                .add_attribute(
                    "royalty_payment_address",
                    match royalty_payment_address {
                        Some(royalty_payment_address) => royalty_payment_address,
                        None => "null".to_string(),
                    },
                )
                .add_attribute("is_paused", is_paused.to_string()),
        ))
    }

    pub fn update_sale(
        deps: DepsMut,
        env: Env,
        info: MessageInfo,
        contract_address: String,
        token_id: String,
        price: Coin,
    ) -> Result<Response, ContractError> {
        // check owner
        let contract_address = deps.api.addr_validate(&contract_address)?;
        let owner =
            Cw721Contract::<Empty, Empty>(contract_address.clone(), PhantomData, PhantomData)
                .owner_of(&deps.querier, token_id.clone(), false)?;

        if owner.owner != info.sender.to_string() {
            return Err(ContractError::Unauthorized {});
        }

        // check approval

        let approval =
            Cw721Contract::<Empty, Empty>(contract_address.clone(), PhantomData, PhantomData)
                .approval(
                    &deps.querier,
                    token_id.clone(),
                    env.contract.address.to_string(),
                    Some(false),
                );

        if approval.is_err() {
            return Err(ContractError::NotApproved {});
        }

        let native_denom = NATIVE_DENOM.load(deps.storage)?;

        if price.denom != native_denom {
            return Err(ContractError::DenomNotSupported {});
        }

        SALES.save(
            deps.storage,
            (contract_address.clone(), token_id.clone()),
            &Sale {
                owner_address: info.sender,
                price: price.clone(),
            },
        )?;

        // check approval
        Ok(Response::new().add_event(
            Event::new("update_sale")
                .add_attribute("contract_address", contract_address)
                .add_attribute("token_id", token_id)
                .add_attribute("price", price.amount),
        ))
    }

    pub fn remove_sale(
        deps: DepsMut,
        info: MessageInfo,
        contract_address: String,
        token_id: String,
    ) -> Result<Response, ContractError> {
        // check owner
        let contract_address = deps.api.addr_validate(&contract_address)?;
        let owner =
            Cw721Contract::<Empty, Empty>(contract_address.clone(), PhantomData, PhantomData)
                .owner_of(&deps.querier, token_id.clone(), false)?;

        if owner.owner != info.sender.to_string() {
            return Err(ContractError::Unauthorized {});
        }

        SALES.remove(deps.storage, (contract_address.clone(), token_id.clone()));

        // check approval
        Ok(Response::new().add_event(
            Event::new("remove_sale")
                .add_attribute("contract_address", contract_address.to_string())
                .add_attribute("token_id", token_id),
        ))
    }

    pub fn buy(
        deps: DepsMut,
        info: MessageInfo,
        contract_address: String,
        token_id: String,
    ) -> Result<Response, ContractError> {
        let contract_address = deps.api.addr_validate(&contract_address)?;
        let sale = SALES.load(deps.storage, (contract_address.clone(), token_id.clone()))?;

        let fund_input = cw_utils::must_pay(&info, &sale.price.denom).unwrap();

        if fund_input != sale.price.amount {
            return Err(ContractError::InsufficientFunds {});
        }

        SALES.remove(deps.storage, (contract_address.clone(), token_id.clone()));

        let taker_fee = TAKERFEE.load(deps.storage)?;
        let taker_funds = fund_input * Decimal::percent(taker_fee);

        let mut messages: Vec<CosmosMsg> = Vec::new();

        if taker_funds.u128() > 0 {
            let send_taker_funds_msg = BankMsg::Send {
                to_address: TAKERADDRESS.load(deps.storage).unwrap().to_string(),
                amount: coins(taker_funds.u128(), &sale.price.denom),
            };

            messages.push(send_taker_funds_msg.into());
        }

        // royalties

        let collection = COLLECTIONS.load(deps.storage, contract_address.clone())?;

        let mut royalty_funds = Uint128::from(0u128);

        if let Some(royalty_percentage) = collection.royalty_percentage {
            if let Some(royalty_payment_address) = collection.royalty_payment_address {
                royalty_funds = fund_input * Decimal::percent(royalty_percentage);
                if royalty_funds.u128() > 0 {
                    let send_royalty_funds_msg = BankMsg::Send {
                        to_address: royalty_payment_address.to_string(),
                        amount: coins(royalty_funds.u128(), &sale.price.denom),
                    };

                    messages.push(send_royalty_funds_msg.into());
                }
            }
        }

        let owner_funds = fund_input - taker_funds - royalty_funds;
        if owner_funds.u128() > 0 {
            let send_owner_funds_msg = BankMsg::Send {
                to_address: sale.owner_address.to_string(),
                amount: coins(owner_funds.u128(), &sale.price.denom),
            };

            messages.push(send_owner_funds_msg.into());
        }

        messages.push(
            Cw721Contract::<Empty, Empty>(contract_address.clone(), PhantomData, PhantomData)
                .call(ExecuteMsg::<Empty>::TransferNft {
                    recipient: info.sender.to_string(),
                    token_id: token_id.clone(),
                })?,
        );

        Ok(Response::new().add_messages(messages).add_event(
            Event::new("buy")
                .add_attribute("contract_address", contract_address)
                .add_attribute("token_id", token_id),
        ))
    }

    pub fn update_ownership(
        deps: DepsMut,
        env: Env,
        info: MessageInfo,
        action: cw_ownable::Action,
    ) -> Result<Response, ContractError> {
        let ownership = cw_ownable::update_ownership(deps, &env.block, &info.sender, action)?;
        Ok(Response::new().add_attributes(ownership.into_attributes()))
    }
}

pub mod query {
    use cosmwasm_std::{Deps, StdResult, Uint64};

    use crate::{
        msg::{CollectionsInfo, SalesInfo, TakerFeeInfo},
        state::{COLLECTIONS, SALES, TAKERFEE},
    };

    pub fn get_sale(
        deps: Deps,
        contract_address: String,
        token_id: String,
    ) -> StdResult<SalesInfo> {
        let contract_address = deps.api.addr_validate(&contract_address)?;
        let sale = SALES.load(deps.storage, (contract_address, token_id))?;

        Ok(SalesInfo { sales: vec![sale] })
    }

    pub fn get_collection(deps: Deps, contract_address: String) -> StdResult<CollectionsInfo> {
        let contract_address = deps.api.addr_validate(&contract_address)?;
        let collection = COLLECTIONS.load(deps.storage, contract_address)?;

        Ok(CollectionsInfo {
            collections: vec![collection],
        })
    }

    pub fn get_taker_fee(deps: Deps) -> StdResult<TakerFeeInfo> {
        let taker_fee = TAKERFEE.load(deps.storage)?;

        Ok(TakerFeeInfo {
            taker_fee: Uint64::from(taker_fee),
        })
    }
}
#[cfg(test)]
mod tests {}
