mod msg;
mod error;

use cosmwasm_std::{Empty, Uint64};
use cw2981_royalties::Trait;
use crate::msg::Cw721SellableExecuteMsg;
use cw721_base::Cw721Contract;


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
    pub list_price: Uint64,
}

pub type Extension = Option<Metadata>;

pub type MintExtension = Option<Extension>;

pub type Cw721SellableContract<'a> = Cw721Contract<'a, Extension, Empty>;
pub type ExecuteMsg = Cw721SellableExecuteMsg<Extension>;

// #[cfg(not(feature = "library"))]
pub mod entry {
    use super::*;

    use cosmwasm_std::entry_point;
    use cosmwasm_std::{Binary, Deps, DepsMut, Env, MessageInfo, Response, StdResult};
    use cw2981_royalties::{Cw2981Contract, InstantiateMsg};
    use cw2981_royalties::msg::Cw2981QueryMsg;
    use crate::error::ContractError;
    use crate::msg::{Cw721SellableExecuteMsg, try_list};

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
    pub fn query(
        deps: DepsMut,
        env: Env,
        info: MessageInfo,
        msg: Cw2981QueryMsg,
    ) -> StdResult<Response> {
        Cw721SellableContract::default().instantiate(deps, env, info, msg)
    }

    #[entry_point]
    pub fn execute(
        deps: DepsMut,
        env: Env,
        info: MessageInfo,
        msg: ExecuteMsg,
    ) -> Result<Response, ContractError> {
        match msg {
            Cw721SellableExecuteMsg::List { listings } => try_list(deps, info, listings),
            _ => Cw2981Contract::default().execute(deps, env, info, msg.into()),
        }
    }
}
