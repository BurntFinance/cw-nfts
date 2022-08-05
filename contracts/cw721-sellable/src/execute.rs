use crate::error::ContractError;
use crate::error::ContractError::{LimitBelowLowestOffer, NoListedTokensError};
use crate::Cw721SellableContract;
use cosmwasm_std::{
    Addr, BankMsg, Coin, Deps, DepsMut, Env, MessageInfo, Order, Response, StdError, Uint64,
};
use schemars::Map;


pub fn try_buy(deps: DepsMut, info: MessageInfo, limit: Uint64) -> Result<Response, ContractError> {
    let coin = deps.querier.query_balance(&info.sender, "burnt")?;
    if coin.amount < limit.into() {
        return Err(ContractError::Unauthorized);
    }

    let contract = Cw721SellableContract::default();

    // todo: there might be a better way to do this than a scan
    let mut lowest: Result<(String, Addr, Uint64), ContractError> = Err(NoListedTokensError);
    for (id, info) in contract
        .tokens
        .range(deps.storage, None, None, Order::Ascending)
        .flatten()
    {
        let opt_price = info.extension.and_then(|meta| meta.list_price);
        if let Some(list_price) = opt_price {
            if let Ok((_, _, lowest_price)) = lowest {
                if list_price < lowest_price {
                    lowest = Ok((id, info.owner, list_price))
                }
            } else {
                lowest = Ok((id, info.owner, list_price))
            }
        }
    }

    lowest
        .and_then(|l @ (_, _, lowest_price)| {
            if lowest_price <= limit {
                Ok(l)
            } else {
                Err(LimitBelowLowestOffer {
                    limit,
                    lowest_price,
                })
            }
        })
        .and_then(|(lowest_token_id, lowest_token_owner, lowest_price)| {
            contract.tokens.update::<_, ContractError>(
                deps.storage,
                lowest_token_id.as_str(),
                |old| {
                    let mut token_info = old.unwrap();
                    let mut meta = token_info.extension.unwrap();
                    meta.list_price = None;
                    token_info.extension = Some(meta);
                    token_info.owner = info.sender.clone();
                    Ok(token_info)
                },
            )?;

            let payment_coin = Coin::new(lowest_price.u64() as u128, "burnt");

            Ok(Response::new()
                .add_attribute("method", "buy")
                .add_message(BankMsg::Send {
                    to_address: lowest_token_owner.to_string(),
                    amount: Vec::from([payment_coin]),
                }))
        })
}

pub fn try_list(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    listings: Map<String, Uint64>,
) -> Result<Response, ContractError> {
    let contract = Cw721SellableContract::default();
    for (token_id, price) in listings.iter() {
        check_can_send(deps.as_ref(), &env, &info, token_id)?;
        contract
            .tokens
            .update::<_, ContractError>(deps.storage, token_id, |old| {
                old.ok_or(StdError::not_found("SellableToken").into())
                    .map(|mut old| {
                        let opt_price = if (*price) > Uint64::new(0) {
                            Some(*price)
                        } else {
                            None
                        };
                        // TODO: get rid of this unwrap
                        let mut meta = old.extension.unwrap();
                        meta.list_price = opt_price;
                        old.extension = Some(meta);
                        old
                    })
            })?;
    }

    Ok(Response::new().add_attribute("method", "list"))
}

// todo: is there a way to use the cw721 base function here?
pub fn check_can_send(
    deps: Deps,
    env: &Env,
    info: &MessageInfo,
    token_id: &String,
) -> Result<(), ContractError> {
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
    contract
        .operators
        .may_load(deps.storage, (&token.owner, &info.sender))
        .map_err(|e| e.into())
        .and_then(|opt_exp| match opt_exp {
            Some(opt) if !opt.is_expired(&env.block) => Ok(()),
            _ => Err(ContractError::Unauthorized),
        })
}
