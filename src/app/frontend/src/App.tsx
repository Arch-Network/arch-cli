import React, { useState } from 'react';
import { BrowserRouter as Router, Route, Link, Routes } from 'react-router-dom';
import { AddressPurpose, BitcoinNetworkType, getAddress, GetAddressOptions, GetAddressResponse } from 'sats-connect';
import { bitcoinService } from './services/bitcoinService';
import { ArchRpcClient, Pubkey } from 'arch-typescript-sdk';
import TransactionHistoryPage from './components/TransactionHistoryPage';
import BlockDetailsPage from './components/BlockDetailsPage';


const client = new ArchRpcClient('http://localhost:9002');

const App: React.FC = () => {
  const [address, setAddress] = useState<string>('');
  const [balance, setBalance] = useState<number | null>(null);
  const [archAddress, setArchAddress] = useState<string>('');

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

  const connectToArch = async (): Promise<void> => {
    try {
      const isReady = await client.isNodeReady();
      console.log('Is node ready?', isReady);
  
      const pubkeyBytes = new Uint8Array(32);
      crypto.getRandomValues(pubkeyBytes);

      const pubkey = new Pubkey(pubkeyBytes);
      console.log('Pubkey:', pubkey.toString());
  
      const accountAddress = await client.getAccountAddress(pubkey);
      console.log('Account address:', accountAddress);
      setArchAddress(accountAddress);

      // ... (rest of the connectToArch function remains the same)
    } catch (error) {
      console.error('Error:', error);
      if (error instanceof Error && 'response' in error) {
        console.error('Error data:', (error as any).response.data);
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
                <button 
                  onClick={connectToArch} 
                  className="w-full bg-arch-white text-arch-black font-bold py-2 px-4 rounded transition duration-300"
                >
                  Connect to Arch
                </button>
                {address && (
                  <div className="mt-4 p-4 bg-arch-black rounded">
                    <p className="text-sm text-arch-white">Connected Bitcoin Address:</p>
                    <p className="font-mono text-xs break-all text-arch-orange">{address}</p>
                  </div>
                )}
                {archAddress && (
                  <div className="mt-4 p-4 bg-arch-black rounded">
                    <p className="text-sm text-arch-white">Connected Arch Address:</p>
                    <p className="font-mono text-xs break-all text-arch-orange">{archAddress}</p>
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