use borsh::{ BorshDeserialize, BorshSerialize };
use serde::{ Deserialize, Serialize };
use bitcoin::{ key::Secp256k1, Address, PublicKey };

#[repr(C)]
#[derive(
    Clone,
    Debug,
    Eq,
    PartialEq,
    Hash,
    PartialOrd,
    Ord,
    Default,
    Copy,
    Serialize,
    Deserialize,
    BorshSerialize,
    BorshDeserialize
)]
pub struct Pubkey(pub [u8; 32]);

impl Pubkey {
    pub fn serialize(&self) -> [u8; 32] {
        self.0
    }

    pub fn from_slice(data: &[u8]) -> Self {
        let mut tmp = [0u8; 32];
        tmp[..data.len()].copy_from_slice(data);
        Self(tmp)
    }

    pub fn system_program() -> Self {
        let mut tmp = [0u8; 32];
        tmp[31] = 1;
        Self(tmp)
    }

    pub fn is_system_program(&self) -> bool {
        let mut tmp = [0u8; 32];
        tmp[31] = 1;
        self.0 == tmp
    }

    /// Log a `Pubkey` from a program
    pub fn log(&self) {
        unsafe { crate::syscalls::sol_log_pubkey(self.as_ref() as *const _ as *const u8) }
    }

    pub fn to_bitcoin_address(
        &self,
        _network: bitcoin::network::Network
    ) -> Result<Address, Box<dyn std::error::Error>> {
        // Create a Secp256k1 context
        let _secp = Secp256k1::new();

        // Create a full PublicKey from the 32-byte array
        // We're assuming this is a compressed public key, so we prepend 0x02 or 0x03
        let mut pubkey_bytes = [0u8; 33];
        pubkey_bytes[0] = 2; // Assume it's a "even" y-coordinate. If not, this might need to be 3.
        pubkey_bytes[1..].copy_from_slice(&self.0);

        let full_pubkey = PublicKey::from_slice(&pubkey_bytes)?;

        // Create a Bitcoin address from the public key
        let address = Address::p2wpkh(&full_pubkey, bitcoin::network::Network::Regtest)?;

        Ok(address)
    }
}

impl std::fmt::LowerHex for Pubkey {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        let ser = self.serialize();
        for ch in &ser[..] {
            write!(f, "{:02x}", *ch)?;
        }
        Ok(())
    }
}

use core::fmt;

/// TODO:
///  Change this in future according to the correct base implementation
impl fmt::Display for Pubkey {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{:?}", self.0)
    }
}

impl AsRef<[u8]> for Pubkey {
    fn as_ref(&self) -> &[u8] {
        &self.0[..]
    }
}

impl AsMut<[u8]> for Pubkey {
    fn as_mut(&mut self) -> &mut [u8] {
        &mut self.0[..]
    }
}

impl From<[u8; 32]> for Pubkey {
    fn from(value: [u8; 32]) -> Self {
        Pubkey(value)
    }
}
