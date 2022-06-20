use crate::error::ContractError::BaseError;
use cosmwasm_std::StdError;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum ContractError {
    #[error("{0}")]
    Std(#[from] StdError),

    #[error("Unauthorized")]
    Unauthorized {},

    #[error("Custom Error val: {val:?}")]
    CustomError { val: String },

    #[error("No tokens listed for sale")]
    NoListedTokensError {},

    #[error("{0}")]
    BaseError(cw721_base::ContractError),
}

impl From<cw721_base::ContractError> for ContractError {
    fn from(err: cw721_base::ContractError) -> Self {
        BaseError(err)
    }
}
