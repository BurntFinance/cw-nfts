use crate::{Cw721SellableContract, Extension, Metadata};
use cosmwasm_std::{Deps, Order, StdResult};
use cw721_base::state::TokenInfo;
use cw_storage_plus::Bound;
use schemars::JsonSchema;
use schemars::Map;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

const DEFAULT_LIMIT: u32 = 500;
const MAX_LIMIT: u32 = 10000;

pub fn listed_tokens(
    deps: Deps,
    start_after: Option<String>,
    limit: Option<u32>,
) -> StdResult<ListedTokensResponse> {
    let contract = Cw721SellableContract::default();

    let limit = limit.unwrap_or(DEFAULT_LIMIT).min(MAX_LIMIT) as usize;
    let start = start_after.map(|s| Bound::ExclusiveRaw(s.into()));

    let token_vec: Vec<(String, TokenInfo<Extension>)> = contract
        .tokens
        .range(deps.storage, start, None, Order::Ascending)
        .flat_map(|result| match result {
            Ok(
                pair @ (
                    _,
                    TokenInfo {
                        extension:
                            Some(Metadata {
                                list_price: Some(_),
                                ..
                            }),
                        ..
                    },
                ),
            ) => Some(pair),
            _ => None,
        })
        .take(limit)
        .collect();

    Ok(ListedTokensResponse { tokens: token_vec })
}

#[derive(Serialize, Deserialize, Clone, PartialEq, JsonSchema, Debug)]
pub struct ListedTokensResponse {
    /// Contains all token_ids in lexicographical ordering
    /// If there are more than `limit`, use `start_from` in future queries
    /// to achieve pagination.
    pub tokens: Vec<(String, TokenInfo<Extension>)>,
}
