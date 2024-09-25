import React from 'react';
import { AlertCircle } from 'lucide-react';

interface ErrorMessageProps {
  message: string;
}

const ErrorMessage: React.FC<ErrorMessageProps> = ({ message }) => {
  return (
    <div className="flex flex-col items-center justify-center h-64 text-arch-white">
      <AlertCircle className="w-16 h-16 text-arch-orange mb-4" />
      <h2 className="text-2xl font-bold mb-2">Oops! Something went wrong</h2>
      <p className="text-center mb-4">{message}</p>
      <ol className="list-decimal list-inside text-left">
        <li className="mb-2">Run <code className="bg-arch-gray px-2 py-1 rounded">arch-cli server start</code> to ensure the servers are up and running.</li>
        <li className="mb-2">Check if the RPC server specified in your configuration is active and accessible.</li>
        <li>If the issue persists, please try again later or contact support.</li>
      </ol>
    </div>
  );
};

export default ErrorMessage;