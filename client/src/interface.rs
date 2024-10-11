use cosmwasm_std::{Response, StdError};
use sylvia::interface;
use sylvia::types::{ExecCtx, QueryCtx};

#[interface]
pub trait EurekaLightClient {
    type Error: From<StdError>;

    #[sv::msg(exec)]
    fn update(&self, ctx: ExecCtx, header: Vec<u8>) -> Result<Response, Self::Error>;

    #[sv::msg(query)]
    fn check_membership(
        &self,
        ctx: QueryCtx,
        height: u64,
        key: Vec<u8>,
        value: Vec<u8>,
    ) -> Result<bool, Self::Error>;

    #[sv::msg(query)]
    fn check_non_membership(
        &self,
        ctx: QueryCtx,
        height: u64,
        key: Vec<u8>,
    ) -> Result<bool, Self::Error>;
}
