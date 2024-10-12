use cosmwasm_schema::cw_serde;
use cosmwasm_std::{Addr, Binary, Reply, SubMsg, WasmMsg};
use cw_storey::containers::{Item, Map};
use cw_storey::CwStorage;
use eureka_app::interface::sv::Executor;
use eureka_app::interface::Application;
use eureka_client::interface::sv::Querier;
use eureka_client::interface::LightClient;
use storey::containers::IterableAccessor;
use sylvia::contract;
use sylvia::cw_std::{Response, StdError, StdResult};
use sylvia::types::{ExecCtx, InstantiateCtx, QueryCtx, Remote, ReplyCtx};

const INSTANTIATE_REPLY_ID: u64 = 1;

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
    pub owned_contracts: Map<String, Item<()>>,
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
            owned_contracts: Map::new(b'0'),
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
            label: "new IBC app".into(),
        };

        let submessage = SubMsg::reply_on_success(instantiate_msg, INSTANTIATE_REPLY_ID)
            .with_payload(code_id.to_le_bytes());

        Ok(Response::default().add_submessage(submessage))
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

        let mut storage = CwStorage(ctx.deps.storage);

        if self
            .owned_contracts
            .access(&mut storage)
            .entry(&packet.header.client_source.to_string())
            .get()?
            .is_none()
        {
            return Err(StdError::generic_err(format!(
                "unauthorized source client: {}",
                packet.header.client_source
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

        let mut msgs = vec![];

        for payload in payloads {
            let PayloadHeader {
                application_source,
                application_destination,
                nonce,
            } = payload.header;

            if self
                .owned_contracts
                .access(&mut storage)
                .entry(&application_source.to_string())
                .get()?
                .is_none()
            {
                return Err(StdError::generic_err(format!(
                    "unauthorized source application: {}",
                    application_source
                )));
            }

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

        let mut storage = CwStorage(ctx.deps.storage);

        if self
            .owned_contracts
            .access(&mut storage)
            .entry(&packet.header.client_destination.to_string())
            .get()?
            .is_none()
        {
            return Err(StdError::generic_err(format!(
                "unauthorized destination client: {}",
                packet.header.client_destination
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

        let mut msgs = vec![];

        for payload in payloads {
            let PayloadHeader {
                application_source,
                application_destination,
                nonce,
            } = payload.header;

            if self
                .owned_contracts
                .access(&mut storage)
                .entry(&application_destination.to_string())
                .get()?
                .is_none()
            {
                return Err(StdError::generic_err(format!(
                    "unauthorized destination application: {}",
                    application_destination
                )));
            }

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
    fn owned_contracts(&self, ctx: QueryCtx) -> StdResult<Vec<String>> {
        let mut storage = CwStorage(ctx.deps.storage);
        self.owned_contracts
            .access(&mut storage)
            .keys()
            .map(|k| {
                k.map(|v| v.0)
                    .map_err(|e| StdError::generic_err(e.to_string()))
            })
            .collect()
    }

    #[sv::msg(reply)]
    fn reply(&self, ctx: ReplyCtx, reply: Reply) -> StdResult<Response> {
        match reply.id {
            INSTANTIATE_REPLY_ID => {
                // let code_id = u64::from_le_bytes(reply.payload.as_slice().try_into().unwrap());

                let events = reply
                    .result
                    .into_result()
                    .map_err(StdError::generic_err)?
                    .events;

                assert_eq!(events.len(), 1);

                let event = &events[0];

                assert_eq!(event.ty, "instantiate");
                assert_eq!(event.attributes.len(), 2);
                assert_eq!(event.attributes[1].key, "code_id");
                // assert_eq!(event.attributes[1].value, code_id.to_string());
                assert_eq!(event.attributes[0].key, "_contract_address");

                let mut storage = CwStorage(ctx.deps.storage);
                self.owned_contracts
                    .access(&mut storage)
                    .entry_mut(&event.attributes[0].value)
                    .set(&())?;

                Ok(Response::default())
            }
            id => Err(StdError::generic_err(format!("Unknown reply id: {}", id))),
        }
    }
}
