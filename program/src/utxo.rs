#[derive(Clone, Debug, PartialEq, Eq)]
#[repr(C)]
pub struct UtxoMeta([u8; 36]);

impl UtxoMeta {
    pub fn from(txid: [u8; 32], vout: u32) -> Self {
        let mut data: [u8; 36] = [0; 36];
        data[..32].copy_from_slice(&txid);
        data[32..].copy_from_slice(&vout.to_le_bytes());
        Self(data)
    }

    pub fn from_outpoint(txid: Txid, vout: u32) -> Self {
        let mut data: [u8; 36] = [0; 36];
        data[..32].copy_from_slice(
            &bitcoin::consensus::serialize(&txid)
                .into_iter()
                .rev()
                .collect::<Vec<u8>>(),
        );
        data[32..].copy_from_slice(&vout.to_le_bytes());
        Self(data)
    }

    pub fn to_outpoint(&self) -> OutPoint {
        OutPoint {
            txid: Txid::from_str(&hex::encode(self.txid())).unwrap(),
            vout: self.vout(),
        }
    }

    pub fn from_slice(data: &[u8]) -> Self {
        Self(data[..36].try_into().expect("utxo meta is 36 bytes long"))
    }

    pub fn txid(&self) -> &[u8] {
        &self.0[..32]
    }

    pub fn txid_mut(&mut self) -> &mut [u8] {
        &mut self.0[..32]
    }

    pub fn vout_bytes_mut(&mut self) -> &mut [u8] {
        &mut self.0[32..]
    }

    pub fn vout(&self) -> u32 {
        u32::from_le_bytes(self.0[32..].try_into().expect("utxo meta unreachable"))
    }

    pub fn serialize(&self) -> [u8; 36] {
        self.0
    }
}

#[test]
fn test_outpoint() {
    assert_eq!(
        OutPoint::new(
            Txid::from_str("c5cc9251192330191366016c8dab0f67dc345bd024a206c313dbf26db0a66bb1")
                .unwrap(),
            0
        ),
        UtxoMeta::from(
            hex::decode("c5cc9251192330191366016c8dab0f67dc345bd024a206c313dbf26db0a66bb1")
                .unwrap()
                .try_into()
                .unwrap(),
            0
        )
        .to_outpoint()
    );
}

use core::fmt;
use std::str::FromStr;

use bitcoin::OutPoint;
use bitcoin::Txid;

/// TODO:
///  Change this in future according to the correct base implementation
impl fmt::Display for UtxoMeta {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{:?}", self.0)
    }
}

impl AsRef<[u8]> for UtxoMeta {
    fn as_ref(&self) -> &[u8] {
        &self.0[..]
    }
}

impl AsMut<[u8]> for UtxoMeta {
    fn as_mut(&mut self) -> &mut [u8] {
        &mut self.0[..]
    }
}

impl From<[u8; 36]> for UtxoMeta {
    fn from(value: [u8; 36]) -> Self {
        UtxoMeta(value)
    }
}
