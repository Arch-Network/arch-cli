import React, { useState } from 'react';
import { ArchRpcClient, Pubkey } from 'arch-typescript-sdk';
import { request } from 'sats-connect';
import { QRCodeSVG } from 'qrcode.react';
import { Buffer } from 'buffer';
import axios from 'axios';

const SYSTEM_PROGRAM_PUBKEY = (import.meta as any).env.VITE_SYSTEM_PROGRAM_PUBKEY;
const NETWORK = (import.meta as any).env.VITE_NETWORK;
const client = new ArchRpcClient((import.meta as any).env.VITE_ARCH_NODE_URL || 'http://localhost:9002');

// btc-rpc-explorer configuration
const BTC_RPC_EXPLORER_URL = 'http://localhost:3000';

const CreateArchAccount: React.FC = () => {
  const [step, setStep] = useState(0);
  const [error, setError] = useState<string | null>(null);
  const [qrCodeData, setQrCodeData] = useState<string | null>(null);
  const [txid, setTxid] = useState<string | null>(null);
  const [bitcoinAddress, setBitcoinAddress] = useState<string | null>(null);

  const steps = [
    'Get system program address',
    'Send Bitcoin transaction',
    'Create Arch account'
  ];

  const handleCreateAccount = async () => {
    try {
      // Step 1: Get system program address
      setStep(1);
      const pubkeyBytes = Buffer.from(SYSTEM_PROGRAM_PUBKEY, 'hex');
      const systemPubkey = new Pubkey(pubkeyBytes);
      const address = await client.getAccountAddress(systemPubkey);
      setBitcoinAddress(address);
      
      // Step 2: Send Bitcoin transaction
      setStep(2);
      const qrData = `bitcoin:${address}?amount=0.00003`; // 3000 satoshis
      setQrCodeData(qrData);
      
      // Wait for the transaction
      const txid = await waitForTransaction(address);
      setTxid(txid);
      
      // Step 3: Create Arch account
      setStep(3);
      const privateKey = localStorage.getItem('archPrivateKey');
      if (!privateKey) throw new Error('Private key not found');
      
      const privateKeyBytes = new Uint8Array(privateKey.match(/.{1,2}/g)!.map((byte: string) => parseInt(byte, 16)));
      const archTxId = await client.createArchAccount(privateKeyBytes, txid, 0);
      
      console.log('Arch account created, txId:', archTxId);
      setStep(4); // Complete
    } catch (error) {
      console.error('Error creating Arch account:', error);
      setError(`Failed to create Arch account: ${error instanceof Error ? error.message : String(error)}`);
    }
  };

  const waitForTransaction = async (address: string): Promise<string> => {
    if (NETWORK === 'regtest') {
      return waitForTransactionRegtest(address);
    } else {
      return waitForTransactionMainnet(address);
    }
  };

  const waitForTransactionRegtest = async (address: string): Promise<string> => {
    const requiredBalance = 3000; // 3000 satoshis
    const maxAttempts = 30; // Adjust as needed
    const delayBetweenAttempts = 2000; // 2 seconds

    for (let attempt = 0; attempt < maxAttempts; attempt++) {
      try {
        const response = await axios.get(`${BTC_RPC_EXPLORER_URL}/api/address/${address}`);
        
        if (response.data.error) {
          throw new Error(`Bitcoin RPC error: ${response.data.error}`);
        }

        const balance = response.data.txHistory.balanceSat;
        
        if (balance >= requiredBalance) {
          // Get the most recent transaction
          const txids = response.data.txHistory.txids;
          if (txids.length > 0) {
            return txids[0]; // The txids are already sorted in descending order
          } else {
            throw new Error('No transactions found');
          }
        }

        // Wait before next attempt
        await new Promise(resolve => setTimeout(resolve, delayBetweenAttempts));
      } catch (error) {
        console.error('Error checking balance:', error);
        // Continue to next attempt
      }
    }

    throw new Error('Timeout waiting for transaction');
  };

  const waitForTransactionMainnet = (address: string): Promise<string> => {
    return new Promise((resolve, reject) => {
      const checkInterval = setInterval(async () => {
        try {
          const result = await request('getBalance', null);
          if (result.status === 'success' && BigInt(result.result.confirmed) >= 3000n) {
            clearInterval(checkInterval);
            resolve('dummy_txid_' + Date.now());
          }
        } catch (error) {
          clearInterval(checkInterval);
          reject(error);
        }
      }, 5000);
    });
  };

  return (
    <div className="bg-arch-gray p-8 rounded-lg shadow-md max-w-md mx-auto">
      <h2 className="text-2xl font-bold mb-4 text-center">Create Arch Account</h2>
      <div className="mb-4">
        {steps.map((stepText, index) => (
          <div key={index} className={`flex items-center ${index <= step ? 'text-arch-orange' : 'text-arch-white'}`}>
            <div className={`w-6 h-6 rounded-full ${index < step ? 'bg-arch-orange' : 'bg-arch-gray border border-arch-white'} flex items-center justify-center mr-2`}>
              {index < step && '✓'}
            </div>
            <span>{stepText}</span>
          </div>
        ))}
      </div>
      {bitcoinAddress && step >= 2 && (
        <div className="mb-4 p-2 bg-arch-black rounded">
          <p className="text-sm text-arch-white">Bitcoin Address:</p>
          <p className="font-mono text-xs break-all text-arch-orange">{bitcoinAddress}</p>
        </div>
      )}
      {qrCodeData && step === 2 && NETWORK !== 'regtest' && (
        <div className="mb-4 flex flex-col items-center">
          <p className="text-sm text-arch-white mb-2">Scan to send 3000 satoshis:</p>
          <QRCodeSVG value={qrCodeData} size={200} />
        </div>
      )}
      <button 
        onClick={handleCreateAccount}
        disabled={step > 0 && step < 4}
        className={`w-full bg-arch-white text-arch-black font-bold py-2 px-4 rounded transition duration-300 ${step > 0 && step < 4 ? 'opacity-50 cursor-not-allowed' : 'hover:bg-arch-orange'}`}
      >
        {step === 0 ? 'Create Arch Account' : step === 4 ? 'Account Created!' : 'Creating...'}
      </button>
      {error && (
        <div className="mt-4 p-4 bg-red-500 text-white rounded">
          <p>{error}</p>
        </div>
      )}
    </div>
  );
};

export default CreateArchAccount;