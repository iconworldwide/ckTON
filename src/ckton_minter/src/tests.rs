
use std::str::FromStr;
use std::time::Duration;

use candid::{decode_args, encode_args, Decode, Nat, Principal};
use icrc_ledger_types::icrc1::account::{Account, Subaccount};
use pocket_ic::common::rest::{CanisterHttpHeader, CanisterHttpReply, CanisterHttpResponse, MockCanisterHttpResponse, RawMessageId};
use pocket_ic::{PocketIc, PocketIcBuilder};
use reqwest::blocking::Client;

const WASM_BYTES: &[u8] = include_bytes!("../../../target/wasm32-unknown-unknown/release/ckton_minter.wasm");


#[test]
fn test_ton_address_generation() {
    
    let sender = Principal::from_str("lxa3f-k4w5m-3jyvz-gau5t-bceou-czpd4-thtwe-b4unx-vvrho-dtvwh-qqe").unwrap();

    let (pic, minter_id) = setup(sender);

    let ton_address = generate_ton_address(&pic, minter_id, sender);

    println!("TON Address: {}", ton_address)

}

#[test]
fn test_wallet_balance() {
    let sender = Principal::from_str("lxa3f-k4w5m-3jyvz-gau5t-bceou-czpd4-thtwe-b4unx-vvrho-dtvwh-qqe").unwrap();

    let (pic, minter_id) = setup(sender);

    let ton_address = generate_ton_address(&pic, minter_id, sender);

    let balance = wallet_balance(&pic, minter_id, ton_address, sender);

    for _ in 0..6 {
        pic.tick();
    }
    

    let requests = pic.get_canister_http();

    assert_eq!(requests.len(), 1);

    for request in requests {

        
        let mock_response = call_proxy_service(&request);

        pic.mock_canister_http_response(mock_response);
    }

   let result = pic.await_call(balance).unwrap();

    let data = match result {   
        pocket_ic::WasmResult::Reply(items) => items,
        pocket_ic::WasmResult::Reject(d) => panic!("Wallet balance rejected with :{}", d),
    };

    let balance : u64 = Decode!(&data, u64).unwrap();

    println!("Balance: {}", balance);
}

fn call_proxy_service(request: &pocket_ic::common::rest::CanisterHttpRequest) -> MockCanisterHttpResponse {
    let client = Client::new();
    let request_handler = match request.http_method {
        pocket_ic::common::rest::CanisterHttpMethod::GET => todo!(),
        pocket_ic::common::rest::CanisterHttpMethod::POST => {
            let r_builder = client.post(request.url.clone()).body(request.body.clone());
            let r_builder = request.headers.iter().fold(r_builder, |builder, hv| {
                builder.header(hv.name.as_str(), hv.value.as_str())
            });
            r_builder.send()
        },
        pocket_ic::common::rest::CanisterHttpMethod::HEAD => todo!(),
    };
    let response = request_handler.unwrap();

    // println!("{:?}", serde_json::from_slice::<serde_json::Value>(response.bytes().unwrap().as_ref()).unwrap());

    let canister_http_response = CanisterHttpResponse::CanisterHttpReply(CanisterHttpReply{ 
        status: response.status().as_u16(), 
        headers: response.headers().iter().map(|hv|{
            CanisterHttpHeader{ 
                name: hv.0.as_str().to_string().to_lowercase(), 
                value: hv.1.to_str().unwrap().to_string().to_lowercase() 
            }
        }).collect(),
        body: response.bytes().unwrap().to_vec() 
    });
    
    MockCanisterHttpResponse { 
        subnet_id: request.subnet_id, 
        request_id: request.request_id, 
        response: canister_http_response, 
        additional_responses: vec![] 
    }
}

#[test]
fn test_get_time_in_secs() {
    let sender = Principal::from_str("lxa3f-k4w5m-3jyvz-gau5t-bceou-czpd4-thtwe-b4unx-vvrho-dtvwh-qqe").unwrap();

    let (pic, minter_id) = setup(sender);

    let time = get_time_in_secs(&pic, minter_id, sender);

    println!("Time in secs: {}", time);
}

fn get_time_in_secs(pic: &PocketIc, minter_id: Principal, sender: Principal) -> u32 {
    let result = pic.query_call(minter_id, sender, "get_time_in_secs", encode_args(()).unwrap()).unwrap();

    let data = match result {
        pocket_ic::WasmResult::Reply(items) => items,
        pocket_ic::WasmResult::Reject(d) => panic!("Get time in secs rejected with :{}", d),
    };

    let time : u32 = Decode!(&data, u32).unwrap();

    time
}

#[test]
fn test_wallet_deploy() {
    let sender = Principal::from_str("lxa3f-k4w5m-3jyvz-gau5t-bceou-czpd4-thtwe-b4unx-vvrho-dtvwh-qqe").unwrap();

    let (pic, minter_id) = setup(sender);

    let result = deploy_ton_wallet(&pic, minter_id, sender);

    for _ in 0..50 {
        pic.tick();
    }

    let requests = pic.get_canister_http();

    println!("First Request len: {}", requests.len());

    assert_eq!(requests.len(), 1);

    for request in requests {
        let response = call_proxy_service(&request);

        pic.mock_canister_http_response(response);
    }

    // for _ in 0..16 {
    //     pic.tick();
    // }

    // let requests = pic.get_canister_http();

    // println!("Second Request len: {}", requests.len());

    // for request in requests {
    //     let response = call_proxy_service(&request);

    //     pic.mock_canister_http_response(response);
    // }

    // for _ in 0..12 {
    //     pic.tick();
    // }

    let result = pic.await_call(result).unwrap();

    let data = match result {
        pocket_ic::WasmResult::Reply(items) => items,
        pocket_ic::WasmResult::Reject(d) => panic!("Deploy ton wallet rejected with :{}", d),
    };

    let deploy_result = Decode!(&data, Result<String, String>).unwrap();

    println!("Deploy result: {:?}", deploy_result);

    assert!(deploy_result.is_ok());
    
}

#[test]
fn test_destroy_ton_wallet() {
    let sender = Principal::from_str("lxa3f-k4w5m-3jyvz-gau5t-bceou-czpd4-thtwe-b4unx-vvrho-dtvwh-qqe").unwrap();

    let (pic, minter_id) = setup(sender);

    let result = destroy_ton_wallet(&pic, minter_id, sender);

    for _ in 0..12 {
        pic.tick();
    }   

    let requests = pic.get_canister_http();

    assert_eq!(requests.len(), 1);

    for request in requests {
        let response = call_proxy_service(&request);

        pic.mock_canister_http_response(response);
    }

    for _ in 0..12 {
        pic.tick();
    }

    let requests = pic.get_canister_http();

    assert_eq!(requests.len(), 1);

    for request in requests {
        let response = call_proxy_service(&request);

        pic.mock_canister_http_response(response);
    }

    for _ in 0..12 {
        pic.tick();
    }

    let result = pic.await_call(result).unwrap();

    let data = match result {   
        pocket_ic::WasmResult::Reply(items) => items,
        pocket_ic::WasmResult::Reject(d) => panic!("Destroy ton wallet rejected with :{}", d),
    };

    let destroy_result = Decode!(&data, Result<String, String>).unwrap();

    println!("Destroy result: {:?}", destroy_result);

    assert!(destroy_result.is_ok());

}

#[test]
fn test_mint_ton_wallet() {

    let sender = Principal::from_str("lxa3f-k4w5m-3jyvz-gau5t-bceou-czpd4-thtwe-b4unx-vvrho-dtvwh-qqe").unwrap();

    let (pic, minter_id) = setup(sender);

    let ton_address = generate_ton_address(&pic, minter_id, sender);

    // deploy_ton_wallet(&pic, minter_id, sender);

    let admin_setup_result = call_admin_setup(&pic, minter_id, sender);

    println!("Admin setup result: {:?}", admin_setup_result);

    assert!(admin_setup_result.is_ok(), "Admin setup failed");

    let result = mint_ton_wallet(&pic, minter_id, sender, ton_address);
    
    for _ in 0..12 {
        pic.tick();
    }

    let requests = pic.get_canister_http();

    println!("First Request len: {}", requests.len());

    assert!(requests.len() > 0, "No requests made");

    for request in requests {
        let response = call_proxy_service(&request);

        pic.mock_canister_http_response(response);
    }

    for _ in 0..24 {
        pic.tick();
    }

    let requests = pic.get_canister_http();

    println!("Second Request len: {}", requests.len());

    assert!(requests.len() > 0, "No requests made");

    for request in requests {
        let response = call_proxy_service(&request);

        pic.mock_canister_http_response(response);
    }

    let ledger_id = ledger_id(&pic, minter_id, sender);

    let ckton_ledger = Principal::from_str(&ledger_id).unwrap();

     let result = pic.await_call(result).unwrap();

    let data = match result {
        pocket_ic::WasmResult::Reply(items) => items,
        pocket_ic::WasmResult::Reject(d) => panic!("Mint ton wallet rejected with :{}", d),
    };

    let mint_result = Decode!(&data, Result<String, String>).unwrap();

    println!("Mint result: {:?}", mint_result);

    assert!(mint_result.is_ok(), "Mint ton wallet failed");

    println!("Sleeping for 30 seconds");
    std::thread::sleep(Duration::from_secs(30));

    println!("Advancing time");
    pic.advance_time(Duration::from_secs(20));

    for _ in 0..16 {
        pic.tick();
    }

    let requests = pic.get_canister_http();


    assert!(requests.len() > 0, "No requests made");

    for request in requests {
        let response = call_proxy_service(&request);

        pic.mock_canister_http_response(response);
    }


    for _ in 0..32 {

        pic.tick();
    }


    let logs = pic.fetch_canister_logs(minter_id, sender);

    assert!(logs.is_ok(), "Failed to fetch logs");

    let logs = logs.unwrap();

    for log in logs {
        let log_str = String::from_utf8(log.content).unwrap();

        println!("Log: {}", log_str);
    }
   

    let bal = ledger_balance(&pic, ckton_ledger, sender);

    assert!(bal > 0, "ckTon balance is 0");

}

fn ledger_balance(pic: &PocketIc, ledger_id: Principal, sender: Principal) -> u64 {
    let account = Account{
        owner: sender,
        subaccount: None
    };

    let result = pic.query_call(ledger_id, sender, "icrc1_balance_of", encode_args((account,)).unwrap()).unwrap();

    let data = match result {
        pocket_ic::WasmResult::Reply(items) => items,
        pocket_ic::WasmResult::Reject(d) => panic!("Ledger balance rejected with :{}", d),
    };

    let balance : Nat = Decode!(&data, Nat).unwrap();

    balance.0.try_into().unwrap()
}

fn ledger_id(pic: &PocketIc, minter_id: Principal, sender: Principal) -> String {
    let result = pic.query_call(minter_id, sender, "ledger_id", encode_args(()).unwrap()).unwrap();

    let data = match result {
        pocket_ic::WasmResult::Reply(items) => items,
        pocket_ic::WasmResult::Reject(d) => panic!("Ledger id rejected with :{}", d),
    };

    let ledger_id : String = Decode!(&data, String).unwrap();

    ledger_id
}

fn call_admin_setup(pic: &PocketIc, minter_id: Principal, sender: Principal) -> Result<(), String> {
    let result = pic.update_call(minter_id, sender, "admin_setup", encode_args(()).unwrap()).unwrap();

    let data = match result {
        pocket_ic::WasmResult::Reply(items) => items,
        pocket_ic::WasmResult::Reject(d) => panic!("Admin setup rejected with :{}", d),
    };

    let admin_setup_result = Decode!(&data, Result<(), String>).unwrap();

    admin_setup_result
}

fn mint_ton_wallet(pic: &PocketIc, minter_id: Principal, sender: Principal, ton_address: String) -> RawMessageId {
    let expire = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_secs() + 60;

    let to_account = Account{
        owner: sender,
        subaccount: None
    };

    let result = pic.submit_call(minter_id, sender, "mint", encode_args((to_account, 200000u64, None::<Subaccount>, Some(expire as u32))).unwrap()).unwrap();

    result
}

fn destroy_ton_wallet(pic: &PocketIc, minter_id: Principal, sender: Principal) -> RawMessageId {
    let expire = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_secs() + 60;

    let result = pic.submit_call(minter_id, sender, "destroy_ton_wallet", encode_args(("0QCUBdLXvkZcp2Gj17VHHxnxPHF406y1HYDh8Ctrc6Rml7ZQ", None::<Subaccount>, Some::<u32>(expire.try_into().unwrap()))).unwrap()).unwrap();

    result
}

fn deploy_ton_wallet(pic: &PocketIc, minter_id: Principal, sender: Principal) -> RawMessageId {
    let expire = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_secs() as u32 + 60;

    println!("Expire: {}", expire);
    let result = pic.submit_call(minter_id, sender, "deploy_ton_wallet", encode_args((None::<Subaccount>, Some(expire))).unwrap()).unwrap();

    result
}

fn wallet_balance(pic: &PocketIc, minter_id: Principal, ton_address: String, sender: Principal) -> RawMessageId {
    let result = pic.submit_call(minter_id, sender, "wallet_balance", encode_args((ton_address,)).unwrap()).unwrap();

    // let data = match result {   
    //     pocket_ic::WasmResult::Reply(items) => items,
    //     pocket_ic::WasmResult::Reject(d) => panic!("Wallet balance rejected with :{}", d),
    // };

    // let balance : u64 = Decode!(&data, u64).unwrap();

    result
}

fn setup(sender: Principal) -> (PocketIc, Principal) {
    let pic = PocketIcBuilder::new().with_nns_subnet().with_ii_subnet().with_application_subnet().build();

    let minter_id = pic.create_canister_with_settings(Some(sender), None);

    pic.add_cycles(minter_id, 200_000_000_000_000);

    pic.install_canister(minter_id, WASM_BYTES.to_vec(), encode_args(()).unwrap(), Some(sender));

    (pic, minter_id)
}

fn generate_ton_address(pic: &PocketIc, minter_id: Principal, sender: Principal) -> String {
    let result = pic.update_call(minter_id, sender, "generate_ton_address", encode_args((None::<Principal>, None::<Subaccount>)).unwrap()).unwrap();
    let data = match result {
        pocket_ic::WasmResult::Reply(items) => items,
        pocket_ic::WasmResult::Reject(d) => panic!("Generat ton address rejected with :{}", d),
    };

    let ton_addr : String = Decode!(&data, String).unwrap();

    ton_addr

}