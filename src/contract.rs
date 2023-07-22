#[cfg(not(feature = "library"))]
use cosmwasm_std::entry_point;
use cosmwasm_std::{Binary, Deps, DepsMut, Env, MessageInfo, Response, StdResult};
use cw2::set_contract_version;

use crate::error::ContractError;
use crate::msg::{ExecuteMsg, InstantiateMsg, MigrateMsg, QueryMsg};
use crate::state::{NATIVE_DENOM, TAKERFEE};

use self::execute::{
    admin_remove_sale, register_collection, remove_sale, update_collection, update_sale,
    update_taker_fee,
};

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
            contract_address,
            token_id,
        } => todo!(),
        ExecuteMsg::Buy {
            contract_address,
            token_id,
        } => todo!(),
        ExecuteMsg::CreateCollectionOffer {
            contract_address,
            price,
        } => todo!(),
        ExecuteMsg::RemoveCollectionOffer { contract_address } => todo!(),
        ExecuteMsg::UpdateOwnership(_) => todo!(),
    }
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn query(_deps: Deps, _env: Env, _msg: QueryMsg) -> StdResult<Binary> {
    unimplemented!()
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn migrate(deps: DepsMut, _env: Env, _msg: MigrateMsg) -> Result<Response, ContractError> {
    set_contract_version(deps.storage, CONTRACT_NAME, CONTRACT_VERSION)?;

    Ok(Response::default())
}

pub mod execute {
    use std::marker::PhantomData;

    use cosmwasm_std::{Addr, Coin, DepsMut, Empty, Env, Event, MessageInfo, Response, Uint64};
    use cw721_rewards::helpers::Cw721Contract;

    use crate::{
        state::{Collection, Sale, COLLECTIONS, NATIVE_DENOM, SALES, TAKERFEE},
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
}

#[cfg(test)]
mod tests {}
