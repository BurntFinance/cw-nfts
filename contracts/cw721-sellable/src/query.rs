use crate::{Cw721SellableContract, Extension};
use cosmwasm_std::{Deps, Order, StdResult, Uint64};
use cw721_base::state::TokenInfo;
use cw_storage_plus::Bound;
use schemars::JsonSchema;
use schemars::Map;
use serde::{Deserialize, Serialize};

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

    let token_map: Map<String, TokenInfo<Extension>> = Map::new();

    for result in contract
        .tokens
        .range(deps.storage, start, None, Order::Ascending)
        .filter(|result| result.unwrap().1.extension.unwrap().list_price > Uint64::zero())
        .take(limit)
    {
        let (id, info) = result.unwrap();
        token_map.insert(id, info);
    }

    Ok(ListedTokensResponse { tokens: token_map })
}

#[derive(Serialize, Deserialize, Clone, PartialEq, JsonSchema, Debug)]
pub struct ListedTokensResponse {
    /// Contains all token_ids in lexicographical ordering
    /// If there are more than `limit`, use `start_from` in future queries
    /// to achieve pagination.
    pub tokens: Map<String, TokenInfo<Extension>>,
}
