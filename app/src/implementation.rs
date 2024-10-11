use cosmwasm_std::{Response, StdError};
use cw_storey::CwStorage;
use sylvia::types::ExecCtx;

use crate::interface::EurekaApplication;
use crate::Contract;

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
