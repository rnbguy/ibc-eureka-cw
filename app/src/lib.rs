pub mod interface;

use cosmwasm_std::Addr;
use cw_storey::containers::Item;
use cw_storey::CwStorage;
use sylvia::contract;
use sylvia::cw_std::{Response, StdError, StdResult};
use sylvia::types::{ExecCtx, InstantiateCtx, QueryCtx};

use crate::interface::Application;

pub struct Contract {
    pub authority: Item<Addr>,
    pub sent: Item<String>,
    pub received: Item<String>,
}

#[cfg_attr(not(feature = "library"), sylvia::entry_points)]
#[contract]
#[sv::error(StdError)]
#[sv::messages(crate::interface)]
impl Contract {
    pub const fn new() -> Self {
        Self {
            authority: Item::new(b'A'),
            sent: Item::new(b'B'),
            received: Item::new(b'B'),
        }
    }

    #[sv::msg(instantiate)]
    fn instantiate(&self, ctx: InstantiateCtx) -> StdResult<Response> {
        let mut storage = CwStorage(ctx.deps.storage);

        self.authority.access(&mut storage).set(&ctx.info.sender)?;
        self.sent.access(&mut storage).set(&"null".to_string())?;
        self.received
            .access(&mut storage)
            .set(&"null".to_string())?;

        Ok(Response::default())
    }

    #[sv::msg(query)]
    fn authority(&self, ctx: QueryCtx) -> StdResult<Addr> {
        let mut storage = CwStorage(ctx.deps.storage);
        Ok(self.authority.access(&mut storage).get()?.unwrap())
    }

    #[sv::msg(query)]
    fn sent_value(&self, ctx: QueryCtx) -> StdResult<String> {
        let mut storage = CwStorage(ctx.deps.storage);
        Ok(self.sent.access(&mut storage).get()?.unwrap_or_default())
    }

    #[sv::msg(query)]
    fn received_value(&self, ctx: QueryCtx) -> StdResult<String> {
        let mut storage = CwStorage(ctx.deps.storage);
        Ok(self
            .received
            .access(&mut storage)
            .get()?
            .unwrap_or_default())
    }
}

impl Application for Contract {
    type Error = StdError;

    fn send(
        &self,
        ctx: ExecCtx,
        destination: Addr,
        packet: Vec<u8>,
    ) -> Result<Response, Self::Error> {
        let mut storage = CwStorage(ctx.deps.storage);

        if Some(&ctx.info.sender) != self.authority.access(&mut storage).get()?.as_ref() {
            return Err(StdError::generic_err("unauthorized"));
        }
        self.sent.access(&mut storage).set(&format!(
            "{}(via {}) receives {}",
            destination,
            ctx.info.sender,
            String::from_utf8_lossy(&packet),
        ))?;
        Ok(Response::default())
    }

    fn receive(
        &self,
        ctx: ExecCtx,
        source: Addr,
        packet: Vec<u8>,
    ) -> Result<Response, Self::Error> {
        let mut storage = CwStorage(ctx.deps.storage);

        if Some(&ctx.info.sender) != self.authority.access(&mut storage).get()?.as_ref() {
            return Err(StdError::generic_err("unauthorized"));
        }
        self.received.access(&mut storage).set(&format!(
            "{}(via {}) sent {}",
            source,
            ctx.info.sender,
            String::from_utf8_lossy(&packet),
        ))?;
        Ok(Response::default())
    }
}
