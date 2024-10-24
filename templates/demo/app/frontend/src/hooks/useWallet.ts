import { useState, useEffect } from 'react';
import { AddressPurpose, request } from 'sats-connect';
import { generatePrivateKey, generatePubkeyFromPrivateKey } from '../utils/cryptoHelpers';

interface WalletState {
  isConnected: boolean;
  publicKey: string | null;
  privateKey: string | null;
  connect: () => Promise<void>;
  disconnect: () => void;
  signMessage: (message: string) => Promise<string>;
}

export function useWallet() {
  const NETWORK = import.meta.env.VITE_NETWORK || 'development';
  const [state, setState] = useState<WalletState>({
    isConnected: false,
    publicKey: null,
    privateKey: null,
    connect: async () => {},
    disconnect: () => {},
    signMessage: async () => '',
  });

  const connectRegtest = async () => {
    const privateKey = generatePrivateKey();
    const publicKey = generatePubkeyFromPrivateKey(privateKey);
    
    setState(prev => ({
      ...prev,
      isConnected: true,
      privateKey,
      publicKey: publicKey.toString(),
    }));
  };

  const connectWallet = async () => {    
    try {
      const result = await request('getAddresses', {
        purposes: [AddressPurpose.Ordinals],
        message: 'Connect to Graffiti Wall',
      });

      console.log(result);

      if (result.result.addresses && result.result.addresses.length > 0) {
        console.log(result.result.addresses[0].publicKey);
        setState(prev => ({
          ...prev,
          isConnected: true,
          publicKey: result.result.addresses[0].publicKey,
          privateKey: null,
        }));
      }
    } catch (error) {
      console.error('Error connecting wallet:', error);
      throw error;
    }
  };

  const connect = async () => {
    if (NETWORK === 'development') {
      await connectRegtest();
    } else {
      await connectWallet();
    }
  };

  const disconnect = () => {
    setState(prev => ({
      ...prev,
      isConnected: false,
      publicKey: null,
      privateKey: null,
    }));
  };

  const signMessage = async (message: string): Promise<string> => {
    if (!state.isConnected) throw new Error('Wallet not connected');

    if (NETWORK === 'development' && state.privateKey) {
      // Use local signing for regtest
      // Implement your local signing logic here using the private key
      return 'signed-message';
    } else {
      // Use wallet signing for testnet/mainnet
      const signResult = await request('signMessage', {
        payload: {
          message,
          address: state.publicKey!,
        },
      });
      return signResult.signature;
    }
  };

  useEffect(() => {
    setState(prev => ({
      ...prev,
      connect,
      disconnect,
      signMessage,
    }));
  }, []);

  return state;
}