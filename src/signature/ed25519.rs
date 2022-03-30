
use super::curve25519::{ge_scalarmult_base, is_identity, sc_muladd, sc_reduce, GeP2, GeP3};
use super::error::Error;
use super::sha512;
use core::fmt;
use core::ops::Deref;

/// A public key.
#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub struct PublicKey([u8; PublicKey::BYTES]);

impl PublicKey {
    /// Number of raw bytes in a public key.
    pub const BYTES: usize = 32;

    /// Creates a public key from raw bytes.
    pub fn new(pk: [u8; PublicKey::BYTES]) -> Self {
        PublicKey(pk)
    }

    /// Creates a public key from a slice.
    pub fn from_slice(pk: &[u8]) -> Result<Self, Error> {
        let mut pk_ = [0u8; PublicKey::BYTES];
        if pk.len() != pk_.len() {
            return Err(Error::InvalidPublicKey);
        }
        pk_.copy_from_slice(pk);
        Ok(PublicKey::new(pk_))
    }
}

impl Deref for PublicKey {
    type Target = [u8; PublicKey::BYTES];

    /// Returns a public key as bytes.
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

/// An Ed25519 signature.
#[derive(Copy, Clone)]
pub struct Signature([u8; Signature::BYTES]);

impl fmt::Debug for Signature {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_fmt(format_args!("{:x?}", self))
    }
}

impl AsRef<[u8]> for Signature {
    fn as_ref(&self) -> &[u8] {
        &self.0
    }
}

impl Signature {
    /// Number of raw bytes in a signature.
    pub const BYTES: usize = 64;

    /// Creates a signature from raw bytes.
    pub fn new(bytes: [u8; Signature::BYTES]) -> Self {
        Signature(bytes)
    }

    /// Creates a signature key from a slice.
    pub fn from_slice(signature: &[u8]) -> Result<Self, Error> {
        let mut signature_ = [0u8; Signature::BYTES];
        if signature.len() != signature_.len() {
            return Err(Error::InvalidSignature);
        }
        signature_.copy_from_slice(signature);
        Ok(Signature::new(signature_))
    }
}

impl Deref for Signature {
    type Target = [u8; Signature::BYTES];

    /// Returns a signture as bytes.
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl PublicKey {
    /// Verifies that the signature `signature` is valid for the message `message`.
    pub fn verify(&self, message: impl AsRef<[u8]>, signature: &Signature) -> Result<(), Error> {
        let s = &signature[32..64];
        if check_lt_l(s) {
            return Err(Error::InvalidSignature);
        }
        if is_identity(self) || self.iter().fold(0, |acc, x| acc | x) == 0 {
            return Err(Error::WeakPublicKey);
        }
        let a = match GeP3::from_bytes_negate_vartime(self) {
            Some(g) => g,
            None => {
                return Err(Error::InvalidPublicKey);
            }
        };

        let mut hasher = sha512::Hash::new();
        hasher.update(&signature[0..32]);
        hasher.update(&self[..]);
        hasher.update(message);
        let mut hash = hasher.finalize();
        sc_reduce(&mut hash);

        let r = GeP2::double_scalarmult_vartime(hash.as_ref(), a, s);
        if r.to_bytes()
            .as_ref()
            .iter()
            .zip(signature.iter())
            .fold(0, |acc, (x, y)| acc | (x ^ y))
            != 0
        {
            Err(Error::SignatureMismatch)
        } else {
            Ok(())
        }
    }
}


static L: [u8; 32] = [
    0x10, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
    0x14, 0xde, 0xf9, 0xde, 0xa2, 0xf7, 0x9c, 0xd6, 0x58, 0x12, 0x63, 0x1a, 0x5c, 0xf5, 0xd3, 0xed,
];

fn check_lt_l(s: &[u8]) -> bool {
    let mut c: u8 = 0;
    let mut n: u8 = 1;

    let mut i = 31;
    loop {
        c |= ((((s[i] as i32) - (L[i] as i32)) >> 8) as u8) & n;
        n &= ((((s[i] ^ L[i]) as i32) - 1) >> 8) as u8;
        if i == 0 {
            break;
        } else {
            i -= 1;
        }
    }
    c == 0
}

#[cfg(feature = "traits")]
mod ed25519_trait {
    use super::{PublicKey, SecretKey, Signature};
    use ::ed25519::signature as ed25519_trait;

    impl ed25519_trait::Signature for Signature {
        fn from_bytes(bytes: &[u8]) -> Result<Self, ed25519_trait::Error> {
            let mut bytes_ = [0u8; Signature::BYTES];
            bytes_.copy_from_slice(bytes);
            Ok(Signature::new(bytes_))
        }
    }

    impl ed25519_trait::Signer<Signature> for SecretKey {
        fn try_sign(&self, message: &[u8]) -> Result<Signature, ed25519_trait::Error> {
            Ok(self.sign(message, None))
        }
    }

    impl ed25519_trait::Verifier<Signature> for PublicKey {
        fn verify(
            &self,
            message: &[u8],
            signature: &Signature,
        ) -> Result<(), ed25519_trait::Error> {
            #[cfg(feature = "std")]
            {
                self.verify(message, signature)
                    .map_err(|e| ed25519_trait::Error::from_source(e))
            }

            #[cfg(not(feature = "std"))]
            {
                self.verify(message, signature)
                    .map_err(|_| ed25519_trait::Error::new())
            }
        }
    }
}
