use alloc::vec::Vec;
use alloc::string::String;
use alloc::vec;
use bitcoin_hashes::{sha256, Hash as HashTrait};
use core::fmt::{self, Write};
use core::str::FromStr;
use crate::error;
use crate::network::Network;
use crate::base58;
use crate::network::Network::Mainnet;
use crate::signature::Signature;
use rand::Rng;
use secp256k1;


/// A Secp256k1 private key
#[derive(Clone, PartialEq, Eq)]
pub struct SecretKey {
    /// Whether this private key should be serialized as compressed
    pub compressed: bool,
    /// The network on which this key should be used
    pub network: Network,
    /// The actual Secp256k1 key
    pub key: secp256k1::SecretKey,
}

impl SecretKey {
    /// Creates a new random secret key. Requires compilation with the "rand" feature.
    pub fn generate<R>(csprng: &mut R) -> Self where R: Rng {
        Self {
            compressed: false,
            network: Mainnet,
            key: secp256k1::SecretKey::random(csprng),
        }
    }

    /// Serialize the private key to bytes
    pub fn to_bytes(&self) -> Vec<u8> {
        self.key.serialize().to_vec()
    }

    /// Format the private key to WIF format.
    pub fn fmt_wif(&self, fmt: &mut dyn fmt::Write) -> fmt::Result {
        let mut ret = [0; 34];
        ret[0] = match self.network {
            Network::Mainnet => 128,
            Network::Testnet => 239,
        };
        ret[1..33].copy_from_slice(&self.key.serialize());
        let privkey = if self.compressed {
            ret[33] = 1;
            base58::check_encode_slice(&ret[..])
        } else {
            base58::check_encode_slice(&ret[..33])
        };

        fmt.write_str(&privkey)
    }

    /// Get WIF encoding of this private key.
    pub fn to_wif(&self) -> String {
        let mut buf = String::new();
        buf.write_fmt(format_args!("{}", self)).unwrap();
        buf.shrink_to_fit();

        buf
    }

    /// Parse WIF encoded private key.
    pub fn from_wif(wif: &str) -> Result<SecretKey, error::Error> {
        let data = base58::from_check(wif)?;

        let compressed = match data.len() {
            33 => false,
            34 => true,
            _ => { return Err(error::Error::Base58(base58::Error::InvalidLength(data.len()))); }
        };

        let network = match data[0] {
            128 => Network::Mainnet,
            239 => Network::Testnet,
            x => { return Err(error::Error::Base58(base58::Error::InvalidVersion(vec![x]))); }
        };

        Ok(SecretKey {
            compressed,
            network,
            key: secp256k1::SecretKey::parse_slice(&data[1..33])?,
        })
    }

    /// Deserialize a secret key from a slice
    pub fn from_slice(data: &[u8]) -> Result<SecretKey, error::Error> {
        let compressed: bool = match data.len() {
            33 => true,
            65 => false,
            len => { return Err(base58::Error::InvalidLength(len).into()); }
        };

        Ok(SecretKey {
            compressed,
            network: Mainnet,
            key: secp256k1::SecretKey::parse_slice(data)?,
        })
    }

    pub fn sign(&self, message_slice: &[u8]) -> Signature {
        let msg_hash = sha256::Hash::hash(&message_slice);
        let msg = secp256k1::Message::parse(&msg_hash.into_inner());
        let (mut sig, recv_id) = secp256k1::sign(&msg, &self.key);
        sig.normalize_s();

        Signature {
            recv_id,
            sig,
        }
    }
}

impl fmt::Display for SecretKey {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        self.fmt_wif(f)
    }
}

impl fmt::Debug for SecretKey {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "[private key data]")
    }
}

impl FromStr for SecretKey {
    type Err = error::Error;
    fn from_str(s: &str) -> Result<SecretKey, error::Error> {
        SecretKey::from_wif(s)
    }
}

#[cfg(test)]
mod test {
    use super::SecretKey;
    use crate::public::PublicKey;
    use alloc::string::ToString;
    #[cfg(feature = "std")]
    use rand::thread_rng;

    #[cfg(feature = "std")]
    #[test]
    fn sk_generate_should_work() {
        let mut rng = thread_rng();
        let _sk = SecretKey::generate(&mut rng);
    }

    #[test]
    fn sk_from_wif_should_work() {
        let wif = "5HrBLKfeEdqH9KLMv1daHLVjrXV3DGVERAkN5cdSSc58bzqqfT4";
        let sk = SecretKey::from_wif(wif);
        assert!(sk.is_ok());
    }

    #[test]
    fn sk_sign_should_work() {
        let sk = SecretKey::from_wif("5KJVA9P4xsiRC3zPy1KPa3GA6ffvmyZSxhKPbE924YJphvSCG4F");
        assert!(sk.is_ok());
        let sk = sk.unwrap();
        let pk = PublicKey::from(&sk);
        assert_eq!(pk.to_string(), "EOS55KuLPN3u9qii2hEhJhkdQSdaVLVPTHdwdkEhszhhCWDthQtfi");
        let sig = sk.sign("hello".as_bytes());
        assert_eq!(sig.to_string(), "SIG_K1_KumC85Ykop62rdA7enDgHHNRNbUqBqzJoyLj5zQHJxeJepZ9EPXqJWSc1KT7Fo5QyX3EavjgYWaqjHpeCg88g457dFQYwh");
    }
}
