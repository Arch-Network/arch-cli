import React, { useState, useEffect } from 'react';
import { useParams, Link } from 'react-router-dom';
import { ArchRpcClient } from 'arch-typescript-sdk';
import { Buffer } from 'buffer';
import { motion, AnimatePresence } from 'framer-motion';
import { ArrowLeft, Clock, Hash, Database, ChevronDown, ChevronUp } from 'lucide-react';

const client = new ArchRpcClient('http://localhost:9002');

interface BlockDetails {
  height: number;
  hash: string;
  previous_block_hash: string;
  transactions: string[];
  timestamp: number;
}

const BlockDetailsPage: React.FC = () => {
  const { blockHashOrHeight } = useParams<{ blockHashOrHeight: string }>();
  const [blockDetails, setBlockDetails] = useState<BlockDetails | null>(null);
  const [loading, setLoading] = useState<boolean>(true);
  const [error, setError] = useState<string | null>(null);
  const [expandedTx, setExpandedTx] = useState<string | null>(null);

  useEffect(() => {
    const fetchBlockDetails = async () => {
      try {
        setLoading(true);
        let block;
        if (blockHashOrHeight!.length === 64) {
          block = await client.getBlock(Buffer.from(blockHashOrHeight!, 'hex').toString('utf8'));
        } else {
          const height = parseInt(blockHashOrHeight!);
          const blockHash = await client.getBlockHash(height);
          block = await client.getBlock(Buffer.from(blockHash, 'hex').toString('utf8'));
        }
        setBlockDetails({
          height: block.height,
          hash: blockHashOrHeight!,
          previous_block_hash: block.previous_block_hash,
          transactions: block.transactions || [],
          timestamp: block.timestamp || 0,
        });
      } catch (err) {
        console.error('Error fetching block details:', err);
        setError('Failed to fetch block details. Please try again.');
      } finally {
        setLoading(false);
      }
    };
    fetchBlockDetails();
  }, [blockHashOrHeight]);

  const toggleTxExpansion = (txId: string) => {
    setExpandedTx(expandedTx === txId ? null : txId);
  };

  if (loading) return (
    <div className="flex justify-center items-center h-screen">
      <div className="animate-spin rounded-full h-32 w-32 border-t-2 border-b-2 border-arch-orange"></div>
    </div>
  );
  if (error) return <div className="text-center py-4 text-arch-orange">{error}</div>;
  if (!blockDetails) return <div className="text-center py-4 text-arch-white">No block details found.</div>;

  return (
    <div className="p-4 max-w-6xl mx-auto text-arch-white">
      <Link to="/transactions" className="text-arch-orange hover:underline mb-4 inline-flex items-center transition-colors duration-300">
        <ArrowLeft className="mr-2" /> Back to Block History
      </Link>
      <motion.h1 
        initial={{ opacity: 0, y: -20 }}
        animate={{ opacity: 1, y: 0 }}
        className="text-4xl font-bold mb-6"
      >
        Block <span className="text-arch-orange">Details</span>
      </motion.h1>
      <motion.div 
        initial={{ opacity: 0, y: 20 }}
        animate={{ opacity: 1, y: 0 }}
        className="bg-arch-gray shadow-lg rounded-lg p-6"
      >
        <div className="grid grid-cols-1 md:grid-cols-2 gap-4 mb-6">
          <div className="flex items-center">
            <Database className="text-arch-orange mr-2" />
            <p><strong className="text-arch-orange">Height:</strong> {blockDetails.height}</p>
          </div>
          <div className="flex items-center">
            <Clock className="text-arch-orange mr-2" />
            <p><strong className="text-arch-orange">Timestamp:</strong> {new Date(blockDetails.timestamp * 1000).toLocaleString()}</p>
          </div>
          <div className="flex items-center col-span-2">
            <Hash className="text-arch-orange mr-2" />
            <p className="truncate"><strong className="text-arch-orange">Hash:</strong> {blockDetails.hash}</p>
          </div>
          <div className="flex items-center col-span-2">
            <Hash className="text-arch-orange mr-2" />
            <p className="truncate"><strong className="text-arch-orange">Previous Block Hash:</strong> {blockDetails.previous_block_hash}</p>
          </div>
        </div>
        <h2 className="text-2xl font-semibold mt-6 mb-4 text-arch-orange">Transactions</h2>
        {blockDetails.transactions.length > 0 ? (
          <ul className="space-y-2">
            <AnimatePresence>
              {blockDetails.transactions.map((txId, index) => (
                <motion.li
                  key={txId}
                  initial={{ opacity: 0, x: -20 }}
                  animate={{ opacity: 1, x: 0 }}
                  exit={{ opacity: 0, x: 20 }}
                  transition={{ duration: 0.3, delay: index * 0.05 }}
                  className="bg-arch-black p-3 rounded hover:bg-arch-gray transition-colors duration-300"
                >
                  <div 
                    className="flex justify-between items-center cursor-pointer"
                    onClick={() => toggleTxExpansion(txId)}
                  >
                    <span className="text-arch-orange mr-2">{index + 1}.</span>
                    <span className="truncate flex-grow">{txId}</span>
                    {expandedTx === txId ? <ChevronUp className="text-arch-orange" /> : <ChevronDown className="text-arch-orange" />}
                  </div>
                  <AnimatePresence>
                    {expandedTx === txId && (
                      <motion.div
                        initial={{ opacity: 0, height: 0 }}
                        animate={{ opacity: 1, height: 'auto' }}
                        exit={{ opacity: 0, height: 0 }}
                        transition={{ duration: 0.3 }}
                        className="mt-2 text-sm"
                      >
                        <p><strong className="text-arch-orange">Transaction ID:</strong> {txId}</p>
                        {/* Add more transaction details here as needed */}
                      </motion.div>
                    )}
                  </AnimatePresence>
                </motion.li>
              ))}
            </AnimatePresence>
          </ul>
        ) : (
          <p className="italic text-arch-gray-400">No transactions in this block</p>
        )}
      </motion.div>
    </div>
  );
};

export default BlockDetailsPage;