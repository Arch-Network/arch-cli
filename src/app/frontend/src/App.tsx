import React, { useState, useEffect } from 'react';
import { BrowserRouter as Router, Route, Link, Routes } from 'react-router-dom';
import { AddressPurpose, BitcoinNetworkType, getAddress, GetAddressOptions, GetAddressResponse } from 'sats-connect';
import { bitcoinService } from './services/bitcoinService';
import { ArchRpcClient, Message, Pubkey} from 'arch-typescript-sdk';
import TransactionHistoryPage from './components/TransactionHistoryPage';
import BlockDetailsPage from './components/BlockDetailsPage';
import { createTransaction, generatePrivateKey, generatePubkeyFromPrivateKey } from './utils/cryptoHelpers';

const client = new ArchRpcClient('http://localhost:9002');

interface Transaction {
  version: number;
  signatures: string[];
  message: Message;
}

const App: React.FC = () => {
  const [address, setAddress] = useState<string>('');
  const [balance, setBalance] = useState<number | null>(null);
  const [archAddress, setArchAddress] = useState<string>('');

  const [privateKey, setPrivateKey] = useState<string>('');
  const [generatedPubkey, setGeneratedPubkey] = useState<string>('');
  const [programId, setProgramId] = useState<string>('');
  const [accountPubkey, setAccountPubkey] = useState<string>('');
  const [accountIsSigner, setAccountIsSigner] = useState<boolean>(false);
  const [accountIsWritable, setAccountIsWritable] = useState<boolean>(true);
  const [instructionData, setInstructionData] = useState<string>('');
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    // Check if private key exists in local storage
    const storedPrivateKey = localStorage.getItem('archPrivateKey');
    if (storedPrivateKey) {
      setPrivateKey(storedPrivateKey);
      handleGeneratePubkey(storedPrivateKey);
    } else {
      // Generate a new private key
      const newPrivateKey = generatePrivateKey();
      localStorage.setItem('archPrivateKey', newPrivateKey);
      setPrivateKey(newPrivateKey);
      handleGeneratePubkey(newPrivateKey);
    }
  }, []);

  const connectWallet = async (): Promise<void> => {
    try {
      const options: GetAddressOptions = {
        payload: {
          purposes: [AddressPurpose.Payment],
          message: 'Address for Bitcoin Regtest App',
          network: {
            type: BitcoinNetworkType.Testnet
          },
        },
        onFinish: (response: GetAddressResponse) => {
          console.log(response);
          setAddress(response.addresses[0].address);

        },
        onCancel: () => console.log('User cancelled the request'),
      };
      await getAddress(options);
    } catch (error) {
      console.error('Error connecting wallet:', error);
    }
  };

  const checkBalance = async (): Promise<void> => {
    if (address) {
      const balance = await bitcoinService.getBalance(address);
      setBalance(balance);
    }
  };

  const handleGeneratePubkey = (key: string = privateKey) => {
    try {
      console.log('Generating pubkey from private key:', key);
      const pubkey = generatePubkeyFromPrivateKey(key);
      console.log('Generated pubkey:', pubkey.toString());
      setGeneratedPubkey(pubkey.toString());
      setAccountPubkey(pubkey.toString());
      setError(null);
    } catch (error) {
      console.error('Error generating pubkey:', error);
      setError(`Failed to generate pubkey: ${error instanceof Error ? error.message : String(error)}`);
    }
  };

  const createArchAccount = async (): Promise<void> => {
    try {

      const transaction = await createTransaction(
        programId,
        accountPubkey,
        accountIsSigner,
        accountIsWritable,
        instructionData,
        privateKey
      );

      const txId = await client.sendTransaction(transaction);
      console.log('Transaction sent, txId:', txId);
      setError(null);
    } catch (error) {
      console.error('Error creating transaction:', error);
      setError(`Failed to create transaction: ${error instanceof Error ? error.message : String(error)}`);
    }
  };

  const connectToArch = async (): Promise<void> => {
    try {
      // Use the stored or generated private key
      const privateKeyBytes = new Uint8Array(privateKey.match(/.{1,2}/g)!.map((byte: string) => parseInt(byte, 16)));

      const txId = await client.createArchAccount(privateKeyBytes, "", 0);
      console.log('Transaction sent, txId:', txId);

      const pubkey = generatePubkeyFromPrivateKey(privateKey);
      console.log('Generated pubkey:', pubkey.toString());
      const accountCreated = await client.readAccountInfo(pubkey);
      console.log('Account created:', accountCreated);


      setError(null);
    } catch (error) {
      console.error('Error in connectToArch:', error);
      if (error instanceof Error) {
        setError(`Error: ${error.message}`);
        if ('response' in error) {
          console.error('Error data:', (error as any).response.data);
        }
      } else {
        setError('An unknown error occurred');
      }
    }
  };

  return (
    <Router>
      <div className="min-h-screen bg-arch-black text-arch-white">
        <nav className="bg-arch-gray shadow-md p-4">
        <div className="container mx-auto flex justify-between items-center">
            <img src="/arch_logo.svg" alt="Arch Network" className="h-8" />
            <ul className="flex space-x-4">
              <li>
                <Link to="/" className="text-arch-white hover:text-arch-orange transition duration-300">Home</Link>
              </li>
              <li>
                <Link to="/transactions" className="text-arch-white hover:text-arch-orange transition duration-300">Block Explorer</Link>
              </li>
            </ul>
          </div>
        </nav>

        <div className="container mx-auto p-4">
          <Routes>
          <Route path="/" element={
              <div className="bg-arch-gray p-8 rounded-lg shadow-md max-w-md mx-auto">
                <h1 className="text-3xl font-bold mb-6 text-center">
                  Bitcoin <span className="text-arch-orange">Regtest App</span>
                </h1>
                <button 
                  onClick={connectWallet} 
                  className="w-full bg-arch-white text-arch-black font-bold py-2 px-4 rounded transition duration-300 mb-4 hover:bg-arch-orange"
                >
                  Connect Bitcoin Wallet
                </button>

                {/* Form for private key and pubkey generation */}
                <form className="mb-4">
                  <input
                    type="text"
                    placeholder="Private Key (32-byte hex)"
                    value={privateKey}
                    onChange={(e) => setPrivateKey(e.target.value)}
                    className="w-full p-2 mb-2 bg-arch-black text-arch-white rounded"
                  />
                  <button
                    type="button"
                    onClick={handleGeneratePubkey}
                    className="w-full bg-arch-white text-arch-black font-bold py-2 px-4 rounded transition duration-300 mb-2 hover:bg-arch-orange"
                  >
                    Generate Pubkey
                  </button>
                  {generatedPubkey && (
                    <div className="mb-4 p-2 bg-arch-black rounded">
                      <p className="text-sm text-arch-white">Generated Pubkey:</p>
                      <p className="font-mono text-xs break-all text-arch-orange">{generatedPubkey}</p>
                    </div>
                  )}
                </form>

                {/* Form for transaction details */}
                <form className="mb-4">
                  <input
                    type="text"
                    placeholder="Program ID (hex)"
                    value={programId}
                    onChange={(e) => setProgramId(e.target.value)}
                    className="w-full p-2 mb-2 bg-arch-black text-arch-white rounded"
                  />
                  <input
                    type="text"
                    placeholder="Account Pubkey (hex)"
                    value={accountPubkey}
                    onChange={(e) => setAccountPubkey(e.target.value)}
                    className="w-full p-2 mb-2 bg-arch-black text-arch-white rounded"
                  />
                  <div className="flex mb-2">
                    <label className="mr-4">
                      <input
                        type="checkbox"
                        checked={accountIsSigner}
                        onChange={(e) => setAccountIsSigner(e.target.checked)}
                        className="mr-2"
                      />
                      Is Signer
                    </label>
                    <label>
                      <input
                        type="checkbox"
                        checked={accountIsWritable}
                        onChange={(e) => setAccountIsWritable(e.target.checked)}
                        className="mr-2"
                      />
                      Is Writable
                    </label>
                  </div>
                  <input
                    type="text"
                    placeholder="Instruction Data (hex)"
                    value={instructionData}
                    onChange={(e) => setInstructionData(e.target.value)}
                    className="w-full p-2 mb-2 bg-arch-black text-arch-white rounded"
                  />
                </form>

                <button 
                  onClick={connectToArch} 
                  className="w-full bg-arch-white text-arch-black font-bold py-2 px-4 rounded transition duration-300"
                >
                  Connect to Arch and Send Transaction
                </button>

                {error && (
                  <div className="mt-4 p-4 bg-red-500 text-white rounded">
                    <p>{error}</p>
                  </div>
                )}
              </div>
            } />
            <Route path="/transactions" element={<TransactionHistoryPage />} />
            <Route path="/block/:blockHashOrHeight" element={<BlockDetailsPage />} />
          </Routes>
        </div>
      </div>
    </Router>
  );
}

export default App;