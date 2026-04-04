use std::{fmt, str::FromStr};

use digest::DynDigest;
use serde::{Deserialize, Deserializer, Serialize, Serializer, de::Error as DeError};
use sha2::{Digest, Sha256, Sha512};

#[derive(Debug, Clone, Hash, PartialEq, Eq)]
pub enum Hash {
    SHA256(String),
    SHA512(String),
}

impl Hash {
    /// Создаёт хешер для вычисления хеша
    pub fn hasher(&self) -> Box<dyn DynDigest> {
        match self {
            Hash::SHA256(_) => Box::new(Sha256::new()),
            Hash::SHA512(_) => Box::new(Sha512::new()),
        }
    }

    /// Проверяет, совпадает ли хеш данных с принимаемым файлом
    pub fn verify(&self, data: &[u8]) -> bool {
        match self {
            Hash::SHA256(expected) => {
                let actual = Sha256::digest(data);
                hex::encode(actual) == *expected
            },
            Hash::SHA512(expected) => {
                let actual = Sha512::digest(data);
                hex::encode(actual) == *expected
            },
        }
    }

    /// Проверяет, совпадает ли хеш данных с ожидаемым значением через digest
    pub fn verify_digest(&self, digest_bytes: &[u8]) -> bool {
        let expected = match self {
            Hash::SHA256(s) | Hash::SHA512(s) => s,
        };
        hex::encode(digest_bytes) == *expected
    }
}

impl fmt::Display for Hash {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Hash::SHA256(value) => write!(f, "sha256:{value}"),
            Hash::SHA512(value) => write!(f, "sha512:{value}"),
        }
    }
}

impl FromStr for Hash {
    type Err = &'static str;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let Some((kind, value)) = s.split_once(':') else {
            return Err("hash must be in `<algorithm>:<hex>` format");
        };

        match kind.to_ascii_lowercase().as_str() {
            "sha256" => Ok(Hash::SHA256(value.to_owned())),
            "sha512" => Ok(Hash::SHA512(value.to_owned())),
            _ => Err("unsupported hash algorithm"),
        }
    }
}

impl Serialize for Hash {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_str(&self.to_string())
    }
}

impl<'de> Deserialize<'de> for Hash {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let value = String::deserialize(deserializer)?;
        Hash::from_str(&value).map_err(D::Error::custom)
    }
}
