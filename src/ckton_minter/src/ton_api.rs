use candid::Nat;
use ic_cdk::{api::management_canister::{http_request::{self, http_request, CanisterHttpRequestArgument, HttpHeader, HttpResponse, TransformArgs, TransformContext}, main::raw_rand}, query, update};
use serde::Deserialize;
use serde_json::json;

use crate::{consts::{PROXY_API_KEY, PROXY_URL, TON_API_KEY, TON_RPC_URL}, types::ProxyRequest};

#[derive(Debug, Deserialize)]
pub struct TonTransactionMessage {
    pub body_hash : String,
    pub hash : String,
    pub value : String,
    pub destination : String,
}

#[derive(Debug, Deserialize)]
pub struct TonTransaction {
    pub in_msg : TonTransactionMessage,
    pub out_msgs : Vec<TonTransactionMessage>,
}


#[derive(Debug, Deserialize)]
pub struct TonWalletInfo {
    pub balance: String,
    pub wallet: bool,
    pub seqno: Option<u64>,
    pub account_state: String,
}
#[derive(Debug, Deserialize)]
pub struct TonSendBocRetunHashResult {
    pub hash: String,
}
#[derive(Debug, Deserialize)]
pub struct TonResponse<T> {
    pub ok: bool,
    pub code: Option<u16>,
    pub error: Option<String>,
    pub message: Option<String>,
    pub result: Option<T>
}

pub async fn send_boc_to_ton(boc: String) -> Result<TonResponse<TonSendBocRetunHashResult>, String> {

    // TON Center API endpoint for sendBoc
    let url = format!("{}/sendBocReturnHash", TON_RPC_URL);

    let (idem_key,) = raw_rand().await.unwrap();

    // Prepare the HTTP request
    let request = json!({
        "boc": boc,
    });

    let proxy_request = ProxyRequest{ destination_url: url.to_string(), method: crate::types::ProxyMethod::POST, headers: vec![("X-API-Key".to_string(), TON_API_KEY.to_string()), ("Content-Type".to_string(), "application/json".to_string()) ], body: Some(request), idempotency_key: hex::encode(idem_key) };

    let json_bytes = serde_json::to_string(&proxy_request).map_err(|e| e.to_string())?;

    let header = vec![
        HttpHeader{name: "X-API-Key".to_string(), value: PROXY_API_KEY.to_string()}, 
        HttpHeader{name: "Content-Type".to_string(), value: "application/json".to_string()}
    ];

    let transform_context = TransformContext::from_name("http_transform".to_string(), vec![]);

    let http_arg = CanisterHttpRequestArgument{ url: PROXY_URL.to_string(), max_response_bytes: Some(1_000_000), method: ic_cdk::api::management_canister::http_request::HttpMethod::POST, headers: header, body: Some(json_bytes.as_bytes().to_vec()), transform: Some(transform_context) };

    // Make the HTTP outcall
    let (response,) = http_request::http_request(http_arg, 20_000_000_000)
    .await
    .map_err(|e| format!("HTTP request failed: {:?}", e))?;

    if response.status != Nat::from(200u32) {
        return Err(format!("HTTP request failed: {:?}", response.status));
    }

    // Parse the response
    let response_body = String::from_utf8(response.body)
        .map_err(|e| format!("Failed to parse response body: {:?}", e))?;

    let ton_response : TonResponse<TonSendBocRetunHashResult> = serde_json::from_str(&response_body).unwrap();
    Ok(ton_response)
}


#[query]
async fn http_transform(arg : TransformArgs) -> HttpResponse {
    let res = HttpResponse { status: arg.response.status, headers: vec![], body: arg.response.body };
    res
}

pub async fn get_ton_transactions(address: String) -> Result<TonResponse<Vec<TonTransaction>>, String> {
    // TON Center API endpoint for address balance
    let url = format!("{}/getTransactions?address={}", TON_RPC_URL, address);

    let (idem_key,) = raw_rand().await.unwrap();

    let proxy_request = ProxyRequest {
        destination_url: url,
        method: crate::types::ProxyMethod::GET,
        headers: vec![("X-API-Key".to_string(), TON_API_KEY.to_string()), ("Content-Type".to_string(), "application/json".to_string()) ],
        body: None,
        idempotency_key: hex::encode(idem_key)
    };

    let json_bytes = serde_json::to_string(&proxy_request).map_err(|e| e.to_string())?;

    let header = vec![
        HttpHeader{name: "X-API-Key".to_string(), value: PROXY_API_KEY.to_string()},
        HttpHeader{name: "Content-Type".to_string(), value: "application/json".to_string()}
    ];

    let transform_context = TransformContext::from_name("http_transform".to_string(), vec![]);

    let http_arg = CanisterHttpRequestArgument {
        url: PROXY_URL.to_string(),
        max_response_bytes: Some(1_000_000),
        method: ic_cdk::api::management_canister::http_request::HttpMethod::POST,
        headers: header,
        body: Some(json_bytes.as_bytes().to_vec()),
        transform: Some(transform_context),
    };

    // Make the HTTP outcall
    let (response,) = http_request::http_request(http_arg, 20_000_000_000)
        .await
        .map_err(|e| format!("HTTP request failed: {:?}", e))?;

    if response.status != Nat::from(200u32) {
        return Err(format!("HTTP request failed: {:?}", response.status));
    }

    // Parse the response
    let response_body = String::from_utf8(response.body)
        .map_err(|e| format!("Failed to parse response body: {:?}", e))?;


    let ton_response : TonResponse<Vec<TonTransaction>> = serde_json::from_str(&response_body).unwrap();
    Ok(ton_response)
}

pub async fn get_ton_wallet_info(address: String) -> Result<TonResponse<TonWalletInfo>, String> {
    // TON Center API endpoint for address state
    let url = format!("{}/getWalletInformation?address={}",TON_RPC_URL, address);

    let (idem_key,) = raw_rand().await.unwrap();

        let proxy_request = ProxyRequest {
        destination_url: url,
        method: crate::types::ProxyMethod::GET,
        headers: vec![("X-API-Key".to_string(), TON_API_KEY.to_string()), ("Content-Type".to_string(), "application/json".to_string())],
        body: None,
        idempotency_key: hex::encode(idem_key)
    };

    let json_bytes = serde_json::to_string(&proxy_request).map_err(|e| e.to_string())?;

    let header = vec![
        HttpHeader{name: "X-API-Key".to_string(), value: PROXY_API_KEY.to_string()},
        HttpHeader{name: "Content-Type".to_string(), value: "application/json".to_string()}
    ];

    let transform_context = TransformContext::from_name("http_transform".to_string(), vec![]);

    let http_arg = CanisterHttpRequestArgument {
        url: PROXY_URL.to_string(),
        max_response_bytes: Some(1_000_000),
        method: ic_cdk::api::management_canister::http_request::HttpMethod::POST,
        headers: header,
        body: Some(json_bytes.as_bytes().to_vec()),
        transform: Some(transform_context),
    };

    // Make the HTTP outcall
    let (response,) = http_request::http_request(http_arg, 20_000_000_000)
        .await
        .map_err(|e| format!("HTTP request failed: {:?}", e))?;

    if response.status != Nat::from(200u32) {
        return Err(format!("HTTP request failed: {:?}", response.status));
    }

    // Parse the response
    let response_body = String::from_utf8(response.body)
        .map_err(|e| format!("Failed to parse response body: {:?}", e))?;

    let ton_response : TonResponse<TonWalletInfo> = serde_json::from_str(&response_body).unwrap();
    Ok(ton_response)
}