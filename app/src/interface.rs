use cosmwasm_std::{Response, StdError};
use sylvia::interface;
use sylvia::types::ExecCtx;

#[interface]
pub trait EurekaApplication {
    type Error: From<StdError>;

    #[sv::msg(exec)]
    fn send(&self, _ctx: ExecCtx, _packet: Vec<u8>) -> Result<Response, Self::Error>;

    #[sv::msg(exec)]
    fn receive(&self, ctx: ExecCtx, packet: Vec<u8>) -> Result<Response, Self::Error>;
}
