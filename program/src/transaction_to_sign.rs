use crate::input_to_sign::InputToSign;

#[repr(C)]
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct TransactionToSign<'a> {
    pub tx_bytes: &'a [u8],
    pub inputs_to_sign: &'a [InputToSign],
}

impl<'a> TransactionToSign<'a> {
    pub fn serialise(&self) -> Vec<u8> {
        let mut serialized = vec![];

        serialized.push(self.tx_bytes.len() as u8);
        serialized.extend_from_slice(self.tx_bytes);
        serialized.push(self.inputs_to_sign.len() as u8);
        for input_to_sign in self.inputs_to_sign.iter() {
            serialized.extend_from_slice(&input_to_sign.index.to_le_bytes());
            serialized.extend_from_slice(&input_to_sign.signer.serialize());
        }

        serialized
    }
}

// #[cfg(test)]
// mod tests {
//     use crate::{input_to_sign::InputToSign, pubkey::Pubkey};

//     use super::TransactionToSign;

//     #[test]
//     fn test_serialize_and_back() {
//         let program_return = TransactionToSign {
//             tx_bytes: vec![],
//             inputs_to_sign: vec![],
//         };
//         assert_eq!(program_return, TransactionToSign::from_slice(&program_return.serialise()));

//         let program_return = TransactionToSign {
//             tx_bytes: vec![0; 32],
//             inputs_to_sign: vec![InputToSign {
//                 index: 0,
//                 signer: Pubkey::system_program()
//             }],
//         };
//         assert_eq!(program_return, TransactionToSign::from_slice(&program_return.serialise()));
//     }

// }
