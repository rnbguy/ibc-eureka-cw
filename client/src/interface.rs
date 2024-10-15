use cosmwasm_std::{Response, StdError};
use sylvia::interface;
use sylvia::types::{ExecCtx, QueryCtx};

#[interface]
pub trait LightClient {
    type Error: From<StdError>;

    #[sv::msg(exec)]
    fn update(&self, ctx: ExecCtx, header: Vec<u8>) -> Result<Response, Self::Error>;

    #[sv::msg(query)]
    fn check_membership(
        &self,
        ctx: QueryCtx,
        key: Vec<u8>,
        value: Vec<u8>,
        commitment_prefix: Vec<u8>,
        height: u64,
        proof: Vec<u8>,
    ) -> Result<bool, Self::Error>;

    #[sv::msg(query)]
    fn check_non_membership(
        &self,
        ctx: QueryCtx,
        key: Vec<u8>,
        commitment_prefix: Vec<u8>,
        height: u64,
        proof: Vec<u8>,
    ) -> Result<bool, Self::Error>;
}
