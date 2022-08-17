use crate::error::ContractError;
use crate::error::ContractError::{LimitBelowLowestOffer, NoListedTokensError};
use crate::{Cw721SellableContract, Extension};

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
        let opt_price = info.extension.as_ref().and_then(|meta| meta.list_price);
        let metadata = info.extension.ok_or(ContractError::NoMetadataPresent)?;
        if let Some(list_price) = opt_price {
            if !metadata.redeemed {
                if let Ok((_, _, lowest_price)) = lowest {
                    if list_price < lowest_price {
                        lowest = Ok((id, info.owner, list_price))
                    }
                } else {
                    lowest = Ok((id, info.owner, list_price))
                }
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

pub fn try_redeem(
    deps: DepsMut,
    info: MessageInfo,
    address: String,
    ticket_id: &String,
) -> Result<Response, ContractError> {
    let contract = Cw721SellableContract::default();

    // Validate only contract owner can call method
    let minter = contract.minter.load(deps.storage)?;
    if info.sender != minter {
        return Err(ContractError::Unauthorized {});
    }

    // Load ticket, error if ticket does not exist
    let mut ticket = contract.tokens.load(deps.storage, ticket_id)?;

    // Make sure owner param matches ticket owner
    if ticket.owner != address {
        return Err(ContractError::Unauthorized);
    }

    // Make sure ticket isn't locked or redeemed
    if let Some(ref mut metadata) = ticket.extension {
        if metadata.redeemed {
            return Err(ContractError::TicketRedeemed);
        } else if metadata.locked {
            return Err(ContractError::TicketLocked);
        } else {
            // Mark ticket as redeemed and locked
            metadata.redeemed = true;
            metadata.locked = true;
            // de-list ticket if it is listed
            metadata.list_price = None;
        }
    } else {
        return Err(ContractError::NoMetadataPresent);
    }

    // Save change into storage
    contract.tokens.save(deps.storage, ticket_id, &ticket)?;

    return Ok(Response::new().add_attribute("method", "redeem"));
}

// todo: is there a way to use the cw721 base function here?
pub fn check_can_send(
    deps: Deps,
    env: &Env,
    info: &MessageInfo,
    token_id: &String,
) -> Result<(), ContractError> {
    let contract = Cw721SellableContract::default();
    let mut token = contract.tokens.load(deps.storage, token_id)?;
    // confirm token aren't locked or redeemed
    if let Some(ref mut metadata) = token.extension {
        if metadata.redeemed {
            return Err(ContractError::TicketRedeemed);
        } else if metadata.locked {
            return Err(ContractError::TicketLocked);
        }
    }
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

fn get_ticket_id(msg: &cw721_base::ExecuteMsg<Extension>) -> Option<String> {
    // get token id from msg
    return match msg {
        cw721_base::ExecuteMsg::TransferNft { token_id, .. } => Some(token_id.to_string()),
        cw721_base::ExecuteMsg::SendNft { token_id, .. } => Some(token_id.to_string()),
        cw721_base::ExecuteMsg::Approve { token_id, .. } => Some(token_id.to_string()),
        cw721_base::ExecuteMsg::Revoke { token_id, .. } => Some(token_id.to_string()),
        _ => None,
    };
}

pub fn validate_locked_ticket(
    deps: &DepsMut,
    msg: &cw721_base::ExecuteMsg<Extension>,
) -> Result<(), ContractError> {
    let ticket_id = get_ticket_id(msg);

    if let Some(ticket_id) = ticket_id {
        let contract = Cw721SellableContract::default();
        let ticket = contract.tokens.load(deps.storage, ticket_id.as_str())?;
        // confirm token aren't locked or redeemed
        if let Some(ref metadata) = ticket.extension {
            if metadata.redeemed {
                return Err(ContractError::TicketRedeemed);
            } else if metadata.locked {
                return Err(ContractError::TicketLocked);
            } else {
                return Ok(());
            }
        } else {
            return Err(ContractError::NoMetadataPresent);
        }
    } else {
        return Ok(());
    }
}
