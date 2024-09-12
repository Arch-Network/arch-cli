import React, { useState, useEffect, useCallback } from 'react';
import { ArchRpcClient, Pubkey } from 'arch-typescript-sdk';
import { request } from 'sats-connect';
import { QRCodeSVG } from 'qrcode.react';
import { Buffer } from 'buffer';
import { AlertCircle, Check, Cpu, Send, UserPlus } from 'lucide-react';
import { Schema, serialize } from 'borsh';

const NETWORK = (import.meta as any).env.VITE_NETWORK;
const client = new ArchRpcClient((import.meta as any).env.VITE_ARCH_NODE_URL || 'http://localhost:9002');

interface CreateArchAccountProps {
  accountPubkey: string;
}

class GraffitiWallParams {
    name: string;
    message: string;

    constructor(fields: { name: string; message: string }) {
        this.name = fields.name;
        this.message = fields.message;
    }
}

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
  const [name, setName] = useState('');
  const [message, setMessage] = useState('');
  const [wallData, setWallData] = useState<GraffitiWallParams[]>([]);
  const [isFormValid, setIsFormValid] = useState(false);
  const [isPubkeyAvailable, setIsPubkeyAvailable] = useState(false);

  const steps = [
    { text: 'Get account address', icon: Cpu },
    { text: 'Send Bitcoin transaction', icon: Send },
    { text: 'Create Arch account', icon: UserPlus }
  ];

  const checkAccountCreated = useCallback(async () => {
    if (!accountPubkey) {
      console.log("Account pubkey not available yet");
      return;
    }

    try {
      const formalPubkey = Pubkey.fromString(accountPubkey);
      const userAccount = await client.readAccountInfo(formalPubkey);
      setIsAccountCreated(userAccount.data.length > 0);
      setIsPubkeyAvailable(true);
    } catch (error) {
      console.error('Error checking account:', error);
      setError(`Failed to check account: ${error instanceof Error ? error.message : String(error)}`);
    }
  }, [accountPubkey]);

  const fetchWallData = useCallback(async () => {
    if (!accountPubkey) {
      console.log("Account pubkey not available yet");
      return;
    }

    try {
      const formalPubkey = Pubkey.fromString(accountPubkey);
      const userAccount = await client.readAccountInfo(formalPubkey);
      
      if (userAccount.data.length === 0) {
        setWallData([]);
        return;
      }
  
      const asciiString = new TextDecoder().decode(new Uint8Array(userAccount.data));
      const entries = asciiString.split('|');
      const wallDataArray = [];
  
      for (let i = 0; i < entries.length; i += 2) {
        if (entries[i] && entries[i + 1]) {
          wallDataArray.push(new GraffitiWallParams({
            name: entries[i],
            message: entries[i + 1]
          }));
        }
      }
  
      setWallData(wallDataArray.reverse());
    } catch (error) {
      console.error('Error fetching wall data:', error);
      setError(`Failed to fetch wall data: ${error instanceof Error ? error.message : String(error)}`);
    }
  }, [accountPubkey]);

  useEffect(() => {
    if (accountPubkey) {
      checkAccountCreated();
      fetchWallData();
      const interval = setInterval(fetchWallData, 5000); // Refresh every 5 seconds
      return () => clearInterval(interval);
    }
  }, [accountPubkey, checkAccountCreated, fetchWallData]);

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

  const handleAddToWall = async () => {
    if (!isFormValid) {
      setError("Name and message are required.");
      return;
    }

    try {
      const programPubkey = "ab3fd900df6e708bf805a8ca3298f1b2fb4546c1be743465fa5665ba9ddd5089";

      const privateKey = localStorage.getItem('archPrivateKey');
      if (!privateKey) throw new Error('Private key not found');
      const privateKeyBytes = new Uint8Array(privateKey.match(/.{1,2}/g)!.map((byte: string) => parseInt(byte, 16)));

      const formalPubkey = Pubkey.fromString(accountPubkey);
      const userAccount = await client.readAccountInfo(formalPubkey);

      if (Buffer.from(userAccount.owner).toString('hex') !== programPubkey) {
        await client.transferAccountOwnership(privateKeyBytes, programPubkey);
        console.log("ownership transferred to the program");
      }

      const params = new GraffitiWallParams({ name, message });
      const instructionData = serialize(GraffitiWallParamsSchema, params);
      const instructionDataNumbers = Array.from(instructionData);

      await client.callProgram(privateKeyBytes, programPubkey, instructionDataNumbers);

      await fetchWallData();

      setName('');
      setMessage('');
      setIsFormValid(false);
    } catch (error) {
      console.error('Error adding to wall:', error);
      setError(`Failed to add to wall: ${error instanceof Error ? error.message : String(error)}`);
    }
  };

  const handleNameChange = (e: React.ChangeEvent<HTMLInputElement>) => {
    const newName = e.target.value.slice(0, 60);
    setName(newName);
    validateForm(newName, message);
  };

  const handleMessageChange = (e: React.ChangeEvent<HTMLTextAreaElement>) => {
    const newMessage = e.target.value.slice(0, 128);
    setMessage(newMessage);
    validateForm(name, newMessage);
  };

  const validateForm = (name: string, message: string) => {
    setIsFormValid(name.trim() !== '' && message.trim() !== '');
  };

  return (
    <div className="bg-gradient-to-br from-arch-gray to-gray-900 p-8 rounded-lg shadow-lg max-w-4xl mx-auto">
      <h2 className="text-3xl font-bold mb-6 text-center text-arch-white">Graffiti Wall</h2>
      
      {!isPubkeyAvailable ? (
        <div className="text-center text-arch-white">
          Loading account information...
        </div>
      ) : (
      <div className="flex flex-col md:flex-row gap-8">
        <div className="flex-1">
          {!isAccountCreated ? (
            <div className="bg-arch-black p-6 rounded-lg">
              <h3 className="text-2xl font-bold mb-4 text-arch-white">Create Account</h3>
              <div className="mb-4">
                {steps.map((stepItem, index) => (
                  <div key={index} className={`flex items-center mb-2 ${index <= step ? 'text-arch-orange' : 'text-arch-white'}`}>
                    <div className={`w-8 h-8 rounded-full ${index < step ? 'bg-arch-orange' : 'bg-arch-gray border-2 border-arch-white'} flex items-center justify-center mr-3 transition-all duration-300`}>
                      {index < step ? <Check className="w-4 h-4" /> : <stepItem.icon className="w-4 h-4" />}
                    </div>
                    <span className="text-sm">{stepItem.text}</span>
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
                  className="w-full bg-arch-orange text-arch-black font-bold py-2 px-4 rounded-lg transition duration-300 hover:bg-arch-white hover:text-arch-orange"
                >
                  Create Arch Account
                </button>
              )}
            </div>
          ) : (
            <div className="bg-arch-black p-6 rounded-lg">
              <h3 className="text-2xl font-bold mb-4 text-arch-white">Add to Wall</h3>
              <input
                type="text"
                value={name}
                onChange={handleNameChange}
                placeholder="Your Name (required, max 60 characters)"
                maxLength={60}
                className="w-full px-3 py-2 bg-arch-gray text-arch-white rounded-md focus:outline-none focus:ring-2 focus:ring-arch-orange mb-2"
                required
              />
              <textarea
                value={message}
                onChange={handleMessageChange}
                placeholder="Your Message (required, max 128 characters)"
                maxLength={128}
                className="w-full px-3 py-2 bg-arch-gray text-arch-white rounded-md focus:outline-none focus:ring-2 focus:ring-arch-orange mb-2"
                required
              />
              <button 
                onClick={handleAddToWall}
                className={`w-full font-bold py-2 px-4 rounded-lg transition duration-300 ${
                  isFormValid 
                    ? 'bg-arch-orange text-arch-black hover:bg-arch-white hover:text-arch-orange' 
                    : 'bg-gray-500 text-gray-300 cursor-not-allowed'
                }`}
                disabled={!isFormValid}
              >
                Add to the Wall
              </button>
            </div>
          )}
        </div>
        
        <div className="flex-1">
          <div className="bg-arch-black p-6 rounded-lg">
            <h3 className="text-2xl font-bold mb-4 text-arch-white">Wall Messages</h3>
            <div className="space-y-4 max-h-96 overflow-y-auto">
              {wallData.map((item, index) => (
                <div key={index} className="bg-arch-gray p-3 rounded-lg">
                  <p className="font-bold text-arch-orange">{item.name}</p>
                  <p className="text-arch-white">{item.message}</p>
                </div>
              ))}
            </div>
          </div>
        </div>
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