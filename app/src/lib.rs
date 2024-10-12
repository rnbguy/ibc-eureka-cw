pub mod interface;

use cosmwasm_std::Addr;
use cw_storey::containers::Item;
use cw_storey::CwStorage;
use sylvia::contract;
use sylvia::cw_std::{Response, StdError, StdResult};
use sylvia::types::ExecCtx;
use sylvia::types::{InstantiateCtx, QueryCtx};

use crate::interface::EurekaApplication;

pub struct Contract {
    pub authority: Item<Addr>,
    pub value: Item<String>,
}

#[cfg_attr(not(feature = "library"), sylvia::entry_points)]
#[contract]
#[sv::error(StdError)]
#[sv::messages(crate::interface)]
impl Contract {
    pub const fn new() -> Self {
        Self {
            authority: Item::new(b'A'),
            value: Item::new(b'V'),
        }
    }

    #[sv::msg(instantiate)]
    fn instantiate(&self, ctx: InstantiateCtx, authority: Addr) -> StdResult<Response> {
        let mut storage = CwStorage(ctx.deps.storage);

        self.authority.access(&mut storage).set(&authority)?;
        self.value
            .access(&mut storage)
            .set(&"hello world".to_string())?;
        Ok(Response::default())
    }

    #[sv::msg(query)]
    fn query(&self, ctx: QueryCtx) -> StdResult<String> {
        let mut storage = CwStorage(ctx.deps.storage);
        Ok(self.value.access(&mut storage).get()?.unwrap_or_default())
    }
}

impl EurekaApplication for Contract {
    type Error = StdError;

    fn send(&self, _ctx: ExecCtx, _packet: Vec<u8>) -> Result<Response, Self::Error> {
        Ok(Response::default())
    }

    fn receive(&self, ctx: ExecCtx, packet: Vec<u8>) -> Result<Response, Self::Error> {
        let mut storage = CwStorage(ctx.deps.storage);

        if Some(ctx.info.sender) != self.authority.access(&mut storage).get()? {
            return Err(StdError::generic_err("unauthorized"));
        }
        self.value
            .access(&mut storage)
            .set(&String::from_utf8_lossy(&packet).to_string())?;
        Ok(Response::default())
    }
}
