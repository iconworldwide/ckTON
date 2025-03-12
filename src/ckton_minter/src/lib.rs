use std::{
    borrow::Cow, cell::RefCell, collections::{HashMap, VecDeque}, ops::{Deref, Div}, str::FromStr, time::Duration
};

use base64::prelude::*;
use candid::{candid_method, encode_args, Nat, Principal};
use ic_cdk::{
    api::{
        is_controller,
        management_canister::{
            http_request::{HttpResponse, TransformArgs},
            main::{
                create_canister, install_code, CanisterSettings, CreateCanisterArgument,
                InstallCodeArgument,
            },
        },
        time,
    },
    caller, id, query,
};
use ic_ton_lib::{
    cell::BagOfCells,
    message::{CommonMsgInfo, InternalMessage, TonMessage, TransferMessage},
    num_bigint::BigUint,
    wallet::TonWallet,
    TonAddress,
};
use icrc_ledger_types::icrc1::{
    account::{principal_to_subaccount, Account},
    transfer::TransferError,
};
use ledger_args::{ArchiveOptions, FeatureFlags, IndexArg, InitArg, InitArgs, LedgerArgument};
use ton_api::TonTransaction;
use types::{AdminSetup, ICTonSigner, MultiPOPVec, PendingTasks, TONDeployedWallet};
use utils::{get_ic_pub_key, get_path};

mod consts;
mod ledger_args;
mod ton_api;
mod types;
mod utils;

#[cfg(test)]
mod tests;

#[cfg(network = "local")]
const INDEXER_WASM: &[u8] = include_bytes!("../bin/index-ng.wasm");

#[cfg(network = "local")]
const LEDGER_WASM: &[u8] = include_bytes!("../bin/ledger.wasm");

thread_local! {
    static DEPLOYED_WALLET: RefCell<HashMap<Account, TONDeployedWallet>> = RefCell::new(HashMap::new());
    static ACCOUNT_BALANCES: RefCell<HashMap<Account, u64>> = RefCell::new(HashMap::new());
    static PENDING_TASKS: RefCell<MultiPOPVec<PendingTasks>> = RefCell::new(MultiPOPVec::new());
    static APP_TON_ADDRESS: RefCell<TonAddress> = RefCell::new(TonAddress::NULL);
    static CK_LEDGER_CANISTER: RefCell<Principal> = RefCell::new(Principal::anonymous());
    static CK_INDEXER_CANISTER: RefCell<Principal> = RefCell::new(Principal::anonymous());
    static CKTON_TRANSFER_FEE: RefCell<u64> = RefCell::new(1000);
    static TON_FEE: RefCell<u64> = RefCell::new(5_500u64);
}

#[ic_cdk::init]
fn init() {
    ic_cdk_timers::set_timer_interval(Duration::from_secs(5), || {
        ic_cdk::println!("Starting task for {} pending tasks", PENDING_TASKS.with_borrow(|tasks| tasks.len()));
        PENDING_TASKS.with_borrow_mut(|tasks| {

            let tasks = tasks.pop_many(Some(8));

            for task in tasks {
                if let Some(task) = task {
                    match task {
                        PendingTasks::DeployWallet(account, ton_address, retry_count) => {
                            ic_cdk::println!("Processing DeployWallet task for account: {}", account);
                            ic_cdk::spawn(async move {
                                if retry_count > 10 {
                                    return;
                                }
    
                                let ton_wallet_info = ton_api::get_ton_wallet_info(ton_address.clone())
                                    .await
                                    .unwrap();
    
                                if !ton_wallet_info.ok {
                                    PENDING_TASKS.with_borrow_mut(|tasks| {
                                        tasks.push_back(PendingTasks::DeployWallet(account, ton_address, retry_count + 1));
                                    });
                                    return;
                                }
                                let result = ton_wallet_info.result.unwrap();
    
                                if result.wallet {
                                    DEPLOYED_WALLET.with_borrow_mut(|store| {
                                        store.insert(
                                            account,
                                            TONDeployedWallet {
                                                ton_address,
                                                sequence_number: result.seqno.unwrap(),
                                            },
                                        );
                                    });
                                } else {
                                    PENDING_TASKS.with_borrow_mut(|tasks| {
                                        tasks.push_back(PendingTasks::DeployWallet(account, ton_address, retry_count + 1));
                                    });
                                }
                            });
                        }
                        PendingTasks::Mint(account, amount, hash, ton_addr, retry_count) => {
                            ic_cdk::println!("Processing Mint task for hash: {}", hash);
                            ic_cdk::spawn(async move {
                                if retry_count > 10 {
                                    return;
                                }
    
                                ic_cdk::println!("Fetching TON transactions for address: {}", ton_addr);
                                let ton_transactions =
                                    ton_api::get_ton_transactions(ton_addr.clone()).await.unwrap();
    
                                ic_cdk::println!("Received TON transactions response");
                                let tx_list = ton_transactions.result.unwrap();
                                ic_cdk::println!("Found {} transactions in the list", tx_list.len());
                                let tx = tx_list
                                    .iter()
                                    .find(|tx| tx.in_msg.hash == hash);
                                
                                if tx.is_some() {
                                    ic_cdk::println!("Found matching transaction with hash: {}", hash);
    
                                    let ledger_canister =
                                        CK_LEDGER_CANISTER.with_borrow_mut(|canister| canister.clone());
                                    ic_cdk::println!("Using ledger canister: {}", ledger_canister);
    
                                    let amount = amount - CKTON_TRANSFER_FEE.with_borrow_mut(|fee| *fee);
                                    ic_cdk::println!("Calculated transfer amount: {} (original: {})", amount, amount + CKTON_TRANSFER_FEE.with_borrow_mut(|fee| *fee));
    
                                    let arg = icrc_ledger_types::icrc1::transfer::TransferArg {
                                        from_subaccount: None,
                                        to: account,
                                        fee: None,
                                        created_at_time: None,
                                        memo: None,
                                        amount: amount.into(),
                                    };
                                    ic_cdk::println!("Calling icrc1_transfer to account owner: {}", account.owner);
                                    let (res,): (Result<Nat, TransferError>,) =
                                        ic_cdk::call(ledger_canister, "icrc1_transfer", (arg,))
                                            .await
                                            .unwrap();
                                    ic_cdk::println!("Transfer result: {:?}", res);
                                } else {
                                    PENDING_TASKS.with_borrow_mut(|tasks| {
                                        tasks.push_back(PendingTasks::Mint(account, amount, hash.clone(), ton_addr, retry_count + 1));
                                    });
                                    ic_cdk::println!("No matching transaction found for hash: {}", hash);
                                }
                            });
                        },
    
                        PendingTasks::Burn(caller, amount, hash, ton_addr, retry_count) => {
                            ic_cdk::spawn(async move {
                                if retry_count > 10 {
                                    return;
                                }
    
                                let app_ton_address = APP_TON_ADDRESS.with_borrow_mut(|address| address.clone());
    
                                let ton_response = ton_api::get_ton_transactions(get_ton_address_from_address(&app_ton_address)).await.unwrap();
    
                                if !ton_response.ok {
                                    PENDING_TASKS.with_borrow_mut(|tasks| {
                                        tasks.push_back(PendingTasks::Burn(caller, amount, hash.clone(), ton_addr, retry_count + 1));
                                    });
    
                                    return;
                                }
    
                                let tx_list = ton_response.result.unwrap();
    
                                let tx = tx_list.iter().find(|tx| tx.in_msg.hash == hash);
    
                                if tx.is_none() {
                                    PENDING_TASKS.with_borrow_mut(|tasks| {
                                        tasks.push_back(PendingTasks::Burn(caller, amount, hash.clone(), ton_addr, retry_count + 1));
                                    });
    
                                    return;
                                }
    
                                let tx = tx.unwrap();
    
                                let recipient_ton_addr : TonAddress = ton_addr.parse::<TonAddress>().unwrap();
    
                                verify_mint_transaction(&tx, &recipient_ton_addr).unwrap();
    
                                let app_purse = Account {
                                    owner: id(),
                                    subaccount: None,
                                };
    
                                let arg = icrc_ledger_types::icrc1::transfer::TransferArg {
                                    from_subaccount: Some(principal_to_subaccount(caller)),
                                    to: app_purse,
                                    fee: None,
                                    created_at_time: None,
                                    memo: None,
                                    amount: amount.into(),
                                };
    
                                let ledger_canister = CK_LEDGER_CANISTER.with_borrow_mut(|canister| canister.clone());
    
                                let (res,): (Result<Nat, TransferError>,) =
                                    ic_cdk::call(ledger_canister, "icrc1_transfer", (arg,))
                                        .await
                                        .unwrap();
    
                                if res.is_err() {
                                    PENDING_TASKS.with_borrow_mut(|tasks| {
                                        tasks.push_back(PendingTasks::Burn(caller, amount, hash.clone(), ton_addr, retry_count + 1));
                                    });
    
                                    return;
                                }
    
                                ic_cdk::println!("Burned {} CKTON", amount);
                            });
                        }
                    }
                }
            }
        });
    });
}

#[ic_cdk::post_upgrade]
fn post_upgrade() {
    init();
}

fn verify_mint_transaction(tx: &TonTransaction, dest_addr: &TonAddress) -> Result<u64, String> {
    let mssg = tx.out_msgs.iter().find(|msg| {
        let dest = msg.destination.parse::<TonAddress>();
        if dest.is_err() {
            return false;
        }
        if &dest.unwrap() == dest_addr {
            return true;
        }
        false
    });

    if mssg.is_none() {
        return Err("No mint transaction found".to_string());
    }

    let mssg = mssg.unwrap();

    let amount = mssg.value.parse::<u64>().map_err(|e| e.to_string())?;


    Ok(amount)
}

#[cfg(network = "local")]
#[ic_cdk::update]
async fn wallet_balance(ton_address: String) -> u64 {
    let ton_response = ton_api::get_ton_wallet_info(ton_address).await.unwrap();
    
    if !ton_response.ok {
        return 0;
    }

    let result = ton_response.result.unwrap();

    result.balance.parse::<u64>().unwrap()
}


#[cfg(network = "local")]
#[ic_cdk::update]
async fn manual_mint(mssg_hash: String) -> Result<u64, String> {
    let wallet = create_ton_wallet(caller(), None).await?;
    let caller_ton_address = get_ton_address_from_wallet(&wallet);

    let ton_response = ton_api::get_ton_transactions(caller_ton_address).await?;

    if !ton_response.ok {
        return Err(ton_response.error.unwrap_or("Failed to get transactions".to_string()));
    }

    let tx_list = ton_response.result.unwrap();

    let tx = tx_list.iter().find(|tx| tx.in_msg.hash == mssg_hash);

    if tx.is_none() {
        return Err("Transaction not found".to_string());
    }

    let tx = tx.unwrap();

    let app_ton_address = APP_TON_ADDRESS.with_borrow_mut(|address| address.clone());

    let amount = verify_mint_transaction(&tx, &app_ton_address)?;
    
    let ledger_canister = CK_LEDGER_CANISTER.with_borrow_mut(|canister| canister.clone());

    let arg = icrc_ledger_types::icrc1::transfer::TransferArg {
        from_subaccount: None,
        to: Account {
            owner: caller(),
            subaccount: None,
        },
        fee: None,
        created_at_time: None,
        memo: None,
        amount: amount.into(),
    };

    let (res,): (Result<Nat, TransferError>,) =
        ic_cdk::call(ledger_canister, "icrc1_transfer", (arg,))
            .await
            .unwrap();

    if res.is_err() {
        return Err(format!("Transfer failed: {:?}", res.err().unwrap()));
    }

    let block = res.unwrap().0.try_into().unwrap();

    Ok(block)
}

#[ic_cdk::update]
async fn generate_ton_address(owner: Option<Principal>, subaccount: Option<[u8; 32]>) -> String {
    let path = get_path(owner, subaccount);

    let pubkey = get_ic_pub_key(path.clone()).await.unwrap();

    let ton_signer = ICTonSigner::new(pubkey, path);

    let wallet =
        TonWallet::derive_default(ic_ton_lib::wallet::WalletVersion::V4R2, &ton_signer).unwrap();


    get_ton_address_from_wallet(&wallet)
}


#[ic_cdk::query(guard = is_authenticated)]
fn get_ton_wallet_address(
    owner: Option<Principal>,
    subaccount: Option<[u8; 32]>,
) -> Option<String> {
    let acc = Account {
        owner: owner.unwrap_or_else(caller),
        subaccount,
    };

    DEPLOYED_WALLET.with_borrow(|store| store.get(&acc).map(|wallet| wallet.ton_address.clone()))
}

#[ic_cdk::query(guard = is_authenticated)]
async fn get_deposit_address(owner: Option<Principal>) -> String {
    let acc = Account {
        owner: id(),
        subaccount: Some(principal_to_subaccount(owner.unwrap_or(caller()))),
    };

    acc.to_string()
}

#[ic_cdk::update(guard = is_authenticated)]
async fn deploy_ton_wallet(subaccount: Option<[u8; 32]>, expire : Option<u32>) -> Result<String, String> {
   _deploy_wallet(caller(), subaccount, expire).await
}

async fn _deploy_wallet(owner: Principal, subaccount: Option<[u8; 32]>, expire : Option<u32>) -> Result<String, String> {
    let acc = Account {
        owner,
        subaccount,
    };

    let is_deployed = DEPLOYED_WALLET.with_borrow(|store| store.contains_key(&acc));

    if is_deployed {
        return Err("Wallet already deployed".to_string());
    };

    let wallet = create_ton_wallet(acc.owner, subaccount).await?;

    // removed it because i don't want to make cycles calling TON rpc to check wallet state

    // let ton_response = ton_api::get_ton_wallet_info(get_ton_address(&wallet)).await.unwrap();

    // if !ton_response.ok {
    //     return Err(ton_response.error.unwrap_or("Failed to get wallet info".to_string()));
    // }

    // if ton_response.result.unwrap().wallet {
    //     return Err("Wallet already deployed".to_string());
    // }

    // let internal_mssg = wallet
    //     .create_internal_message()
    //     .map_err(|s| s.to_string())?;

    let expire_at = expire.unwrap_or(nanos_to_seconds(time()) + 60);

    let external_mssg = wallet
        .create_external_body(
            expire_at,
            0,
            vec![],
        )
        .map_err(|s| s.to_string())?;

    let signed = wallet
        .sign_external_body(&external_mssg)
        .await
        .map_err(|s| s.to_string())?;

    let wrapped = wallet
        .wrap_signed_body(signed, true)
        .map_err(|s| s.to_string())?;

    let boc = BagOfCells::from_root(wrapped);

    let tx = boc.serialize(true).map_err(|s| s.to_string())?;

    let enc = BASE64_STANDARD.encode(tx);

    let ton_response = ton_api::send_boc_to_ton(enc).await?;

    if !ton_response.ok {
        return Err(ton_response.error.unwrap_or("Unknown error".to_string()));
    }

    PENDING_TASKS.with_borrow_mut(|tasks| {
        tasks.push_back(PendingTasks::DeployWallet(
            acc,
            get_ton_address_from_wallet(&wallet),
            0,
        ));
    });

    Ok(ton_response.result.unwrap().hash)
    
}

#[ic_cdk::update(guard = is_authenticated)]
async fn destroy_ton_wallet(to_ton_address: String, subaccount: Option<[u8; 32]>, expire : Option<u32>) -> Result<String, String> {
    let acc = Account {
        owner: caller(),
        subaccount,
    };

    #[cfg(network = "ic")]
    let is_deployed = DEPLOYED_WALLET.with_borrow(|store| store.contains_key(&acc));

    #[cfg(network = "ic")]
    if !is_deployed {
        return Err("Wallet not deployed".to_string());
    }

    let wallet = create_ton_wallet(acc.owner, subaccount).await?;

    let expire_at = expire.unwrap_or(nanos_to_seconds(time()) + 60);

    let ton_response = ton_api::get_ton_wallet_info(get_ton_address_from_wallet(&wallet)).await?;

    if !ton_response.ok {
        return Err(ton_response.error.unwrap_or("Failed to get wallet info".to_string()));
    }
    
    let seqno = ton_response.result.unwrap().seqno.unwrap();

    let dest : TonAddress = to_ton_address.parse::<TonAddress>().map_err(|s| s.to_string())?;

    let common_message = CommonMsgInfo::InternalMessage(InternalMessage {
        ihr_disabled: false,
        bounce: false,
        bounced: false,
        src: TonAddress::NULL,
        dest,
        value: BigUint::ZERO,
        ihr_fee: BigUint::ZERO,
        fwd_fee: BigUint::ZERO,
        created_lt: 0,
        created_at: 0,
    });

    let transfer_message = TransferMessage::new(common_message)
        .build()
        .map_err(|s| s.to_string())?;


    let external_mssg = wallet
        .create_external_body_with_mode(expire_at, seqno.try_into().unwrap(), vec![(160, transfer_message.to_arc())])
        .map_err(|s| s.to_string())?;

    let signed = wallet
        .sign_external_body(&external_mssg)
        .await
        .map_err(|s| s.to_string())?;

    let wrapped = wallet
        .wrap_signed_body(signed, false)
        .map_err(|s| s.to_string())?;

    let boc = BagOfCells::from_root(wrapped);
    let enc = BASE64_STANDARD.encode(boc.serialize(true).map_err(|s| s.to_string())?);

    let ton_response = ton_api::send_boc_to_ton(enc).await?;

    if !ton_response.ok {
        return Err(ton_response.error.unwrap_or("Unknown error".to_string()));
    }

    Ok(ton_response.result.unwrap().hash)
}



fn nanos_to_seconds(nanos: u64) -> u32 {
    (nanos / 1_000_000_000).try_into().unwrap()
}

fn get_ton_address_from_wallet(wallet: &TonWallet<ICTonSigner>) -> String {
    #[cfg(app_env = "dev")]
    let address = wallet.address.to_base64_url_flags(true, true);

    #[cfg(app_env = "prod")]
    let address = wallet.address.to_base64_url();

    address
}

fn get_ton_address_from_address(address: &TonAddress) -> String {
    #[cfg(app_env = "dev")]
    let address = address.to_base64_url_flags(true, true);

    #[cfg(app_env = "prod")]
    let address = wallet.address.to_base64_url();

    address
}

#[ic_cdk::update(guard = is_authenticated)]
async fn withdraw_native(to_ton_address: String, amount: u64) -> Result<(String, u64), String> {
   
    let ledger_canister = CK_LEDGER_CANISTER.with_borrow_mut(|canister| canister.clone());

    let app_ton_address = APP_TON_ADDRESS.with_borrow_mut(|address| address.clone());

    let ton_response = ton_api::get_ton_wallet_info(app_ton_address.to_base64_url())
        .await
        .unwrap();

    if !ton_response.ok {
        return Err(ton_response.error.unwrap_or("Failed to get wallet info".to_string()));
    }

    let result = ton_response.result.unwrap();

    if !result.wallet {
        return Err("Wallet not deployed".to_string());
    }

    // Check if the caller has enough balance in the ICRC ledger
    let caller_account = Account {
        owner: id(),
        subaccount: Some(principal_to_subaccount(caller())),
    };
    
    let (caller_balance,): (Nat,) = ic_cdk::call(ledger_canister, "icrc1_balance_of", (caller_account,))
        .await
        .map_err(|e| format!("Failed to check balance: {:?}", e))?;
    
    let caller_balance_u64: u64 = caller_balance.0.try_into().map_err(|_| "Balance too large to convert to u64".to_string())?;

    let amount_to_burn = amount - CKTON_TRANSFER_FEE.with_borrow_mut(|fee| *fee);
    
    if caller_balance_u64 < amount {
        return Err(format!("Insufficient balance: {} < {}", caller_balance_u64, amount));
    }

    let ton_fee = TON_FEE.with_borrow_mut(|fee| *fee);

    let amount_to_send = amount_to_burn - ton_fee;

    let ton_wallet = create_ton_wallet(id(), None).await?;


    let dest : TonAddress = to_ton_address.parse::<TonAddress>().map_err(|s| s.to_string())?;

    let common_message = CommonMsgInfo::InternalMessage(InternalMessage {
        ihr_disabled: false,
        bounce: false,
        bounced: false,
        src: TonAddress::NULL,
        dest,
        value: BigUint::from(amount_to_send),
        ihr_fee: BigUint::ZERO,
        fwd_fee: BigUint::ZERO,
        created_lt: 0,
        created_at: 0,
    });


    let expire_at = nanos_to_seconds(time()) + 60;

    let transfer_message = TransferMessage::new(common_message)
        .build()
        .map_err(|s| s.to_string())?;

    let seqno = result.seqno.unwrap();
    let seqno_u32 = u32::try_from(seqno)
        .map_err(|_| "Sequence number too large to convert to u32".to_string())?;

    let internal_mssg = ton_wallet
        .create_external_body(
            expire_at,
            seqno_u32,
            vec![transfer_message.to_arc()],
        )
        .map_err(|s| s.to_string())?;

    let signed = ton_wallet
        .sign_external_body(&internal_mssg)
        .await
        .map_err(|s| s.to_string())?;

    let wrapped = ton_wallet
        .wrap_signed_body(signed, false)
        .map_err(|s| s.to_string())?;

    let boc = BagOfCells::from_root(wrapped);

    let enc = BASE64_STANDARD.encode(boc.serialize(true).map_err(|s| s.to_string())?);

    let result = ton_api::send_boc_to_ton(enc).await?;

    let hash = result.result.unwrap().hash;

    PENDING_TASKS.with_borrow_mut(|tasks| {
        tasks.push_back(PendingTasks::Burn(
            caller(),
            amount_to_burn,
            hash.clone(),
            to_ton_address,
            0,
        ));
    });

    Ok((hash, amount_to_send))
}

async fn create_ton_wallet(
    owner: Principal,
    subaccount: Option<[u8; 32]>,
) -> Result<TonWallet<ICTonSigner>, String> {
    let path = get_path(Some(owner), subaccount);

    let pubkey = get_ic_pub_key(path.clone()).await.unwrap();

    let ton_signer = ICTonSigner::new(pubkey, path);

    let wallet =
        TonWallet::derive_default(ic_ton_lib::wallet::WalletVersion::V4R2, &ton_signer).unwrap();

    Ok(wallet)
}

#[ic_cdk::update(guard = is_authenticated)]
async fn mint(
    to_account: String,
    amount: u64,
    subaccount: Option<[u8; 32]>, expire : Option<u32>
) -> Result<String, String> {
    let caller_acc = Account {
        owner: caller(),
        subaccount,
    };

    let to_account =  Account::from_str(&to_account).map_err(|op| op.to_string())?;

    #[cfg(network = "ic")]
    let is_deployed = DEPLOYED_WALLET.with_borrow(|store| {
        store
            .get(&caller_acc)
            .map(|wallet| wallet.ton_address.clone())
    });

    #[cfg(network = "ic")]
    if is_deployed.is_none() {
        return Err("Wallet not deployed".to_string());
    };

    let wallet = create_ton_wallet(caller_acc.owner, subaccount).await?;

    #[cfg(network = "ic")]
    let ton_address = is_deployed.unwrap();

    #[cfg(network = "local")]
    let ton_address = get_ton_address_from_wallet(&wallet);

    let ton_response = ton_api::get_ton_wallet_info(ton_address.clone())
        .await
        .unwrap();

    if !ton_response.ok {
        return Err(ton_response.error.unwrap_or("Failed to get wallet info".to_string()));
    }

    let result = ton_response.result.unwrap();

    if !result.wallet {
        return Err("Wallet not deployed".to_string());
    }

    let balance = result.balance;
    let balance_int = balance.parse::<u64>().unwrap();
    if balance_int < amount {
        return Err("Insufficient balance".to_string());
    }

    let seqno = result.seqno.unwrap();
    let seqno_u32 = u32::try_from(seqno)
        .map_err(|_| "Sequence number too large to convert to u32".to_string())?;

    let expire_at = expire.unwrap_or(nanos_to_seconds(time()) + 60);

    let app_ton_address = APP_TON_ADDRESS.with_borrow_mut(|address| {
        address.clone()
    });

    let common_message = CommonMsgInfo::InternalMessage(InternalMessage {
        ihr_disabled: false,
        bounce: false,
        bounced: false,
        src: TonAddress::NULL,
        dest: app_ton_address,
        value: BigUint::from(amount),
        ihr_fee: BigUint::ZERO,
        fwd_fee: BigUint::ZERO,
        created_lt: 0,
        created_at: 0,
    });


    let transfer_message = TransferMessage::new(common_message)
        .build()
        .map_err(|s| s.to_string())?;

    let external_mssg = wallet
        .create_external_body(
            expire_at,
            seqno_u32,
            vec![transfer_message.to_arc()],
        )
        .map_err(|s| s.to_string())?;

    let signed = wallet
        .sign_external_body(&external_mssg)
        .await
        .map_err(|s| s.to_string())?;

    let wrapped = wallet
        .wrap_signed_body(signed, false)
        .map_err(|s| s.to_string())?;

    let boc = BagOfCells::from_root(wrapped);

    let enc = BASE64_STANDARD.encode(boc.serialize(true).map_err(|s| s.to_string())?);

    let ton_response = ton_api::send_boc_to_ton(enc).await?;

    if !ton_response.ok {
        return Err(ton_response.error.unwrap_or("Failed to send boc to ton".to_string()));
    }

    let result = ton_response.result.unwrap();

    PENDING_TASKS.with_borrow_mut(|tasks| {
        tasks.push_back(PendingTasks::Mint(
            to_account,
            amount,
            result.hash.clone(),
            ton_address,
            0,
        ));
    });

    Ok(result.hash)
}

#[ic_cdk::query]
async fn ledger_id() -> String {
    let ledger_canister = CK_LEDGER_CANISTER.with_borrow_mut(|canister| canister.clone());

    ledger_canister.to_string()
}

#[ic_cdk::query]
async fn minter_ton_address() -> String {
    let app_ton_address = APP_TON_ADDRESS.with_borrow_mut(|address| address.clone());

    app_ton_address.to_string()
}

#[ic_cdk::query]
async fn wallet_deployed(account: Account) -> bool {
    DEPLOYED_WALLET.with_borrow(|store| store.contains_key(&account))
}

#[ic_cdk::query]
async fn wallet_count() -> u64 {
    DEPLOYED_WALLET.with_borrow(|store| store.len() as u64)
}

#[cfg(network = "ic")]
#[ic_cdk::update(guard = is_mint_controller)]
async fn admin_setup(setup_args: AdminSetup) -> Result<(), String> {
    let (ledger_record, indexer_record, ckton_transfer_fee, ton_fee) = (setup_args.ledger_canister, setup_args.indexer_canister, setup_args.ckton_transfer_fee, setup_args.ton_fee);

    CK_LEDGER_CANISTER.set(ledger_record);
    CK_INDEXER_CANISTER.set(indexer_record);

    if let Some(ckton_transfer_fee) = ckton_transfer_fee {
        CKTON_TRANSFER_FEE.set(ckton_transfer_fee);
    }

    if let Some(ton_fee) = ton_fee {
        TON_FEE.set(ton_fee);
    }

    let wallet = create_ton_wallet(id(), None).await?;

    APP_TON_ADDRESS.set(wallet.address);

    // deploy wallet
    _deploy_wallet(id(), None, None).await?;

    Ok(())
}

#[cfg(network = "local")]
#[ic_cdk::update(guard = is_mint_controller)]
async fn admin_setup(setup_args: Option<AdminSetup>) -> Result<(), String> {

    let (ledger_record, indexer_record, ckton_transfer_fee, ton_fee) = match setup_args {
        Some(setup) => {
           (setup.ledger_canister, setup.indexer_canister, setup.ckton_transfer_fee, setup.ton_fee)
        },
        None => {
            let args = CreateCanisterArgument {
                settings: Some(CanisterSettings {
                    controllers: Some(vec![id(), caller()]),
                    compute_allocation: None,
                    memory_allocation: None,
                    freezing_threshold: None,
                    reserved_cycles_limit: None,
                    log_visibility: None,
                    wasm_memory_limit: None,
                }),
            };
            let (ledger_record,) = create_canister(args.clone(), 20_000_000_000_000u128)
                .await
                .map_err(|s| s.1)?;
        
            let (indexer_record,) = create_canister(args, 20_000_000_000_000u128)
                .await
                .map_err(|s| s.1)?;
        
            let archive_options = ArchiveOptions {
                num_blocks_to_archive: 1000,
                max_transactions_per_response: None,
                trigger_threshold: 2000,
                more_controller_ids: None,
                max_message_size_bytes: None,
                cycles_for_archive_creation: None,
                node_max_memory_size_bytes: None,
                controller_id: id(),
            };
        
            let ledger_init_args = InitArgs {
                decimals: Some(9),
                token_symbol: "ckTON".to_string(),
                transfer_fee: CKTON_TRANSFER_FEE.with_borrow_mut(|fee| *fee).into(),
                metadata: vec![],
                minting_account: Account {
                    owner: id(),
                    subaccount: None,
                },
                initial_balances: vec![],
                fee_collector_account: Some(Account {
                    owner: id(),
                    subaccount: None,
                }),
                archive_options,
                max_memo_length: None,
                token_name: "ckTON".to_string(),
                feature_flags: Some(FeatureFlags { icrc2: true }),
            };
        
            install_code(InstallCodeArgument {
                canister_id: ledger_record.canister_id,
                wasm_module: LEDGER_WASM.to_vec(),
                arg: encode_args((LedgerArgument::Init(ledger_init_args),)).map_err(|s| s.to_string())?,
                mode: ic_cdk::api::management_canister::main::CanisterInstallMode::Install,
            })
            .await
            .unwrap();
        
            let index_args = InitArg {
                ledger_id: ledger_record.canister_id,
                retrieve_blocks_from_ledger_interval_seconds: None,
            };
        
            install_code(InstallCodeArgument {
                mode: ic_cdk::api::management_canister::main::CanisterInstallMode::Install,
                canister_id: indexer_record.canister_id,
                wasm_module: INDEXER_WASM.to_vec(),
                arg: encode_args((IndexArg::Init(index_args),)).map_err(|s| s.to_string())?,
            })
            .await
            .unwrap();

            (ledger_record.canister_id, indexer_record.canister_id, None, None)
        },
    };
    

    CK_LEDGER_CANISTER.set(ledger_record);
    CK_INDEXER_CANISTER.set(indexer_record);
    
    if let Some(ckton_transfer_fee) = ckton_transfer_fee {
        CKTON_TRANSFER_FEE.set(ckton_transfer_fee);
    }

    if let Some(ton_fee) = ton_fee {
        TON_FEE.set(ton_fee);
    }

    let wallet = create_ton_wallet(id(), None).await?;

    APP_TON_ADDRESS.set(wallet.address);

    // deploy wallet
    _deploy_wallet(id(), None, None).await?;

    Ok(())
}

fn is_authenticated() -> Result<(), String> {
    let caller = caller();
    if caller == Principal::anonymous() {
        return Err("Unauthorized".to_string());
    }
    Ok(())
}

fn is_mint_controller() -> Result<(), String> {
    if !is_controller(&caller()) {
        return Err("Unauthorized".to_string());
    }
    Ok(())
}

#[query]
#[candid_method(query)]
fn export_candid() -> String {
    ic_cdk::export_candid!();
    __export_service()
}
