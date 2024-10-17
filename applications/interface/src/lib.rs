use cosmwasm_std::{Addr, Coin, Response, StdError};
use sylvia::interface;
use sylvia::types::ExecCtx;

#[interface]
pub trait Application {
    type Error: From<StdError>;

    #[sv::msg(exec)]
    fn send(
        &self,
        ctx: ExecCtx,
        packet_sender: Addr,
        lightclient_local: (Addr, Vec<u8>),
        lightclient_remote: (Addr, Vec<u8>),
        application_remote: Addr,
        packet: Vec<u8>,
    ) -> Result<Response, Self::Error>;

    #[sv::msg(exec)]
    fn receive(
        &self,
        ctx: ExecCtx,
        sent_funds: Vec<Coin>,
        lightclient_local: (Addr, Vec<u8>),
        lightclient_remote: (Addr, Vec<u8>),
        application_remote: Addr,
        packet: Vec<u8>,
    ) -> Result<Response, Self::Error>;
}
