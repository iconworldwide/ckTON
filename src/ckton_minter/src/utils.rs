use candid::Principal;
use ic_cdk::{api::management_canister::schnorr::{self, SchnorrKeyId, SchnorrPublicKeyArgument}, caller};

use crate::consts::SCHNORR_KEY_NAME;

pub async fn get_ic_pub_key(path: Vec<Vec<u8>>) -> Result<Vec<u8>, String> {
    let (resp,) = schnorr::schnorr_public_key(SchnorrPublicKeyArgument {
        canister_id: None,
        derivation_path: path,
        key_id: SchnorrKeyId {
            algorithm: schnorr::SchnorrAlgorithm::Ed25519,
            name: SCHNORR_KEY_NAME.to_string(),
        },
    })
    .await
    .map_err(|e| format!("reason: {}", e.1))?;
    Ok(resp.public_key)
}

pub fn get_path(owner: Option<Principal>, subaccount: Option<[u8; 32]>) -> Vec<Vec<u8>> {
    let owner = if owner.is_none() {
        caller()
    } else {
        owner.unwrap()
    };
    let mut vec = owner.to_text().as_bytes().to_vec();
    if subaccount.is_some() {
        let sub = subaccount.unwrap();
        vec.extend(sub);
    }

    return vec![vec];
}
