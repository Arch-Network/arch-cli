import React from 'react';
import { Link } from 'react-router-dom';
import { motion } from 'framer-motion';

interface BlockData {
  height: number;
  hash: string;
  timestamp: number;
  transactions: string[];
}

interface BlockListProps {
  blocks: BlockData[];
}

const BlockList: React.FC<BlockListProps> = ({ blocks }) => {
  return (
    <div className="overflow-x-auto">
      <table className="min-w-full bg-arch-gray rounded-lg overflow-hidden">
        <thead className="bg-arch-black">
          <tr>
            <th className="px-4 py-2 text-left text-arch-orange">Block Number</th>
            <th className="px-4 py-2 text-left text-arch-orange">Block Hash</th>
            <th className="px-4 py-2 text-left text-arch-orange">Time</th>
            <th className="px-4 py-2 text-left text-arch-orange">Tx Count</th>
          </tr>
        </thead>
        <tbody>
          {blocks.map((block, index) => (
            <motion.tr
              key={block.hash}
              initial={{ opacity: 0, y: 20 }}
              animate={{ opacity: 1, y: 0 }}
              transition={{ duration: 0.3, delay: index * 0.05 }}
              className="border-b border-arch-gray-700 hover:bg-arch-black transition-colors duration-300"
            >
              <td className="px-4 py-2 text-arch-white">
                <Link to={`/block/${block.height}`} className="hover:text-arch-orange transition-colors duration-300">
                  {block.height}
                </Link>
              </td>
              <td className="px-4 py-2 text-arch-white">
                <Link to={`/block/${block.hash}`} className="hover:text-arch-orange transition-colors duration-300">
                  {block.hash.length > 25 ? `${block.hash.substring(0, 12)}...${block.hash.substring(block.hash.length - 10)}` : block.hash}
                </Link>
              </td>
              <td className="px-4 py-2 text-arch-white">
                {new Date(block.timestamp * 1000).toLocaleString()}
              </td>
              <td className="px-4 py-2 text-arch-white">{block.transactions.length}</td>
            </motion.tr>
          ))}
        </tbody>
      </table>
    </div>
  );
};

export default BlockList;