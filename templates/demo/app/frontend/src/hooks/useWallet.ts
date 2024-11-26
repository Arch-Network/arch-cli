import { useState } from 'react';
import { AddressPurpose, request, MessageSigningProtocols } from 'sats-connect';
import { generatePrivateKey, generatePubkeyFromPrivateKey, hexToUint8Array } from '../utils/cryptoHelpers';
import * as secp256k1 from 'noble-secp256k1';

interface WalletState {
  isConnected: boolean;
  publicKey: string | null;
  privateKey: string | null;
  address: string | null;
}

export function useWallet() {
  const NETWORK = import.meta.env.VITE_NETWORK || 'development';
  const [state, setState] = useState<WalletState>(() => {
    // Initialize from localStorage
    const savedState = localStorage.getItem('walletState');
    if (savedState) {
      const parsed = JSON.parse(savedState);
      return {
        isConnected: parsed.isConnected,
        publicKey: parsed.publicKey,
        privateKey: parsed.privateKey,
        address: parsed.address,
      };
    }
    return {
      isConnected: false,
      publicKey: null,
      privateKey: null,
      address: null,
    };
  });

  const connectRegtest = async () => {
    const privateKey = generatePrivateKey();
    const publicKey = generatePubkeyFromPrivateKey(privateKey);

    const newState = {
      isConnected: true,
      privateKey,
      publicKey: publicKey.toString(),
      address: null,
    };
    setState(newState);
    localStorage.setItem('walletState', JSON.stringify(newState));
  };

  const connectWallet = async () => {
    try {
      const result = await request('getAddresses', {
        purposes: [AddressPurpose.Ordinals],
        message: 'Connect to Graffiti Wall',
      });
      console.log(`Addresses: ${JSON.stringify(result.result.addresses)}`);

      if (result.result.addresses && result.result.addresses.length > 0) {
        const newState = {
          isConnected: true,
          address: result.result.addresses[0].address,
          publicKey: result.result.addresses[0].publicKey,
          privateKey: null,
        };
        setState(newState);
        localStorage.setItem('walletState', JSON.stringify(newState));
      }
    } catch (error) {
      console.error('Error connecting wallet:', error);
      throw error;
    }
  };

  const connect = async () => {
    if (NETWORK === 'deveopment') {
      await connectRegtest();
    } else {
      await connectWallet();
    }
  };

  const disconnect = () => {
    localStorage.removeItem('walletState');
    setState({
      isConnected: false,
      publicKey: null,
      privateKey: null,
      address: null,
    });
  };

  const signMessage = async (message: string): Promise<string> => {
    if (!state.isConnected) throw new Error('Wallet not connected');

    if (NETWORK === 'regtest' && state.privateKey) {
      try {
        const messageBytes = new TextEncoder().encode(message);
        const messageHash = await crypto.subtle.digest('SHA-256', messageBytes);
        const hashArray = new Uint8Array(messageHash);
        const privateKeyBytes = hexToUint8Array(state.privateKey);
        const signature = await secp256k1.sign(hashArray, privateKeyBytes);
        return Buffer.from(signature).toString('hex');
      } catch (error) {
        console.error('Error signing message:', error);
        throw new Error('Failed to sign message');
      }
    } else {
      console.debug(`Signing message: ${message}`);
      try {
        console.log(`Signing key: ${state.publicKey}`);
        const signResult = await request('signMessage', {
          address: state.address!,
          message: message,
          protocol: MessageSigningProtocols.BIP322,
        });
        console.log(`Signature: ${signResult.result.signature}`);
        return signResult.result.signature;
      } catch (error) {
        console.error('Error signing with wallet:', error);
        throw error;
      }
    }
  };

  return {
    ...state,
    connect,
    disconnect,
    signMessage,
  };
}
