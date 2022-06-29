use crate::error::ContractError::{BaseError, Std};
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
    BaseError(#[from] cw721_base::ContractError),
}
