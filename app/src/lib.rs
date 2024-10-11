pub mod implementation;
pub mod interface;

use cosmwasm_std::Addr;
use cw_storey::containers::Item;
use cw_storey::CwStorage;
use sylvia::contract;
use sylvia::cw_std::{Response, StdError, StdResult};
use sylvia::types::{InstantiateCtx, QueryCtx};

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
