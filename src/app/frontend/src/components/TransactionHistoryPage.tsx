import React, { useState, useEffect, useCallback } from 'react';
import { ArchRpcClient } from 'arch-typescript-sdk';
import { Buffer } from 'buffer';
import { motion } from 'framer-motion';
import SearchBar from './SearchBar';
import BlockList from './BlockList';

const client = new ArchRpcClient(import.meta.env.VITE_RPC_URL as string);

interface BlockData {
  height: number;
  hash: string;
  transactions: string[];
  timestamp: number;
}

const BLOCKS_PER_PAGE = parseInt(import.meta.env.VITE_BLOCKS_PER_PAGE || '20', 10);

const TransactionHistoryPage: React.FC = () => {
  const [blocks, setBlocks] = useState<BlockData[]>([]);
  const [loading, setLoading] = useState<boolean>(true);
  const [error, setError] = useState<string | null>(null);
  const [currentPage, setCurrentPage] = useState<number>(1);
  const [totalBlocks, setTotalBlocks] = useState<number>(0);

  const fetchBlocks = useCallback(async (page: number): Promise<void> => {
    try {
      setLoading(true);
      const blockHeight = await client.getBlockCount();
      setTotalBlocks(blockHeight);

      const startBlock = Math.max(1, blockHeight - (page * BLOCKS_PER_PAGE) + 1);
      const endBlock = Math.max(1, startBlock + BLOCKS_PER_PAGE - 1);

      const newBlocks: BlockData[] = [];

      for (let i = endBlock; i >= startBlock; i--) {
        try {
          const hexBlockHash = await client.getBlockHash(i);
          const asciiBlockHash = Buffer.from(hexBlockHash, 'hex').toString('utf8');
          const block = await client.getBlock(asciiBlockHash);

          newBlocks.push({
            height: i,
            hash: asciiBlockHash,
            transactions: block.transactions || [],
            timestamp: block.timestamp || 0,
          });
        } catch (blockError) {
          console.warn(`Failed to fetch block ${i}: ${blockError}`);
        }
      }

      setBlocks(newBlocks);
    } catch (err) {
      console.error('Error fetching blocks:', err);
      setError('Failed to fetch blocks. Please try again later.');
    } finally {
      setLoading(false);
    }
  }, []);

  useEffect(() => {
    fetchBlocks(currentPage);
  }, [currentPage, fetchBlocks]);

  if (loading) return (
    <div className="flex justify-center items-center h-64">
      <div className="animate-spin rounded-full h-32 w-32 border-t-2 border-b-2 border-arch-orange"></div>
    </div>
  );
  
  if (error) return <div className="text-center py-4 text-arch-orange">{error}</div>;

  return (
    <div className="p-4 max-w-7xl mx-auto text-arch-white">
      <motion.h1 
        initial={{ opacity: 0, y: -20 }}
        animate={{ opacity: 1, y: 0 }}
        className="text-4xl font-bold mb-6 text-center"
      >
        Block <span className="text-arch-orange">Explorer</span>
      </motion.h1>
      
      <SearchBar />
      
      <BlockList blocks={blocks} />
      
      <div className="mt-6 flex justify-center items-center space-x-4">
        <button
          onClick={() => setCurrentPage(prev => Math.max(1, prev - 1))}
          disabled={currentPage === 1}
          className="px-4 py-2 bg-arch-gray text-arch-white rounded hover:bg-arch-orange disabled:bg-arch-gray disabled:text-gray-500 transition duration-300"
        >
          Previous
        </button>
        <span className="text-arch-white">
          Page {currentPage} of {Math.ceil(totalBlocks / BLOCKS_PER_PAGE)}
        </span>
        <button
          onClick={() => setCurrentPage(prev => Math.min(Math.ceil(totalBlocks / BLOCKS_PER_PAGE), prev + 1))}
          disabled={currentPage === Math.ceil(totalBlocks / BLOCKS_PER_PAGE)}
          className="px-4 py-2 bg-arch-gray text-arch-white rounded hover:bg-arch-orange disabled:bg-arch-gray disabled:text-gray-500 transition duration-300"
        >
          Next
        </button>
      </div>
    </div>
  );
};

export default TransactionHistoryPage;