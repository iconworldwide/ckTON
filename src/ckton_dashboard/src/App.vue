<script setup>
import { ref, onMounted } from 'vue';
import { ckton_dashboard } from 'declarations/ckton_dashboard/index';
import { createActor as createCktonMinter } from 'declarations/ckton_minter/index';
import { CKTON_DASHBOARD_CANISTER_ID, CKTON_LEDGER_CANISTER_ID, CKTON_MINTER_CANISTER_ID, IC_HOST, II_URL } from './config';
import { AuthClient } from '@dfinity/auth-client';
import { decodeIcrcAccount, IcrcLedgerCanister } from "@dfinity/ledger-icrc"
import { Principal } from '@dfinity/principal';
import { createAgent } from '@dfinity/utils';
import TonWeb from 'tonweb';




// State variables
const network = ref('testnet'); // 'mainnet' or 'testnet'
const ckTonBalance = ref(0);
const tonBalance = ref(0);
const tonAddress = ref('');
const icAccountAddress = ref('');
const mintAmount = ref('');
const withdrawAmount = ref('');
const withdrawAddress = ref('');
const isLoading = ref(false);
const notification = ref({ show: false, message: '', type: 'info' });
const isAuthenticated = ref(false);
const receiveAddress = ref('');
const isWalletDeployed = ref(false); // New state variable to track if wallet is deployed

// Switch network
function switchNetwork() {
  network.value = network.value === 'mainnet' ? 'testnet' : 'mainnet';
  fetchBalances();
}

async function getIdentity() {
  let authClient = await AuthClient.create();
  if (await authClient.isAuthenticated()) {
    return authClient.getIdentity();
  }
  return null;
}

// Connect to Internet Identity
async function connectToII() {
  if (isAuthenticated.value) {
    let authClient = await AuthClient.create();
    await authClient.logout();
    isAuthenticated.value = false;
    showNotification('Successfully disconnected from Internet Identity', 'success');
    return;
  }
  let authClient = await AuthClient.create();
  await authClient.login({
    identityProvider: II_URL || 'https://identity.ic0.app',
    onSuccess: () => {
      showNotification('Successfully connected to Internet Identity', 'success');
      isAuthenticated.value = true;
    }
  });
}

async function get_ckton_minter() {
  let identity = await getIdentity()
  if (!identity) {
    showNotification('Please connect to Internet Identity', 'error');
    return null;
  }

  return createCktonMinter(CKTON_MINTER_CANISTER_ID, {
    agentOptions: {
      host: IC_HOST,
      identity
    }
  })


}

// Generate/get TON address
async function generateTonAddress() {
  let identity = await getIdentity();
  if (!identity) {
    showNotification('Please connect to Internet Identity', 'error');
    return;
  }
  try {
    showLoading('Generating TON address...');
    let cktonMinter = await get_ckton_minter();
    if (!cktonMinter) return;

    let address = await cktonMinter.generate_ton_address([], []);
    if (!address) {
      address = await cktonMinter.generate_ton_address()
    }

    tonAddress.value = address;
    showNotification('TON address generated successfully', 'success');
  } catch (error) {
    showNotification('Failed to generate TON address: ' + error.message, 'error');
  } 
}

// Get IC account address
async function getIcAccountAddress() {
  try {
    showLoading('Retrieving IC account address...');
    const minter = await get_ckton_minter();
    if (!minter) return
    let address = await minter.get_deposit_address([])
    icAccountAddress.value = address;
    showNotification('IC account address retrieved', 'success');
  } catch (error) {
    showNotification('Failed to get IC account address: ' + error.message, 'error');
  }
}

// Fetch balances
async function fetchBalances() {
  try {
    showLoading('Fetching balances...');

    let identity = await getIdentity();
    if (!identity) return;

    console.log(identity)
    let agent = await createAgent({
      host: IC_HOST,
      identity,
      fetchRootKey: true,
    })

    let account = decodeIcrcAccount(icAccountAddress.value);

    let ledger = IcrcLedgerCanister.create({
      canisterId: Principal.fromText(CKTON_LEDGER_CANISTER_ID),
      agent
    })

    let balance = await ledger.balance(account)
    // let ckTon = Number(balance) / 10 ** 9;
    ckTonBalance.value = balance;

    if (tonAddress.value) {
      let provider = new TonWeb.HttpProvider("https://testnet.toncenter.com/api/v2/jsonRPC");
      let tonweb = new TonWeb(provider);
      
      // Check if the wallet is deployed
      try {
        let walletInfo = await tonweb.provider.getAddressInfo(tonAddress.value);
        isWalletDeployed.value = walletInfo.state === "active";
      } catch (error) {
        console.error("Error checking wallet state:", error);
        isWalletDeployed.value = false;
      }
      
      let wallet_balance = await tonweb.getBalance(tonAddress.value);
      tonBalance.value = wallet_balance;
    }
  } catch (error) {
    showNotification('Failed to fetch balances: ' + error.message, 'error');
  } 
}

// Mint ckTon
async function mintCkTon() {

  if (!mintAmount.value || parseFloat(mintAmount.value) <= 0) {
    showNotification('Please enter a valid amount', 'error');
    return;
  }

  if (!receiveAddress.value) {
    showNotification('Please enter a receive address', 'error');
    return;
  }

  let minter = await get_ckton_minter();
  // BigInt()
  // let amount = parseFloat(mintAmount.value)
  let amount_ckton = BigInt(mintAmount.value)

  if (!minter) return;

  try {
    showLoading('Minting ckTON...');
    let result = await minter.mint(receiveAddress.value, amount_ckton, [], [])

    if (result.Ok) {
      showNotification(`Successfully minted with hash ${result.Ok}`, 'success');
      mintAmount.value = '';
      receiveAddress.value = '';
      // fetchBalances();
    } else {
      showNotification(`Failed to mint ckTON: ${result.Err}`, 'error');
    }
  } catch (error) {
    showNotification('Failed to mint ckTON: ' + error.message, 'error');
  }
}

// Withdraw ckTon to native TON
async function withdrawCkTon() {
  
  if (!withdrawAmount.value || parseFloat(withdrawAmount.value) <= 0) {
    showNotification('Please enter a valid amount', 'error');
    return;
  }

  if (!withdrawAddress.value) {
    showNotification('Please enter a withdrawal address', 'error');
    return;
  }

  let minter = await get_ckton_minter();
  if (!minter) return;

  // let amount = parseFloat(withdrawAmount.value)
  let amount_ckton = BigInt(withdrawAmount.value)

  console.log(amount_ckton)

  try {
    showLoading('Processing withdrawal...');
    // Replace with actual canister call
    let result = await minter.withdraw_native(withdrawAddress.value, amount_ckton)

    if (result.Ok) {
      showNotification(`Withdrawal with hash ${result.Ok[0]} is processing, please wait...`, 'success');
      withdrawAmount.value = '';
      withdrawAddress.value = '';
      // fetchBalances();
    } else {
      showNotification(`Failed to withdraw ckTON: ${result.Err}`, 'error');
    }
  } catch (error) {
    showNotification('Failed to withdraw ckTON: ' + error.message, 'error');
  }
}

// Show notification
function showNotification(message, type = 'info') {
  notification.value = {
    show: true,
    message,
    type
  };

  // Only auto-hide non-loading notifications
  if (type !== 'loading') {
    setTimeout(() => {
      notification.value.show = false;
    }, 5000);
  }
}

// Show loading notification
function showLoading(message = 'Processing...') {
  notification.value = {
    show: true,
    message,
    type: 'loading'
  };
}

// Hide loading notification
function hideLoading() {
  notification.value.show = false;
}

async function load_auth() {
  let authClient = await AuthClient.create();
  if (await authClient.isAuthenticated()) {
    isAuthenticated.value = true;
  }
}

// Initialize
onMounted(async () => {
  await load_auth()
  if (isAuthenticated.value) {
    await generateAllAddresses()
  }
});

// New function to deploy TON wallet
async function deployTonWallet() {
  if (!tonAddress.value) {
    showNotification('No TON address available', 'error');
    return;
  }
  // Check if TON balance is 0
  if (tonBalance.value === 0) {
    showNotification('You need to have TON in your wallet to deploy it', 'error');
    return;
  }
  
  try {
    showLoading('Deploying wallet...');
    
    let minter = await get_ckton_minter();
    if (!minter) return;
    
    let result = await minter.deploy_ton_wallet([], []);
    
    if (result.Ok) {
      showNotification('Wallet deployment initiated. This may take a few minutes to complete.', 'success');
      // We'll check the status again after a delay
      setTimeout(() => fetchBalances(), 10000);
    } else {
      showNotification(`Failed to deploy wallet: ${result.Err}`, 'error');
    }
  } catch (error) {
    showNotification('Failed to deploy wallet: ' + error.message, 'error');
  }
}

// Add a new function to generate both addresses at once
async function generateAllAddresses() {
  showLoading('Generating addresses...');
  try {
    // First get the IC account address
    const minter = await get_ckton_minter();
    if (!minter) return;
    
    // Get IC account address
    let icAddress = await minter.get_deposit_address([]);
    icAccountAddress.value = icAddress;
    
    // Generate TON address
    let tonAddr = await minter.generate_ton_address([], []);
    if (!tonAddr) {
      tonAddr = await minter.generate_ton_address();
    }
    tonAddress.value = tonAddr;
    
    // Fetch balances to update wallet deployment status
    await fetchBalances();
    
    showNotification('Addresses generated successfully', 'success');
  } catch (error) {
    showNotification('Failed to generate addresses: ' + error.message, 'error');
  }
}
</script>

<template>
  <div class="min-h-screen bg-gray-100 text-gray-900">
    <!-- Header -->
    <header class="bg-blue-600 text-white p-4 shadow-md">
      <div class="container mx-auto flex justify-between items-center">
        <div class="flex items-center">
          <img src="/ckton.png" alt="ckTON Logo" class="h-8 w-8 mr-3" />
          <h1 class="text-2xl font-bold">ckTON Wallet</h1>
        </div>
        <div class="flex items-center space-x-4">
          <span class="text-sm">{{ network === 'mainnet' ? 'Mainnet' : 'Testnet' }}</span>
          <button @click="connectToII"
            class="bg-white text-blue-600 px-3 py-1 rounded-full text-sm font-medium hover:bg-blue-100 transition-colors">
            {{ isAuthenticated ? 'Disconnect' : 'Connect to Internet Identity' }}
          </button>

          <!-- <button @click="switchNetwork"
            class="bg-white text-blue-600 px-3 py-1 rounded-full text-sm font-medium hover:bg-blue-100 transition-colors">
            Switch to {{ network === 'mainnet' ? 'Testnet' : 'Mainnet' }}
          </button> -->
        </div>
      </div>
    </header>

    <!-- Main content -->
    <main class="container mx-auto p-4 mt-6">
      <!-- Notification -->
      <div v-if="notification.show" :class="{
        'bg-green-100 border-green-500 text-green-700': notification.type === 'success',
        'bg-red-100 border-red-500 text-red-700': notification.type === 'error',
        'bg-blue-100 border-blue-500 text-blue-700': notification.type === 'info',
        'bg-gray-100 border-gray-500 text-gray-700': notification.type === 'loading'
      }" class="border-l-4 p-4 mb-6 rounded shadow-sm flex items-center">
        <div v-if="notification.type === 'loading'" class="animate-spin rounded-full h-5 w-5 border-b-2 border-blue-600 mr-3"></div>
        <div v-else-if="notification.type === 'success'" class="text-green-500 mr-3">
          <svg xmlns="http://www.w3.org/2000/svg" class="h-5 w-5" viewBox="0 0 20 20" fill="currentColor">
            <path fill-rule="evenodd" d="M10 18a8 8 0 100-16 8 8 0 000 16zm3.707-9.293a1 1 0 00-1.414-1.414L9 10.586 7.707 9.293a1 1 0 00-1.414 1.414l2 2a1 1 0 001.414 0l4-4z" clip-rule="evenodd" />
          </svg>
        </div>
        <div v-else-if="notification.type === 'error'" class="text-red-500 mr-3">
          <svg xmlns="http://www.w3.org/2000/svg" class="h-5 w-5" viewBox="0 0 20 20" fill="currentColor">
            <path fill-rule="evenodd" d="M18 10a8 8 0 11-16 0 8 8 0 0116 0zm-7 4a1 1 0 11-2 0 1 1 0 012 0zm-1-9a1 1 0 00-1 1v4a1 1 0 102 0V6a1 1 0 00-1-1z" clip-rule="evenodd" />
          </svg>
        </div>
        <div>
          <p class="font-medium">
            {{ notification.type === 'success' ? 'Success!' : notification.type === 'error' ? 'Error!' : notification.type === 'loading' ? 'Processing...' : 'Information' }}
          </p>
          <p>{{ notification.message }}</p>
        </div>
      </div>

      <div class="grid grid-cols-1 md:grid-cols-2 gap-6">
        <!-- Balances Card -->
        <div class="bg-white rounded-lg shadow-md p-6">
          <h2 class="text-xl font-semibold mb-4">Your Balances</h2>
          <div class="space-y-4">
            <div class="flex justify-between items-center p-3 bg-gray-50 rounded">
              <span>ckTON Balance:</span>
              <span class="font-medium">{{ ckTonBalance }} nano-ckTON</span>
            </div>
            <div class="flex justify-between items-center p-3 bg-gray-50 rounded">
              <span>TON Balance:</span>
              <span class="font-medium">{{ tonBalance }} nanoton</span>
            </div>
            <button @click="fetchBalances"
              class="w-full mt-2 bg-blue-600 text-white py-2 rounded hover:bg-blue-700 transition-colors">
              Refresh Balances
            </button>
          </div>
        </div>

        <!-- Addresses Card -->
        <div class="bg-white rounded-lg shadow-md p-6">
          <h2 class="text-xl font-semibold mb-4">Your Addresses</h2>
          
          <!-- Generate All Addresses Button -->
         
          
          <div class="space-y-4">
            <div>
              <div class="flex justify-between mb-1">
                <span>TON Address:</span>
              </div>
              <div class="p-3 bg-gray-50 rounded break-all">
                {{ tonAddress || 'No address generated yet' }}
              </div>
              <div v-if="tonAddress && !isWalletDeployed" class="mt-2">
                <button @click="deployTonWallet" 
                  class="w-full text-white bg-yellow-500 py-2 rounded hover:bg-yellow-600 transition-colors">
                  Deploy Wallet
                </button>
                <p class="text-xs text-gray-500 mt-1">
                  Your wallet is not deployed yet. Deploy it to start using it.
                </p>
              </div>
              <div v-else-if="tonAddress && isWalletDeployed" class="mt-2">
                <p class="text-xs text-green-600">
                  ✓ Wallet is active and ready to use
                </p>
              </div>
            </div>
            <div>
              <div class="flex justify-between mb-1">
                <span>IC Account Address for Burning:</span>
              </div>
              <div class="p-3 bg-gray-50 rounded break-all">
                {{ icAccountAddress || 'Not available' }}
              </div>
            </div>

             <button @click="generateAllAddresses" 
            class="w-full bg-blue-600 text-white py-2 rounded hover:bg-blue-700 transition-colors mb-4">
            Generate All Addresses
          </button>
          </div>
        </div>

        <!-- Mint ckTON Card -->
        <div class="bg-white rounded-lg shadow-md p-6">
          <h2 class="text-xl font-semibold mb-4">Mint ckTON</h2>
          <form @submit.prevent="mintCkTon" class="space-y-4">
            <div>
              <label for="mintAmount" class="block mb-1">Amount to Mint</label>
              <input id="mintAmount" v-model="mintAmount" type="number" min="0" step="0.01" placeholder="Enter amount"
                class="w-full p-2 border rounded focus:outline-none focus:ring-2 focus:ring-blue-500" required />
            </div>
            <div>
              <label for="receiveAddress" class="block mb-1">Receive Address</label>
              <input id="receiveAddress" v-model="receiveAddress" type="text" placeholder="Enter TON receive address"
                class="w-full p-2 border rounded focus:outline-none focus:ring-2 focus:ring-blue-500" required />
            </div>
            <button type="submit"
              class="w-full bg-green-600 text-white py-2 rounded hover:bg-green-700 transition-colors">
              Mint ckTON
            </button>
          </form>
        </div>

        <!-- Withdraw ckTON Card -->
        <div class="bg-white rounded-lg shadow-md p-6">
          <h2 class="text-xl font-semibold mb-4">Withdraw to Native TON</h2>
          <form @submit.prevent="withdrawCkTon" class="space-y-4">
            <div>
              <label for="withdrawAmount" class="block mb-1">Amount to Withdraw</label>
              <input id="withdrawAmount" v-model="withdrawAmount" type="number" min="0" step="0.01"
                placeholder="Enter amount"
                class="w-full p-2 border rounded focus:outline-none focus:ring-2 focus:ring-blue-500" required />
            </div>
            <div>
              <label for="withdrawAddress" class="block mb-1">Receive TON Address</label>
              <input id="withdrawAddress" v-model="withdrawAddress" type="text" placeholder="Enter TON address"
                class="w-full p-2 border rounded focus:outline-none focus:ring-2 focus:ring-blue-500" required />
            </div>
            <button type="submit"
              class="w-full bg-purple-600 text-white py-2 rounded hover:bg-purple-700 transition-colors">
              Withdraw to TON
            </button>
          </form>
        </div>
      </div>
    </main>

    <!-- Footer -->
    <footer class="bg-gray-800 text-white p-4 mt-12">
      <div class="container mx-auto text-center">
        <p>© 2023 ckTON Wallet. All rights reserved.</p>
      </div>
    </footer>
  </div>
</template>
