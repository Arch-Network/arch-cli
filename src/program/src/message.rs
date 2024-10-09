use crate::instruction::Instruction;
use crate::pubkey::Pubkey;

use borsh::{BorshDeserialize, BorshSerialize};
use serde::{Deserialize, Serialize};
use sha256::digest;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, BorshSerialize, BorshDeserialize)]
pub struct Message {
    pub signers: Vec<Pubkey>,
    pub instructions: Vec<Instruction>,
}

impl Message {
    pub fn serialize(&self) -> Vec<u8> {
        let mut serialized = vec![];

        serialized.push(self.signers.len() as u8);
        for signer in self.signers.iter() {
            serialized.extend(&signer.serialize());
        }
        serialized.push(self.instructions.len() as u8);
        for instruction in self.instructions.iter() {
            serialized.extend(&instruction.serialize());
        }

        serialized
    }

    pub fn from_slice(data: &[u8]) -> Self {
        let mut size = 0;

        let signers_len = data[size] as usize;
        size += 1;
        let mut signers = Vec::with_capacity(signers_len);
        for _ in 0..signers_len {
            signers.push(Pubkey::from_slice(&data[size..(size + 32)]));
            size += 32;
        }

        let instructions_len = data[size] as usize;
        size += 1;
        let mut instructions = Vec::with_capacity(instructions_len);
        for _ in 0..instructions_len {
            instructions.push(Instruction::from_slice(&data[size..]));
            size += instructions.last().unwrap().serialize().len();
        }

        Self {
            signers,
            instructions,
        }
    }

    pub fn hash(&self) -> String {
        let serialized_message = self.serialize();
        let first_hash = digest(serialized_message);
        digest(first_hash.as_bytes())
    }
}

#[cfg(test)]
mod tests {
    use crate::{account::AccountMeta, message::Message, pubkey::Pubkey};

    use super::Instruction;

    #[test]
    fn test_serialize_deserialize() {
        let instruction = Instruction {
            program_id: Pubkey::system_program(),
            accounts: vec![AccountMeta {
                pubkey: Pubkey::system_program(),
                is_signer: true,
                is_writable: true,
            }],
            data: vec![10; 364],
        };

        let message = Message {
            instructions: vec![],
            signers: vec![],
        };

        assert_eq!(message, Message::from_slice(&message.serialize()));

        let message = Message {
            instructions: vec![instruction],
            signers: vec![Pubkey::system_program()],
        };

        assert_eq!(message, Message::from_slice(&message.serialize()));
    }
}
