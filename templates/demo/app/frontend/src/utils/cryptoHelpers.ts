// src/utils/cryptoHelpers.ts

import * as secp256k1 from 'noble-secp256k1';
import { Instruction, Message, AccountMeta, RuntimeTransaction } from '@saturnbtcio/arch-sdk';
import { PubkeyUtil } from '@saturnbtcio/arch-sdk';

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

export function generatePubkeyFromPrivateKey(privateKeyHex: string): Uint8Array {
  const privateKeyBytes = hexToUint8Array(privateKeyHex);
  if (privateKeyBytes.length !== 32) {
    throw new Error(`Invalid private key length: ${privateKeyBytes.length} bytes`);
  }
  const publicKeyBytes = secp256k1.getPublicKey(privateKeyBytes);
  return publicKeyBytes.slice(1, 33); // Return Uint8Array directly
}

export async function createTransaction(
programId: string,
accountPubkey: string,
accountIsSigner: boolean,
accountIsWritable: boolean,
instructionData: string,
privateKey: string
): Promise<RuntimeTransaction> {
// Convert program ID and account pubkey to Uint8Array using SDK utility
const programIdPubkey = PubkeyUtil.fromHex(programId);
const accountPubkeyObj = PubkeyUtil.fromHex(accountPubkey);

const accountMeta: AccountMeta = {
  pubkey: accountPubkeyObj,
  is_signer: accountIsSigner,
  is_writable: accountIsWritable
};

const instruction: Instruction = {
  program_id: programIdPubkey,
  accounts: [accountMeta],
  data: hexToUint8Array(instructionData)
};

const signerPubkey = generatePubkeyFromPrivateKey(privateKey);

const message: Message = {
  signers: [signerPubkey],
  instructions: [instruction]
};

const transaction: RuntimeTransaction = {
  version: 0,
  signatures: [], // Will be Uint8Array[]
  message: message
};

return transaction;
}