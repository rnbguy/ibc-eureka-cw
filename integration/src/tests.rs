use eureka_tao::sv::mt::{CodeId, TaoContractProxy};
use rstest::rstest;
use sylvia::cw_std::Addr;
use sylvia::multitest::App;

#[rstest]
#[case(1, 2, 3)]
fn it_works(#[case] a: u64, #[case] b: u64, #[case] c: u64) {
    assert_eq!(a + b, c);
}

#[rstest]
fn cw_contract() {
    let app = App::default();
    let code_id = CodeId::store_code(&app);

    let owner = Addr::unchecked("owner");

    let contract = code_id.instantiate().call(&owner).unwrap();

    let resp = contract.query().unwrap();
    assert!(resp.is_empty());
}
