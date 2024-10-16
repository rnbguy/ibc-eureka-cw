use cosmwasm_schema::cw_serde;
use cosmwasm_std::{to_json_binary, Addr, Coin, Reply, SubMsg, Uint128, WasmMsg};
use cw20::Cw20ExecuteMsg;
use cw_storey::containers::{Item, Map};
use cw_storey::CwStorage;
use eureka_application_interface::Application;
use sylvia::contract;
use sylvia::cw_std::{Response, StdError, StdResult};
use sylvia::types::{ExecCtx, InstantiateCtx, QueryCtx, ReplyCtx};

const REPLY_INSTANTIATE_ID: u64 = 0;

#[cw_serde]
pub struct Channel {
    pub lightclient_local: (Addr, Vec<u8>),
    pub lightclient_remote: (Addr, Vec<u8>),
    pub application_remote: Addr,
}

pub struct Contract {
    // owner can perform sudo level operations
    pub owner: Item<Addr>,

    // only tao can call send and receive
    pub tao_contract: Item<Addr>,

    // allowed channel
    pub allowed_channel: Item<Channel>,

    // cw20 code id
    pub cw20_code_id: Item<u64>,

    // minted cw20 tokens
    pub channel_to_cw20: Map<String, Item<Addr>>,
    pub cw20_to_channel: Map<String, Item<(Channel, String)>>,

    // reply
    pub pending_packet: Item<(Channel, String, TransferPacket)>,
}

#[cfg_attr(not(feature = "library"), sylvia::entry_points)]
#[contract]
#[sv::error(StdError)]
#[sv::messages(eureka_application_interface)]
impl Contract {
    pub const fn new() -> Self {
        Self {
            owner: Item::new(b'O'),
            cw20_code_id: Item::new(b'C'),
            tao_contract: Item::new(b'T'),
            allowed_channel: Item::new(b'A'),
            channel_to_cw20: Map::new(b'W'),
            cw20_to_channel: Map::new(b'L'),
            pending_packet: Item::new(b'P'),
        }
    }

    #[sv::msg(instantiate)]
    fn instantiate(
        &self,
        ctx: InstantiateCtx,
        cw20_code_id: u64,
        tao_addr: Addr,
    ) -> StdResult<Response> {
        let mut storage = CwStorage(ctx.deps.storage);

        self.owner.access(&mut storage).set(&ctx.info.sender)?;
        self.tao_contract.access(&mut storage).set(&tao_addr)?;
        self.cw20_code_id.access(&mut storage).set(&cw20_code_id)?;

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
        lightclient_local: (Addr, Vec<u8>),
        lightclient_remote: (Addr, Vec<u8>),
        application_remote: Addr,
    ) -> Result<Response, StdError> {
        let mut storage = CwStorage(ctx.deps.storage);

        if Some(&ctx.info.sender) != self.owner.access(&mut storage).get()?.as_ref() {
            return Err(StdError::generic_err("unauthorized"));
        }
        self.allowed_channel.access(&mut storage).set(&Channel {
            lightclient_local,
            lightclient_remote,
            application_remote,
        })?;
        Ok(Response::default())
    }

    #[sv::msg(reply)]
    fn reply(&self, ctx: ReplyCtx, reply: Reply) -> StdResult<Response> {
        let mut storage = CwStorage(ctx.deps.storage);

        match reply.id {
            REPLY_INSTANTIATE_ID => {
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
                assert_eq!(
                    event.attributes[1].value,
                    self.cw20_code_id
                        .access(&mut storage)
                        .get()?
                        .unwrap()
                        .to_string()
                );
                assert_eq!(event.attributes[0].key, "_contract_address");

                let new_cw20_addr = Addr::unchecked(event.attributes[0].value.clone());

                let (channel, origin, TransferPacket { receiver, fund, .. }) =
                    self.pending_packet.access(&mut storage).get()?.unwrap();

                self.pending_packet.access(&mut storage).remove();

                self.cw20_to_channel
                    .access(&mut storage)
                    .entry_mut(&new_cw20_addr.to_string())
                    .set(&(channel.clone(), origin.to_string()))
                    .unwrap();

                self.channel_to_cw20
                    .access(&mut storage)
                    .entry_mut(&format!("{:?}-{:?}", channel, origin))
                    .set(&new_cw20_addr)
                    .unwrap();

                // unescrow or mint tokens

                let TransferCoin { amount, denom } = &fund;

                let msg = match denom {
                    TransferDenom::Native(origin) => {
                        // create new cw20 token, if not present
                        let local_cw20 = self
                            .channel_to_cw20
                            .access(&mut storage)
                            .entry(&format!("{:?}-{:?}", channel, origin))
                            .get()?
                            .ok_or_else(|| StdError::generic_err("cw20 token not found"))?;

                        // mint tokens
                        cw20::Cw20Contract(local_cw20).call(Cw20ExecuteMsg::Mint {
                            recipient: receiver.to_string(),
                            amount: *amount,
                        })?
                    }
                    TransferDenom::Bridged { origin, .. } => {
                        // unescrow tokens
                        cw20::Cw20Contract(Addr::unchecked(origin)).call(
                            Cw20ExecuteMsg::Transfer {
                                recipient: receiver.to_string(),
                                amount: *amount,
                            },
                        )?
                    }
                };

                Ok(Response::default().add_message(msg))
            }
            _ => Err(StdError::generic_err("unknown reply id")),
        }
    }
}

#[cw_serde]
pub enum TransferDenom {
    Native(String),
    Bridged { wrapped: String, origin: String },
}

#[cw_serde]
pub struct TransferCoin {
    pub amount: Uint128,
    pub denom: TransferDenom,
}

#[cw_serde]
pub struct TransferPacket {
    pub sender: Addr,
    pub receiver: Addr,
    pub fund: TransferCoin,
    pub memo: String,
}

impl Application for Contract {
    type Error = StdError;

    fn send(
        &self,
        ctx: ExecCtx,
        _sender_local: Addr,
        lightclient_local: (Addr, Vec<u8>),
        lightclient_remote: (Addr, Vec<u8>),
        application_remote: Addr,
        packet: Vec<u8>,
    ) -> Result<Response, Self::Error> {
        let mut storage = CwStorage(ctx.deps.storage);

        if Some(&ctx.info.sender) != self.tao_contract.access(&mut storage).get()?.as_ref() {
            return Err(StdError::generic_err("send can only be called by tao"));
        }

        let channel = Channel {
            lightclient_local,
            lightclient_remote,
            application_remote: application_remote.clone(),
        };

        if Some(&channel) != self.allowed_channel.access(&mut storage).get()?.as_ref() {
            // ICS20 like check
            return Err(StdError::generic_err("not allowed channel"));
        }

        // _sender_local is ignored because anyone can send tokens to the remote application
        // in ICA, it is not the case. Only the sender can send packets to the remote application

        assert!(
            packet.len() <= 1024,
            "packet size must be less than or equal to 1024 bytes"
        );

        let TransferPacket { sender, fund, .. } =
            serde_json::from_slice(&packet).map_err(|e| StdError::generic_err(e.to_string()))?;

        // escrow or burn tokens

        let TransferCoin { amount, denom } = &fund;

        let msg = match denom {
            TransferDenom::Native(origin) => {
                assert_ne!(
                    Some(&channel),
                    self.cw20_to_channel
                        .access(&mut storage)
                        .entry(origin)
                        .get()?
                        .as_ref()
                        .map(|(channel, _)| channel)
                );

                // escrow tokens
                cw20::Cw20Contract(Addr::unchecked(origin)).call(Cw20ExecuteMsg::TransferFrom {
                    owner: sender.to_string(),
                    recipient: ctx.env.contract.address.to_string(),
                    amount: *amount,
                })?
            }
            TransferDenom::Bridged { wrapped, origin } => {
                assert_eq!(
                    Some((&channel, origin)),
                    self.cw20_to_channel
                        .access(&mut storage)
                        .entry(wrapped)
                        .get()?
                        .as_ref()
                        .map(|(channel, origin)| (channel, origin))
                );
                // burn tokens
                cw20::Cw20Contract(Addr::unchecked(wrapped)).call(Cw20ExecuteMsg::BurnFrom {
                    owner: sender.to_string(),
                    amount: *amount,
                })?
            }
        };

        Ok(Response::default().add_message(msg))
    }

    fn receive(
        &self,
        ctx: ExecCtx,
        _sent_funds: Vec<Coin>,
        lightclient_local: (Addr, Vec<u8>),
        lightclient_remote: (Addr, Vec<u8>),
        application_remote: Addr,
        packet: Vec<u8>,
    ) -> Result<Response, Self::Error> {
        // ignoring sender_remote (like, sender_local in send), as remote tao contract is trusted

        let mut storage = CwStorage(ctx.deps.storage);

        if Some(&ctx.info.sender) != self.tao_contract.access(&mut storage).get()?.as_ref() {
            return Err(StdError::generic_err("receive can only be called by tao"));
        }

        let channel = Channel {
            lightclient_local,
            lightclient_remote,
            application_remote,
        };

        if Some(&channel) != self.allowed_channel.access(&mut storage).get()?.as_ref() {
            return Err(StdError::generic_err("not allowed channel"));
        }

        assert!(
            packet.len() <= 1024,
            "packet size must be less than or equal to 1024 bytes"
        );

        let TransferPacket { receiver, fund, .. } =
            serde_json::from_slice(&packet).map_err(|e| StdError::generic_err(e.to_string()))?;

        // create new cw20 token, if not present

        let TransferCoin { denom, .. } = &fund;

        if let TransferDenom::Native(origin) = denom {
            if self
                .channel_to_cw20
                .access(&mut storage)
                .entry(&format!("{:?}-{:?}", channel, origin))
                .get()?
                .is_none()
            {
                // instantiate new cw20 token

                let instantiate_msg = cw20_base::msg::InstantiateMsg {
                    name: format!("{:?}-{:?}", &channel, &origin),
                    symbol: origin.clone(),
                    decimals: 6,
                    initial_balances: vec![],
                    mint: Some(cw20::MinterResponse {
                        minter: ctx.env.contract.address.to_string(),
                        cap: None,
                    }),
                    marketing: None,
                };

                let wasm_msg = WasmMsg::Instantiate {
                    code_id: self.cw20_code_id.access(&mut storage).get()?.unwrap(),
                    msg: to_json_binary(&instantiate_msg).unwrap(),
                    funds: vec![],
                    label: format!("Instantiate CW20 for {:?}-{:?}", &channel, &origin),
                    admin: None,
                };

                let sub_msg = SubMsg::reply_on_success(wasm_msg, REPLY_INSTANTIATE_ID);

                return Ok(Response::default().add_submessage(sub_msg));
            }
        }

        // unescrow or mint tokens

        let TransferCoin { amount, denom } = &fund;

        let msg = match denom {
            TransferDenom::Native(origin) => {
                // create new cw20 token, if not present
                let local_cw20 = self
                    .channel_to_cw20
                    .access(&mut storage)
                    .entry(&format!("{:?}-{:?}", channel, origin))
                    .get()?
                    .unwrap();

                // mint tokens
                cw20::Cw20Contract(local_cw20).call(Cw20ExecuteMsg::Mint {
                    recipient: receiver.to_string(),
                    amount: *amount,
                })?
            }
            TransferDenom::Bridged { origin, .. } => {
                // unescrow tokens
                cw20::Cw20Contract(Addr::unchecked(origin)).call(Cw20ExecuteMsg::Transfer {
                    recipient: receiver.to_string(),
                    amount: *amount,
                })?
            }
        };

        // the memo is ignored
        // since, we support multi payload, we don't need memo hack for atomic IBC packets

        Ok(Response::default().add_message(msg))
    }
}
