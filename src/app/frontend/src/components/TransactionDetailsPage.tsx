import React, { useState, useEffect } from 'react';
import { useParams, Link } from 'react-router-dom';
import { ArchRpcClient, ProcessedTransaction } from 'arch-typescript-sdk';
import { ArrowLeft, Clock, Hash, Database, CheckCircle, AlertCircle } from 'lucide-react';
import bs58 from 'bs58';

const client = new ArchRpcClient(import.meta.env.VITE_RPC_URL as string);

const TransactionDetailsPage: React.FC = () => {
  const { txId } = useParams<{ txId: string }>();
  const [transaction, setTransaction] = useState<ProcessedTransaction | null>(null);
  const [loading, setLoading] = useState<boolean>(true);
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    const fetchTransactionDetails = async () => {
      if (!txId) return;

      try {
        setLoading(true);
        const txDetails = await client.getProcessedTransaction(txId);
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

  return (
    <div className="p-4 max-w-6xl mx-auto text-arch-white">
      <Link to="/transactions" className="text-arch-orange hover:underline mb-4 inline-flex items-center transition-colors duration-300">
        <ArrowLeft className="mr-2" /> Back to Transaction History
      </Link>
      <h1 className="text-4xl font-bold mb-6">
        Transaction <span className="text-arch-orange">Details</span>
      </h1>
      <div className="bg-arch-black shadow-lg rounded-lg p-6 mb-6">
        <div className="grid grid-cols-1 md:grid-cols-2 gap-4">
          <div className="flex items-center">
            <Hash className="text-arch-orange mr-2" size={20} />
            <p className="truncate"><strong className="text-arch-orange">Transaction ID:</strong> {txId}</p>
          </div>
          <div className="flex items-center">
            <Clock className="text-arch-orange mr-2" size={20} />
            <p><strong className="text-arch-orange">Status:</strong> {transaction.status}</p>
          </div>
          <div className="flex items-center col-span-2">
            <Database className="text-arch-orange mr-2" size={20} />
            <p><strong className="text-arch-orange">Version:</strong> {transaction.runtime_transaction.version}</p>
          </div>
        </div>
      </div>
      {/* Add more transaction details here */}
    </div>
  );
};

export default TransactionDetailsPage;