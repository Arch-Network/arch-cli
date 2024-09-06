import React, { useState, useEffect } from 'react';
import { useParams, Link } from 'react-router-dom';
import { ArchRpcClient, ProcessedTransaction } from 'arch-typescript-sdk';
import { Buffer } from 'buffer';
import { motion, AnimatePresence } from 'framer-motion';
import { ArrowLeft, Clock, Hash, Database, ChevronDown, ChevronUp, CheckCircle, AlertCircle, FileText, Layers, Bitcoin } from 'lucide-react';
import bs58 from 'bs58';

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
  const [transactionDetails, setTransactionDetails] = useState<ProcessedTransaction | null>(null);

  useEffect(() => {
    const fetchBlockDetails = async () => {
      try {
        setLoading(true);
        let block;
        if (blockHashOrHeight!.length === 64) {
          block = await client.getBlock(blockHashOrHeight!);
          block.height = 0;
          block.hash = blockHashOrHeight!;
        } else {
          const height = parseInt(blockHashOrHeight!);
          const blockHash = await client.getBlockHash(height);
          block = await client.getBlock(Buffer.from(blockHash, 'hex').toString('utf8'));
          block.height = height;
          block.hash = Buffer.from(blockHash, 'hex').toString('utf8');
          console.log(block);
        }
        setBlockDetails({
          height: block.height,
          hash: block.hash,
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

  const fetchTransactionDetails = async (txId: string) => {
    try {
      setLoading(true);
      const txDetails = await client.getProcessedTransaction(txId);
      setTransactionDetails(txDetails);
    } catch (err) {
      console.error('Error fetching transaction details:', err);
      setError('Failed to fetch transaction details. Please try again.');
    } finally {
      setLoading(false);
    }
  };

  const toggleTxExpansion = async (txId: string) => {
    if (expandedTx === txId) {
      setExpandedTx(null);
      setTransactionDetails(null);
    } else {
      setExpandedTx(txId);
      await fetchTransactionDetails(txId);
    }
  };

  const getStatusString = (status: number): string => {
    return status === 0 ? 'Processing' : 'Processed';
  };

  const renderTransactionDetails = (txDetails: ProcessedTransaction) => (
    <div className="grid grid-cols-1 md:grid-cols-2 gap-4 mt-4">
      <div className="bg-arch-gray rounded-lg p-4 flex items-start">
        <div className="mr-3 mt-1">
          {txDetails.status === 1 ? (
            <CheckCircle className="text-green-500" size={20} />
          ) : (
            <AlertCircle className="text-yellow-500" size={20} />
          )}
        </div>
        <div>
          <h3 className="text-arch-orange font-semibold mb-2">Status</h3>
          <p>{getStatusString(txDetails.status)}</p>
        </div>
      </div>
      <div className="bg-arch-gray rounded-lg p-4 flex items-start">
        <FileText className="text-arch-orange mr-3 mt-1" size={20} />
        <div>
          <h3 className="text-arch-orange font-semibold mb-2">Version</h3>
          <p>{txDetails.runtime_transaction.version}</p>
        </div>
      </div>
      <div className="bg-arch-gray rounded-lg p-4 flex items-start col-span-2">
        <Hash className="text-arch-orange mr-3 mt-1" size={20} />
        <div>
          <h3 className="text-arch-orange font-semibold mb-2">Signatures</h3>
          <ul className="space-y-2">
            {txDetails.runtime_transaction.signatures.map((sig, index) => {
              const base58Sig = bs58.encode(Buffer.from(sig, 'hex'));
              return (
                <li key={index} className="text-sm">
                  {base58Sig}
                  <button
                    className="ml-2 text-arch-orange hover:underline"
                    onClick={() => navigator.clipboard.writeText(base58Sig)}
                  >
                    Copy
                  </button>
                </li>
              );
            })}
          </ul>
        </div>
      </div>
      <div className="bg-arch-gray rounded-lg p-4 flex items-start">
        <Layers className="text-arch-orange mr-3 mt-1" size={20} />
        <div>
          <h3 className="text-arch-orange font-semibold mb-2">Instructions</h3>
          <p>{txDetails.runtime_transaction.message.instructions.length}</p>
        </div>
      </div>
      <div className="bg-arch-gray rounded-lg p-4 flex items-start">
        <Bitcoin className="text-arch-orange mr-3 mt-1" size={20} />
        <div>
          <h3 className="text-arch-orange font-semibold mb-2">Bitcoin TxIDs</h3>
          <p>{txDetails.bitcoin_txids.length > 0 ? txDetails.bitcoin_txids.join(', ') : 'None'}</p>
        </div>
      </div>
    </div>
  );

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
      <h1 className="text-4xl font-bold mb-6">
        Block <span className="text-arch-orange">Details</span>
      </h1>
      <div className="bg-arch-black shadow-lg rounded-lg p-6 mb-6">
        <div className="grid grid-cols-1 md:grid-cols-2 gap-4">
          <div className="flex items-center">
            <Database className="text-arch-orange mr-2" size={20} />
            <p><strong className="text-arch-orange">Height:</strong> {blockDetails.height}</p>
          </div>
          <div className="flex items-center">
            <Clock className="text-arch-orange mr-2" size={20} />
            <p><strong className="text-arch-orange">Timestamp:</strong> {new Date(blockDetails.timestamp * 1000).toLocaleString()}</p>
          </div>
          <div className="flex items-center col-span-2">
            <Hash className="text-arch-orange mr-2" size={20} />
            <p className="truncate"><strong className="text-arch-orange">Hash:</strong> {blockDetails.hash}</p>
          </div>
          <div className="flex items-center col-span-2">
            <Hash className="text-arch-orange mr-2" size={20} />
            <p className="truncate"><strong className="text-arch-orange">Previous Block Hash:</strong> {blockDetails.previous_block_hash}</p>
          </div>
        </div>
      </div>
      <h2 className="text-2xl font-semibold mb-4 text-arch-orange">Transactions</h2>
      {blockDetails.transactions.length > 0 ? (
        <ul className="space-y-4">
          <AnimatePresence>
            {blockDetails.transactions.map((txId, index) => (
              <motion.li
                key={txId}
                initial={{ opacity: 0, y: -10 }}
                animate={{ opacity: 1, y: 0 }}
                exit={{ opacity: 0, y: -10 }}
                transition={{ duration: 0.2 }}
                className="bg-arch-black p-4 rounded-lg shadow-md"
              >
                <div 
                  className="flex justify-between items-center cursor-pointer"
                  onClick={() => toggleTxExpansion(txId)}
                >
                  <span className="text-arch-orange mr-2 font-semibold">{index + 1}.</span>
                  <span className="truncate flex-grow text-arch-white hover:text-arch-orange transition-colors duration-300">{txId}</span>
                  {expandedTx === txId ? <ChevronUp className="text-arch-orange" /> : <ChevronDown className="text-arch-orange" />}
                </div>
                <AnimatePresence>
                  {expandedTx === txId && transactionDetails && (
                    <motion.div
                      initial={{ opacity: 0, height: 0 }}
                      animate={{ opacity: 1, height: 'auto' }}
                      exit={{ opacity: 0, height: 0 }}
                      transition={{ duration: 0.3 }}
                    >
                      {renderTransactionDetails(transactionDetails)}
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
    </div>
  );
};

export default BlockDetailsPage;