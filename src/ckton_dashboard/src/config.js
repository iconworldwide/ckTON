export const CKTON_MINTER_CANISTER_ID = process.env.CANISTER_ID_CKTON_MINTER; //rocess.env.NODE_ENV === 'development' ? 'bd3sg-teaaa-aaaaa-qaaba-cai' : 'dvqed-mqaaa-aaaam-aegmq-cai';
export const CKTON_DASHBOARD_CANISTER_ID = process.env.CANISTER_ID_CKTON_DASHBOARD; //process.env.NODE_ENV === 'development' ? 'rrkah-fqaaa-aaaaa-aaaaq-cai' : 'dsrcx-biaaa-aaaam-aegma-cai';
export const II_URL = process.env.NODE_ENV === 'development' ? " http://be2us-64aaa-aaaaa-qaabq-cai.localhost:4943" : 'https://identity.ic0.app';
// 'mainnet' or 'testnet'
export const IC_HOST = process.env.NODE_ENV === 'development' ? "http://127.0.0.1:4943" : 'https://icp0.io';
export const CKTON_LEDGER_CANISTER_ID = process.env.CANISTER_ID_ICRC1_LEDGER_CANISTER; //process.env.NODE_ENV === 'development' ? 'c2lt4-zmaaa-aaaaa-qaaiq-cai' : 'd3sjl-xaaaa-aaaam-aegnq-cai';
export const CKTON_INDEXER_CANISTER_ID = process.env.CANISTER_ID_ICRC1_INDEX_CANISTER; //process.env.NODE_ENV === 'development' ? 'c2lt4-zmaaa-aaaaa-qaaiq-cai' : 'd3sjl-xaaaa-aaaam-aegnq-cai';