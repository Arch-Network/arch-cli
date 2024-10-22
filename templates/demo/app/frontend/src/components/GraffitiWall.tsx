import React, { useState, useEffect, useCallback } from 'react';
import { RpcConnection, Pubkey, AccountUtil, InstructionUtil, MessageUtil, PubkeyUtil } from '@saturnbtcio/arch-sdk';
import { Copy, Check, AlertCircle } from 'lucide-react';
import { Buffer } from 'buffer';

const NETWORK = (import.meta as any).env.VITE_NETWORK;
const client = new RpcConnection((import.meta as any).env.VITE_ARCH_NODE_URL || 'http://localhost:9002');
const PROGRAM_PUBKEY = (import.meta as any).env.VITE_PROGRAM_PUBKEY;
const WALL_PRIVATE_KEY = (import.meta as any).env.VITE_WALL_PRIVATE_KEY;
// const BACKEND_URL = (import.meta as any).env.VITE_BACKEND_URL || 'http://localhost:5174';
const WALL_ACCOUNT_PUBKEY = (import.meta as any).env.VITE_WALL_ACCOUNT_PUBKEY;

window.Buffer = Buffer;

class GraffitiMessage {
  constructor(
    public timestamp: number,
    public name: string,
    public message: string
  ) {}
}

const GraffitiWall: React.FC = () => {
  const [error, setError] = useState<string | null>(null);
  const [isAccountCreated, setIsAccountCreated] = useState(false);
  const [message, setMessage] = useState('');
  const [wallData, setWallData] = useState<GraffitiMessage[]>([]);
  const [isFormValid, setIsFormValid] = useState(false);
  const [name, setName] = useState('');
  const [copied, setCopied] = useState(false);  

  const accountPubkey = PubkeyUtil.fromHex(WALL_ACCOUNT_PUBKEY);

  const copyToClipboard = () => {
    navigator.clipboard.writeText(`arch-cli account create --name <unique_name>--program-id ${PROGRAM_PUBKEY}`);
    setCopied(true);
    setTimeout(() => setCopied(false), 2000);
  };

  const checkAccountCreated = useCallback(async () => {
    try {
      await client.readAccountInfo(accountPubkey);
      setIsAccountCreated(true);
      setError(null);
    } catch (error) {
      console.error('Error checking account:', error);
      setIsAccountCreated(false);
      setError('The Arch Graffiti program has not been deployed to the network yet. They need to run `arch-cli deploy` for this dapp.');
    }
  }, [accountPubkey]);

  const fetchWallData = useCallback(async () => {
    if (!isAccountCreated) return;

    try {
      const userAccount = await client.readAccountInfo(accountPubkey);

      if (userAccount.data.length === 0) {
        setWallData([]);
        return;
      }
  
      const dataView = new DataView(userAccount.data.buffer);
      const messages: GraffitiMessage[] = [];
      let offset = 0;

      const messageCount = dataView.getUint32(offset, true);
      offset += 4;

      for (let i = 0; i < messageCount; i++) {
        const timestamp = Number(dataView.getBigUint64(offset, true));
        offset += 8;

        const nameBytes = userAccount.data.slice(offset, offset + 16);
        const name = new TextDecoder().decode(nameBytes).replace(/\0+$/, '');
        offset += 16;

        const messageBytes = userAccount.data.slice(offset, offset + 64);
        const message = new TextDecoder().decode(messageBytes).replace(/\0+$/, '');
        offset += 64;
  
        messages.push(new GraffitiMessage(timestamp, name, message));
      }
  
      setWallData(messages.reverse());
    } catch (error) {
      console.error('Error fetching wall data:', error);
      setError(`Failed to fetch wall data: ${error instanceof Error ? error.message : String(error)}`);
    }
  }, [isAccountCreated, accountPubkey]);

  useEffect(() => {
    checkAccountCreated();
    if (isAccountCreated) {
      fetchWallData();
      const interval = setInterval(fetchWallData, 5000);
      return () => clearInterval(interval);
    }
  }, [accountPubkey, isAccountCreated, checkAccountCreated, fetchWallData]);

  const handleAddToWall = async () => {
    if (!message.trim() || !name.trim() || !isAccountCreated) {
      setError("Name and message are required, and account must be created.");
      return;
    }
  
    try {
      const privateKey = PubkeyUtil.fromHex(WALL_PRIVATE_KEY);
      const publicKey = privateKey;

    //   const instruction = {
    //     programId: PubkeyUtil.fromHex(PROGRAM_PUBKEY),
    //     accounts: [
    //       AccountUtil.serialize({ pubkey: publicKey, is_signer: true, is_writable: true }),
    //       AccountUtil.serialize({ pubkey: publicKey, is_signer: false, is_writable: true }),
    //     ],
    //     data: InstructionUtil.serialize({ name, message }),
    //   };

    //   const messageObj = {
    //     signers: [publicKey],
    //     instructions: [instruction],
    //   };

    //   const serializedMessage = MessageUtil.serialize(messageObj);
    //   const messageHash = MessageUtil.hash(messageObj);

    //   // Note: You'll need to implement the signing logic here
    //   // const signature = sign(messageHash, privateKey);

    //   const transaction = {
    //     version: 0,
    //     signatures: [/* signature */],
    //     message: serializedMessage,
    //   };

    //   const result = await client.sendTransaction(transaction);

    //   if (result) {
    //     await fetchWallData();
    //     setMessage('');
    //     setName('');
    //   } else {
    //     throw new Error('Failed to add to wall');
    //   }
    } catch (error) {
      console.error('Error adding to wall:', error);
      setError(`Failed to add to wall: ${error instanceof Error ? error.message : String(error)}`);
    }
  };

  const handleNameChange = (e: React.ChangeEvent<HTMLInputElement>) => {
    const newName = e.target.value;
    const encoder = new TextEncoder();
    const bytes = encoder.encode(newName);

    if (bytes.length <= 16) {
      setName(newName);
      setIsFormValid(newName.trim() !== '' && message.trim() !== '');
    }
  };

  const handleMessageChange = (e: React.ChangeEvent<HTMLTextAreaElement>) => {
    const newMessage = e.target.value;
    const encoder = new TextEncoder();
    const bytes = encoder.encode(newMessage);

    if (bytes.length <= 64) {
      setMessage(newMessage);
      setIsFormValid(newMessage.trim() !== '');
    }
  };

  const handleKeyDown = (e: React.KeyboardEvent<HTMLTextAreaElement>) => {
    if (e.key === 'Enter' && !e.shiftKey) {
      e.preventDefault();
      if (isFormValid) {
        handleAddToWall();
      }
    }
  };


  return (
  <div className="bg-gradient-to-br from-arch-gray to-gray-900 p-8 rounded-lg shadow-lg max-w-4xl mx-auto">
      <h2 className="text-3xl font-bold mb-6 text-center text-arch-white">Graffiti Wall</h2>
      
      {!isAccountCreated ? (
        <div className="bg-arch-black p-6 rounded-lg">
          <h3 className="text-2xl font-bold mb-4 text-arch-white">Account Setup Required</h3>
          <p className="text-arch-white mb-4">To participate in the Graffiti Wall, please create an account using the Arch CLI:</p>
          <div className="relative mb-4">
            <pre className="bg-gray-800 p-4 rounded-lg text-arch-white overflow-x-auto">
              <code>
                arch-cli account create --name graffiti --program-id {PROGRAM_PUBKEY}
              </code>
            </pre>
            <button
              onClick={copyToClipboard}
              className="absolute top-2 right-2 p-2 bg-arch-orange text-arch-black rounded hover:bg-arch-white transition-colors duration-300"
              title="Copy to clipboard"
            >
              {copied ? <Check size={20} /> : <Copy size={20} />}
            </button>
          </div>
          <p className="text-arch-white mb-4">Run this command in your terminal to set up your account.</p>
          
        </div>
      ) : (
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
                onKeyDown={handleKeyDown}
                placeholder="Your Message (required, max 64 bytes)"
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
      )}
      
      {error && (
        <div className="mt-6 p-4 bg-red-500 text-white rounded-lg">
          <div className="flex items-center mb-2">
            <AlertCircle className="w-6 h-6 mr-2" />
            <p className="font-bold">Program Error</p>
          </div>
          <p>{error}</p>
        </div>
      )}
    </div>
  );
};
export default GraffitiWall;