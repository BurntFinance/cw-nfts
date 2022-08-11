use cosmwasm_std::{StdError, Uint64};
use thiserror::Error;

#[derive(Error, Debug)]
pub enum ContractError {
    #[error("{0}")]
    Std(#[from] StdError),

    #[error("Unauthorized")]
    Unauthorized,

    #[error("Redeemed")]
    Redeemed,

    #[error("Locked")]
    Locked,

    #[error("No tokens listed for sale")]
    NoListedTokensError,

    #[error("Limit of {limit} below lowest offer of {lowest_price}")]
    LimitBelowLowestOffer { limit: Uint64, lowest_price: Uint64 },

    #[error("{0}")]
    BaseError(#[from] cw721_base::ContractError),
}
