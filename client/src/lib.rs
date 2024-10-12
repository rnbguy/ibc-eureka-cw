pub mod interface;

use cosmwasm_std::StdError;
use cw_storey::containers::{Item, Map};
use cw_storey::CwStorage;
use sylvia::contract;
use sylvia::cw_std::{Response, StdResult};
use sylvia::types::{ExecCtx, InstantiateCtx, QueryCtx};

use crate::interface::LightClient;

pub struct Contract {
    pub client_state: Item<Vec<u8>>,
    pub consensus_states: Map<u64, Item<Vec<u8>>>,
}

#[cfg_attr(not(feature = "library"), sylvia::entry_points)]
#[contract]
#[sv::error(StdError)]
#[sv::messages(crate::interface)]
impl Contract {
    pub const fn new() -> Self {
        Self {
            client_state: Item::new(b'C'),
            consensus_states: Map::new(b'S'),
        }
    }

    #[sv::msg(instantiate)]
    fn instantiate(
        &self,
        ctx: InstantiateCtx,
        client_state: Vec<u8>,
        consensus_state: Vec<u8>,
    ) -> StdResult<Response> {
        let mut storage = CwStorage(ctx.deps.storage);
        self.client_state.access(&mut storage).set(&client_state)?;
        self.consensus_states
            .access(&mut storage)
            .entry_mut(&0)
            .set(&consensus_state)?;
        Ok(Response::default())
    }
}

impl LightClient for Contract {
    type Error = StdError;

    fn update(&self, _ctx: ExecCtx, _header: Vec<u8>) -> Result<Response, Self::Error> {
        Ok(Response::default())
    }

    fn check_membership(
        &self,
        _ctx: QueryCtx,
        _key: Vec<u8>,
        _value: Vec<u8>,
        _height: u64,
        _proof: Vec<u8>,
    ) -> Result<bool, Self::Error> {
        Ok(true)
    }

    fn check_non_membership(
        &self,
        _ctx: QueryCtx,
        _key: Vec<u8>,
        _height: u64,
        _proof: Vec<u8>,
    ) -> Result<bool, Self::Error> {
        Ok(true)
    }
}
