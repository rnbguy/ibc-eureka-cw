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
        sender_local: Addr,
        client_local: (Addr, Vec<u8>),
        client_remote: (Addr, Vec<u8>),
        application_remote: Addr,
        packet: Vec<u8>,
    ) -> Result<Response, Self::Error>;

    #[sv::msg(exec)]
    fn receive(
        &self,
        ctx: ExecCtx,
        client_local: (Addr, Vec<u8>),
        client_remote: (Addr, Vec<u8>),
        application_remote: Addr,
        packet: Vec<u8>,
    ) -> Result<Response, Self::Error>;
}
