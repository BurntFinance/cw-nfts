use cosmwasm_std::{Addr, BankMsg, Coin, Deps, DepsMut, Env, MessageInfo, Order, Response, StdResult, Uint64};
use cw721_base::state::TokenInfo;
use schemars::Map;
use crate::{Cw721SellableContract, Extension, msg};
use crate::error::ContractError;

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
        if (list_price < lowest_price) && (list_price > 0) {
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
