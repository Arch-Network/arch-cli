// src/utils/cryptoHelpers.ts

import * as secp256k1 from 'noble-secp256k1';
import { Pubkey, Instruction, Message, AccountMeta, RuntimeTransaction } from 'arch-typescript-sdk';
import { Buffer } from 'buffer';

export interface Transaction {
  version: number;
  signatures: string[];
  message: Message;
}

export function hexToUint8Array(hex: string): Uint8Array {
  const cleanHex = hex.replace(/^0x/, '').replace(/\s/g, '');
  if (cleanHex.length % 2 !== 0) {
    throw new Error(`Invalid hex string length: ${cleanHex.length}`);
  }
  const bytes = new Uint8Array(cleanHex.length / 2);
  for (let i = 0; i < bytes.length; i++) {
    const byte = parseInt(cleanHex.substr(i * 2, 2), 16);
    if (isNaN(byte)) {
      throw new Error(`Invalid hex string at position ${i * 2}`);
    }
    bytes[i] = byte;
  }
  return bytes;
}

export function generatePrivateKey(): string {
  const privateKeyBytes = new Uint8Array(32);
  crypto.getRandomValues(privateKeyBytes);
  return Array.from(privateKeyBytes).map(byte => byte.toString(16).padStart(2, '0')).join('');
}

export function generatePubkeyFromPrivateKey(privateKeyHex: string): Pubkey {
    const privateKeyBytes = hexToUint8Array(privateKeyHex);
    if (privateKeyBytes.length !== 32) {
      throw new Error(`Invalid private key length: ${privateKeyBytes.length} bytes`);
    }
    const publicKeyBytes = secp256k1.getPublicKey(privateKeyBytes);
    const xOnlyPubKey = publicKeyBytes.slice(1, 33);
    return new Pubkey(xOnlyPubKey);
  }

  export async function createTransaction(
    programId: string,
    accountPubkey: string,
    accountIsSigner: boolean,
    accountIsWritable: boolean,
    instructionData: string,
    privateKey: string
  ): Promise<RuntimeTransaction> {
    const programIdPubkey = Pubkey.fromString(programId);
    const accountPubkeyObj = Pubkey.fromString(accountPubkey);
    const accountMeta: AccountMeta = {
      pubkey: accountPubkeyObj,
      is_signer: accountIsSigner,
      is_writable: accountIsWritable
    };
    const instructionDataBytes = hexToUint8Array(instructionData);
    const instruction: Instruction = {
      program_id: programIdPubkey,
      accounts: [accountMeta],
      data: Array.from(instructionDataBytes)
    };
    const signerPubkey = generatePubkeyFromPrivateKey(privateKey);
    const message: Message = {
      signers: [signerPubkey],
      instructions: [instruction]
    };
    
    // // Implement proper signature generation
    // const messageBytes = encodeMessage(message);
    // const messageHash = await secp256k1.utils.sha256(messageBytes);
    // const signature = await secp256k1.schnorr.sign(messageHash, hexToUint8Array(privateKey));
    
    const transaction: RuntimeTransaction = {
      version: 0,
      signatures: [],
      // signatures: [Buffer.from(signature).toString('hex')],
      message: message
    };
    return transaction;
  }