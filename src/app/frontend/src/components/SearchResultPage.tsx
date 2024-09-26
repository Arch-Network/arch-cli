import React, { useState, useEffect } from 'react';
import { useParams, Link } from 'react-router-dom';
import { AlertCircle, ArrowLeft } from 'lucide-react';
import axios from 'axios';

const INDEXER_API_URL = import.meta.env.VITE_INDEXER_API_URL || 'http://localhost:3003/api';

interface SearchResult {
  type: 'transaction' | 'block';
  data: any;
}

const SearchResultPage: React.FC = () => {
  const { term } = useParams<{ term: string }>();
  const [loading, setLoading] = useState<boolean>(true);
  const [error, setError] = useState<string | null>(null);
  const [result, setResult] = useState<SearchResult | null>(null);

  useEffect(() => {
    const searchTerm = async () => {
      if (!term) {
        setError('No search term provided.');
        setLoading(false);
        return;
      }

      try {
        setLoading(true);
        const response = await axios.get(`${INDEXER_API_URL}/search?term=${term}`);
        setResult(response.data);
      } catch (err) {
        console.error('Error during search:', err);
        setError('An error occurred during the search. Please try again.');
      } finally {
        setLoading(false);
      }
    };

    searchTerm();
  }, [term]);

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
        </div>
      </div>
    );
  }

  if (!result) {
    return (
      <div className="max-w-2xl mx-auto mt-8 p-6 bg-arch-black rounded-lg shadow-lg">
        <Link to="/transactions" className="text-arch-orange hover:underline mb-4 inline-flex items-center">
          <ArrowLeft className="mr-2" /> Back to Transaction History
        </Link>
        <div className="text-center py-4">
          <AlertCircle className="mx-auto h-12 w-12 text-arch-orange mb-4" />
          <h2 className="text-2xl font-bold mb-4 text-arch-white">No Results Found</h2>
          <p className="text-arch-white mb-4">No matching transaction or block found for the given term.</p>
        </div>
      </div>
    );
  }

  return (
    <div className="max-w-4xl mx-auto mt-8 p-6 bg-arch-black rounded-lg shadow-lg">
      <Link to="/transactions" className="text-arch-orange hover:underline mb-4 inline-flex items-center">
        <ArrowLeft className="mr-2" /> Back to Transaction History
      </Link>
      <h2 className="text-2xl font-bold mb-4 text-arch-white">
        {result.type === 'transaction' ? 'Transaction Details' : 'Block Details'}
      </h2>
      <pre className="bg-arch-gray p-4 rounded-lg overflow-x-auto text-arch-white">
        {JSON.stringify(result.data, null, 2)}
      </pre>
    </div>
  );
};

export default SearchResultPage;