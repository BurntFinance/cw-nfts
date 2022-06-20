use cosmwasm_std::{Binary, Uint128, Uint64};
use schemars::{JsonSchema, Map};
use serde::{Deserialize, Serialize};

use crate::Extension;
use cw2981_royalties::msg::Cw2981QueryMsg;
use cw2981_royalties::MintMsg;
use cw721::Expiration;

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

type BaseExecuteMsg = cw721_base::ExecuteMsg<Extension>;

impl From<Cw721SellableExecuteMsg<Extension>> for BaseExecuteMsg {
    fn from(msg: Cw721SellableExecuteMsg<Extension>) -> BaseExecuteMsg {
        match msg {
            Cw721SellableExecuteMsg::TransferNft {
                recipient,
                token_id,
            } => BaseExecuteMsg::TransferNft {
                recipient,
                token_id,
            },
            Cw721SellableExecuteMsg::SendNft {
                contract,
                token_id,
                msg,
            } => BaseExecuteMsg::SendNft {
                contract,
                token_id,
                msg,
            },
            Cw721SellableExecuteMsg::Approve {
                spender,
                token_id,
                expires,
            } => BaseExecuteMsg::Approve {
                spender,
                token_id,
                expires,
            },
            Cw721SellableExecuteMsg::Revoke { spender, token_id } => {
                BaseExecuteMsg::Revoke { spender, token_id }
            }
            Cw721SellableExecuteMsg::ApproveAll { operator, expires } => {
                BaseExecuteMsg::ApproveAll { operator, expires }
            }
            Cw721SellableExecuteMsg::RevokeAll { operator } => {
                BaseExecuteMsg::RevokeAll { operator }
            }
            Cw721SellableExecuteMsg::Mint(mint_msg) => BaseExecuteMsg::Mint(mint_msg),
            Cw721SellableExecuteMsg::Burn { token_id } => BaseExecuteMsg::Burn { token_id },

            _ => panic!("cannot covert {:?} to Cw2981QueryMsg", msg),
        }
    }
}

#[derive(Serialize, Deserialize, Clone, PartialEq, JsonSchema, Debug)]
#[serde(rename_all = "snake_case")]
pub enum Cw721SellableQueryMsg {
    /// Returns all currently listed tokens
    ListedTokens {
        start_after: Option<String>,
        limit: Option<u32>,
    },

    /// Should be called on sale to see if royalties are owed
    /// by the marketplace selling the NFT, if CheckRoyalties
    /// returns true
    /// See https://eips.ethereum.org/EIPS/eip-2981
    RoyaltyInfo {
        token_id: String,
        // the denom of this sale must also be the denom returned by RoyaltiesInfoResponse
        // this was originally implemented as a Coin
        // however that would mean you couldn't buy using CW20s
        // as CW20 is just mapping of addr -> balance
        sale_price: Uint128,
    },
    /// Called against contract to determine if this NFT
    /// implements royalties. Should return a boolean as part of
    /// CheckRoyaltiesResponse - default can simply be true
    /// if royalties are implemented at token level
    /// (i.e. always check on sale)
    CheckRoyalties {},
    /// Return the owner of the given token, error if token does not exist
    /// Return type: OwnerOfResponse
    OwnerOf {
        token_id: String,
        /// unset or false will filter out expired approvals, you must set to true to see them
        include_expired: Option<bool>,
    },
    /// List all operators that can access all of the owner's tokens.
    /// Return type: `OperatorsResponse`
    AllOperators {
        owner: String,
        /// unset or false will filter out expired approvals, you must set to true to see them
        include_expired: Option<bool>,
        start_after: Option<String>,
        limit: Option<u32>,
    },
    /// Total number of tokens issued
    NumTokens {},

    /// With MetaData Extension.
    /// Returns top-level metadata about the contract: `ContractInfoResponse`
    ContractInfo {},
    /// With MetaData Extension.
    /// Returns metadata about one particular token, based on *ERC721 Metadata JSON Schema*
    /// but directly from the contract: `NftInfoResponse`
    NftInfo { token_id: String },
    /// With MetaData Extension.
    /// Returns the result of both `NftInfo` and `OwnerOf` as one query as an optimization
    /// for clients: `AllNftInfo`
    AllNftInfo {
        token_id: String,
        /// unset or false will filter out expired approvals, you must set to true to see them
        include_expired: Option<bool>,
    },

    /// With Enumerable extension.
    /// Returns all tokens owned by the given address, [] if unset.
    /// Return type: TokensResponse.
    Tokens {
        owner: String,
        start_after: Option<String>,
        limit: Option<u32>,
    },
    /// With Enumerable extension.
    /// Requires pagination. Lists all token_ids controlled by the contract.
    /// Return type: TokensResponse.
    AllTokens {
        start_after: Option<String>,
        limit: Option<u32>,
    },
}

impl From<Cw721SellableQueryMsg> for Cw2981QueryMsg {
    fn from(msg: Cw721SellableQueryMsg) -> Cw2981QueryMsg {
        match msg {
            Cw721SellableQueryMsg::OwnerOf {
                token_id,
                include_expired,
            } => Cw2981QueryMsg::OwnerOf {
                token_id,
                include_expired,
            },
            Cw721SellableQueryMsg::AllOperators {
                owner,
                include_expired,
                start_after,
                limit,
            } => Cw2981QueryMsg::AllOperators {
                owner,
                include_expired,
                start_after,
                limit,
            },
            Cw721SellableQueryMsg::NumTokens {} => Cw2981QueryMsg::NumTokens {},
            Cw721SellableQueryMsg::ContractInfo {} => Cw2981QueryMsg::ContractInfo {},
            Cw721SellableQueryMsg::NftInfo { token_id } => Cw2981QueryMsg::NftInfo { token_id },
            Cw721SellableQueryMsg::AllNftInfo {
                token_id,
                include_expired,
            } => Cw2981QueryMsg::AllNftInfo {
                token_id,
                include_expired,
            },
            Cw721SellableQueryMsg::Tokens {
                owner,
                start_after,
                limit,
            } => Cw2981QueryMsg::Tokens {
                owner,
                start_after,
                limit,
            },
            Cw721SellableQueryMsg::AllTokens { start_after, limit } => {
                Cw2981QueryMsg::AllTokens { start_after, limit }
            }
            Cw721SellableQueryMsg::CheckRoyalties {} => Cw2981QueryMsg::CheckRoyalties {},
            Cw721SellableQueryMsg::RoyaltyInfo {
                token_id,
                sale_price,
            } => Cw2981QueryMsg::RoyaltyInfo {
                token_id,
                sale_price,
            },
            _ => panic!("cannot covert {:?} to Cw2981QueryMsg", msg),
        }
    }
}
