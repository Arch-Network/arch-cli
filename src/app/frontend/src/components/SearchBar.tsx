import React, { useState } from 'react';
import { useNavigate } from 'react-router-dom';
import { Search } from 'lucide-react';
import { motion } from 'framer-motion';

const SearchBar: React.FC = () => {
  const [searchTerm, setSearchTerm] = useState('');
  const navigate = useNavigate();

  const handleSubmit = (e: React.FormEvent) => {
    e.preventDefault();
    if (searchTerm) {
      if (searchTerm.length === 64) {
        // Assume it's a transaction ID or block hash
        navigate(`/search/${searchTerm}`);
      } else if (!isNaN(Number(searchTerm))) {
        // Assume it's a block height
        navigate(`/block/${searchTerm}`);
      } else {
        // Invalid input
        alert('Please enter a valid block hash, transaction ID, or block height.');
      }
    }
  };

  return (
    <motion.form
      initial={{ opacity: 0, y: -20 }}
      animate={{ opacity: 1, y: 0 }}
      transition={{ duration: 0.3 }}
      onSubmit={handleSubmit}
      className="mb-6"
    >
      <div className="relative">
        <input
          type="text"
          value={searchTerm}
          onChange={(e) => setSearchTerm(e.target.value)}
          placeholder="Search by block hash, transaction ID, or block height"
          className="w-full px-4 py-2 bg-arch-gray text-arch-white border border-arch-gray rounded-lg focus:outline-none focus:ring-2 focus:ring-arch-orange transition-all duration-300"
        />
        <button
          type="submit"
          className="absolute right-2 top-1/2 transform -translate-y-1/2 text-arch-orange hover:text-arch-white transition-colors duration-300"
        >
          <Search size={20} />
        </button>
      </div>
    </motion.form>
  );
};

export default SearchBar;