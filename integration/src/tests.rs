use eureka_app::sv::mt::{CodeId as AppCodeId, ContractProxy};
use eureka_app::Contract as AppContract;
use eureka_client::interface::sv::mt::LightClientProxy;
use eureka_client::sv::mt::CodeId as ClientCodeId;
use eureka_client::Contract as ClientContract;
use eureka_tao::sv::mt::{CodeId as TaoCodeId, ContractProxy as TaoContractProxy};
use eureka_tao::{Packet, PacketHeader, Payload, PayloadHeader};
use rstest::rstest;
use sylvia::cw_multi_test::BasicApp;
use sylvia::cw_std::Addr;
use sylvia::multitest::{App, Proxy};

#[rstest]
fn test_ibc_eureka_cw() {
    let chain_1 = App::default();

    let client_code_id = ClientCodeId::store_code(&chain_1);
    let tao_code_id = TaoCodeId::store_code(&chain_1);
    let app_1_code_id = AppCodeId::store_code(&chain_1);
    let app_2_code_id = AppCodeId::store_code(&chain_1);

    let authorized = Addr::unchecked("authorized");
    let unauthorized = Addr::unchecked("unauthorized");

    let tao_contract = tao_code_id.instantiate().call(&authorized).unwrap();

    let client_1_deploy = tao_contract
        .deploy(
            client_code_id.code_id(),
            r#"{"client_state": [], "consensus_state": []}"#.as_bytes().into(),
        )
        .call(&unauthorized)
        .unwrap();

    let client_1_addrs = client_1_deploy
        .events
        .iter()
        .filter(|e| {
            e.ty == "instantiate"
                && e.attributes[0].key == "_contract_address"
                && e.attributes[1].key == "code_id"
                && e.attributes[1].value == client_code_id.code_id().to_string()
        })
        .map(|e| e.attributes[0].value.clone())
        .collect::<Vec<_>>();

    assert_eq!(client_1_addrs.len(), 1);

    let client_1_addr = Addr::unchecked(&client_1_addrs[0]);

    let client_contract: Proxy<'_, BasicApp, ClientContract> =
        Proxy::new(client_1_addr.clone(), &chain_1);

    client_contract.update(vec![]).call(&unauthorized).unwrap();

    let app_1_deploy = tao_contract
        .deploy(app_1_code_id.code_id(), "{}".as_bytes().into())
        .call(&unauthorized)
        .unwrap();

    let app_1_addrs = app_1_deploy
        .events
        .iter()
        .filter(|e| {
            e.ty == "instantiate"
                && e.attributes[0].key == "_contract_address"
                && e.attributes[1].key == "code_id"
                && e.attributes[1].value == app_1_code_id.code_id().to_string()
        })
        .map(|e| e.attributes[0].value.clone())
        .collect::<Vec<_>>();

    assert_eq!(app_1_addrs.len(), 1);

    let app_1_addr = Addr::unchecked(&app_1_addrs[0]);

    let app_2_deploy = tao_contract
        .deploy(app_2_code_id.code_id(), "{}".as_bytes().into())
        .call(&unauthorized)
        .unwrap();

    let app_2_addrs = app_2_deploy
        .events
        .iter()
        .filter(|e| {
            e.ty == "instantiate"
                && e.attributes[0].key == "_contract_address"
                && e.attributes[1].key == "code_id"
                && e.attributes[1].value == app_2_code_id.code_id().to_string()
        })
        .map(|e| e.attributes[0].value.clone())
        .collect::<Vec<_>>();

    assert_eq!(app_2_addrs.len(), 1);

    let app_2_addr = Addr::unchecked(&app_2_addrs[0]);

    let app_1_contract: Proxy<'_, BasicApp, AppContract> = Proxy::new(app_1_addr.clone(), &chain_1);
    let app_2_contract: Proxy<'_, BasicApp, AppContract> = Proxy::new(app_2_addr.clone(), &chain_1);

    let owned_contracts = tao_contract.owned_contracts().unwrap();
    assert_eq!(owned_contracts.len(), 3);

    assert_eq!(app_1_contract.sent_value().unwrap(), "null");
    assert_eq!(app_1_contract.received_value().unwrap(), "null");
    assert_eq!(app_2_contract.sent_value().unwrap(), "null");
    assert_eq!(app_2_contract.received_value().unwrap(), "null");

    let data_1_2 = "1 to 2";

    let packet_1_2 = Packet {
        header: PacketHeader {
            client_source: client_contract.contract_addr.clone(),
            client_destination: client_contract.contract_addr.clone(),
            timeout: chain_1.block_info().time.seconds() + 10,
        },
        payloads: vec![Payload {
            header: PayloadHeader {
                application_source: app_1_contract.contract_addr.clone(),
                application_destination: app_2_contract.contract_addr.clone(),
                nonce: 1,
            },
            data: data_1_2.as_bytes().to_vec(),
        }],
    };

    tao_contract
        .send_packet(packet_1_2.clone())
        .call(&authorized)
        .unwrap();

    assert_eq!(
        app_1_contract.sent_value().unwrap(),
        format!(
            "{}(via {}) receives {}",
            app_2_contract.contract_addr, tao_contract.contract_addr, data_1_2
        )
    );

    tao_contract
        .receive_packet(packet_1_2, 0, vec![])
        .call(&unauthorized)
        .unwrap();

    assert_eq!(
        app_2_contract.received_value().unwrap(),
        format!(
            "{}(via {}) sent {}",
            app_1_contract.contract_addr, tao_contract.contract_addr, data_1_2
        )
    );

    let data_2_1 = "2 to 1";

    let packet_2_1 = Packet {
        header: PacketHeader {
            client_source: client_contract.contract_addr.clone(),
            client_destination: client_contract.contract_addr.clone(),
            timeout: chain_1.block_info().time.seconds() + 10,
        },
        payloads: vec![Payload {
            header: PayloadHeader {
                application_source: app_2_contract.contract_addr.clone(),
                application_destination: app_1_contract.contract_addr.clone(),
                nonce: 1,
            },
            data: data_2_1.as_bytes().to_vec(),
        }],
    };

    tao_contract
        .send_packet(packet_2_1.clone())
        .call(&authorized)
        .unwrap();

    assert_eq!(
        app_2_contract.sent_value().unwrap(),
        format!(
            "{}(via {}) receives {}",
            app_1_contract.contract_addr, tao_contract.contract_addr, data_2_1
        )
    );

    tao_contract
        .receive_packet(packet_2_1, 0, vec![])
        .call(&unauthorized)
        .unwrap();

    assert_eq!(
        app_1_contract.received_value().unwrap(),
        format!(
            "{}(via {}) sent {}",
            app_2_contract.contract_addr, tao_contract.contract_addr, data_2_1
        )
    );
}
