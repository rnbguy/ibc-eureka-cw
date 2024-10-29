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
        lightclient_local: (Addr, Vec<u8>),
        lightclient_remote: (Addr, Vec<u8>),
        application_remote: Addr,
        packet: Vec<u8>,
        packet_sender: Addr,
    ) -> Result<Response, Self::Error>;

    #[allow(clippy::too_many_arguments)]
    #[sv::msg(exec)]
    fn receive(
        &self,
        ctx: ExecCtx,
        lightclient_local: (Addr, Vec<u8>),
        lightclient_remote: (Addr, Vec<u8>),
        application_remote: Addr,
        packet: Vec<u8>,
        relayer: Addr,
        sent_funds: Vec<Coin>,
    ) -> Result<Response, Self::Error>;
}
