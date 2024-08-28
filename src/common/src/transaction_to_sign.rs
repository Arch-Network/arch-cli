use arch_program::{input_to_sign::InputToSign, pubkey::Pubkey};

#[derive(Debug, Clone, Default)]
pub struct TransactionToSign {
    pub tx_bytes: Vec<u8>,
    pub inputs_to_sign: Vec<InputToSign>,
}

impl TransactionToSign {
    pub fn from_slice(data: &[u8]) -> Self {
        let mut size = data[0] as usize + 1;
        let inputs_to_sign_length: usize = data[size] as usize;
        let mut program_return = TransactionToSign {
            tx_bytes: data[1..size].to_vec(),
            inputs_to_sign: vec![],
        };

        size += 1;
        for _ in 0..inputs_to_sign_length {
            let index = u32::from_le_bytes(
                data[size..size + 4]
                    .try_into()
                    .expect("slice with incorrect length"),
            );
            size += 4;
            let signer = Pubkey::from_slice(&data[size..size + 32]);
            size += 32;
            program_return
                .inputs_to_sign
                .push(InputToSign { index, signer });
        }

        program_return
    }
}
