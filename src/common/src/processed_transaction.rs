use anyhow::Result;
use borsh::{BorshDeserialize, BorshSerialize};
use serde::{Deserialize, Serialize};

use crate::runtime_transaction::RuntimeTransaction;

#[derive(Clone, Debug, Deserialize, Serialize, BorshDeserialize, BorshSerialize)]
pub enum Status {
    Processing,
    Processed,
}

#[derive(Clone, Debug, Serialize, Deserialize, BorshSerialize, BorshDeserialize)]
pub struct ProcessedTransaction {
    pub runtime_transaction: RuntimeTransaction,
    pub status: Status,
    pub bitcoin_txids: Vec<String>,
}

impl ProcessedTransaction {
    pub fn txid(&self) -> String {
        self.runtime_transaction.txid()
    }

    pub fn to_vec(&self) -> Result<Vec<u8>> {
        let mut serialized = vec![];

        serialized.push(self.status.clone() as u8);
        serialized.extend((self.runtime_transaction.serialize().len() as u64).to_le_bytes());
        serialized.extend(self.runtime_transaction.serialize());

        serialized.extend((self.bitcoin_txids.len() as u64).to_le_bytes());
        for bitcoin_txid in &self.bitcoin_txids {
            serialized.extend(hex::decode(bitcoin_txid)?);
        }
        Ok(serialized)
    }

    pub fn from_vec(data: &[u8]) -> Result<Self> {
        let status = match data[0] {
            0 => Status::Processing,
            1 => Status::Processed,
            _ => unreachable!("status doesn't exist"),
        };
        let runtime_transaction_len = u64::from_le_bytes(data[1..9].try_into().unwrap()) as usize;
        let mut size = 9;
        let runtime_transaction =
            RuntimeTransaction::from_slice(&data[9..9 + runtime_transaction_len])?;
        size += runtime_transaction_len;

        let bitcoin_txids_len =
            u64::from_le_bytes(data[size..size + 8].try_into().unwrap()) as usize;
        size += 8;
        let mut bitcoin_txids = vec![];
        for _ in 0..bitcoin_txids_len {
            bitcoin_txids.push(hex::encode(&data[size + 32..size + 64]));
            size += 64;
        }

        Ok(ProcessedTransaction {
            runtime_transaction,
            status,
            bitcoin_txids,
        })
    }
}
