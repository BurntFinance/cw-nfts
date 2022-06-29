use crate::error::ContractError;
use crate::msg::Cw721SellableQueryMsg;
use crate::{entry, Cw721SellableContract, ExecuteMsg};
use cosmwasm_std::testing::{
    mock_dependencies, mock_dependencies_with_balances, mock_env, mock_info, MockApi, MockQuerier,
    MockStorage,
};
use cosmwasm_std::{
    from_binary, Coin, MessageInfo, OwnedDeps, Response, StdResult,
};
use serde::de::DeserializeOwned;

pub struct Context<'a> {
    pub deps: OwnedDeps<MockStorage, MockApi, MockQuerier>,
    contract: Cw721SellableContract<'a>,
}

pub struct ContractInfo {
    pub name: String,
    pub symbol: String,
}

impl Context<'_> {
    pub fn new<'a>(
        contract_info: ContractInfo,
        creator: &'a str,
        balances: Option<&[(&str, &[Coin])]>,
    ) -> Context<'a> {
        let mut deps = if let Some(balances) = balances {
            mock_dependencies_with_balances(balances)
        } else {
            mock_dependencies()
        };

        let contract = Cw721SellableContract::default();
        let creator_info = mock_info(creator, &[]);
        let init_msg = cw721_base::InstantiateMsg {
            name: contract_info.name,
            symbol: contract_info.symbol,
            minter: creator.to_string(),
        };
        contract
            .instantiate(deps.as_mut(), mock_env(), creator_info.clone(), init_msg)
            .unwrap();

        Context { deps, contract }
    }

    pub fn execute(
        &mut self,
        creator_info: MessageInfo,
        msg: ExecuteMsg,
    ) -> Result<Response, ContractError> {
        entry::execute(self.deps.as_mut(), mock_env(), creator_info, msg)
    }

    pub fn query<T: DeserializeOwned>(&self, msg: Cw721SellableQueryMsg) -> StdResult<T> {
        let binary_res = entry::query(self.deps.as_ref(), mock_env(), msg);
        binary_res.and_then(|bin| from_binary(&bin))
    }
}

impl Default for Context<'_> {
    fn default() -> Self {
        Context::new(
            ContractInfo {
                name: "SpaceShips".into(),
                symbol: "SPACE".into(),
            },
            "creator",
            None,
        )
    }
}
