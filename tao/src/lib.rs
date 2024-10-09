use cosmwasm_schema::cw_serde;
use cosmwasm_std::Addr;
use cw_storage_plus::Map;
use eureka_app::sv::Executor;
use eureka_app::Contract as EurekaApp;
use eureka_client::sv::Querier;
use eureka_client::Contract as EurekaClient;
use sylvia::contract;
use sylvia::cw_std::{Response, StdError, StdResult};
use sylvia::types::{ExecCtx, InstantiateCtx, QueryCtx, Remote};

#[cw_serde]
pub struct ApplicationInstance {
    pub client: Addr,
    pub application: Addr,
}

#[cw_serde]
pub struct Channel {
    pub source: ApplicationInstance,
    pub destination: ApplicationInstance,
    pub commitment_prefix: Vec<u8>,
}

#[cw_serde]
pub struct PacketHeader {
    pub client_source: Addr,
    pub client_destination: Addr,
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

pub struct TaoContract {
    pub nonce: Map<String, u64>,
    pub commitment: Map<(String, u64), Packet>,
}

#[cfg_attr(not(feature = "library"), sylvia::entry_points)]
#[contract]
impl TaoContract {
    pub const fn new() -> Self {
        Self {
            nonce: Map::new("NONCE"),
            commitment: Map::new("COMMITMENT"),
        }
    }

    #[sv::msg(instantiate)]
    fn instantiate(&self, _ctx: InstantiateCtx) -> StdResult<Response> {
        Ok(Response::default())
    }

    #[sv::msg(exec)]
    fn send_packet(&self, ctx: ExecCtx, packet: Packet) -> StdResult<Response> {
        if packet.header.timeout < ctx.env.block.time.seconds() {
            return Err(StdError::generic_err("timeout"));
        }

        let Packet {
            header:
                PacketHeader {
                    client_source,
                    client_destination,
                    ..
                },
            payloads,
        } = packet.clone();

        for payload in payloads {
            let PayloadHeader {
                application_source,
                application_destination,
            } = payload.header;

            let source = ApplicationInstance {
                client: client_source.clone(),
                application: application_source.clone(),
            };
            let destination = ApplicationInstance {
                client: client_destination.clone(),
                application: application_destination,
            };

            let channel = Channel {
                source,
                destination,
                commitment_prefix: vec![],
            };

            // inter contract call
            Remote::new(application_source)
                .executor()
                .send(payload.data)?;

            let channel_str = format!("{:?}", channel);

            let nonce = self
                .nonce
                .may_load(ctx.deps.storage, channel_str.clone())?
                .unwrap_or_default()
                + 1;

            self.nonce
                .save(ctx.deps.storage, channel_str.clone(), &nonce)?;
            self.commitment
                .save(ctx.deps.storage, (channel_str.clone(), nonce), &packet)?;
        }

        Ok(Response::default())
    }

    #[sv::msg(exec)]
    fn receive_packet(&self, ctx: ExecCtx, packet: Packet) -> StdResult<Response> {
        if packet.header.timeout < ctx.env.block.time.seconds() {
            return Err(StdError::generic_err("timeout"));
        }

        let Packet {
            header:
                PacketHeader {
                    client_source,
                    client_destination,
                    ..
                },
            payloads,
        } = packet.clone();

        for payload in payloads {
            let PayloadHeader {
                application_source,
                application_destination,
            } = payload.header;

            let source = ApplicationInstance {
                client: client_source.clone(),
                application: application_source.clone(),
            };
            let destination = ApplicationInstance {
                client: client_destination.clone(),
                application: application_destination,
            };

            let channel = Channel {
                source,
                destination,
                commitment_prefix: vec![],
            };

            // validate commitment proof

            Remote::<'_, EurekaClient>::new(client_source.clone())
                .querier(&ctx.deps.querier)
                .check_membership(0, vec![], vec![])?;

            // inter contract call
            Remote::<'_, EurekaApp>::new(application_source)
                .executor()
                .receive(payload.data)?;

            let channel_str = format!("{:?}", channel);

            let nonce = self
                .nonce
                .may_load(ctx.deps.storage, channel_str.clone())?
                .unwrap_or_default()
                + 1;

            self.nonce
                .save(ctx.deps.storage, channel_str.clone(), &nonce)?;
            self.commitment
                .save(ctx.deps.storage, (channel_str.clone(), nonce), &packet)?;
        }

        Ok(Response::default())
    }

    #[sv::msg(query)]
    fn query(&self, _ctx: QueryCtx) -> StdResult<Vec<u8>> {
        Ok(vec![])
    }
}
