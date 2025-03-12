use std::{env, path::Path};

use dotenv::from_path;
fn main() {
    // Specify the path to your .env file
    let env_path = Path::new("../../.env");
    from_path(env_path).expect("There's no env File in the root");
    println!("cargo::rustc-check-cfg=cfg(network, values(\"ic\", \"local\"))");
    println!("cargo::rustc-check-cfg=cfg(app_env, values(\"dev\", \"prod\"))");
    let network = env::var("DFX_NETWORK");
    let app_env = env::var("APP_ENV");
    let ton_api_key = env::var("TON_API_KEY").expect("There's no TON_API_KEY in the env");
    let ton_rpc_url = env::var("TON_RPC_URL").expect("There's no TON_RPC_URL in the env");
    let proxy_url = env::var("PROXY_URL").expect("There's no PROXY_URL in the env");

    // Debug prints - these will show up in the build output
    println!("Debug - TON_API_KEY: {:?}", ton_api_key);
    println!("Debug - TON_API_URL: {:?}", ton_rpc_url);

    println!("cargo:rustc-env=TON_API_KEY={}", ton_api_key);
    println!("cargo:rustc-env=TON_RPC_URL={}", ton_rpc_url);
    println!("cargo:rustc-env=PROXY_URL={}", proxy_url);
    // // if ton_api_key.is_ok() {
    // //     let key = ton_api_key.unwrap();
    // //     println!("cargo:rustc-env=TON_API_KEY={}", key)
    // // }
    // if ton_api_url.is_ok() {
    //     let url = ton_api_url.unwrap();
    //     println!("cargo:rustc-env=TON_API_URL={}", url)
    // }

    if app_env.is_err() {
        println!("cargo:rustc-cfg=app_env=\"dev\"");
    } else {
        match app_env.unwrap().as_str() {
            "dev" => {
                println!("cargo:rustc-cfg=app_env=\"dev\"");
            }
            "prod" => {
                println!("cargo:rustc-cfg=app_env=\"prod\"");
            }
            _ => {
                println!("cargo:rustc-cfg=app_env=\"dev\"");
            }
        }
    }

    if network.is_err() {
        println!("cargo:rustc-cfg=network=\"local\"");
    } else {
        match network.unwrap().as_str() {
            "local" => {
                println!("cargo:rustc-cfg=network=\"local\"")
            }
            "ic" => {
                println!("cargo:rustc-cfg=network=\"ic\"")
            }
            _ => {
                println!("cargo:rustc-cfg=network=\"local\"")
            }
        }
    }
}
