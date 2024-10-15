use eureka_app::sv::mt::{CodeId as AppCodeId, ContractProxy};
use eureka_client::interface::sv::mt::LightClientProxy;
use eureka_client::sv::mt::CodeId as ClientCodeId;
use eureka_tao::sv::mt::{CodeId as TaoCodeId, ContractProxy as TaoContractProxy};
use eureka_tao::{Packet, PacketHeader, Payload, PayloadHeader};
use rstest::rstest;
use sylvia::cw_std::Addr;
use sylvia::multitest::App;

#[rstest]
fn test_ibc_eureka_cw() {
    let chain_1 = App::default();

    let client_code_id = ClientCodeId::store_code(&chain_1);
    let tao_code_id = TaoCodeId::store_code(&chain_1);
    let application_code_id = AppCodeId::store_code(&chain_1);

    let gov = Addr::unchecked("gov-module");
    let dao = Addr::unchecked("dao");
    let alice = Addr::unchecked("alice");
    let hacker = Addr::unchecked("hacker");

    let tao_contract = tao_code_id.instantiate().call(&gov).unwrap();

    let client_1_contract = client_code_id
        .instantiate(vec![], vec![])
        .call(&dao)
        .unwrap();

    client_1_contract.update(vec![]).call(&hacker).unwrap();

    let client_2_contract = client_code_id
        .instantiate(vec![], vec![])
        .call(&dao)
        .unwrap();

    client_2_contract.update(vec![]).call(&hacker).unwrap();

    let app_1_contract = application_code_id
        .instantiate(tao_contract.contract_addr.clone())
        .call(&alice)
        .unwrap();
    let app_2_contract = application_code_id
        .instantiate(tao_contract.contract_addr.clone())
        .call(&alice)
        .unwrap();

    // only contract initiator can set allowed channel

    app_1_contract
        .set_allowed_channel(
            (client_1_contract.contract_addr.clone(), vec![]),
            (client_2_contract.contract_addr.clone(), vec![]),
            app_2_contract.contract_addr.clone(),
        )
        .call(&hacker)
        .unwrap_err();

    app_2_contract
        .set_allowed_channel(
            (client_2_contract.contract_addr.clone(), vec![]),
            (client_1_contract.contract_addr.clone(), vec![]),
            app_1_contract.contract_addr.clone(),
        )
        .call(&hacker)
        .unwrap_err();

    app_1_contract
        .set_allowed_channel(
            (client_1_contract.contract_addr.clone(), vec![]),
            (client_2_contract.contract_addr.clone(), vec![]),
            app_2_contract.contract_addr.clone(),
        )
        .call(&alice)
        .unwrap();

    app_2_contract
        .set_allowed_channel(
            (client_2_contract.contract_addr.clone(), vec![]),
            (client_1_contract.contract_addr.clone(), vec![]),
            app_1_contract.contract_addr.clone(),
        )
        .call(&alice)
        .unwrap();

    assert_eq!(app_1_contract.sent_value().unwrap(), "null");
    assert_eq!(app_1_contract.received_value().unwrap(), "null");
    assert_eq!(app_2_contract.sent_value().unwrap(), "null");
    assert_eq!(app_2_contract.received_value().unwrap(), "null");

    let data_1_2 = "1 to 2";

    let packet_1_2 = Packet {
        header: PacketHeader {
            client_source: (client_1_contract.contract_addr.clone(), vec![]),
            client_destination: (client_2_contract.contract_addr.clone(), vec![]),
            nonce: 1,
            timeout: chain_1.block_info().time.seconds() + 10,
        },
        payloads: vec![Payload {
            header: PayloadHeader {
                application_source: app_1_contract.contract_addr.clone(),
                application_destination: app_2_contract.contract_addr.clone(),
            },
            data: data_1_2.as_bytes().to_vec(),
        }],
    };

    // only alice is allowed to send packet
    tao_contract
        .send_packet(packet_1_2.clone())
        .call(&hacker)
        .unwrap_err();

    tao_contract
        .send_packet(packet_1_2.clone())
        .call(&alice)
        .unwrap();

    assert_eq!(
        app_1_contract.sent_value().unwrap(),
        format!(
            "{}(via {}) receives {}",
            app_2_contract.contract_addr, tao_contract.contract_addr, data_1_2
        )
    );

    // anyone can relay received packet, as commitment proof is included
    tao_contract
        .receive_packet(packet_1_2, 0, vec![])
        .call(&hacker)
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
            client_source: (client_2_contract.contract_addr.clone(), vec![]),
            client_destination: (client_1_contract.contract_addr.clone(), vec![]),
            nonce: 1,
            timeout: chain_1.block_info().time.seconds() + 10,
        },
        payloads: vec![Payload {
            header: PayloadHeader {
                application_source: app_2_contract.contract_addr.clone(),
                application_destination: app_1_contract.contract_addr.clone(),
            },
            data: data_2_1.as_bytes().to_vec(),
        }],
    };

    // only alice is allowed to send packet
    tao_contract
        .send_packet(packet_2_1.clone())
        .call(&hacker)
        .unwrap_err();

    tao_contract
        .send_packet(packet_2_1.clone())
        .call(&alice)
        .unwrap();

    assert_eq!(
        app_2_contract.sent_value().unwrap(),
        format!(
            "{}(via {}) receives {}",
            app_1_contract.contract_addr, tao_contract.contract_addr, data_2_1
        )
    );

    // anyone can relay received packet, as commitment proof is included
    tao_contract
        .receive_packet(packet_2_1, 0, vec![])
        .call(&hacker)
        .unwrap();

    assert_eq!(
        app_1_contract.received_value().unwrap(),
        format!(
            "{}(via {}) sent {}",
            app_2_contract.contract_addr, tao_contract.contract_addr, data_2_1
        )
    );
}
