extern crate core;

mod error;
mod execute;
mod msg;
mod query;
mod test_utils;

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
    pub locked: Option<bool>,
    pub redeemed: Option<bool>
}

pub type Extension = Option<Metadata>;

pub type MintExtension = Option<Extension>;

pub type Cw721SellableContract<'a> = Cw721Contract<'a, Extension, Empty, Empty>;

pub type ExecuteMsg = Cw721SellableExecuteMsg<Extension>;

// #[cfg(not(feature = "library"))]
mod entry {
    use super::*;

    use crate::error::ContractError;
    use crate::execute::{try_buy, try_list, try_redeem};
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
            RedeemTicket { address, ticket_id } => try_redeem(deps, info, address, &ticket_id),
            BaseMsg(base_msg) => Cw721SellableContract::default()
                .execute(deps, env, info, base_msg)
                .map_err(|x| x.into()),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_utils::test_utils::{Context, ContractInfo};
    use cosmwasm_std::{Addr, BankMsg, Coin, CosmosMsg};
    use crate::entry::{execute, instantiate, query};
    use crate::error::ContractError;
    
    use crate::msg::Cw721SellableQueryMsg;
    use crate::query::ListedTokensResponse;
    use cosmwasm_std::testing::{mock_info};
    use schemars::Map;
    use cw2981_royalties::InstantiateMsg;
    use cw2981_royalties::msg::{Cw2981QueryMsg};
    use cw721::{NftInfoResponse};
    use cw721::Cw721Query;

    const CREATOR: &str = "creator";
    const OWNER: &str = "owner";
    const BUYER: &str = "buyer";
    const NO_MONEY: &str = "no_money";

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
            let creator_info = mock_info(CREATOR, &[]);
            context.execute(creator_info, exec_msg).unwrap();
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
        let exec_res = context.execute(owner_info.clone(), list_msg);
        exec_res.expect("expected list call to be successful");

        let mut query_res: ListedTokensResponse = context.query(query_msg.clone()).unwrap();
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

        let delist_msg = Cw721SellableExecuteMsg::List {
            listings: Map::from([("Voyager".to_string(), Uint64::zero())]),
        };
        context
            .execute(owner_info.clone(), delist_msg)
            .expect("expected delist to be successful");

        query_res = context.query(query_msg.clone()).unwrap();
        assert_eq!(0, query_res.tokens.len());
    }

    #[test]
    fn buy_token() {
        let million_tokens = &[Coin::new(1_000_000, "burnt")];
        let zero_tokens = &[Coin::new(0, "burnt")];
        let balances: &[(&str, &[Coin])] = &[
            (CREATOR, million_tokens),
            (OWNER, million_tokens),
            (BUYER, million_tokens),
            (NO_MONEY, zero_tokens),
        ];
        let mut context = Context::new(
            ContractInfo {
                name: "SpaceShips".into(),
                symbol: "SPACE".into(),
            },
            CREATOR,
            Some(balances),
        );
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
            let creator_info = mock_info(CREATOR, &[]);
            context
                .execute(creator_info, exec_msg)
                .expect("expected mint to succeed");
        }

        // List a token
        let owner_info = mock_info(OWNER, &[]);
        let list_msg = Cw721SellableExecuteMsg::List {
            listings: Map::from([("Voyager".to_string(), Uint64::from(30 as u64))]),
        };
        context
            .execute(owner_info.clone(), list_msg)
            .expect("expected list call to be successful");

        // Buy a token
        let create_buy_msg = |limit: u64| Cw721SellableExecuteMsg::Buy {
            limit: Uint64::new(limit),
        };
        let buyer_info = mock_info(BUYER, &[]);
        let no_money_info = mock_info(NO_MONEY, &[]);
        context
            .execute(buyer_info.clone(), create_buy_msg(20))
            .expect_err("expected buy below list price to fail");

        context
            .execute(no_money_info.clone(), create_buy_msg(30))
            .expect_err("expected buy from user without funds to fail");

        let response = context
            .execute(buyer_info.clone(), create_buy_msg(30))
            .expect("expected buy at list price to succeed");

        assert_eq!(
            response.messages.len(),
            1,
            "expected one message in response"
        );

        let message = &response.messages.get(0).unwrap().msg;
        match message {
            CosmosMsg::Bank(BankMsg::Send { to_address, amount })
                if to_address.eq(OWNER)
                    && amount == &Vec::from([Coin::new(30 as u128, "burnt")]) =>
            {
                assert!(true)
            }
            _ => assert!(false),
        }
    }

    #[test]
    fn lowest_listing_sells() {
        let million_tokens = &[Coin::new(1_000_000, "burnt")];
        let zero_tokens = &[Coin::new(0, "burnt")];
        let balances: &[(&str, &[Coin])] = &[
            (CREATOR, million_tokens),
            (OWNER, million_tokens),
            (BUYER, million_tokens),
            (NO_MONEY, zero_tokens),
        ];
        let mut context = Context::new(
            ContractInfo {
                name: "SpaceShips".into(),
                symbol: "SPACE".into(),
            },
            CREATOR,
            Some(balances),
        );
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
            let creator_info = mock_info(CREATOR, &[]);
            context
                .execute(creator_info, exec_msg)
                .expect("expected mint to succeed");
        }

        // List a token
        let owner_info = mock_info(OWNER, &[]);
        let list_msg = Cw721SellableExecuteMsg::List {
            listings: Map::from([
                ("Voyager".to_string(), Uint64::from(30 as u64)),
                ("Enterprise".to_string(), Uint64::from(25 as u64)),
            ]),
        };
        context
            .execute(owner_info.clone(), list_msg)
            .expect("expected list call to be successful");

        // Buy a token
        let create_buy_msg = |limit: u64| Cw721SellableExecuteMsg::Buy {
            limit: Uint64::new(limit),
        };
        let buyer_info = mock_info(BUYER, &[]);
        let no_money_info = mock_info(NO_MONEY, &[]);
        context
            .execute(buyer_info.clone(), create_buy_msg(20))
            .expect_err("expected buy below list price to fail");

        context
            .execute(no_money_info.clone(), create_buy_msg(30))
            .expect_err("expected buy from user without funds to fail");

        let response = context
            .execute(buyer_info.clone(), create_buy_msg(30))
            .expect("expected buy at list price to succeed");

        let message = &response.messages.get(0).unwrap().msg;
        match message {
            CosmosMsg::Bank(BankMsg::Send { to_address, amount })
                if to_address.eq(OWNER) && amount == &Vec::from([Coin::new(25, "burnt")]) =>
            {
                assert!(true)
            }
            _ => assert!(false),
        }

        let enterprise_info = context
            .contract
            .tokens
            .load(&context.deps.storage, "Enterprise")
            .expect("expected token to exist");

        assert_eq!(enterprise_info.owner, Addr::unchecked(BUYER));
    }
    #[test]
    fn redeem_ticket() {
        let mut context = Context::new(
            ContractInfo {
                name: "Ticketing".to_string(),
                symbol: "TICK".to_string(),
            }, 
            CREATOR,
            Some(&[])
        );

        // Make sure only the owner of the contract can call method
        let msg = Cw721SellableExecuteMsg::RedeemTicket {
            address: String::from(OWNER),
            ticket_id: String::from("OWNER_TICKET")
        };
        let exec_res = context.execute(mock_info(OWNER, &[]), msg).err();
        match exec_res {
            Some(ContractError::Unauthorized) => assert!(true),
            _ => assert!(false),
        };

        // Throw Error if ticket does exists in the contract
        let msg = Cw721SellableExecuteMsg::RedeemTicket {
            address: String::from(OWNER),
            ticket_id: String::from("OWNER_TICKET")
        };
        let exec_res = context.execute(mock_info(CREATOR, &[]), msg).err();
        match exec_res {
            None => assert!(false),
            _ => assert!(true)
        };

        // Make sure the owner param is the same as ticket owner in contract
        let token_id = "Burnt_Event#1";
        let mint_msg = cw721_base::MintMsg {
            token_id: token_id.to_string(),
            owner: OWNER.to_string(), // Create a ticket belonging to OWNER
            token_uri: Some("https://starships.example.com/Starship/Enterprise.json".into()),
            extension: Some(Metadata {
                description: Some("Burnt event ticket #1".into()),
                name: Some("Ticket #1".to_string()),
                ..Metadata::default()
            }),
        };
        let exec_msg = ExecuteMsg::BaseMsg(cw721_base::ExecuteMsg::Mint(mint_msg.clone()));
        context.execute(mock_info(CREATOR, &[]), exec_msg).unwrap();

        let msg = Cw721SellableExecuteMsg::RedeemTicket {
            address: String::from(BUYER), // Ticket owner is BUYER here
            ticket_id: String::from("Burnt_Event#1")
        };
        let exec_res = context.execute(mock_info(CREATOR, &[]), msg).err();
        match exec_res {
            Some(ContractError::Unauthorized) => assert!(true),
            _ => assert!(false)
        };

        // Make sure the ticket is not locked  or redeemed
        let locked_token_id = "Burnt_Locked#1";
        let mint_msg = cw721_base::MintMsg {
            token_id: locked_token_id.to_string(),
            owner: OWNER.to_string(),
            token_uri: Some("https://starships.example.com/Starship/Enterprise.json".into()),
            extension: Some(Metadata {
                description: Some("Burnt locked event ticket #1".into()),
                name: Some("Locked ticket #1".to_string()),
                locked: Some(true),
                ..Metadata::default()
            }),
        };
        let exec_msg = ExecuteMsg::BaseMsg(cw721_base::ExecuteMsg::Mint(mint_msg.clone()));
        context.execute(mock_info(CREATOR, &[]), exec_msg).unwrap();
        // Try to redeem locked ticket
        let msg = Cw721SellableExecuteMsg::RedeemTicket {
            address: String::from(OWNER),
            ticket_id: String::from("Burnt_Locked#1")
        };
        let exec_res = context.execute(mock_info(CREATOR, &[]), msg).err();
        match exec_res {
            Some(ContractError::Locked) => assert!(true),
            _ => assert!(false)
        };

        // Make sure the ticket metadata is updated
        let token_id = "Burnt_Event#2";
        let mint_msg = cw721_base::MintMsg {
            token_id: token_id.to_string(),
            owner: OWNER.to_string(), // Create a ticket belonging to OWNER
            token_uri: Some("https://starships.example.com/Starship/Enterprise.json".into()),
            extension: Some(Metadata {
                description: Some("Burnt event ticket #1".into()),
                name: Some("Ticket #2".to_string()),
                locked: Some(false),
                redeemed: Some(false),
                ..Metadata::default()
            }),
        };
        let exec_msg = ExecuteMsg::BaseMsg(cw721_base::ExecuteMsg::Mint(mint_msg.clone()));
        context.execute(mock_info(CREATOR, &[]), exec_msg).unwrap();

        let msg = Cw721SellableExecuteMsg::RedeemTicket {
            address: String::from(OWNER),
            ticket_id: String::from(token_id)
        };
        context.execute(mock_info(CREATOR, &[]), msg).unwrap();

        let contract = Cw721SellableContract::default();

        let res = contract.nft_info(context.deps.as_ref(), token_id.to_string()).unwrap();
        match res {
            NftInfoResponse::<Extension> { token_uri, extension } => {
                let metadata = extension.unwrap();
                if metadata.redeemed.unwrap() {
                    assert!(true);
                } else {
                    assert!(false);
                }
            },
            _ => assert!(false)
        }
    }
}
