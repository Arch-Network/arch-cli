import React, { useState, useEffect, useCallback } from 'react';
import { ArchRpcClient, Pubkey } from 'arch-typescript-sdk';
import { Info, Copy, Check, AlertCircle } from 'lucide-react';

const NETWORK = (import.meta as any).env.VITE_NETWORK;
const client = new ArchRpcClient((import.meta as any).env.VITE_ARCH_NODE_URL || 'http://localhost:9002');
const PROGRAM_PUBKEY = (import.meta as any).env.VITE_PROGRAM_PUBKEY;

interface CreateArchAccountProps {
  accountPubkey: string;
}

class GraffitiMessage {
  constructor(
    public timestamp: number,
    public name: string,
    public message: string
  ) {}
}

const GraffitiWall: React.FC<CreateArchAccountProps> = ({ accountPubkey }) => {
  const [error, setError] = useState<string | null>(null);
  const [isAccountCreated, setIsAccountCreated] = useState(false);
  const [message, setMessage] = useState('');
  const [wallData, setWallData] = useState<GraffitiMessage[]>([]);
  const [isFormValid, setIsFormValid] = useState(false);
  const [name, setName] = useState('');
  const [copied, setCopied] = useState(false);
  const [privateKey, setPrivateKey] = useState('');

  const copyToClipboard = () => {
    navigator.clipboard.writeText(`arch-cli account create --name graffiti --program-id ${PROGRAM_PUBKEY}`);
    setCopied(true);
    setTimeout(() => setCopied(false), 2000);
  };

  const checkAccountCreated = useCallback(async () => {
    if (!accountPubkey) {
      console.log("Account pubkey not available yet");
      return;
    }
  
    try {
      const formalPubkey = Pubkey.fromString(accountPubkey);
      await client.readAccountInfo(formalPubkey);
      setIsAccountCreated(true);
      setError(null); // Clear any previous errors
    } catch (error) {
      console.error('Error checking account:', error);
      setIsAccountCreated(false);
      setError("Network Error: Please ensure your network is up and the Arch server is running. You can start the server using the command:\n\n```\narch-cli server start\n```");
    }
  }, [accountPubkey]);

  const fetchWallData = useCallback(async () => {
    if (!accountPubkey || !isAccountCreated) return;

    try {
      const formalPubkey = Pubkey.fromString(accountPubkey);
      const userAccount = await client.readAccountInfo(formalPubkey);

      if (userAccount.data.length === 0) {
        setWallData([]);
        return;
      }
  
      const dataView = new DataView(new Uint8Array(userAccount.data).buffer);
      const messages: GraffitiMessage[] = [];
      let offset = 0;

      const messageCount = dataView.getUint32(offset, true);
      offset += 4;

      for (let i = 0; i < messageCount; i++) {
        const timestamp = Number(dataView.getBigInt64(offset, true));
        offset += 8;

        const nameBytes = new Uint8Array(userAccount.data.slice(offset, offset + 16));
        const name = new TextDecoder().decode(nameBytes).replace(/\0+$/, '');
        offset += 16;

        const messageBytes = new Uint8Array(userAccount.data.slice(offset, offset + 64));
        const message = new TextDecoder().decode(messageBytes).replace(/\0+$/, '');
        offset += 64;
  
        messages.push(new GraffitiMessage(timestamp, name, message));
      }
  
      setWallData(messages.reverse());
    } catch (error) {
      console.error('Error fetching wall data:', error);
      setError(`Failed to fetch wall data: ${error instanceof Error ? error.message : String(error)}`);
    }
  }, [accountPubkey, isAccountCreated]);

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
      const privateKey = localStorage.getItem('archPrivateKey');
      if (!privateKey) throw new Error('Private key not found in localStorage');
      const privateKeyBytes = new Uint8Array(privateKey.match(/.{1,2}/g)!.map((byte: string) => parseInt(byte, 16)));

      const encoder = new TextEncoder();
      const nameBytes = encoder.encode(name.slice(0, 16).padEnd(16, '\0')).slice(0, 16);
      const messageBytes = encoder.encode(message).slice(0, 64);

      const instructionData = new Uint8Array(80); // 16 bytes for name, 64 bytes for message
      instructionData.set(nameBytes, 0);
      instructionData.set(messageBytes, 16);

      await client.callProgram(privateKeyBytes, PROGRAM_PUBKEY, Array.from(instructionData));

      await fetchWallData();
      setMessage('');
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

  const handlePrivateKeySubmit = () => {
    if (privateKey.trim()) {
      localStorage.setItem('archPrivateKey', privateKey.trim());
      setPrivateKey('');
      alert('Private key saved successfully!');
    } else {
      alert('Please enter a valid private key.');
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
          
          <div className="mt-6">
            <h4 className="text-xl font-bold mb-2 text-arch-white">Enter Your Private Key</h4>
            <div className="flex items-center">
              <input
                type="password"
                value={privateKey}
                onChange={(e) => setPrivateKey(e.target.value)}
                placeholder="Enter your private key"
                className="flex-grow px-3 py-2 bg-arch-gray text-arch-white rounded-l-md focus:outline-none focus:ring-2 focus:ring-arch-orange"
              />
              <button
                onClick={handlePrivateKeySubmit}
                className="bg-arch-orange text-arch-black px-4 py-2 rounded-r-md hover:bg-arch-white transition-colors duration-300"
              >
                Save
              </button>
            </div>
            <p className="text-sm text-arch-white mt-2">Your private key will be securely stored in your browser's local storage.</p>
          </div>
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
            <p className="font-bold">Network Error</p>
          </div>
          <p className="mb-2">Please ensure your network is up and the Arch server is running. You can start the server using the command:</p>
          <pre className="bg-red-600 p-2 rounded">
            <code>arch-cli server start</code>
          </pre>
        </div>
      )}
    </div>
  );
};
export default GraffitiWall;