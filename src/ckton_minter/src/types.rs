use std::{
    collections::VecDeque,
    ops::{Deref, DerefMut},
    sync::Arc,
};

use candid::{CandidType, Principal};
use ic_cdk::api::management_canister::schnorr::{self, SchnorrKeyId, SignWithSchnorrArgument};
use ic_ton_lib::types::ICSigner;
use icrc_ledger_types::icrc1::account::Account;
use serde::{Deserialize, Serialize};

use crate::consts::SCHNORR_KEY_NAME;

impl<T> Deref for MultiPOPVec<T> {
    type Target = VecDeque<T>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl<T> DerefMut for MultiPOPVec<T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

impl<T> MultiPOPVec<T> {
    pub fn pop_many(&mut self, count: Option<u32>) -> Vec<Option<T>> {
        let count = count.unwrap_or(1);
        (0..count).map(|_| self.0.pop_front()).collect()
    }
}
pub struct MultiPOPVec<T>(VecDeque<T>);

impl<T> MultiPOPVec<T> {
    pub fn new() -> Self {
        Self(VecDeque::new())
    }
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub enum ProxyMethod {
    GET,
    POST,
}
// Request model that includes the idempotency key
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ProxyRequest {
    pub idempotency_key: String,
    pub destination_url: String,
    pub method: ProxyMethod,
    pub headers: Vec<(String, String)>,
    pub body: Option<serde_json::Value>,
}

pub struct TONDeployedWallet {
    pub ton_address: String,
    pub sequence_number: u64,
}

pub enum PendingTasks {
    DeployWallet(Account, String, u32),
    Mint(Account, u64, String, String, u32),
    Burn(Principal, u64, String, String, u32),
}

#[derive(Clone)]
pub struct ICTonSigner {
    pub_key: Vec<u8>,
    path: Vec<Vec<u8>>,
}

impl ICTonSigner {
    pub fn new(pk: Vec<u8>, path: Vec<Vec<u8>>) -> Self {
        Self { pub_key: pk, path }
    }
}

impl ICSigner for ICTonSigner {
    fn public_key(&self) -> &[u8] {
        &self.pub_key
    }

    async fn sign(&self, mssg: &[u8]) -> Result<Vec<u8>, String> {
        let arg = SignWithSchnorrArgument {
            message: mssg.to_vec(),
            derivation_path: self.path.clone(),
            key_id: SchnorrKeyId {
                algorithm: schnorr::SchnorrAlgorithm::Ed25519,
                name: SCHNORR_KEY_NAME.to_string(),
            },
        };
        let (sign_result,) = schnorr::sign_with_schnorr(arg).await.map_err(|err| err.1)?;

        Ok(sign_result.signature)
    }
}


#[derive(Debug, CandidType, Deserialize, Serialize)]
pub struct AdminSetup {
    pub ledger_canister: Principal,
    pub indexer_canister: Principal,
    pub ckton_transfer_fee: Option<u64>,
    pub ton_fee: Option<u64>,
}
