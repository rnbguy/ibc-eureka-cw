use cosmwasm_std::{Addr, Response, StdError};
use sylvia::interface;
use sylvia::types::ExecCtx;

#[interface]
pub trait Application {
    type Error: From<StdError>;

    #[sv::msg(exec)]
    fn send(
        &self,
        ctx: ExecCtx,
        destination: Addr,
        packet: Vec<u8>,
    ) -> Result<Response, Self::Error>;

    #[sv::msg(exec)]
    fn receive(&self, ctx: ExecCtx, source: Addr, packet: Vec<u8>)
        -> Result<Response, Self::Error>;
}
