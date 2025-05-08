use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use thiserror::Error;
use utoipa::ToSchema;
use uuid::Uuid;

use super::crypto::{Address, DigitalSignature, verify_signature, CryptoError};

/// Errors that can occur during transaction operations
#[derive(Debug, Error)]
pub enum TransactionError {
    #[error("Invalid signature")]
    InvalidSignature,

    #[error("Insufficient funds: required {required}, available {available}")]
    InsufficientFunds { required: f64, available: f64 },

    #[error("Invalid sender address: {0}")]
    InvalidSenderAddress(String),

    #[error("Invalid recipient address: {0}")]
    InvalidRecipientAddress(String),

    #[error("Invalid amount: {0}")]
    InvalidAmount(String),

    #[error("Transaction already signed")]
    AlreadySigned,

    #[error("Transaction not signed")]
    NotSigned,

    #[error("Crypto error: {0}")]
    CryptoError(#[from] CryptoError),

    #[error("System error: {0}")]
    SystemError(String),
}

/// Represents a transaction in the blockchain
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct Transaction {
    /// Version of the transaction structure
    #[serde(default = "default_version")]
    pub version: u32,

    /// Unique identifier for the transaction
    pub id: String,

    /// Sender's address
    pub sender: Address,

    /// Recipient's address
    pub recipient: Address,

    /// Amount being transferred
    pub amount: f64,

    /// Transaction fee
    pub fee: f64,

    /// Nonce to prevent replay attacks
    pub nonce: u64,

    /// Digital signature of the transaction
    #[serde(skip_serializing_if = "Option::is_none")]
    pub signature: Option<DigitalSignature>,

    /// Timestamp when the transaction was created
    #[schema(value_type = String, example = "2023-01-01T12:00:00Z")]
    pub timestamp: DateTime<Utc>,
}

/// Default version for transactions
fn default_version() -> u32 {
    1
}

impl Transaction {
    /// Creates a new unsigned transaction
    ///
    /// # Arguments
    ///
    /// * `sender` - The address of the sender
    /// * `recipient` - The address of the recipient
    /// * `amount` - The amount to transfer
    /// * `fee` - The transaction fee
    /// * `nonce` - The transaction nonce
    ///
    /// # Returns
    ///
    /// A new Transaction instance
    pub fn new(sender: Address, recipient: Address, amount: f64, fee: f64, nonce: u64) -> Self {
        Transaction {
            version: default_version(),
            id: Uuid::new_v4().to_string(),
            sender,
            recipient,
            amount,
            fee,
            nonce,
            signature: None,
            timestamp: Utc::now(),
        }
    }

    /// Creates a new coinbase transaction (mining reward)
    ///
    /// # Arguments
    ///
    /// * `recipient` - The address of the miner
    /// * `amount` - The reward amount
    ///
    /// # Returns
    ///
    /// A new Transaction instance
    pub fn new_coinbase(recipient: Address, amount: f64) -> Self {
        let system_address = Address("0".to_string());

        Transaction {
            version: default_version(),
            id: Uuid::new_v4().to_string(),
            sender: system_address,
            recipient,
            amount,
            fee: 0.0,
            nonce: 0,
            signature: None,
            timestamp: Utc::now(),
        }
    }

    /// Signs the transaction with a wallet
    ///
    /// # Arguments
    ///
    /// * `wallet` - The wallet to sign with
    ///
    /// # Returns
    ///
    /// Result indicating success or failure
    pub fn sign(&mut self, wallet: &super::crypto::Wallet) -> Result<(), TransactionError> {
        // Check if the transaction is already signed
        if self.signature.is_some() {
            return Err(TransactionError::AlreadySigned);
        }

        // Check if the wallet address matches the sender address
        if wallet.address() != &self.sender {
            return Err(TransactionError::InvalidSenderAddress(
                "Wallet address does not match sender address".to_string(),
            ));
        }

        // Create a message from the transaction data
        let message = self.to_bytes()?;

        // Sign the message
        let signature = wallet.sign(&message)?;

        // Set the signature
        self.signature = Some(signature);

        Ok(())
    }

    /// Verifies the transaction's signature
    ///
    /// # Returns
    ///
    /// Result indicating if the signature is valid
    pub fn verify_signature(&self) -> Result<bool, TransactionError> {
        // Check if the transaction is signed
        let signature = match &self.signature {
            Some(sig) => sig,
            None => return Err(TransactionError::NotSigned),
        };

        // Get the sender's public key
        let public_key = self.sender.to_public_key()?;

        // Create a message from the transaction data
        let message = self.to_bytes_without_signature()?;

        // Verify the signature
        verify_signature(&message, signature, &public_key)
            .map_err(TransactionError::from)
    }

    /// Converts the transaction to bytes for signing
    fn to_bytes(&self) -> Result<Vec<u8>, TransactionError> {
        let data = serde_json::json!({
            "version": self.version,
            "id": self.id,
            "sender": self.sender.0,
            "recipient": self.recipient.0,
            "amount": self.amount,
            "fee": self.fee,
            "nonce": self.nonce,
            "timestamp": self.timestamp,
        });

        serde_json::to_vec(&data)
            .map_err(|e| TransactionError::SystemError(e.to_string()))
    }

    /// Converts the transaction to bytes without the signature
    fn to_bytes_without_signature(&self) -> Result<Vec<u8>, TransactionError> {
        let data = serde_json::json!({
            "version": self.version,
            "id": self.id,
            "sender": self.sender.0,
            "recipient": self.recipient.0,
            "amount": self.amount,
            "fee": self.fee,
            "nonce": self.nonce,
            "timestamp": self.timestamp,
        });

        serde_json::to_vec(&data)
            .map_err(|e| TransactionError::SystemError(e.to_string()))
    }

    /// Checks if the transaction is a coinbase transaction
    pub fn is_coinbase(&self) -> bool {
        self.sender.0 == "0" && self.fee == 0.0 && self.nonce == 0
    }

    /// Gets the total amount required for the transaction (amount + fee)
    pub fn total_amount(&self) -> f64 {
        self.amount + self.fee
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::blockchain::crypto::Wallet;

    #[test]
    fn test_new_transaction() {
        let sender_wallet = Wallet::new().unwrap();
        let recipient_wallet = Wallet::new().unwrap();
        let amount = 10.5;
        let fee = 0.1;
        let nonce = 1;

        let transaction = Transaction::new(
            sender_wallet.address().clone(),
            recipient_wallet.address().clone(),
            amount,
            fee,
            nonce,
        );

        assert_eq!(transaction.sender, *sender_wallet.address());
        assert_eq!(transaction.recipient, *recipient_wallet.address());
        assert_eq!(transaction.amount, amount);
        assert_eq!(transaction.fee, fee);
        assert_eq!(transaction.nonce, nonce);
        assert!(!transaction.id.is_empty());
        assert!(transaction.signature.is_none());
    }

    #[test]
    fn test_sign_transaction() {
        let sender_wallet = Wallet::new().unwrap();
        let recipient_wallet = Wallet::new().unwrap();
        let amount = 10.5;
        let fee = 0.1;
        let nonce = 1;

        let mut transaction = Transaction::new(
            sender_wallet.address().clone(),
            recipient_wallet.address().clone(),
            amount,
            fee,
            nonce,
        );

        // Sign the transaction
        transaction.sign(&sender_wallet).unwrap();

        // Verify that the transaction is signed
        assert!(transaction.signature.is_some());

        // Verify the signature
        assert!(transaction.verify_signature().unwrap());
    }

    #[test]
    fn test_coinbase_transaction() {
        let miner_wallet = Wallet::new().unwrap();
        let reward = 50.0;

        let transaction = Transaction::new_coinbase(miner_wallet.address().clone(), reward);

        assert_eq!(transaction.sender.0, "0");
        assert_eq!(transaction.recipient, *miner_wallet.address());
        assert_eq!(transaction.amount, reward);
        assert_eq!(transaction.fee, 0.0);
        assert_eq!(transaction.nonce, 0);
        assert!(transaction.is_coinbase());
    }
}
