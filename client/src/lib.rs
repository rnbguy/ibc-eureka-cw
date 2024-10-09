use cw_storage_plus::{Item, Map};
use sylvia::contract;
use sylvia::cw_std::{Response, StdResult};
use sylvia::types::{ExecCtx, InstantiateCtx, QueryCtx};

pub struct Contract {
    pub client_state: Item<Vec<u8>>,
    pub consensus_states: Map<u64, Vec<u8>>,
}

#[cfg_attr(not(feature = "library"), sylvia::entry_points)]
#[contract]
impl Contract {
    pub const fn new() -> Self {
        Self {
            client_state: Item::new("CLIENT_STATE"),
            consensus_states: Map::new("CONSENSUS_STATES"),
        }
    }

    #[sv::msg(instantiate)]
    fn instantiate(
        &self,
        ctx: InstantiateCtx,
        client_state: Vec<u8>,
        consensus_state: Vec<u8>,
    ) -> StdResult<Response> {
        self.client_state.save(ctx.deps.storage, &client_state)?;
        self.consensus_states
            .save(ctx.deps.storage, 0, &consensus_state)?;
        Ok(Response::default())
    }

    #[sv::msg(exec)]
    fn update(&self, _ctx: ExecCtx, _header: Vec<u8>) -> StdResult<Response> {
        // TODO: update logic
        Ok(Response::default())
    }

    #[sv::msg(query)]
    fn check_membership(
        &self,
        _ctx: QueryCtx,
        _height: u64,
        _key: Vec<u8>,
        _value: Vec<u8>,
    ) -> StdResult<bool> {
        Ok(true)
    }

    #[sv::msg(query)]
    fn check_non_membership(&self, _ctx: QueryCtx, _height: u64, _key: Vec<u8>) -> StdResult<bool> {
        Ok(true)
    }
}
