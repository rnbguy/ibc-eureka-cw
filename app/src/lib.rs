use cosmwasm_std::Addr;
use cw_storage_plus::Item;
use sylvia::contract;
use sylvia::cw_std::{Response, StdError, StdResult};
use sylvia::types::{ExecCtx, InstantiateCtx};

pub struct Contract {
    pub authority: Item<Addr>,
    pub value: Item<String>,
}

#[cfg_attr(not(feature = "library"), sylvia::entry_points)]
#[contract]
impl Contract {
    pub const fn new() -> Self {
        Self {
            authority: Item::new("AUTHORITY"),
            value: Item::new("VALUE"),
        }
    }

    #[sv::msg(instantiate)]
    fn instantiate(&self, ctx: InstantiateCtx, authority: Addr) -> StdResult<Response> {
        self.authority.save(ctx.deps.storage, &authority)?;
        self.value
            .save(ctx.deps.storage, &"hello world".to_string())?;
        Ok(Response::default())
    }

    #[sv::msg(exec)]
    fn send(&self, _ctx: ExecCtx, _packet: Vec<u8>) -> StdResult<Response> {
        Ok(Response::default())
    }

    #[sv::msg(exec)]
    fn receive(&self, ctx: ExecCtx, packet: Vec<u8>) -> StdResult<Response> {
        if ctx.info.sender != self.authority.load(ctx.deps.storage)? {
            return Err(StdError::generic_err("unauthorized"));
        }
        self.value.save(
            ctx.deps.storage,
            &String::from_utf8_lossy(&packet).to_string(),
        )?;
        Ok(Response::default())
    }
}
