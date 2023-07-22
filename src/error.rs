use cosmwasm_std::StdError;
use cw_ownable::OwnershipError;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum ContractError {
    #[error("{0}")]
    Std(#[from] StdError),

    #[error("Unauthorized")]
    Unauthorized {},

    #[error("NotApproved")]
    NotApproved {},

    #[error("CollectionAlreadyRegistered")]
    CollectionAlreadyRegistered {},

    #[error("CollectionNotExist")]
    CollectionNotExist {},

    #[error("SaleDoesNotExist")]
    SaleDoesNotExist {},

    #[error("DenomNotSupported")]
    DenomNotSupported {},

    #[error("InsufficientFunds")]
    InsufficientFunds {},

    #[error(transparent)]
    Ownership(#[from] OwnershipError),
}
