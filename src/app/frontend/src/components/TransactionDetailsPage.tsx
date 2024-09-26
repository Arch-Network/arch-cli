import React, { useState, useEffect } from 'react';
import { useParams, Link } from 'react-router-dom';
import { ArrowLeft, Clock, Hash, Database, CheckCircle, AlertCircle, Layers, FileText, User, Code, Bitcoin } from 'lucide-react';
import { Buffer } from 'buffer';

const INDEXER_API_URL = import.meta.env.VITE_INDEXER_API_URL || 'http://localhost:3003/api';

interface TransactionDetails {
  txid: string;
  block_height: number;
  data: {
    version: number;
    signatures: string[];
    message: {
      signers: string[];
      instructions: {
        program_id: string;
        accounts: {
          pubkey: string;
          is_signer: boolean;
          is_writable: boolean;
        }[];
        data: number[];
      }[];
    };
  };
  status: number;
  bitcoin_txids: string[];
}

const TransactionDetailsPage: React.FC = () => {
  const { txId } = useParams<{ txId: string }>();
  const [transaction, setTransaction] = useState<TransactionDetails | null>(null);
  const [loading, setLoading] = useState<boolean>(true);
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    const fetchTransactionDetails = async () => {
      if (!txId) return;

      try {
        setLoading(true);
        const response = await fetch(`${INDEXER_API_URL}/transactions/${txId}`);
        if (!response.ok) {
          throw new Error('Failed to fetch transaction details');
        }
        const txDetails = await response.json();
        setTransaction(txDetails);
      } catch (err) {
        console.error('Error fetching transaction details:', err);
        setError('Failed to fetch transaction details. Please try again.');
      } finally {
        setLoading(false);
      }
    };

    fetchTransactionDetails();
  }, [txId]);

  if (loading) {
    return (
      <div className="flex justify-center items-center h-screen">
        <div className="animate-spin rounded-full h-32 w-32 border-t-2 border-b-2 border-arch-orange"></div>
      </div>
    );
  }

  if (error) {
    return <div className="text-center py-4 text-arch-orange">{error}</div>;
  }

  if (!transaction) {
    return <div className="text-center py-4 text-arch-white">No transaction details found.</div>;
  }

  const renderInstructionData = (data: number[]) => {
    if (data.length === 37 && data[0] === 0) {
      const bitcoinTxId = Buffer.from(data.slice(1, 33)).toString('hex');
      const output = Buffer.from(data.slice(33)).readUInt32LE(0);
      return (
        <div className="bg-arch-gray p-4 rounded-lg mt-2">
          <p className="font-semibold">Create Account Instruction</p>
          <p><span className="font-semibold">Bitcoin TxID:</span> {bitcoinTxId}</p>
          <p><span className="font-semibold">Output:</span> {output}</p>
        </div>
      );
    } else {
      return (
        <pre className="whitespace-pre-wrap break-all bg-arch-gray p-4 rounded-lg mt-2 text-xs overflow-x-auto">
          {`[${data.join(', ')}]`}
        </pre>
      );
    }
  };

  return (
    <div className="max-w-6xl mx-auto px-4 py-8 text-arch-white">
      <Link to="/transactions" className="text-arch-orange hover:underline mb-6 inline-flex items-center transition-colors duration-300">
        <ArrowLeft className="mr-2" /> Back to Transaction History
      </Link>
      <h1 className="text-4xl font-bold mb-8">
        Transaction <span className="text-arch-orange">Details</span>
      </h1>
      <div className="bg-arch-black shadow-lg rounded-lg p-6 mb-8">
        <div className="grid grid-cols-1 md:grid-cols-2 gap-6">
          <div className="flex items-start">
            <Hash className="text-arch-orange mr-3 mt-1 flex-shrink-0" size={24} />
            <div>
              <p className="font-semibold text-arch-orange mb-1">Transaction ID:</p>
              <p className="break-all">{transaction.txid}</p>
            </div>
          </div>
          <div className="flex items-start">
            <Clock className="text-arch-orange mr-3 mt-1 flex-shrink-0" size={24} />
            <div>
              <p className="font-semibold text-arch-orange mb-1">Status:</p>
              <p>{transaction.status === 0 ? 'Processing' : 'Processed'}</p>
            </div>
          </div>
          <div className="flex items-start">
            <Database className="text-arch-orange mr-3 mt-1 flex-shrink-0" size={24} />
            <div>
              <p className="font-semibold text-arch-orange mb-1">Block Height:</p>
              <Link to={`/block/${transaction.block_height}`} className="text-arch-white hover:text-arch-orange transition-colors duration-300">
                {transaction.block_height}
              </Link>
            </div>
          </div>
          <div className="flex items-start">
            <FileText className="text-arch-orange mr-3 mt-1 flex-shrink-0" size={24} />
            <div>
              <p className="font-semibold text-arch-orange mb-1">Version:</p>
              <p>{transaction.data.version}</p>
            </div>
          </div>
        </div>
      </div>
      <div className="bg-arch-black shadow-lg rounded-lg p-6 mb-8">
        <h2 className="text-2xl font-semibold mb-4 text-arch-orange flex items-center">
          <User className="mr-2" size={24} /> Signatures
        </h2>
        <ul className="space-y-2">
          {transaction.data.signatures.map((sig, index) => (
            <li key={index} className="bg-arch-gray p-3 rounded-lg flex items-center justify-between">
              <span className="text-sm break-all mr-2">{sig}</span>
              <button
                className="text-arch-orange hover:underline flex-shrink-0"
                onClick={() => navigator.clipboard.writeText(sig)}
              >
                Copy
              </button>
            </li>
          ))}
        </ul>
      </div>
      <div className="bg-arch-black shadow-lg rounded-lg p-6 mb-8">
        <h2 className="text-2xl font-semibold mb-4 text-arch-orange flex items-center">
          <User className="mr-2" size={24} /> Signers
        </h2>
        <ul className="space-y-2">
          {transaction.data.message.signers.map((signer, index) => (
            <li key={index} className="bg-arch-gray p-3 rounded-lg flex items-center justify-between">
              <span className="text-sm break-all mr-2">{signer}</span>
              <button
                className="text-arch-orange hover:underline flex-shrink-0"
                onClick={() => navigator.clipboard.writeText(signer)}
              >
                Copy
              </button>
            </li>
          ))}
        </ul>
      </div>
      <div className="bg-arch-black shadow-lg rounded-lg p-6 mb-8">
        <h2 className="text-2xl font-semibold mb-4 text-arch-orange flex items-center">
          <Code className="mr-2" size={24} /> Instructions
        </h2>
        {transaction.data.message.instructions.map((instruction, index) => (
          <div key={index} className="mb-6 bg-arch-gray p-4 rounded-lg">
            <h3 className="text-xl font-semibold text-arch-orange mb-3">Instruction {index + 1}</h3>
            <p className="mb-2"><span className="font-semibold">Program ID:</span> {instruction.program_id}</p>
            <p className="font-semibold mb-2">Accounts:</p>
            <ul className="list-disc list-inside mb-4">
              {instruction.accounts.map((account, accIndex) => (
                <li key={accIndex} className="ml-4">
                  {account.pubkey}
                  {account.is_signer && <span className="text-arch-orange ml-2">(Signer)</span>}
                  {account.is_writable && <span className="text-arch-orange ml-2">(Writable)</span>}
                </li>
              ))}
            </ul>
            <p className="font-semibold mb-2">Data:</p>
            {renderInstructionData(instruction.data)}
          </div>
        ))}
      </div>
      <div className="bg-arch-black shadow-lg rounded-lg p-6">
        <h2 className="text-2xl font-semibold mb-4 text-arch-orange flex items-center">
          <Bitcoin className="mr-2" size={24} /> Bitcoin TxIDs
        </h2>
        {transaction.bitcoin_txids.length > 0 ? (
          <ul className="space-y-2">
            {transaction.bitcoin_txids.map((txid, index) => (
              <li key={index} className="bg-arch-gray p-3 rounded-lg break-all">
                {txid}
              </li>
            ))}
          </ul>
        ) : (
          <p>None</p>
        )}
      </div>
    </div>
  );
};

export default TransactionDetailsPage;