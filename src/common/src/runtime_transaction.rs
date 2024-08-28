use anyhow::{anyhow, Result};
use arch_program::message::Message;
use borsh::{BorshDeserialize, BorshSerialize};
use serde::{Deserialize, Serialize};
use sha256::digest;

use crate::signature::Signature;

pub const RUNTIME_TX_SIZE_LIMIT: usize = 1024;

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize, BorshDeserialize, BorshSerialize)]
pub struct RuntimeTransaction {
    pub version: u32,
    pub signatures: Vec<Signature>,
    pub message: Message,
}

impl RuntimeTransaction {
    pub fn txid(&self) -> String {
        digest(digest(self.serialize()))
    }

    pub fn serialize(&self) -> Vec<u8> {
        let mut serilized = vec![];

        serilized.extend(self.version.to_le_bytes());
        serilized.push(self.signatures.len() as u8);
        for signature in self.signatures.iter() {
            serilized.extend(&signature.serialize());
        }
        serilized.extend(self.message.serialize());

        serilized
    }

    pub fn from_slice(data: &[u8]) -> Result<Self> {
        let mut size = 4;
        let signatures_len = data[size] as usize;
        size += 1;
        let mut signatures = Vec::with_capacity(data[size] as usize);

        for _ in 0..signatures_len {
            signatures.push(Signature::from_slice(&data[size..(size + 64)]));
            size += 64;
        }
        let message = Message::from_slice(&data[size..]);

        Ok(Self {
            version: u32::from_le_bytes(data[..4].try_into().unwrap()),
            signatures,
            message,
        })
    }

    pub fn hash(&self) -> String {
        digest(digest(self.serialize()))
    }

    pub fn check_tx_size_limit(&self) -> Result<()> {
        let serialized_tx = self.serialize();
        if serialized_tx.len() > RUNTIME_TX_SIZE_LIMIT {
            Err(anyhow!(format!(
                "runtime tx size exceeds RUNTIME_TX_SIZE_LIMIT {} {}",
                serialized_tx.len(),
                RUNTIME_TX_SIZE_LIMIT
            )))
        } else {
            Ok(())
        }
    }
}
