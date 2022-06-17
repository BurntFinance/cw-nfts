use cosmwasm_std::{Deps, Order, StdResult};
use cw_storage_plus::Bound;
use schemars::Map;
use cw721_base::state::TokenInfo;
use crate::{Cw721SellableContract, Extension};

pub fn listed_tokens(
    deps: Deps,
    start_after: Option<String>,
    limit: Option<u32>,
) -> StdResult<ListedTokensResponse> {
    let contract = Cw721SellableContract::default();

    let limit = limit.unwrap_or(DEFAULT_LIMIT).min(MAX_LIMIT) as usize;
    let start = start_after.map(|s| Bound::ExclusiveRaw(s.into()));

    let token_map: Map<String, TokenInfo<Extension>> = Map::new();

    for (id, info) in  contract
        .tokens
        .range(deps.storage, start, None, Order::Ascending)
        .filter(|(key, info): (&String, &TokenInfo<Extension>)| info.extension.unwrap().list_price > 0 )
        .take(limit)
        .collect() {
        token_map[id] = info;
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
