import React, { useState, useEffect, useCallback } from 'react';
import { ArchRpcClient, Pubkey, RuntimeTransaction, Instruction, Message } from 'arch-typescript-sdk';
import { sha256 } from '@noble/hashes/sha256';
import { bytesToHex } from '@noble/hashes/utils';
import { AlertCircle } from 'lucide-react';
import { schnorr } from '@noble/secp256k1';
import {
  AddressPurpose,
  MessageSigningProtocols,
  request,
} from "sats-connect";
import { bech32m } from '@scure/base';
import { Buffer } from 'buffer';

const NETWORK = (import.meta as any).env.VITE_NETWORK;
const client = new ArchRpcClient((import.meta as any).env.VITE_ARCH_NODE_URL || 'http://localhost:9002');
const PROGRAM_PUBKEY = (import.meta as any).env.VITE_PROGRAM_PUBKEY;
const WALL_ACCOUNT_PUBKEY = (import.meta as any).env.VITE_WALL_ACCOUNT_PUBKEY;

interface GraffitiWallProps {
  accountPubkey: string;
}

class GraffitiMessage {
  constructor(
    public timestamp: number,
    public name: string,
    public message: string
  ) {}
}

const GraffitiWall: React.FC<GraffitiWallProps> = ({ accountPubkey }) => {
  const [error, setError] = useState<string | null>(null);
  const [message, setMessage] = useState('');
  const [wallData, setWallData] = useState<GraffitiMessage[]>([]);
  const [isFormValid, setIsFormValid] = useState(false);
  const [name, setName] = useState('');

  const fetchWallData = useCallback(async () => {
    try {
      console.log('Fetching wall data', WALL_ACCOUNT_PUBKEY);
      const wallAccount = await client.readAccountInfo(Pubkey.fromString(WALL_ACCOUNT_PUBKEY));

      // Check if account data exists and has sufficient length
      if (!wallAccount.data || wallAccount.data.length < 4) {
        console.log('Wall account is empty or invalid');
        setWallData([]);
        return;
      }
  
      const data = new Uint8Array(wallAccount.data);
      const decoder = new TextDecoder();

      // Rest of the function remains the same
      const messageCount = new DataView(data.buffer).getUint32(0, true);
      let offset = 4;

      const messages: GraffitiMessage[] = [];
      for (let i = 0; i < messageCount; i++) {
        const timestamp = new DataView(data.buffer).getBigInt64(offset, true);
        offset += 8;

        const nameBytes = new Uint8Array(data.slice(offset, offset + 16));
        const name = decoder.decode(nameBytes).replace(/\0+$/, '');
        offset += 16;

        const messageBytes = new Uint8Array(data.slice(offset, offset + 64));
      }
  
      setWallData(messages.sort((a, b) => b.timestamp - a.timestamp));
    } catch (error) {
      console.error('Error fetching wall data:', error);
      setError(`Failed to fetch wall data: ${error instanceof Error ? error.message : String(error)}`);
    }
  }, []);

  useEffect(() => {
    fetchWallData();
    const interval = setInterval(fetchWallData, 5000);
    return () => clearInterval(interval);
  }, [fetchWallData]);

  function hexToBytes(hex: string): Uint8Array {
    return new Uint8Array(hex.match(/.{1,2}/g)?.map(byte => parseInt(byte, 16)) || []);
  }

  const handleSubmit = async (e: React.FormEvent) => {
    e.preventDefault();
  
    try {
      const response = await request("getAddresses", {
        purposes: [AddressPurpose.Ordinals],
      });
  
      if (response.status === 'error') {
        throw new Error(response.error.message);
      }
  
      const userAddress = response.result.addresses[0].address;
  
      const encoder = new TextEncoder();
      const nameBytes = encoder.encode(name.slice(0, 16).padEnd(16, '\0')).slice(0, 16);
      const messageBytes = encoder.encode(message).slice(0, 64);
  
      const instructionData = new Uint8Array(80);
      instructionData.set(nameBytes, 0);
      instructionData.set(messageBytes, 16);
  
      console.log('User address:', userAddress);
  
      let userAddressHex;
      let pubkeyBytes;
      try {
        const decoded = bech32m.decode(userAddress);
        const words = decoded.words;
        const pubkeyWords = words.slice(1);
        pubkeyBytes = bech32m.fromWords(pubkeyWords);
  
        if (pubkeyBytes.length !== 32) {
          throw new Error(`Invalid pubkey length: ${pubkeyBytes.length} bytes`);
        }
  
        userAddressHex = bytesToHex(new Uint8Array(pubkeyBytes));
        console.log('User address (hex):', userAddressHex);
      } catch (error) {
        console.error('Error decoding user address:', error);
        throw new Error(`Failed to decode user address: ${error instanceof Error ? error.message : String(error)}`);
      }
  
      // Create the instruction
      const instruction: Instruction = {
        program_id: Pubkey.fromString(PROGRAM_PUBKEY),
        accounts: [
          {
            pubkey: Pubkey.fromString(userAddressHex),
            is_signer: true,
            is_writable: false,
          },
          {
            pubkey: Pubkey.fromString(WALL_ACCOUNT_PUBKEY),
            is_signer: false,
            is_writable: true,
          }
        ],
        data: Array.from(instructionData),
      };

      // Create the message
      const messageObj: Message = {
        signers: [new Pubkey(pubkeyBytes)],
        instructions: [instruction],
      };

      // Encode the message using the SDK's method
      const encodedMessage = client.encodeMessage(messageObj);

      // Hash the encoded message
      const firstHash = sha256(new Uint8Array(encodedMessage));
      const messageHash = sha256(bytesToHex(firstHash));

      // Sign the message using sats-connect
      const signMessageResponse = await request("signMessage", {
        address: userAddress,
        message: bytesToHex(messageHash),
        protocol: MessageSigningProtocols.BIP322,
      });

      if (signMessageResponse.status === 'error') {
        throw new Error(signMessageResponse.error.message);
      }

      console.log('Signature:', signMessageResponse.result.signature);
      console.log('Protocol:', signMessageResponse.result.protocol);
      console.log('Hash:', bytesToHex(messageHash));
      console.log('Pubkey:', bytesToHex(pubkeyBytes));

      // Convert the signature to a Uint8Array
      const signatureBuffer = Buffer.from(signMessageResponse.result.signature, 'base64');

      console.log('Signature buffer:', signatureBuffer);

      // Strip the first byte without using slice
      const signature = signatureBuffer.slice(2);

      console.log('Signature:', signature);

      // Verify the signature
      const isValid = await schnorr.verify(signature, messageHash, pubkeyBytes);
      console.log('Signature is valid:', isValid);

      // Construct the RuntimeTransaction
      const transaction: RuntimeTransaction = {
        version: 0,
        signatures: [signature.toString('hex')],
        message: messageObj,
      };
  
      // Send the transaction to the Arch node
      const tranResponse = await client.sendTransaction(transaction);
  
      console.log('Successfully added to wall:', tranResponse);
      setName('');
      setMessage('');
      fetchWallData();
    } catch (error: unknown) {
      console.error('Error adding to wall:', error);
      setError(`Failed to add message: ${error instanceof Error ? error.message : String(error)}`);
    }
  };

  const handleNameChange = (e: React.ChangeEvent<HTMLInputElement>) => {
    const newName = e.target.value;
    setName(newName);
    setIsFormValid(newName.trim() !== '' && message.trim() !== '');
  };

  const handleMessageChange = (e: React.ChangeEvent<HTMLTextAreaElement>) => {
    const newMessage = e.target.value;
    setMessage(newMessage);
    setIsFormValid(name.trim() !== '' && newMessage.trim() !== '');
  };

  return (
    <div className="bg-gradient-to-br from-arch-gray to-gray-900 p-8 rounded-lg shadow-lg max-w-4xl mx-auto">
      <h2 className="text-3xl font-bold mb-6 text-center text-arch-white">Graffiti Wall</h2>
      
      <div className="flex flex-col md:flex-row gap-8">
        <div className="flex-1">
          <div className="bg-arch-black p-6 rounded-lg">
            <h3 className="text-2xl font-bold mb-4 text-arch-white">Add to Wall</h3>
            <input
              type="text"
              value={name}
              onChange={handleNameChange}
              placeholder="Your Name (required, max 16 bytes)"
              className="w-full px-3 py-2 bg-arch-gray text-arch-white rounded-md focus:outline-none focus:ring-2 focus:ring-arch-orange mb-2"
              required
            />
            <textarea
              value={message}
              onChange={handleMessageChange}
              placeholder="Your Message (required, max 64 bytes)"
              className="w-full px-3 py-2 bg-arch-gray text-arch-white rounded-md focus:outline-none focus:ring-2 focus:ring-arch-orange mb-2"
              required
            />
            <button 
              onClick={handleSubmit}
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
        </div>
        
        <div className="flex-1">
          <div className="bg-arch-black p-6 rounded-lg">
            <h3 className="text-2xl font-bold mb-4 text-arch-white">Wall Messages</h3>
            <div className="space-y-4 max-h-96 overflow-y-auto">
              {wallData.map((item, index) => (
                <div key={index} className="bg-arch-gray p-3 rounded-lg">
                  <p className="font-bold text-arch-orange">{new Date(item.timestamp * 1000).toLocaleString()}</p>
                  <p className="text-arch-white"><span className="font-semibold">{item.name}:</span> {item.message}</p>
                </div>
              ))}
            </div>
          </div>
        </div>
      </div>
      
      {error && (
        <div className="mt-6 p-4 bg-red-500 text-white rounded-lg">
          <div className="flex items-center mb-2">
            <AlertCircle className="w-6 h-6 mr-2" />
            <p className="font-bold">Error</p>
          </div>
          <p>{error}</p>
        </div>
      )}
    </div>
  );
};

export default GraffitiWall;