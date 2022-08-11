use cosmwasm_std::Uint64;
use schemars::{JsonSchema, Map};
use serde::{Deserialize, Serialize};

use crate::Extension;
use cw2981_royalties::msg::Cw2981QueryMsg;

#[derive(Serialize, Deserialize, Clone, PartialEq, JsonSchema, Debug)]
#[serde(rename_all = "snake_case")]
pub enum Cw721SellableExecuteMsg<T> {
    BaseMsg(cw721_base::ExecuteMsg<T>),

    /// Sellable specific functions

    /// Lists the NFT at the given price
    List {
        listings: Map<String, Uint64>,
    },

    /// Purchases the cheapest listed NFT, below or at the limit
    Buy {
        limit: Uint64,
    },

    /// Mark ticket has redeemed
    RedeemTicket {
        address: String,
        ticket_id: String
    }
}

type BaseExecuteMsg = cw721_base::ExecuteMsg<Extension>;

impl From<Cw721SellableExecuteMsg<Extension>> for BaseExecuteMsg {
    fn from(msg: Cw721SellableExecuteMsg<Extension>) -> BaseExecuteMsg {
        use Cw721SellableExecuteMsg::BaseMsg;

        match msg {
            BaseMsg(msg) => msg,
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

    Cw2981Query(Cw2981QueryMsg),
}

impl From<Cw721SellableQueryMsg> for Cw2981QueryMsg {
    fn from(msg: Cw721SellableQueryMsg) -> Cw2981QueryMsg {
        use Cw721SellableQueryMsg::Cw2981Query;

        match msg {
            Cw2981Query(msg) => msg,
            _ => panic!("cannot covert {:?} to Cw2981QueryMsg", msg),
        }
    }
}
