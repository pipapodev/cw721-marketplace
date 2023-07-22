#[cfg(not(feature = "library"))]
use cosmwasm_std::entry_point;
use cosmwasm_std::{Binary, Deps, DepsMut, Env, MessageInfo, Response, StdResult};
use cw2::set_contract_version;

use crate::error::ContractError;
use crate::msg::{ExecuteMsg, InstantiateMsg, MigrateMsg, QueryMsg};
use crate::state::{NATIVE_DENOM, TAKERFEE};

use self::execute::{register_collection, update_collection};

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
    _env: Env,
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
        } => todo!(),
        ExecuteMsg::UpdateTakerFee { taker_fee } => todo!(),
        ExecuteMsg::CreateSale {
            contract_address,
            token_id,
            price,
        } => todo!(),
        ExecuteMsg::UpdateSale {
            contract_address,
            token_id,
            price,
        } => todo!(),
        ExecuteMsg::RemoveSale {
            contract_address,
            token_id,
        } => todo!(),
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

    use cosmwasm_std::{Addr, Coin, DepsMut, Empty, Env, MessageInfo, Response};
    use cw721_rewards::helpers::Cw721Contract;

    use crate::{
        state::{Collection, Sale, COLLECTIONS, NATIVE_DENOM, SALES},
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
            contract_address,
            &Collection {
                royalty_percentage,
                royalty_payment_address: if let Some(royalty_payment_address) =
                    royalty_payment_address
                {
                    Some(deps.api.addr_validate(&royalty_payment_address).unwrap())
                } else {
                    None
                },
                is_paused: false,
            },
        )?;

        Ok(Response::new())
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
            contract_address,
            &Collection {
                royalty_percentage: royalty_percentage,
                royalty_payment_address: if let Some(royalty_payment_address) =
                    royalty_payment_address
                {
                    Some(deps.api.addr_validate(&royalty_payment_address).unwrap())
                } else {
                    None
                },
                is_paused,
            },
        )?;

        Ok(Response::new())
    }

    pub fn create_sale(
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
            (contract_address, token_id),
            &Sale {
                owner_address: info.sender,
                price,
            },
        )?;

        // check approval
        Ok(Response::new())
    }
}

#[cfg(test)]
mod tests {}
