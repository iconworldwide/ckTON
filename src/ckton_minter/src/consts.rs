// pub const TEST_NET_TON_RPC_API_KEY : &str = "0d1616aea9f9fa3f0b155a1c20d61bbdca6ef393b35814bbd2e438727df799d0";
// pub const TON_API_KEY : &str = "0d1616aea9f9fa3f0b155a1c20d61bbdca6ef393b35814bbd2e438727df799d0";
pub const TON_API_KEY : &str = env!("TON_API_KEY");
pub const TON_RPC_URL : &str = env!("TON_RPC_URL");
pub const PROXY_URL : &str = "http://localhost:3000/proxy";
pub const PROXY_API_KEY : &str = "";

#[cfg(network = "local")]
pub const SCHNORR_KEY_NAME : &str = "dfx_test_key";

#[cfg(app_env = "prod")]
#[cfg(network = "ic")]
pub const SCHNORR_KEY_NAME : &str = "key_1";

#[cfg(app_env = "dev")]
#[cfg(network = "ic")]
pub const SCHNORR_KEY_NAME : &str = "test_key_1";