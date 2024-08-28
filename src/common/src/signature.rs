use borsh::{BorshDeserialize, BorshSerialize};
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize, BorshSerialize, BorshDeserialize)]
pub struct Signature(pub Vec<u8>);

impl Signature {
    pub fn serialize(&self) -> Vec<u8> {
        self.0.clone()
    }

    pub fn from_slice(data: &[u8]) -> Self {
        Self(data[..64].to_vec())
    }
}
