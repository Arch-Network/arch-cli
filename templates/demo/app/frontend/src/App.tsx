import React from 'react';
import { BrowserRouter as Router, Route, Link, Routes } from 'react-router-dom';
import TransactionHistoryPage from './components/TransactionHistoryPage';
import BlockDetailsPage from './components/BlockDetailsPage';
import GraffitiWallComponent from './components/GraffitiWallComponent';
import TransactionDetailsPage from './components/TransactionDetailsPage';
import SearchResultPage from './components/SearchResultPage';
import { LaserEyesProvider, NetworkType, TESTNET4 } from '@omnisat/lasereyes';

const App: React.FC = () => {
  const [network, setNetwork] = React.useState(TESTNET4);
  return (
    <Router>
      <LaserEyesProvider config={{ network: network as NetworkType }}>
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
              <Route path="/" element={<GraffitiWallComponent />} />
              <Route path="/transactions" element={<TransactionHistoryPage />} />
              <Route path="/block/:blockHashOrHeight" element={<BlockDetailsPage />} />
              <Route path="/transaction/:txId" element={<TransactionDetailsPage />} />
              <Route path="/search/:term" element={<SearchResultPage />} />
            </Routes>
          </div>
        </div>
      </LaserEyesProvider>
    </Router>
  );
}

export default App;
