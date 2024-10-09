use crate::account::AccountMeta;
use crate::instruction::Instruction;
use crate::pubkey::Pubkey;
use crate::utxo::UtxoMeta;

#[derive(Clone, PartialEq, Eq, Debug)]
pub enum SystemInstruction {
    CreateAccount(UtxoMeta),
    ExtendBytes(Vec<u8>),
}

impl SystemInstruction {
    pub fn serialise(&self) -> Vec<u8> {
        let mut serialized = vec![];

        match self {
            Self::CreateAccount(utxo) => {
                serialized.push(0);
                serialized.extend(utxo.serialize());
            }
            Self::ExtendBytes(bytes) => {
                serialized.push(1);
                serialized.extend(bytes);
            }
        }

        serialized
    }

    pub fn from_slice(data: &[u8]) -> Self {
        match data[0] {
            0 => Self::CreateAccount(UtxoMeta::from_slice(&data[1..])),
            1 => Self::ExtendBytes(data[1..].to_vec()),
            _ => {
                unreachable!("error deserializing system instruction")
            }
        }
    }

    pub fn new_create_account_instruction(
        txid: [u8; 32],
        vout: u32,
        pubkey: Pubkey,
    ) -> Instruction {
        Instruction {
            program_id: Pubkey::system_program(),
            accounts: vec![AccountMeta {
                pubkey,
                is_signer: true,
                is_writable: true,
            }],
            data: SystemInstruction::CreateAccount(UtxoMeta::from(txid, vout)).serialise(),
        }
    }

    pub fn new_extend_bytes_instruction(data: Vec<u8>, pubkey: Pubkey) -> Instruction {
        Instruction {
            program_id: Pubkey::system_program(),
            accounts: vec![AccountMeta {
                pubkey,
                is_signer: true,
                is_writable: true,
            }],
            data: SystemInstruction::ExtendBytes(data).serialise(),
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::utxo::UtxoMeta;

    use super::SystemInstruction;

    #[test]
    fn test_serialize_and_back() {
        let system_instruction = SystemInstruction::CreateAccount(UtxoMeta::from(
            hex::decode("b6fd4863c8603414e137d4ccb80297cfb7e88a56070cf03b2cb05a07f50e0c02")
                .unwrap()
                .try_into()
                .unwrap(),
            0,
        ));
        assert_eq!(
            system_instruction,
            SystemInstruction::from_slice(&system_instruction.serialise())
        );

        let system_instruction = SystemInstruction::ExtendBytes(vec![0, 4, 5, 5, 8, 9]);
        assert_eq!(
            system_instruction,
            SystemInstruction::from_slice(&system_instruction.serialise())
        );
    }
}
