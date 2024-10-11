use cosmwasm_schema::cw_serde;
use cosmwasm_std::{Addr, Reply};
use cw_storey::containers::{Item, Map};
use cw_storey::CwStorage;
use eureka_app::interface::sv::Executor;
use eureka_app::interface::EurekaApplication;
use eureka_app::Contract as EurekaApp;
use eureka_client::interface::sv::Querier;
use eureka_client::Contract as EurekaClient;
use sylvia::contract;
use sylvia::cw_std::{Response, StdError, StdResult};
use sylvia::types::{ExecCtx, InstantiateCtx, QueryCtx, Remote, ReplyCtx};

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

pub struct Contract {
    pub nonce: Map<String, Item<u64>>,
    pub commitment: Map<String, Map<u64, Item<Packet>>>,
}

#[cfg_attr(not(feature = "library"), sylvia::entry_points)]
#[contract]
impl Contract {
    pub const fn new() -> Self {
        Self {
            nonce: Map::new(b'N'),
            commitment: Map::new(b'C'),
        }
    }

    #[sv::msg(instantiate)]
    fn instantiate(&self, _ctx: InstantiateCtx) -> StdResult<Response> {
        Ok(Response::default())
    }

    #[sv::msg(exec)]
    fn send_packet(&self, ctx: ExecCtx, packet: Packet) -> StdResult<Response> {
        if packet.header.timeout < ctx.env.block.time.seconds() {
            return Err(StdError::generic_err(format!(
                "timeout is in the past: current time: {}, timeout: {}",
                ctx.env.block.time.seconds(),
                packet.header.timeout
            )));
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

        let mut storage = CwStorage(ctx.deps.storage);
        let mut msgs = vec![];

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

            let msg =
                Remote::<'_, dyn EurekaApplication<Error = StdError>>::new(application_source)
                    .executor()
                    .send(payload.data)?
                    .build();

            msgs.push(msg);

            let channel_str = format!("{:?}", channel);

            let nonce = self
                .nonce
                .access(&mut storage)
                .entry(&channel_str)
                .get()?
                .unwrap_or_default()
                + 1;

            self.nonce
                .access(&mut storage)
                .entry_mut(&channel_str)
                .set(&nonce)?;
            self.commitment
                .access(&mut storage)
                .entry_mut(&channel_str)
                .entry_mut(&nonce)
                .set(&packet)?;
        }

        Ok(Response::new().add_messages(msgs))
    }

    #[sv::msg(exec)]
    fn receive_packet(&self, ctx: ExecCtx, packet: Packet) -> StdResult<Response> {
        if packet.header.timeout < ctx.env.block.time.seconds() {
            return Err(StdError::generic_err(format!(
                "timeout is in the past: current time: {}, timeout: {}",
                ctx.env.block.time.seconds(),
                packet.header.timeout
            )));
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

        let mut storage = CwStorage(ctx.deps.storage);
        let mut msgs = vec![];

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

            Remote::<'_, EurekaClient>::new(client_source.clone())
                .querier(&ctx.deps.querier)
                .check_membership(0, vec![], vec![])?;

            let msg = Remote::<'_, EurekaApp>::new(application_source)
                .executor()
                .receive(payload.data)?
                .build();

            // let sub_msg = SubMsg::reply_on_success(msg, SUBMSG_ID);
            msgs.push(msg);

            let channel_str = format!("{:?}", channel);

            let nonce = self
                .nonce
                .access(&mut storage)
                .entry(&channel_str)
                .get()?
                .unwrap_or_default()
                + 1;

            self.nonce
                .access(&mut storage)
                .entry_mut(&channel_str)
                .set(&nonce)?;
            self.commitment
                .access(&mut storage)
                .entry_mut(&channel_str)
                .entry_mut(&nonce)
                .set(&packet)?;
        }

        Ok(Response::new().add_messages(msgs))
    }

    #[sv::msg(query)]
    fn query(&self, _ctx: QueryCtx) -> StdResult<Vec<u8>> {
        Ok(vec![])
    }

    #[sv::msg(reply)]
    fn reply(&self, _ctx: ReplyCtx, _reply: Reply) -> StdResult<Response> {
        // handle reply
        Ok(Response::default())
    }
}
