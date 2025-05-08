use ed25519_dalek::{Signature, SigningKey, Signer, Verifier, VerifyingKey};
use rand::rngs::OsRng;
use serde::{Deserialize, Serialize};
use thiserror::Error;
use utoipa::ToSchema;

use std::fmt;
use std::str::FromStr;

/// Errors that can occur during cryptographic operations
#[derive(Debug, Error)]
pub enum CryptoError {
    #[error("Failed to generate keypair: {0}")]
    KeypairGenerationError(String),

    #[error("Failed to sign message: {0}")]
    SigningError(String),

    #[error("Failed to verify signature: {0}")]
    VerificationError(String),

    #[error("Invalid public key: {0}")]
    InvalidPublicKey(String),

    #[error("Invalid private key: {0}")]
    InvalidPrivateKey(String),

    #[error("Invalid signature: {0}")]
    InvalidSignature(String),

    #[error("Encoding error: {0}")]
    EncodingError(String),

    #[error("Decoding error: {0}")]
    DecodingError(String),
}

/// Represents a wallet address (public key in base58 format)
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize, ToSchema)]
pub struct Address(pub String);

impl Address {
    /// Creates a new address from a public key
    pub fn from_public_key(public_key: &VerifyingKey) -> Self {
        let bytes = public_key.as_bytes();
        let encoded = bs58::encode(bytes).into_string();
        Address(encoded)
    }

    /// Converts the address to a public key
    pub fn to_public_key(&self) -> Result<VerifyingKey, CryptoError> {
        let bytes = bs58::decode(&self.0)
            .into_vec()
            .map_err(|e| CryptoError::DecodingError(e.to_string()))?;

        VerifyingKey::from_bytes(&bytes.try_into().map_err(|_| {
            CryptoError::InvalidPublicKey("Invalid public key bytes".to_string())
        })?)
        .map_err(|e| CryptoError::InvalidPublicKey(e.to_string()))
    }
}

impl fmt::Display for Address {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl FromStr for Address {
    type Err = CryptoError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        // Validate that the string is a valid base58 encoding
        bs58::decode(s)
            .into_vec()
            .map_err(|e| CryptoError::DecodingError(e.to_string()))?;

        Ok(Address(s.to_string()))
    }
}

/// Represents a digital signature
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct DigitalSignature(pub String);

impl DigitalSignature {
    /// Creates a new digital signature from a signature
    pub fn from_signature(signature: &Signature) -> Self {
        let bytes = signature.to_bytes();
        let encoded = bs58::encode(bytes).into_string();
        DigitalSignature(encoded)
    }

    /// Converts the digital signature to a signature
    pub fn to_signature(&self) -> Result<Signature, CryptoError> {
        let bytes = bs58::decode(&self.0)
            .into_vec()
            .map_err(|e| CryptoError::DecodingError(e.to_string()))?;

        let signature_bytes: [u8; 64] = bytes.try_into().map_err(|_| {
            CryptoError::InvalidSignature("Invalid signature length".to_string())
        })?;

        Ok(Signature::from_bytes(&signature_bytes))
    }
}

/// Represents a wallet with a keypair
#[derive(Debug, Clone)]
pub struct Wallet {
    signing_key: SigningKey,
    verifying_key: VerifyingKey,
    address: Address,
}

impl Wallet {
    /// Creates a new wallet with a random keypair
    pub fn new() -> Result<Self, CryptoError> {
        let mut csprng = OsRng;
        let signing_key = SigningKey::generate(&mut csprng);
        let verifying_key = VerifyingKey::from(&signing_key);
        let address = Address::from_public_key(&verifying_key);

        Ok(Wallet {
            signing_key,
            verifying_key,
            address
        })
    }

    /// Creates a wallet from an existing secret key
    pub fn from_secret_key(secret_key_bytes: &[u8]) -> Result<Self, CryptoError> {
        let bytes_array: [u8; 32] = secret_key_bytes.try_into().map_err(|_| {
            CryptoError::InvalidPrivateKey("Invalid private key length".to_string())
        })?;

        let signing_key = SigningKey::from_bytes(&bytes_array);
        let verifying_key = VerifyingKey::from(&signing_key);
        let address = Address::from_public_key(&verifying_key);

        Ok(Wallet {
            signing_key,
            verifying_key,
            address
        })
    }

    /// Gets the wallet's address
    pub fn address(&self) -> &Address {
        &self.address
    }

    /// Gets the wallet's public key
    pub fn public_key(&self) -> &VerifyingKey {
        &self.verifying_key
    }

    /// Signs a message with the wallet's private key
    pub fn sign(&self, message: &[u8]) -> Result<DigitalSignature, CryptoError> {
        let signature = self.signing_key.sign(message);
        Ok(DigitalSignature::from_signature(&signature))
    }

    /// Exports the wallet's secret key as bytes
    pub fn export_secret_key(&self) -> Vec<u8> {
        self.signing_key.to_bytes().to_vec()
    }
}

/// Verifies a signature against a message and public key
pub fn verify_signature(
    message: &[u8],
    signature: &DigitalSignature,
    public_key: &VerifyingKey,
) -> Result<bool, CryptoError> {
    let signature = signature.to_signature()?;

    match public_key.verify(message, &signature) {
        Ok(_) => Ok(true),
        Err(_) => Ok(false),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_wallet_creation() {
        let wallet = Wallet::new().unwrap();
        assert!(!wallet.address.0.is_empty());
    }

    #[test]
    fn test_signing_and_verification() {
        let wallet = Wallet::new().unwrap();
        let message = b"Hello, world!";

        // Sign the message
        let signature = wallet.sign(message).unwrap();

        // Verify the signature
        let result = verify_signature(message, &signature, wallet.public_key()).unwrap();
        assert!(result);

        // Verify with wrong message
        let wrong_message = b"Wrong message";
        let result = verify_signature(wrong_message, &signature, wallet.public_key()).unwrap();
        assert!(!result);
    }

    #[test]
    fn test_address_conversion() {
        let wallet = Wallet::new().unwrap();
        let address = wallet.address();

        // Convert address to public key
        let public_key = address.to_public_key().unwrap();

        // Check that it matches the original public key
        assert_eq!(public_key.as_bytes(), wallet.public_key().as_bytes());
    }
}
