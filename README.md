# ibc-eureka-cw

[`ibc-eureka`][ibc-eureka] prototype in [CosmWasm][cw] smart contract
environment.

_Built using [sylvia][sylvia] and [storey][storey]._

## Transport layer

- [tao](tao)

## Light Client

- [interface](lightclients/interface)
- [dummy](lightclients/dummy): success on everything

## Application

- [interface](applications/interface)
- [pingpong](applications/pingpong): send/receive message between chains
- [cw20 transfer](applications/cw20-transfer): transfer [CW20][cw20] tokens
  between chains

[ibc-eureka]: https://github.com/cosmos/ibc/tree/main/spec/eureka
[cw]: https://cosmwasm.com
[sylvia]: https://github.com/CosmWasm/sylvia
[storey]: https://github.com/CosmWasm/storey
[cw20]: https://github.com/CosmWasm/cw-plus/tree/main/packages/cw20
