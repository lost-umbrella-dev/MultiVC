use digest::DynDigest;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256, Sha512};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Hash {
    SHA256(String),
    SHA512(String),
}

impl Hash {
    pub fn hasher(&self) -> Box<dyn DynDigest> {
        match self {
            Hash::SHA256(_) => Box::new(Sha256::new()),
            Hash::SHA512(_) => Box::new(Sha512::new()),
        }
    }

    pub fn verify_digest(&self, digest_bytes: &[u8]) -> bool {
        let expected = match self {
            Hash::SHA256(s) | Hash::SHA512(s) => s,
        };
        hex::encode(digest_bytes) == *expected
    }
}
