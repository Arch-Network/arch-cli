import React, { useState, useEffect } from 'react';
import { useParams, Link, useNavigate } from 'react-router-dom';
import { Buffer } from 'buffer';
import { motion, AnimatePresence } from 'framer-motion';
import { ArrowLeft, Clock, Hash, Database, ChevronDown, ChevronUp, CheckCircle, AlertCircle, FileText, Layers, Bitcoin, User, Code } from 'lucide-react';
import bs58 from 'bs58';

const INDEXER_API_URL = import.meta.env.VITE_INDEXER_API_URL || 'http://localhost:3003/api';

interface BlockDetails {
  height: number;
  hash: string;
  previous_block_hash: string;
  transactions: string[];
  timestamp: number;
}

const BlockDetailsPage: React.FC = () => {
  const { blockHashOrHeight } = useParams<{ blockHashOrHeight: string }>();
  const navigate = useNavigate();
  const [blockDetails, setBlockDetails] = useState<BlockDetails | null>(null);
  const [loading, setLoading] = useState<boolean>(true);
  const [error, setError] = useState<string | null>(null);
  const [expandedTx, setExpandedTx] = useState<string | null>(null);
  const [transactionDetails, setTransactionDetails] = useState<ProcessedTransaction | null>(null);

  const handlePreviousBlockClick = () => {
    if (blockDetails && blockDetails.previous_block_hash) {
      navigate(`/block/${blockDetails.previous_block_hash}`);
    }
  };

  useEffect(() => {
    const fetchBlockDetails = async () => {
      try {
        setLoading(true);
        let response;
        if (blockHashOrHeight!.length === 64) {
          response = await fetch(`${INDEXER_API_URL}/blocks/${blockHashOrHeight}`);
        } else {
          response = await fetch(`${INDEXER_API_URL}/blocks/${parseInt(blockHashOrHeight!)}`);
        }
        if (!response.ok) {
          throw new Error('Failed to fetch block details');
        }
        const block = await response.json();
        setBlockDetails({
          height: block.height,
          hash: block.hash,
          previous_block_hash: block.previous_block_hash,
          transactions: block.transactions || [],
          timestamp: block.timestamp,
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
      const response = await fetch(`${INDEXER_API_URL}/transactions/${txId}`);
      if (!response.ok) {
        throw new Error('Failed to fetch transaction details');
      }
      const txDetails = await response.json();
      setTransactionDetails(txDetails);
    } catch (err) {
      console.error('Error fetching transaction details:', err);
      setError('Failed to fetch transaction details. Please try again.');
    } finally {
      setLoading(false);
    }
  };

  const wrapData = (data: string, chunkSize: number = 64): string => {
    const regex = new RegExp(`.{1,${chunkSize}}`, 'g');
    return data.match(regex)?.join('\n') || data;
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

  const renderTransactionDetails = (txDetails: ProcessedTransaction) => {
    const renderStatusIcon = (status: number) => {
      return status === 0 ? (
        <AlertCircle className="text-yellow-500" size={20} />
      ) : (
        <CheckCircle className="text-green-500" size={20} />
      );
    };
  
    const renderCopyButton = (text: string) => (
      <button
        className="ml-2 text-arch-orange hover:underline"
        onClick={() => navigator.clipboard.writeText(text)}
      >
        Copy
      </button>
    );
  
    const renderInstructionData = (data: number[]) => {
      if (data.length === 37 && data[0] === 0) {
        const bitcoinTxId = Buffer.from(data.slice(1, 33)).toString('hex');
        const output = Buffer.from(data.slice(33)).readUInt32LE(0).toString();
        return (
          <div>
            <p><strong>Create Account Instruction</strong></p>
            <p><strong>Bitcoin TxID:</strong> {bitcoinTxId}</p>
            <p><strong>Output:</strong> {output}</p>
          </div>
        );
      } else {
        return (
          <pre className="whitespace-pre-wrap break-all bg-arch-black p-2 rounded mt-1 text-xs">
            {`[${data.join(', ')}]`}
          </pre>
        );
      }
    };
  
    return (
      <div className="grid grid-cols-1 gap-4 mt-4">
        {[
          {
            icon: renderStatusIcon(txDetails.status),
            title: "Status",
            content: txDetails.status === 0 ? 'Processing' : 'Processed'
          },
          {
            icon: <FileText className="text-arch-orange" size={20} />,
            title: "Version",
            content: txDetails.runtime_transaction.version
          },
          {
            icon: <Hash className="text-arch-orange" size={20} />,
            title: "Signatures",
            content: (
              <ul className="space-y-2">
                {txDetails.runtime_transaction.signatures.map((sig, index) => {
                  const base58Sig = bs58.encode(Buffer.from(sig));
                  return (
                    <li key={index} className="text-sm">
                      {base58Sig}
                      {renderCopyButton(base58Sig)}
                    </li>
                  );
                })}
              </ul>
            )
          },
          {
            icon: <User className="text-arch-orange" size={20} />,
            title: "Signers",
            content: (
              <ul className="space-y-2">
                {txDetails.runtime_transaction.message.signers.map((signer, index) => {
                  const base58Signer = bs58.encode(Buffer.from(signer));
                  return (
                    <li key={index} className="text-sm">
                      {base58Signer}
                      {renderCopyButton(base58Signer)}
                    </li>
                  );
                })}
              </ul>
            )
          },
          {
            icon: <Code className="text-arch-orange" size={20} />,
            title: "Instructions",
            content: (
              <>
                {txDetails.runtime_transaction.message.instructions.map((instruction, index) => (
                  <div key={index} className="mb-4">
                    <h4 className="text-arch-orange font-semibold">Instruction {index + 1}</h4>
                    <p><strong>Program ID:</strong> {Buffer.from(instruction.program_id).toString('hex')}</p>
                    <p><strong>Accounts:</strong></p>
                    <ul className="list-disc list-inside">
                      {instruction.accounts.map((account, accIndex) => (
                        <li key={accIndex}>
                          {Buffer.from(account.pubkey).toString('hex')}
                          {account.is_signer && ' (Signer)'}
                          {account.is_writable && ' (Writable)'}
                        </li>
                      ))}
                    </ul>
                    <p><strong>Data:</strong></p>
                    {renderInstructionData(instruction.data)}
                  </div>
                ))}
              </>
            )
          },
          {
            icon: <Bitcoin className="text-arch-orange" size={20} />,
            title: "Bitcoin TxIDs",
            content: txDetails.bitcoin_txids.length > 0 ? txDetails.bitcoin_txids.join(', ') : 'None'
          }
        ].map((item, index) => (
          <div key={index} className="bg-arch-gray rounded-lg p-4 flex items-start">
            <div className="mr-3 mt-1">{item.icon}</div>
            <div>
              <h3 className="text-arch-orange font-semibold mb-2">{item.title}</h3>
              {item.content}
            </div>
          </div>
        ))}
      </div>
    );
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
            <p><strong className="text-arch-orange">Timestamp:</strong> {new Date(Number(blockDetails.timestamp)).toLocaleString()}</p>
          </div>
          <div className="flex items-center col-span-2">
            <Hash className="text-arch-orange mr-2" size={20} />
            <p className="truncate"><strong className="text-arch-orange">Hash:</strong> {blockDetails.hash}</p>
          </div>
          <div className="flex items-center col-span-2">
            <Hash className="text-arch-orange mr-2" size={20} />
            <p className="truncate">
              <strong className="text-arch-orange">Previous Block Hash:</strong>
              <button
                onClick={handlePreviousBlockClick}
                className="ml-2 text-arch-white hover:text-arch-orange transition-colors duration-300"
              >
                {blockDetails.previous_block_hash}
              </button>
            </p>
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