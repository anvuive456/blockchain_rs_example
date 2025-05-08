use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use utoipa::ToSchema;

use super::transaction::Transaction;

/// Represents a block in the blockchain
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct Block {
    /// Version of the block structure
    #[serde(default = "default_version")]
    pub version: u32,

    /// Index of the block in the chain
    pub index: u64,

    /// Timestamp when the block was created
    #[schema(value_type = String, example = "2023-01-01T12:00:00Z")]
    pub timestamp: DateTime<Utc>,

    /// List of transactions included in this block
    pub transactions: Vec<Transaction>,

    /// Proof of work (nonce)
    pub proof: u64,

    /// Hash of the previous block
    pub previous_hash: String,

    /// Hash of the current block (calculated)
    #[serde(skip_serializing_if = "String::is_empty")]
    pub hash: String,
}

/// Default version for blocks
fn default_version() -> u32 {
    1
}

impl Block {
    /// Creates a new block
    ///
    /// # Arguments
    ///
    /// * `index` - The index of the block in the chain
    /// * `transactions` - The list of transactions to include in the block
    /// * `proof` - The proof of work (nonce)
    /// * `previous_hash` - The hash of the previous block
    ///
    /// # Returns
    ///
    /// A new Block instance
    pub fn new(index: u64, transactions: Vec<Transaction>, proof: u64, previous_hash: String) -> Self {
        let block = Block {
            version: default_version(),
            index,
            timestamp: Utc::now(),
            transactions,
            proof,
            previous_hash,
            hash: String::new(),
        };

        let hash = block.calculate_hash();

        Block {
            hash,
            ..block
        }
    }

    /// Calculates the hash of the block
    ///
    /// # Returns
    ///
    /// The SHA-256 hash of the block as a hexadecimal string
    pub fn calculate_hash(&self) -> String {
        let mut hasher = Sha256::new();

        // Convert the block to a JSON string
        let block_data = serde_json::json!({
            "version": self.version,
            "index": self.index,
            "timestamp": self.timestamp,
            "transactions": self.transactions,
            "proof": self.proof,
            "previous_hash": self.previous_hash,
        });

        let block_string = serde_json::to_string(&block_data).unwrap();

        // Update the hasher with the block data
        hasher.update(block_string.as_bytes());

        // Return the hash as a hexadecimal string
        format!("{:x}", hasher.finalize())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::blockchain::Address;

    #[test]
    fn test_new_block() {
        let transactions = vec![
            Transaction::new_coinbase(Address("recipient1".to_string()), 10.0),
            Transaction::new_coinbase(Address("recipient2".to_string()), 20.0),
        ];

        let block = Block::new(1, transactions, 100, "previous_hash".to_string());

        assert_eq!(block.index, 1);
        assert_eq!(block.proof, 100);
        assert_eq!(block.previous_hash, "previous_hash");
        assert!(!block.hash.is_empty());
    }

    #[test]
    fn test_calculate_hash() {
        let transactions = vec![
            Transaction::new_coinbase(Address("recipient".to_string()), 10.0),
        ];

        let block = Block::new(1, transactions, 100, "previous_hash".to_string());

        let hash = block.calculate_hash();
        assert!(!hash.is_empty());
        assert_eq!(hash.len(), 64); // SHA-256 hash is 64 characters in hex
    }
}
