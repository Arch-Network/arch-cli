import React, { useState, useEffect } from 'react';
import { useParams, useNavigate, Link } from 'react-router-dom';
import { ArchRpcClient } from 'arch-typescript-sdk';
import { AlertCircle, ArrowLeft } from 'lucide-react';

const client = new ArchRpcClient(import.meta.env.VITE_RPC_URL as string);

const SearchResultPage: React.FC = () => {
  const { term } = useParams<{ term: string }>();
  const navigate = useNavigate();
  const [loading, setLoading] = useState<boolean>(true);
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    const searchTerm = async () => {
      if (!term) {
        setError('No search term provided.');
        setLoading(false);
        return;
      }

      try {
        setLoading(true);
        if (term.length === 64) {
          // Try to get block by hash
          try {
            await client.getBlock(term);
            navigate(`/block/${term}`);
            return;
          } catch (blockError) {
            // If it's not a block hash, try to get transaction
            try {
              await client.getProcessedTransaction(term);
              navigate(`/transaction/${term}`);
              return;
            } catch (txError) {
              setError('No block or transaction found with the given ID.');
            }
          }
        } else if (!isNaN(Number(term))) {
          // Assume it's a block height
          try {
            const blockHash = await client.getBlockHash(Number(term));
            navigate(`/block/${blockHash}`);
            return;
          } catch (heightError) {
            setError('No block found with the given height.');
          }
        } else {
          setError('Invalid search term. Please enter a valid block hash, transaction ID, or block height.');
        }
      } catch (err) {
        console.error('Error during search:', err);
        setError('An error occurred during the search. Please try again.');
      } finally {
        setLoading(false);
      }
    };

    searchTerm();
  }, [term, navigate]);

  if (loading) {
    return (
      <div className="flex justify-center items-center h-screen">
        <div className="animate-spin rounded-full h-32 w-32 border-t-2 border-b-2 border-arch-orange"></div>
      </div>
    );
  }

  if (error) {
    return (
      <div className="max-w-2xl mx-auto mt-8 p-6 bg-arch-black rounded-lg shadow-lg">
        <Link to="/transactions" className="text-arch-orange hover:underline mb-4 inline-flex items-center">
          <ArrowLeft className="mr-2" /> Back to Transaction History
        </Link>
        <div className="text-center py-4">
          <AlertCircle className="mx-auto h-12 w-12 text-arch-orange mb-4" />
          <h2 className="text-2xl font-bold mb-4 text-arch-white">Search Error</h2>
          <p className="text-arch-white mb-4">{error}</p>
          <p className="text-arch-gray-400">
            Please check your input and try again. You can search for:
          </p>
          <ul className="list-disc list-inside text-arch-gray-400 mt-2">
            <li>Block hash (64 characters)</li>
            <li>Transaction ID (64 characters)</li>
            <li>Block height (number)</li>
          </ul>
        </div>
      </div>
    );
  }

  return null; // This component should always redirect or show an error
};

export default SearchResultPage;