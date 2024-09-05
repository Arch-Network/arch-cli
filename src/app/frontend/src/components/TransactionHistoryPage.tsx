import React, { useState, useEffect, useCallback } from 'react';
import { Link, useNavigate } from 'react-router-dom';
import { ArchRpcClient, ProcessedTransaction } from 'arch-typescript-sdk';
import { Buffer } from 'buffer';
import { motion, AnimatePresence } from 'framer-motion';
import { ChevronDown, ChevronUp, Search } from 'lucide-react';

const client = new ArchRpcClient(import.meta.env.VITE_RPC_URL as string);
interface BlockData {
  height: number;
  transactions: ProcessedTransaction[];
  previous_block_hash: string;
  timestamp: number;
}

const BLOCKS_PER_PAGE = parseInt(import.meta.env.VITE_BLOCKS_PER_PAGE || '20', 10);

const TransactionHistoryPage: React.FC = () => {

  const navigate = useNavigate();
  const [blocks, setBlocks] = useState<BlockData[]>([]);
  const [loading, setLoading] = useState<boolean>(true);
  const [error, setError] = useState<string | null>(null);
  const [currentPage, setCurrentPage] = useState<number>(1);
  const [totalBlocks, setTotalBlocks] = useState<number>(0);
  const [expandedBlocks, setExpandedBlocks] = useState<Set<number>>(new Set());
  const [searchTerm, setSearchTerm] = useState<string>('');

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
          const block = await client.getBlock(Buffer.from(hexBlockHash, 'hex').toString('utf8'));

          newBlocks.push({
            height: i,
            transactions: block.transactions || [],
            previous_block_hash: block.previous_block_hash,
            timestamp: block.timestamp || 0,
          });
        } catch (blockError) {
          console.warn(`Failed to fetch block ${i}: ${blockError}`);
          newBlocks.push({
            height: i,
            transactions: [],
            previous_block_hash: '',
            timestamp: 0,
          });
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

  const toggleBlockExpansion = (blockHeight: number) => {
    setExpandedBlocks(prev => {
      const newSet = new Set(prev);
      if (newSet.has(blockHeight)) {
        newSet.delete(blockHeight);
      } else {
        newSet.add(blockHeight);
      }
      return newSet;
    });
  };

  const formatTimestamp = (timestamp: number): string => {
    return new Date(timestamp * 1000).toLocaleString();
  };

  const handleSearch = (e: React.FormEvent) => {
    e.preventDefault();
    if (!searchTerm.trim()) return;

    const trimmedSearchTerm = searchTerm.trim();

    // Check if the search term is a number (potential block height)
    if (/^\d+$/.test(trimmedSearchTerm)) {
      navigate(`/block/${trimmedSearchTerm}`);
    } 
    // Check if the search term is a 64-character hex string (potential block hash)
    else if (/^[a-fA-F0-9]{64}$/.test(trimmedSearchTerm)) {
      navigate(`/block/${trimmedSearchTerm}`);
    } 
    else {
      // If it's neither a valid block height nor a valid block hash
      setError('Invalid search term. Please enter a valid block height or hash.');
      setTimeout(() => setError(null), 3000); // Clear the error after 3 seconds
    }

    // Clear the search input after searching
    setSearchTerm('');
  };

  const renderBlock = (block: BlockData) => (
    <motion.div
      key={block.height}
      initial={{ opacity: 0, y: 20 }}
      animate={{ opacity: 1, y: 0 }}
      exit={{ opacity: 0, y: -20 }}
      transition={{ duration: 0.3 }}
      className="mb-4 bg-arch-gray rounded-lg overflow-hidden shadow-lg"
    >
      <div 
        className="px-4 py-3 flex justify-between items-center cursor-pointer hover:bg-arch-black transition-colors duration-300"
        onClick={() => toggleBlockExpansion(block.height)}
      >
        <Link to={`/block/${block.height}`} className="text-lg font-semibold text-arch-white hover:text-arch-orange transition-colors duration-300">
          Block Height: {block.height}
        </Link>
        <div className="flex items-center">
          <span className="mr-4 text-sm text-arch-white">
            Transactions: <span className="text-arch-orange">{block.transactions.length}</span>
          </span>
          {expandedBlocks.has(block.height) ? <ChevronUp className="text-arch-orange" /> : <ChevronDown className="text-arch-orange" />}
        </div>
      </div>
      <AnimatePresence>
        {expandedBlocks.has(block.height) && (
          <motion.div
            initial={{ opacity: 0, height: 0 }}
            animate={{ opacity: 1, height: 'auto' }}
            exit={{ opacity: 0, height: 0 }}
            transition={{ duration: 0.3 }}
            className="p-4 bg-arch-black"
          >
            <p className="mb-2 text-arch-white"><strong className="text-arch-orange">Timestamp:</strong> {formatTimestamp(block.timestamp)}</p>
            <p className="mb-2 text-arch-white"><strong className="text-arch-orange">Previous Block Hash:</strong> {block.previous_block_hash}</p>
            {block.transactions.length > 0 ? (
              <>
                <p className="mb-2 font-medium text-arch-white">Transactions:</p>
                <ul className="space-y-2">
                  {block.transactions.slice(0, 5).map((txId, index) => (
                    <motion.li
                      key={index}
                      initial={{ opacity: 0, x: -20 }}
                      animate={{ opacity: 1, x: 0 }}
                      transition={{ duration: 0.2, delay: index * 0.1 }}
                      className="text-arch-orange truncate"
                    >
                      {txId}
                    </motion.li>
                  ))}
                </ul>
                {block.transactions.length > 5 && (
                  <Link to={`/block/${block.height}`} className="text-arch-orange hover:underline mt-2 inline-block">
                    View all {block.transactions.length} transactions
                  </Link>
                )}
              </>
            ) : (
              <p className="text-gray-400 italic">No transactions in this block</p>
            )}
          </motion.div>
        )}
      </AnimatePresence>
    </motion.div>
  );

  if (loading) return (
    <div className="flex justify-center items-center h-64">
      <div className="animate-spin rounded-full h-32 w-32 border-t-2 border-b-2 border-arch-orange"></div>
    </div>
  );
  if (error) return <div className="text-center py-4 text-arch-orange">{error}</div>;

  return (
    <div className="p-4 max-w-6xl mx-auto text-arch-white">
      <h1 className="text-4xl font-bold mb-6 text-center">
        Block <span className="text-arch-orange">Explorer</span>
      </h1>
      <form onSubmit={handleSearch} className="mb-6">
        <div className="relative">
          <input
            type="text"
            value={searchTerm}
            onChange={(e) => setSearchTerm(e.target.value)}
            placeholder="Search by block height or hash"
            className="w-full px-4 py-2 bg-arch-gray text-arch-white border border-arch-gray rounded-lg focus:outline-none focus:ring-2 focus:ring-arch-orange"
          />
          <button
            type="submit"
            className="absolute right-2 top-1/2 transform -translate-y-1/2"
          >
            <Search className="text-arch-orange" />
          </button>
        </div>
      </form>
      {error && (
        <motion.div
          initial={{ opacity: 0, y: -10 }}
          animate={{ opacity: 1, y: 0 }}
          exit={{ opacity: 0, y: -10 }}
          className="mb-4 p-2 bg-red-600 text-white rounded-lg text-center"
        >
          {error}
        </motion.div>
      )}
      {blocks.map(renderBlock)}
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