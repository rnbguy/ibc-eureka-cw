use cosmwasm_std::StdError;
use cw_storey::containers::{Item, Map};
use cw_storey::CwStorage;
use eureka_lightclient_interface::{LightClient, Status};
use sylvia::contract;
use sylvia::cw_std::{Response, StdResult};
use sylvia::types::{ExecCtx, InstantiateCtx, QueryCtx};

pub struct Contract {
    pub lightclient_state: Item<Vec<u8>>,
    pub consensus_states: Map<u64, Item<Vec<u8>>>,
}

#[cfg_attr(not(feature = "library"), sylvia::entry_points)]
#[contract]
#[sv::error(StdError)]
#[sv::messages(eureka_lightclient_interface)]
impl Contract {
    pub const fn new() -> Self {
        Self {
            lightclient_state: Item::new(b'C'),
            consensus_states: Map::new(b'S'),
        }
    }

    #[sv::msg(instantiate)]
    fn instantiate(
        &self,
        ctx: InstantiateCtx,
        lightclient_state: Vec<u8>,
        consensus_state: Vec<u8>,
    ) -> StdResult<Response> {
        let mut storage = CwStorage(ctx.deps.storage);
        self.lightclient_state
            .access(&mut storage)
            .set(&lightclient_state)?;
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

    fn status(&self, _ctx: QueryCtx) -> Result<Status, Self::Error> {
        Ok(Status::Active)
    }

    fn timestamp(&self, _ctx: QueryCtx, _height: u64) -> Result<u64, Self::Error> {
        Ok(u64::MAX)
    }

    fn prune(&self, _ctx: ExecCtx) -> Result<Response, Self::Error> {
        Ok(Response::default())
    }

    fn check_membership(
        &self,
        _ctx: QueryCtx,
        _key: Vec<u8>,
        _value: Vec<u8>,
        _commitment_prefix: Vec<u8>,
        _height: u64,
        _proof: Vec<u8>,
    ) -> Result<bool, Self::Error> {
        Ok(true)
    }

    fn check_non_membership(
        &self,
        _ctx: QueryCtx,
        _key: Vec<u8>,
        _commitment_prefix: Vec<u8>,
        _height: u64,
        _proof: Vec<u8>,
    ) -> Result<bool, Self::Error> {
        Ok(true)
    }
}
