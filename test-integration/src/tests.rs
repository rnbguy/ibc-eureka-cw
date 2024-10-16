use eureka_application_pingpong::sv::mt::{CodeId as AppCodeId, ContractProxy};
use eureka_lightclient_dummy::sv::mt::CodeId as lightclientCodeId;
use eureka_lightclient_interface::sv::mt::LightClientProxy;
use eureka_tao::sv::mt::{CodeId as TaoCodeId, ContractProxy as TaoContractProxy};
use eureka_tao::{Packet, PacketHeader, Payload, PayloadHeader};
use rstest::rstest;
use sylvia::cw_std::Addr;
use sylvia::multitest::App;

#[rstest]
fn test_ibc_eureka_cw() {
    let chain_1 = App::default();

    let lightclient_code_id = lightclientCodeId::store_code(&chain_1);
    let tao_code_id = TaoCodeId::store_code(&chain_1);
    let application_code_id = AppCodeId::store_code(&chain_1);

    let gov = Addr::unchecked("gov-module");
    let dao = Addr::unchecked("dao");
    let alice = Addr::unchecked("alice");
    let hacker = Addr::unchecked("hacker");

    let tao_contract = tao_code_id.instantiate().call(&gov).unwrap();

    let lightclient_1_contract = lightclient_code_id
        .instantiate(vec![], vec![])
        .call(&dao)
        .unwrap();

    lightclient_1_contract.update(vec![]).call(&hacker).unwrap();

    let lightclient_2_contract = lightclient_code_id
        .instantiate(vec![], vec![])
        .call(&dao)
        .unwrap();

    lightclient_2_contract.update(vec![]).call(&hacker).unwrap();

    let application_1_contract = application_code_id
        .instantiate(tao_contract.contract_addr.clone())
        .call(&alice)
        .unwrap();
    let application_2_contract = application_code_id
        .instantiate(tao_contract.contract_addr.clone())
        .call(&alice)
        .unwrap();

    // only contract initiator can set allowed channel

    application_1_contract
        .set_allowed_channel(
            (lightclient_1_contract.contract_addr.clone(), vec![]),
            (lightclient_2_contract.contract_addr.clone(), vec![]),
            application_2_contract.contract_addr.clone(),
        )
        .call(&hacker)
        .unwrap_err();

    application_2_contract
        .set_allowed_channel(
            (lightclient_2_contract.contract_addr.clone(), vec![]),
            (lightclient_1_contract.contract_addr.clone(), vec![]),
            application_1_contract.contract_addr.clone(),
        )
        .call(&hacker)
        .unwrap_err();

    application_1_contract
        .set_allowed_channel(
            (lightclient_1_contract.contract_addr.clone(), vec![]),
            (lightclient_2_contract.contract_addr.clone(), vec![]),
            application_2_contract.contract_addr.clone(),
        )
        .call(&alice)
        .unwrap();

    application_2_contract
        .set_allowed_channel(
            (lightclient_2_contract.contract_addr.clone(), vec![]),
            (lightclient_1_contract.contract_addr.clone(), vec![]),
            application_1_contract.contract_addr.clone(),
        )
        .call(&alice)
        .unwrap();

    assert_eq!(application_1_contract.sent_value().unwrap(), "null");
    assert_eq!(application_1_contract.received_value().unwrap(), "null");
    assert_eq!(application_2_contract.sent_value().unwrap(), "null");
    assert_eq!(application_2_contract.received_value().unwrap(), "null");

    let data_1_2 = "1 to 2";

    let packet_1_2 = Packet {
        header: PacketHeader {
            lightclient_source: (lightclient_1_contract.contract_addr.clone(), vec![]),
            lightclient_destination: (lightclient_2_contract.contract_addr.clone(), vec![]),
            nonce: 1,
            timeout: chain_1.block_info().time.seconds() + 10,
        },
        payloads: vec![Payload {
            header: PayloadHeader {
                application_source: application_1_contract.contract_addr.clone(),
                application_destination: application_2_contract.contract_addr.clone(),
                funds: vec![],
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
        application_1_contract.sent_value().unwrap(),
        format!(
            "{}(via {}) receives {}",
            application_2_contract.contract_addr, tao_contract.contract_addr, data_1_2
        )
    );

    // anyone can relay received packet, as commitment proof is included
    tao_contract
        .receive_packet(packet_1_2, 0, vec![])
        .call(&hacker)
        .unwrap();

    assert_eq!(
        application_2_contract.received_value().unwrap(),
        format!(
            "{}(via {}) sent {}",
            application_1_contract.contract_addr, tao_contract.contract_addr, data_1_2
        )
    );

    let data_2_1 = "2 to 1";

    let packet_2_1 = Packet {
        header: PacketHeader {
            lightclient_source: (lightclient_2_contract.contract_addr.clone(), vec![]),
            lightclient_destination: (lightclient_1_contract.contract_addr.clone(), vec![]),
            nonce: 1,
            timeout: chain_1.block_info().time.seconds() + 10,
        },
        payloads: vec![Payload {
            header: PayloadHeader {
                application_source: application_2_contract.contract_addr.clone(),
                application_destination: application_1_contract.contract_addr.clone(),
                funds: vec![],
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
        application_2_contract.sent_value().unwrap(),
        format!(
            "{}(via {}) receives {}",
            application_1_contract.contract_addr, tao_contract.contract_addr, data_2_1
        )
    );

    // anyone can relay received packet, as commitment proof is included
    tao_contract
        .receive_packet(packet_2_1, 0, vec![])
        .call(&hacker)
        .unwrap();

    assert_eq!(
        application_1_contract.received_value().unwrap(),
        format!(
            "{}(via {}) sent {}",
            application_2_contract.contract_addr, tao_contract.contract_addr, data_2_1
        )
    );
}
