pub mod interface;

use cosmwasm_schema::cw_serde;
use cosmwasm_std::{Addr, Coin};
use cw_storey::containers::Item;
use cw_storey::CwStorage;
use sylvia::contract;
use sylvia::cw_std::{Response, StdError, StdResult};
use sylvia::types::{ExecCtx, InstantiateCtx, QueryCtx};

use crate::interface::Application;

#[cw_serde]
pub struct Channel {
    pub client_local: (Addr, Vec<u8>),
    pub client_remote: (Addr, Vec<u8>),
    pub application_remote: Addr,
}

pub struct Contract {
    // owner of the contract
    pub owner: Item<Addr>,

    // only tao can call send and receive
    pub tao_contract: Item<Addr>,

    // allowed channel
    pub allowed_channel: Item<Channel>,

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
            owner: Item::new(b'A'),
            tao_contract: Item::new(b'B'),
            allowed_channel: Item::new(b'C'),
            sent: Item::new(b'D'),
            received: Item::new(b'E'),
        }
    }

    #[sv::msg(instantiate)]
    fn instantiate(&self, ctx: InstantiateCtx, tao_addr: Addr) -> StdResult<Response> {
        let mut storage = CwStorage(ctx.deps.storage);

        self.owner.access(&mut storage).set(&ctx.info.sender)?;
        self.tao_contract.access(&mut storage).set(&tao_addr)?;
        self.sent.access(&mut storage).set(&"null".to_string())?;
        self.received
            .access(&mut storage)
            .set(&"null".to_string())?;

        Ok(Response::default())
    }

    #[sv::msg(query)]
    fn get_tao_contract(&self, ctx: QueryCtx) -> StdResult<Addr> {
        let mut storage = CwStorage(ctx.deps.storage);
        Ok(self.tao_contract.access(&mut storage).get()?.unwrap())
    }

    #[sv::msg(exec)]
    fn set_tao_contract(&self, ctx: ExecCtx, tao_addr: Addr) -> Result<Response, StdError> {
        let mut storage = CwStorage(ctx.deps.storage);

        if Some(&ctx.info.sender) != self.owner.access(&mut storage).get()?.as_ref() {
            return Err(StdError::generic_err("unauthorized"));
        }
        self.tao_contract.access(&mut storage).set(&tao_addr)?;
        Ok(Response::default())
    }

    #[sv::msg(query)]
    fn get_allowed_channel(&self, ctx: QueryCtx) -> StdResult<String> {
        let mut storage = CwStorage(ctx.deps.storage);
        Ok(format!(
            "{:?}",
            self.allowed_channel.access(&mut storage).get()?.unwrap()
        ))
    }

    #[sv::msg(exec)]
    fn set_allowed_channel(
        &self,
        ctx: ExecCtx,
        client_local: (Addr, Vec<u8>),
        client_remote: (Addr, Vec<u8>),
        application_remote: Addr,
    ) -> Result<Response, StdError> {
        let mut storage = CwStorage(ctx.deps.storage);

        if Some(&ctx.info.sender) != self.owner.access(&mut storage).get()?.as_ref() {
            return Err(StdError::generic_err("unauthorized"));
        }
        self.allowed_channel.access(&mut storage).set(&Channel {
            client_local,
            client_remote,
            application_remote,
        })?;
        Ok(Response::default())
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
        sender_local: Addr,
        client_local: (Addr, Vec<u8>),
        client_remote: (Addr, Vec<u8>),
        application_remote: Addr,
        packet: Vec<u8>,
    ) -> Result<Response, Self::Error> {
        let mut storage = CwStorage(ctx.deps.storage);

        if Some(&ctx.info.sender) != self.tao_contract.access(&mut storage).get()?.as_ref() {
            return Err(StdError::generic_err("send can only be called by tao"));
        }

        if Some(&Channel {
            client_local,
            client_remote,
            application_remote: application_remote.clone(),
        }) != self.allowed_channel.access(&mut storage).get()?.as_ref()
        {
            // ICS20 like check
            return Err(StdError::generic_err("not allowed channel"));
        }

        if Some(&sender_local) != self.owner.access(&mut storage).get()?.as_ref() {
            // ICA like check
            return Err(StdError::generic_err("only owner can submit packet"));
        }

        self.sent.access(&mut storage).set(&format!(
            "{}(via {}) receives {}",
            application_remote,
            ctx.info.sender,
            String::from_utf8_lossy(&packet),
        ))?;
        Ok(Response::default())
    }

    fn receive(
        &self,
        ctx: ExecCtx,
        _sent_funds: Vec<Coin>,
        client_local: (Addr, Vec<u8>),
        client_remote: (Addr, Vec<u8>),
        application_remote: Addr,
        packet: Vec<u8>,
    ) -> Result<Response, Self::Error> {
        // ignoring sender_remote (like, sender_local in send), as remote tao contract is trusted

        let mut storage = CwStorage(ctx.deps.storage);

        if Some(&ctx.info.sender) != self.tao_contract.access(&mut storage).get()?.as_ref() {
            return Err(StdError::generic_err("receive can only be called by tao"));
        }

        if Some(&Channel {
            client_local,
            client_remote,
            application_remote: application_remote.clone(),
        }) != self.allowed_channel.access(&mut storage).get()?.as_ref()
        {
            return Err(StdError::generic_err("not allowed channel"));
        }

        self.received.access(&mut storage).set(&format!(
            "{}(via {}) sent {}",
            application_remote,
            ctx.info.sender,
            String::from_utf8_lossy(&packet),
        ))?;
        Ok(Response::default())
    }
}
