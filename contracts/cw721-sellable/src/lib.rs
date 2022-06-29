mod error;
mod execute;
mod msg;
mod query;

use crate::msg::Cw721SellableExecuteMsg;
use cosmwasm_std::{Empty, Uint64};
use cw2981_royalties::Trait;
use cw721_base::Cw721Contract;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

// see: https://docs.opensea.io/docs/metadata-standards
#[derive(Serialize, Deserialize, Clone, PartialEq, JsonSchema, Debug, Default)]
pub struct Metadata {
    pub image: Option<String>,
    pub image_data: Option<String>,
    pub external_url: Option<String>,
    pub description: Option<String>,
    pub name: Option<String>,
    pub attributes: Option<Vec<Trait>>,
    pub background_color: Option<String>,
    pub animation_url: Option<String>,
    pub youtube_url: Option<String>,
    /// This is how much the minter takes as a cut when sold
    /// royalties are owed on this token if it is Some
    pub royalty_percentage: Option<u64>,
    /// The payment address, may be different to or the same
    /// as the minter addr
    /// question: how do we validate this?
    pub royalty_payment_address: Option<String>,
    pub list_price: Option<Uint64>,
}

pub type Extension = Option<Metadata>;

pub type MintExtension = Option<Extension>;

pub type Cw721SellableContract<'a> = Cw721Contract<'a, Extension, Empty, Empty>;

pub type ExecuteMsg = Cw721SellableExecuteMsg<Extension>;

// #[cfg(not(feature = "library"))]
mod entry {
    use super::*;

    use crate::error::ContractError;
    use crate::execute::{try_buy, try_list};
    use crate::msg::{Cw721SellableExecuteMsg, Cw721SellableQueryMsg};
    use crate::query::listed_tokens;
    use cosmwasm_std::{entry_point, to_binary};
    use cosmwasm_std::{Binary, Deps, DepsMut, Env, MessageInfo, Response, StdResult};
    use cw2981_royalties::InstantiateMsg;

    #[entry_point]
    pub fn instantiate(
        deps: DepsMut,
        env: Env,
        info: MessageInfo,
        msg: InstantiateMsg,
    ) -> StdResult<Response> {
        Cw721SellableContract::default().instantiate(deps, env, info, msg)
    }

    #[entry_point]
    pub fn query(deps: Deps, env: Env, msg: Cw721SellableQueryMsg) -> StdResult<Binary> {
        match msg {
            Cw721SellableQueryMsg::ListedTokens { limit, start_after } => {
                to_binary(&listed_tokens(deps, start_after, limit)?)
            }
            Cw721SellableQueryMsg::Cw2981Query(cw2981_msg) => Cw721SellableContract::default()
                .query(deps, env, cw2981_msg.into())
                .map_err(|e| e.into()),
        }
    }

    #[entry_point]
    pub fn execute(
        deps: DepsMut,
        env: Env,
        info: MessageInfo,
        msg: ExecuteMsg,
    ) -> Result<Response, ContractError> {
        use Cw721SellableExecuteMsg::*;

        match msg {
            List { listings } => try_list(deps, env, info, listings),
            Buy { limit } => try_buy(deps, info, limit),
            BaseMsg(base_msg) => Cw721SellableContract::default()
                .execute(deps, env, info, base_msg)
                .map_err(|x| x.into()),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use cosmwasm_std::{from_binary, Coin, MessageInfo, OwnedDeps, Response, StdResult};

    use crate::error::ContractError;
    use crate::msg::Cw721SellableQueryMsg;
    use crate::query::ListedTokensResponse;
    use cosmwasm_std::testing::{
        mock_dependencies_with_balances, mock_env, mock_info, MockApi, MockQuerier, MockStorage,
    };
    use schemars::Map;
    use serde::de::DeserializeOwned;

    const CREATOR: &str = "creator";
    const OWNER: &str = "owner";
    const BUYER: &str = "buyer";
    const NO_MONEY: &str = "no_money";

    struct Context<'a> {
        pub deps: OwnedDeps<MockStorage, MockApi, MockQuerier>,
        contract: Cw721SellableContract<'a>,
        pub creator_info: MessageInfo,
    }

    struct ContractInfo {
        pub name: String,
        pub symbol: String,
    }

    impl Context<'_> {
        pub fn new<'a>(contract_info: ContractInfo) -> Context<'a> {
            let million_burnt = [Coin::new(1_000_000, "burnt")];
            let zero_burnt = [Coin::new(0, "burnt")];
            let mut deps = mock_dependencies_with_balances(&[
                (CREATOR, &million_burnt),
                (OWNER, &million_burnt),
                (BUYER, &million_burnt),
                (NO_MONEY, &zero_burnt),
            ]);

            let contract = Cw721SellableContract::default();
            let creator_info = mock_info(CREATOR, &[]);
            let init_msg = cw721_base::InstantiateMsg {
                name: contract_info.name,
                symbol: contract_info.symbol,
                minter: CREATOR.to_string(),
            };
            contract
                .instantiate(deps.as_mut(), mock_env(), creator_info.clone(), init_msg)
                .unwrap();

            Context {
                deps,
                contract,
                creator_info,
            }
        }

        pub fn execute(
            &mut self,
            info: Option<MessageInfo>,
            msg: ExecuteMsg,
        ) -> Result<Response, ContractError> {
            let creator_info = info.unwrap_or(self.creator_info.clone());
            entry::execute(self.deps.as_mut(), mock_env(), creator_info, msg)
        }

        pub fn query<T: DeserializeOwned>(&self, msg: Cw721SellableQueryMsg) -> StdResult<T> {
            let binary_res = entry::query(self.deps.as_ref(), mock_env(), msg);
            binary_res.and_then(|bin| from_binary(&bin))
        }
    }

    impl Default for Context<'_> {
        fn default() -> Self {
            Context::new(ContractInfo {
                name: "SpaceShips".into(),
                symbol: "SPACE".into(),
            })
        }
    }

    #[test]
    fn list_token() {
        let mut context = Context::default();

        // Mint tokens
        let token_ids = ["Enterprise", "Voyager"];
        for token_id in token_ids {
            let mint_msg = cw721_base::MintMsg {
                token_id: token_id.to_string(),
                owner: OWNER.to_string(),
                token_uri: Some("https://starships.example.com/Starship/Enterprise.json".into()),
                extension: Some(Metadata {
                    description: Some("Spaceship with Warp Drive".into()),
                    name: Some(format!("Starship USS {}", token_id).to_string()),
                    ..Metadata::default()
                }),
            };
            let exec_msg = ExecuteMsg::BaseMsg(cw721_base::ExecuteMsg::Mint(mint_msg.clone()));
            context.execute(None, exec_msg).unwrap();
        }

        // Query saleable tokens
        let query_msg = Cw721SellableQueryMsg::ListedTokens {
            start_after: None,
            limit: None,
        };
        let query_res: ListedTokensResponse = context.query(query_msg.clone()).unwrap();
        assert_eq!(0, query_res.tokens.len());

        let owner_info = mock_info(OWNER, &[]);
        let list_msg = Cw721SellableExecuteMsg::List {
            listings: Map::from([("Voyager".to_string(), Uint64::from(30 as u8))]),
        };
        let exec_res = context.execute(Some(owner_info.clone()), list_msg);
        exec_res.expect("expected list call to be successful");

        let query_res: ListedTokensResponse = context.query(query_msg.clone()).unwrap();
        assert_eq!(1, query_res.tokens.len());
        let (listed_token_id, listed_token_info) = query_res.tokens.get(0).unwrap();
        assert_eq!(
            listed_token_info
                .extension
                .clone()
                .unwrap()
                .list_price
                .unwrap(),
            Uint64::from(30 as u8),
            "listed token price did not match expectation"
        );
        assert_eq!(
            *listed_token_id,
            "Voyager".to_string(),
            "listed token id did not match expectation"
        );
    }

    #[test]
    fn buy_token() {
        let mut context = Context::default();
        // Mint tokens
        let token_ids = ["Enterprise", "Voyager"];
        for token_id in token_ids {
            let mint_msg = cw721_base::MintMsg {
                token_id: token_id.to_string(),
                owner: OWNER.to_string(),
                token_uri: Some("https://starships.example.com/Starship/Enterprise.json".into()),
                extension: Some(Metadata {
                    description: Some("Spaceship with Warp Drive".into()),
                    name: Some(format!("Starship USS {}", token_id).to_string()),
                    ..Metadata::default()
                }),
            };
            let exec_msg = ExecuteMsg::BaseMsg(cw721_base::ExecuteMsg::Mint(mint_msg.clone()));
            context
                .execute(None, exec_msg)
                .expect("expected mint to succeed");
        }

        // List a token
        let owner_info = mock_info(OWNER, &[]);
        let list_msg = Cw721SellableExecuteMsg::List {
            listings: Map::from([("Voyager".to_string(), Uint64::from(30 as u64))]),
        };
        context
            .execute(Some(owner_info.clone()), list_msg)
            .expect("expected list call to be successful");

        // Buy a token
        let create_buy_msg = |limit: u64| Cw721SellableExecuteMsg::Buy {
            limit: Uint64::new(limit),
        };
        let buyer_info = mock_info(BUYER, &[]);
        let no_money_info = mock_info(NO_MONEY, &[]);
        context
            .execute(Some(buyer_info.clone()), create_buy_msg(20))
            .expect_err("expected buy below list price to fail");

        context
            .execute(Some(no_money_info.clone()), create_buy_msg(30))
            .expect_err("expected buy from user without funds to fail");

        context
            .execute(Some(buyer_info.clone()), create_buy_msg(30))
            .expect("expected buy at list price to succeed");
    }
}
