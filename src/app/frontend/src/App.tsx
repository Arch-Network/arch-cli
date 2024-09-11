import React, { useEffect, useState } from 'react';
import { BrowserRouter as Router, Route, Link, Routes } from 'react-router-dom';
import TransactionHistoryPage from './components/TransactionHistoryPage';
import BlockDetailsPage from './components/BlockDetailsPage';
import CreateArchAccount from './components/CreateArchAccount';
import { createTransaction, generatePrivateKey, generatePubkeyFromPrivateKey } from './utils/cryptoHelpers';


const App: React.FC = () => {
  const [privateKey, setPrivateKey] = useState<string>('');  
  const [generatedPubkey, setGeneratedPubkey] = useState<string>('');
  const [accountPubkey, setAccountPubkey] = useState<string>('');
  const [error, setError] = useState<string>('');
  
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

  const handleGeneratePubkey = (key: string = privateKey) => {
    try {
      console.log('Generating pubkey from private key:', key);
      const pubkey = generatePubkeyFromPrivateKey(key);
      console.log('Generated pubkey:', pubkey.toString());
      setGeneratedPubkey(pubkey.toString());
      setAccountPubkey(pubkey.toString());
      setError('');
    } catch (error) {
      console.error('Error generating pubkey:', error);
      setError(`Failed to generate pubkey: ${error instanceof Error ? error.message : String(error)}`);
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
            <Route path="/" element={<CreateArchAccount />} />
            <Route path="/transactions" element={<TransactionHistoryPage />} />
            <Route path="/block/:blockHashOrHeight" element={<BlockDetailsPage />} />
          </Routes>
        </div>
      </div>
    </Router>
  );
}

export default App;