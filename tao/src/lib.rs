use cosmwasm_schema::cw_serde;
use cosmwasm_std::Addr;
use cw_storey::containers::{Item, Map};
use cw_storey::CwStorage;
use eureka_app::interface::sv::Executor;
use eureka_app::interface::Application;
use eureka_client::interface::sv::Querier;
use eureka_client::interface::LightClient;
use sylvia::contract;
use sylvia::cw_std::{Response, StdError, StdResult};
use sylvia::types::{ExecCtx, InstantiateCtx, Remote};

#[cw_serde]
pub struct PacketHeader {
    pub client_source: (Addr, Vec<u8>),
    pub client_destination: (Addr, Vec<u8>),
    pub nonce: u64,
    pub timeout: u64,
}

#[cw_serde]
pub struct Packet {
    pub header: PacketHeader,
    pub payloads: Vec<Payload>,
}

#[cw_serde]
pub struct PayloadHeader {
    pub application_source: Addr,
    pub application_destination: Addr,
}

#[cw_serde]
pub struct Payload {
    pub header: PayloadHeader,
    pub data: Vec<u8>,
}

pub struct Contract {
    pub sent_nonce: Map<String, Item<u64>>,
    pub sent_packet: Map<String, Map<u64, Item<Packet>>>,
    pub received_nonce: Map<String, Item<u64>>,
    pub received_packet: Map<String, Map<u64, Item<Packet>>>,
}

#[cfg_attr(not(feature = "library"), sylvia::entry_points)]
#[contract]
impl Contract {
    pub const fn new() -> Self {
        Self {
            sent_nonce: Map::new(b'A'),
            sent_packet: Map::new(b'B'),
            received_nonce: Map::new(b'C'),
            received_packet: Map::new(b'D'),
        }
    }

    #[sv::msg(instantiate)]
    fn instantiate(&self, _ctx: InstantiateCtx) -> StdResult<Response> {
        Ok(Response::default())
    }

    #[sv::msg(exec)]
    fn send_packet(&self, ctx: ExecCtx, packet: Packet) -> StdResult<Response> {
        let Packet {
            header:
                PacketHeader {
                    client_source,
                    client_destination,
                    nonce,
                    timeout,
                },
            payloads,
        } = &packet;

        if timeout < &ctx.env.block.time.seconds() {
            return Err(StdError::generic_err(format!(
                "timeout is in the past: current time: {}, timeout: {}",
                ctx.env.block.time.seconds(),
                timeout
            )));
        }

        let mut storage = CwStorage(ctx.deps.storage);

        let connection_str = format!("{:?}-{:?}", client_source, client_destination);

        let stored_nonce = self
            .sent_nonce
            .access(&mut storage)
            .entry(&connection_str)
            .get()?
            .unwrap_or_default()
            + 1;

        if nonce != &0 {
            assert_eq!(nonce, &stored_nonce, "nonce mismatch");
        }

        let mut msgs = vec![];

        for payload in payloads {
            let PayloadHeader {
                application_source,
                application_destination,
            } = &payload.header;

            let msg =
                Remote::<'_, dyn Application<Error = StdError>>::new(application_source.clone())
                    .executor()
                    .send(
                        ctx.info.sender.clone(),
                        client_source.clone(),
                        client_destination.clone(),
                        application_destination.clone(),
                        payload.data.clone(),
                    )?
                    .build();

            msgs.push(msg);
        }

        self.sent_nonce
            .access(&mut storage)
            .entry_mut(&connection_str)
            .set(&stored_nonce)?;

        self.sent_packet
            .access(&mut storage)
            .entry_mut(&connection_str)
            .entry_mut(&stored_nonce)
            .set(&packet)?;

        Ok(Response::new().add_messages(msgs))
    }

    #[sv::msg(exec)]
    fn receive_packet(
        &self,
        ctx: ExecCtx,
        packet: Packet,
        height: u64,
        proof: Vec<u8>,
    ) -> StdResult<Response> {
        let Packet {
            header:
                PacketHeader {
                    client_source,
                    client_destination,
                    nonce,
                    timeout,
                },
            payloads,
        } = &packet;

        if timeout < &ctx.env.block.time.seconds() {
            return Err(StdError::generic_err(format!(
                "timeout is in the past: current time: {}, timeout: {}",
                ctx.env.block.time.seconds(),
                timeout
            )));
        }

        let mut storage = CwStorage(ctx.deps.storage);

        let connection_str = format!("{:?}-{:?}", client_source, client_destination);
        let stored_nonce = self
            .received_nonce
            .access(&mut storage)
            .entry(&connection_str)
            .get()?
            .unwrap_or_default()
            + 1;

        if nonce != &0 {
            assert_eq!(nonce, &stored_nonce, "nonce mismatch");
        }

        // validate commitment proof
        Remote::<'_, dyn LightClient<Error = StdError>>::new(client_source.0.clone())
            .querier(&ctx.deps.querier)
            .check_membership(vec![], vec![], client_source.1.clone(), height, proof)?;

        let mut msgs = vec![];

        for payload in payloads {
            let PayloadHeader {
                application_source,
                application_destination,
            } = &payload.header;

            let msg = Remote::<'_, dyn Application<Error = StdError>>::new(
                application_destination.clone(),
            )
            .executor()
            .receive(
                client_destination.clone(),
                client_source.clone(),
                application_source.clone(),
                payload.data.clone(),
            )?
            .build();

            msgs.push(msg);
        }

        self.received_nonce
            .access(&mut storage)
            .entry_mut(&connection_str)
            .set(&stored_nonce)?;

        self.received_packet
            .access(&mut storage)
            .entry_mut(&connection_str)
            .entry_mut(&stored_nonce)
            .set(&packet)?;

        Ok(Response::new().add_messages(msgs))
    }
}
