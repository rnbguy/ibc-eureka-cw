use cosmwasm_schema::cw_serde;
use cosmwasm_std::{Addr, Binary, Reply, WasmMsg};
use cw_storey::containers::{Item, Map};
use cw_storey::CwStorage;
use eureka_app::interface::sv::Executor;
use eureka_app::interface::Application;
use eureka_client::interface::sv::Querier;
use eureka_client::interface::LightClient;
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
    pub nonce: u64,
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
    fn deploy(&self, ctx: ExecCtx, code_id: u64, msg: Binary) -> StdResult<Response> {
        let instantiate_msg = WasmMsg::Instantiate {
            admin: Some(ctx.env.contract.address.to_string()),
            code_id,
            msg,
            // funds: ctx.info.funds.clone(),
            funds: vec![],
            label: "new IBC Eureka app".into(),
        };

        Ok(Response::default().add_message(instantiate_msg))
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
                nonce,
            } = payload.header;

            let msg =
                Remote::<'_, dyn Application<Error = StdError>>::new(application_source.clone())
                    .executor()
                    .send(application_destination.clone(), payload.data)?
                    .build();

            msgs.push(msg);

            let source = ApplicationInstance {
                client: client_source.clone(),
                application: application_source,
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

            let channel_str = format!("{:?}", channel);

            let stored_nonce = self
                .sent_nonce
                .access(&mut storage)
                .entry(&channel_str)
                .get()?
                .unwrap_or_default()
                + 1;

            if nonce != 0 {
                assert_eq!(nonce, stored_nonce, "nonce mismatch");
            }

            self.sent_nonce
                .access(&mut storage)
                .entry_mut(&channel_str)
                .set(&stored_nonce)?;

            self.sent_packet
                .access(&mut storage)
                .entry_mut(&channel_str)
                .entry_mut(&stored_nonce)
                .set(&packet)?;
        }

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

        // validate commitment proof
        Remote::<'_, dyn LightClient<Error = StdError>>::new(client_source.clone())
            .querier(&ctx.deps.querier)
            .check_membership(vec![], vec![], height, proof)?;

        let mut storage = CwStorage(ctx.deps.storage);
        let mut msgs = vec![];

        for payload in payloads {
            let PayloadHeader {
                application_source,
                application_destination,
                nonce,
            } = payload.header;

            let msg = Remote::<'_, dyn Application<Error = StdError>>::new(
                application_destination.clone(),
            )
            .executor()
            .receive(application_source.clone(), payload.data)?
            .build();

            // let sub_msg = SubMsg::reply_on_success(msg, SUBMSG_ID);
            msgs.push(msg);

            let source = ApplicationInstance {
                client: client_source.clone(),
                application: application_source,
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

            let channel_str = format!("{:?}", channel);

            let stored_nonce = self
                .received_nonce
                .access(&mut storage)
                .entry(&channel_str)
                .get()?
                .unwrap_or_default()
                + 1;

            assert_eq!(nonce, stored_nonce, "nonce mismatch");

            self.received_nonce
                .access(&mut storage)
                .entry_mut(&channel_str)
                .set(&stored_nonce)?;

            self.received_packet
                .access(&mut storage)
                .entry_mut(&channel_str)
                .entry_mut(&stored_nonce)
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
