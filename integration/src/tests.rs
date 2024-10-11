use eureka_app::sv::mt::{CodeId as AppCodeId, ContractProxy as AppContractProxy};
use eureka_client::interface::sv::mt::EurekaLightClientProxy;
use eureka_client::sv::mt::CodeId as ClientCodeId;
use eureka_tao::sv::mt::{CodeId as TaoCodeId, ContractProxy as TaoContractProxy};
use eureka_tao::{Packet, PacketHeader, Payload, PayloadHeader};
use rstest::rstest;
use sylvia::cw_std::Addr;
use sylvia::multitest::App;

#[rstest]
fn cw_contract() {
    let chain_1 = App::default();

    let client_code_id = ClientCodeId::store_code(&chain_1);
    let tao_code_id = TaoCodeId::store_code(&chain_1);
    let app_code_id = AppCodeId::store_code(&chain_1);

    let authorized = Addr::unchecked("authorized");
    let unauthorized = Addr::unchecked("unauthorized");

    let client_contract = client_code_id
        .instantiate(vec![], vec![])
        .call(&authorized)
        .unwrap();

    client_contract.update(vec![]).call(&unauthorized).unwrap();

    let tao_contract = tao_code_id.instantiate().call(&authorized).unwrap();

    let app_contract = app_code_id
        .instantiate(tao_contract.contract_addr.clone())
        .call(&tao_contract.contract_addr)
        .unwrap();

    let resp = app_contract.query().unwrap();
    assert_eq!(resp, "hello world");

    let packet = Packet {
        header: PacketHeader {
            client_source: client_contract.contract_addr.clone(),
            client_destination: client_contract.contract_addr.clone(),
            timeout: chain_1.block_info().time.seconds() + 10,
        },
        payloads: vec![Payload {
            header: PayloadHeader {
                application_source: app_contract.contract_addr.clone(),
                application_destination: app_contract.contract_addr.clone(),
            },
            data: b"hello using packet".to_vec(),
        }],
    };

    let _resp = tao_contract
        .receive_packet(packet)
        .call(&unauthorized)
        .unwrap();

    let resp = app_contract.query().unwrap();
    assert_eq!(resp, "hello using packet");
}
