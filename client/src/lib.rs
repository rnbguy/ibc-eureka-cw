pub mod implementation;
pub mod interface;

use cosmwasm_std::StdError;
use cw_storey::containers::{Item, Map};
use cw_storey::CwStorage;
use sylvia::contract;
use sylvia::cw_std::{Response, StdResult};
use sylvia::types::InstantiateCtx;

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
