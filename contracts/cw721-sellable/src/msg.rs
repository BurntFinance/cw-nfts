use std::cmp::min;
use std::num::IntErrorKind::Empty;
use cosmwasm_std::{Addr, BankMsg, BankQuery, Binary, Coin, Deps, DepsMut, Env, MessageInfo, Order, Response, StdResult, Uint128, Uint64};
use cosmwasm_std::QueryRequest::Bank;
use cw_storage_plus::Bound;
use schemars::{JsonSchema, Map};
use serde::{Deserialize, Serialize};
use cw2981_royalties::MintMsg;
use cw721::Expiration;
use cw721_base::state::TokenInfo;
use crate::{Cw721SellableContract, Extension};
use crate::error::ContractError;

#[derive(Serialize, Deserialize, Clone, PartialEq, JsonSchema, Debug)]
#[serde(rename_all = "snake_case")]
pub enum Cw721SellableExecuteMsg<T> {
    /// Transfer is a base message to move a token to another account without triggering actions
    TransferNft { recipient: String, token_id: String },
    /// Send is a base message to transfer a token to a contract and trigger an action
    /// on the receiving contract.
    SendNft {
        contract: String,
        token_id: String,
        msg: Binary,
    },
    /// Allows operator to transfer / send the token from the owner's account.
    /// If expiration is set, then this allowance has a time/height limit
    Approve {
        spender: String,
        token_id: String,
        expires: Option<Expiration>,
    },
    /// Remove previously granted Approval
    Revoke { spender: String, token_id: String },
    /// Allows operator to transfer / send any token from the owner's account.
    /// If expiration is set, then this allowance has a time/height limit
    ApproveAll {
        operator: String,
        expires: Option<Expiration>,
    },
    /// Remove previously granted ApproveAll permission
    RevokeAll { operator: String },

    /// Mint a new NFT, can only be called by the contract minter
    Mint(MintMsg<T>),

    /// Burn an NFT the sender has access to
    Burn { token_id: String },

    /// Sellable specific functions

    /// Lists the NFT at the given price
    List { listings: Map<String, Uint64> },

    /// Purchases the cheapest listed NFT, below or at the limit
    Buy { limit: Uint64 },
}

pub fn try_list(deps: DepsMut, info: MessageInfo, listings: Map<String, Uint64>) -> Result<Response, ContractError> {
    let contract = Cw721SellableContract::default();
    for (token_id, price) in listings.iter() {
        check_can_send(deps.as_ref(), &env, &info, token_id)?;
        contract.tokens.update(deps.storage, token_id, |old| {
            let mut token_info = old.unwrap();
            let mut meta = token_info.extension.unwrap();
            meta.list_price = *price;
            token_info.extension = Some(meta);
            Ok(token_info)
        })?;
    }

    Ok(Response::new().add_attribute("method", "list"))
}

pub fn try_buy(deps: DepsMut, info: MessageInfo, price: Uint64) -> Result<Response, ContractError> {
    let coin = deps.querier.query_balance(info.sender, "burnt")?;
    if coin.amount < price {
        Err(ContractError::Unauthorized {})
    }

    let contract = Cw721SellableContract::default();

    // todo: there might be a better way to do this than a scan
    let mut lowest_price = Uint64::MAX;
    let mut lowest_token_id = String;
    let mut lowest_token_owner= Addr::unchecked("not-found");
    let mut found = false; // in the case there are no listed tokens that meet the limit
    let all: StdResult<(String, TokenInfo<Extension>)> = contract.tokens.range(deps.storage, None, None, Order::Ascending).collect();
    for (id, info) in all {
        let list_price = info.extension.unwrap().list_price;
        if list_price < lowest_price {
            found = true;
            lowest_price = list_price;
            lowest_token_id = id;
            lowest_token_owner = info.owner;
        }
    };

    if !found {
        Err(ContractError::NoListedTokensError {})
    }

    contract.tokens.update(deps.storage, lowest_token_id, |old| {
        let mut token_info = old.unwrap();
        let mut meta = token_info.extension.unwrap();
        meta.list_price = Uint64(0);
        token_info.extension = Some(meta);
        token_info.owner = info.sender.clone();
        Ok(token_info)
    })?;

    let payment_coin = Coin::new(price.into(), "burnt");

    Ok(Response::new().add_attribute("method", "buy")
        .add_message(BankMsg::Send {
            to_address: lowest_token_owner.to_string(),
            amount: Vec::from(payment_coin)
        }))
}


// todo: is there a way to use the cw721 base function here?
pub fn check_can_send(
    deps: Deps,
    env: &Env,
    info: &MessageInfo,
    token_id: &String) -> Result<(), ContractError> {
    let contract = Cw721SellableContract::default();
    let token = contract.tokens.load(deps.storage, token_id)?;
    if token.owner == info.sender {
        return Ok(());
    }

    // any non-expired token approval can send
    if token
        .approvals
        .iter()
        .any(|apr| apr.spender == info.sender && !apr.is_expired(&env.block))
    {
        return Ok(());
    }

    // operator can send
    let op = contract
        .operators
        .may_load(deps.storage, (&token.owner, &info.sender))?;
    match op {
        Some(ex) => {
            if ex.is_expired(&env.block) {
                Err(ContractError::Unauthorized {})
            } else {
                Ok(())
            }
        }
        None => Err(ContractError::Unauthorized {}),
    }
}
