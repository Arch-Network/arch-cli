import React, { useState } from 'react';
import { ArchRpcClient, Pubkey } from 'arch-typescript-sdk';
import { request } from 'sats-connect';
import { QRCodeSVG } from 'qrcode.react';
import { Buffer } from 'buffer';
import { AlertCircle, Check, Cpu, Send, UserPlus } from 'lucide-react';
import { Schema, serialize, deserialize } from 'borsh';

const NETWORK = (import.meta as any).env.VITE_NETWORK;
const client = new ArchRpcClient((import.meta as any).env.VITE_ARCH_NODE_URL || 'http://localhost:9002');

interface CreateArchAccountProps {
  accountPubkey: string;
}

// Define the GraffitiWallParams class
class GraffitiWallParams {
    name: string;
    message: string;

    constructor(fields: { name: string; message: string }) {
        this.name = fields.name;
        this.message = fields.message;
    }
}

// Define the Borsh schema for serialization
const GraffitiWallParamsSchema: Schema = {
    struct: {
        name: 'string',
        message: 'string'
    }
};

const CreateArchAccount: React.FC<CreateArchAccountProps> = ({ accountPubkey }) => {
  const [step, setStep] = useState(0);
  const [error, setError] = useState<string | null>(null);
  const [qrCodeData, setQrCodeData] = useState<string | null>(null);
  const [txid, setTxid] = useState<string>('');
  const [bitcoinAddress, setBitcoinAddress] = useState<string | null>(null);
  const [isAccountCreated, setIsAccountCreated] = useState(false);
  const [helloWorldCalled, setHelloWorldCalled] = useState(false);

  const steps = [
    { text: 'Get account address', icon: Cpu },
    { text: 'Send Bitcoin transaction', icon: Send },
    { text: 'Create Arch account', icon: UserPlus }
  ];

  const handleCreateAccount = async () => {
    try {
      // Step 1: Get account address
      setStep(1);
      const pubkeyBytes = Buffer.from(accountPubkey, 'hex');
      const userPubkey = new Pubkey(pubkeyBytes);
      const address = await client.getAccountAddress(userPubkey);
      setBitcoinAddress(address);
      
      // Step 2: Handle transaction
      setStep(2);
      const qrData = `bitcoin:${address}?amount=0.00003`; // 3000 satoshis
      setQrCodeData(qrData);

      if (NETWORK !== 'regtest') {
        // Wait for the transaction
        const receivedTxid = await waitForTransaction(address);
        setTxid(receivedTxid);
        proceedToCreateAccount(receivedTxid);
      }
      // For regtest, we'll wait for the user to input the txid
      
    } catch (error) {
      console.error('Error in account creation process:', error);
      setError(`Error in account creation process: ${error instanceof Error ? error.message : String(error)}`);
    }
  };

  const proceedToCreateAccount = async (transactionId: string) => {
    try {
      // Step 3: Create Arch account
      setStep(3);
      const privateKey = localStorage.getItem('archPrivateKey');
      if (!privateKey) throw new Error('Private key not found');
      
      const privateKeyBytes = new Uint8Array(privateKey.match(/.{1,2}/g)!.map((byte: string) => parseInt(byte, 16)));
      const archTxId = await client.createArchAccount(privateKeyBytes, transactionId, 0);
      
      console.log('Arch account created, txId:', archTxId);
      setIsAccountCreated(true);
      setStep(4); // Complete
    } catch (error) {
      console.error('Error creating Arch account:', error);
      setError(`Failed to create Arch account: ${error instanceof Error ? error.message : String(error)}`);
    }
  };

  const waitForTransaction = (address: string): Promise<string> => {
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

  const handleTxidSubmit = (e: React.FormEvent) => {
    e.preventDefault();
    if (txid.trim()) {
      proceedToCreateAccount(txid);
    }
  };

  const handleCallHelloWorld = async () => {
    console.log("done");

    const programPubkey = "ab3fd900df6e708bf805a8ca3298f1b2fb4546c1be743465fa5665ba9ddd5089";

    const privateKey = localStorage.getItem('archPrivateKey');
    if (!privateKey) throw new Error('Private key not found');
    const privateKeyBytes = new Uint8Array(privateKey.match(/.{1,2}/g)!.map((byte: string) => parseInt(byte, 16)));

    // If the program doesn't own the user's account, transfer ownership
    const formalPubkey = Pubkey.fromString(accountPubkey);
    const userAccount = await client.readAccountInfo(formalPubkey);

    console.log(userAccount);

    const wallDataString = String.fromCharCode.apply(null, userAccount.data);
    console.log(wallDataString);

    if (Buffer.from(userAccount.owner).toString('hex') !== programPubkey) {
        console.log("transfering ownership");
        await client.transferAccountOwnership(privateKeyBytes, programPubkey);
        console.log("ownership transferred to the program");
    }

    const params = new GraffitiWallParams({ name: 'Brian Hoffman', message: 'Check it out!' });

    const instructionData = serialize(GraffitiWallParamsSchema, params);

    // Convert uint8Array to number[]
    const instructionDataNumbers = Array.from(instructionData);

    const result = await client.callProgram(privateKeyBytes, programPubkey, instructionDataNumbers);
    console.log(result);    

    //setHelloWorldCalled(true);
  };

  return (
    <div className="bg-gradient-to-br from-arch-gray to-gray-900 p-8 rounded-lg shadow-lg max-w-md mx-auto">
      <h2 className="text-3xl font-bold mb-6 text-center text-arch-white">Create Arch Account</h2>
      {!isAccountCreated && (
        <>
          <div className="mb-8">
            {steps.map((stepItem, index) => (
              <div key={index} className={`flex items-center mb-4 ${index <= step ? 'text-arch-orange' : 'text-arch-white'}`}>
                <div className={`w-10 h-10 rounded-full ${index < step ? 'bg-arch-orange' : 'bg-arch-gray border-2 border-arch-white'} flex items-center justify-center mr-4 transition-all duration-300`}>
                  {index < step ? <Check className="w-6 h-6" /> : <stepItem.icon className="w-6 h-6" />}
                </div>
                <span className="text-lg">{stepItem.text}</span>
              </div>
            ))}
          </div>
          {bitcoinAddress && step >= 2 && (
            <div className="mb-6 p-4 bg-arch-black rounded-lg shadow-inner">
              <p className="text-sm text-arch-white mb-1">Bitcoin Address:</p>
              <p className="font-mono text-xs break-all text-arch-orange">{bitcoinAddress}</p>
            </div>
          )}
          {qrCodeData && step === 2 && (
            <div className="mb-6 flex flex-col items-center">
              <p className="text-sm text-arch-white mb-2">Scan to send 3000 satoshis:</p>
              <div className="bg-white p-4 rounded-lg">
                <QRCodeSVG value={qrCodeData} size={200} />
              </div>
            </div>
          )}
          {NETWORK === 'regtest' && step === 2 && (
            <form onSubmit={handleTxidSubmit} className="mb-6">
              <label htmlFor="txid" className="block text-sm font-medium text-arch-white mb-2">
                Enter Transaction ID:
              </label>
              <input
                type="text"
                id="txid"
                value={txid}
                onChange={(e) => setTxid(e.target.value)}
                className="w-full px-3 py-2 bg-arch-black text-arch-white rounded-md focus:outline-none focus:ring-2 focus:ring-arch-orange"
                placeholder="Enter transaction ID"
                required
              />
              <button
                type="submit"
                className="mt-3 w-full bg-arch-orange text-arch-black font-bold py-2 px-4 rounded-lg transition duration-300 hover:bg-arch-white hover:text-arch-orange"
              >
                Submit Transaction ID
              </button>
            </form>
          )}
          {step === 0 && (
            <button 
              onClick={handleCreateAccount}
              className="w-full bg-arch-orange text-arch-black font-bold py-3 px-6 rounded-lg transition duration-300 hover:bg-arch-white hover:text-arch-orange"
            >
              Create Arch Account
            </button>
          )}
        </>
      )}
      {isAccountCreated && (
        <div className="mt-6">
          <div className="text-center text-arch-white font-bold text-xl mb-4">
            Account Created!
          </div>
          <button 
            onClick={handleCallHelloWorld}
            className="w-full bg-arch-orange text-arch-black font-bold py-3 px-6 rounded-lg transition duration-300 hover:bg-arch-white hover:text-arch-orange"
            disabled={helloWorldCalled}
          >
            {helloWorldCalled ? "Hello World Called!" : "Call Hello World"}
          </button>
        </div>
      )}
      {error && (
        <div className="mt-6 p-4 bg-red-500 text-white rounded-lg flex items-center">
          <AlertCircle className="w-6 h-6 mr-2" />
          <p>{error}</p>
        </div>
      )}
    </div>
  );
};

export default CreateArchAccount;