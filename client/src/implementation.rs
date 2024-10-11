use cosmwasm_std::{Response, StdError};
use sylvia::types::{ExecCtx, QueryCtx};

use crate::interface::EurekaLightClient;
use crate::Contract;

impl EurekaLightClient for Contract {
    type Error = StdError;

    fn update(&self, _ctx: ExecCtx, _header: Vec<u8>) -> Result<Response, Self::Error> {
        Ok(Response::default())
    }

    fn check_membership(
        &self,
        _ctx: QueryCtx,
        _height: u64,
        _key: Vec<u8>,
        _value: Vec<u8>,
    ) -> Result<bool, Self::Error> {
        Ok(true)
    }

    fn check_non_membership(
        &self,
        _ctx: QueryCtx,
        _height: u64,
        _key: Vec<u8>,
    ) -> Result<bool, Self::Error> {
        Ok(true)
    }
}
