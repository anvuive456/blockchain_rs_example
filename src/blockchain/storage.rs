use std::path::Path;

use sled::{Db, Tree};
use thiserror::Error;
use log::warn;
use bincode;

use super::block::Block;
use super::crypto::Address;
use super::transaction::Transaction;
use super::account::Account;

/// Errors that can occur during storage operations
#[derive(Debug, Error)]
pub enum StorageError {
    #[error("Database error: {0}")]
    DatabaseError(#[from] sled::Error),

    #[error("Serialization error: {0}")]
    SerializationError(String),

    #[error("Deserialization error: {0}")]
    DeserializationError(String),

    #[error("Item not found: {0}")]
    NotFound(String),
}

/// Storage for blockchain data
pub struct BlockchainStorage {
    /// The database instance
    db: Db,

    /// Tree for blocks
    blocks: Tree,

    /// Tree for transactions
    transactions: Tree,

    /// Tree for accounts
    accounts: Tree,

    /// Tree for metadata
    metadata: Tree,
}

impl std::fmt::Debug for BlockchainStorage {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("BlockchainStorage")
            .finish()
    }
}

impl BlockchainStorage {
    /// Creates a new blockchain storage
    ///
    /// # Arguments
    ///
    /// * `path` - The path to the database directory
    ///
    /// # Returns
    ///
    /// A new BlockchainStorage instance
    pub fn new<P: AsRef<Path>>(path: P) -> Result<Self, StorageError> {
        let db = sled::open(path)?;

        let blocks = db.open_tree("blocks")?;
        let transactions = db.open_tree("transactions")?;
        let accounts = db.open_tree("accounts")?;
        let metadata = db.open_tree("metadata")?;

        Ok(Self {
            db,
            blocks,
            transactions,
            accounts,
            metadata,
        })
    }

    /// Saves a block to the database
    ///
    /// # Arguments
    ///
    /// * `block` - The block to save
    ///
    /// # Returns
    ///
    /// Ok(()) if successful
    pub fn save_block(&self, block: &Block) -> Result<(), StorageError> {
        let key = block.hash.as_bytes();
        let value = bincode::serialize(block)
            .map_err(|e| StorageError::SerializationError(e.to_string()))?;

        self.blocks.insert(key, value)?;

        // Update latest block hash
        self.metadata.insert("latest_block_hash", key)?;

        // Update block height
        let height_bytes = bincode::serialize(&block.index)
            .map_err(|e| StorageError::SerializationError(e.to_string()))?;
        self.metadata.insert("block_height", height_bytes)?;

        Ok(())
    }

    /// Gets a block by its hash
    ///
    /// # Arguments
    ///
    /// * `hash` - The hash of the block
    ///
    /// # Returns
    ///
    /// The block if found
    pub fn get_block(&self, hash: &str) -> Result<Block, StorageError> {
        let key = hash.as_bytes();

        if let Some(value) = self.blocks.get(key)? {
            let block: Block = bincode::deserialize(&value)
                .map_err(|e| StorageError::DeserializationError(e.to_string()))?;

            Ok(block)
        } else {
            Err(StorageError::NotFound(format!("Block with hash {} not found", hash)))
        }
    }

    /// Gets all blocks in the chain
    ///
    /// # Returns
    ///
    /// A vector of all blocks
    pub fn get_all_blocks(&self) -> Result<Vec<Block>, StorageError> {
        let mut blocks = Vec::new();
        let mut deserialization_errors = Vec::new();

        for result in self.blocks.iter() {
            match result {
                Ok((key, value)) => {
                    match bincode::deserialize::<Block>(&value) {
                        Ok(block) => {
                            blocks.push(block);
                        },
                        Err(e) => {
                            // Log the error but continue processing other blocks
                            let key_str = String::from_utf8_lossy(key.as_ref()).to_string();
                            deserialization_errors.push(format!("Failed to deserialize block {}: {}", key_str, e));
                        }
                    }
                },
                Err(e) => {
                    return Err(StorageError::DatabaseError(e));
                }
            }
        }

        // If we have deserialization errors but also some valid blocks, log the errors but continue
        if !deserialization_errors.is_empty() {
            if blocks.is_empty() {
                // If we have no valid blocks, return an error
                return Err(StorageError::DeserializationError(
                    format!("Failed to deserialize any blocks: {}", deserialization_errors.join(", "))
                ));
            } else {
                // Log the errors but continue with the valid blocks
                warn!("Some blocks could not be deserialized: {}", deserialization_errors.join(", "));
            }
        }

        // Sort blocks by index
        blocks.sort_by_key(|block| block.index);

        Ok(blocks)
    }

    /// Saves a transaction to the database
    ///
    /// # Arguments
    ///
    /// * `transaction` - The transaction to save
    ///
    /// # Returns
    ///
    /// Ok(()) if successful
    pub fn save_transaction(&self, transaction: &Transaction) -> Result<(), StorageError> {
        let key = transaction.id.as_bytes();
        let value = bincode::serialize(transaction)
            .map_err(|e| StorageError::SerializationError(e.to_string()))?;

        self.transactions.insert(key, value)?;
        Ok(())
    }

    /// Gets a transaction by its ID
    ///
    /// # Arguments
    ///
    /// * `id` - The ID of the transaction
    ///
    /// # Returns
    ///
    /// The transaction if found
    pub fn get_transaction(&self, id: &str) -> Result<Transaction, StorageError> {
        let key = id.as_bytes();

        if let Some(value) = self.transactions.get(key)? {
            let transaction: Transaction = bincode::deserialize(&value)
                .map_err(|e| StorageError::DeserializationError(e.to_string()))?;

            Ok(transaction)
        } else {
            Err(StorageError::NotFound(format!("Transaction with ID {} not found", id)))
        }
    }

    /// Saves an account to the database
    ///
    /// # Arguments
    ///
    /// * `account` - The account to save
    ///
    /// # Returns
    ///
    /// Ok(()) if successful
    pub fn save_account(&self, account: &Account) -> Result<(), StorageError> {
        let key = account.address.0.as_bytes();
        let value = bincode::serialize(account)
            .map_err(|e| StorageError::SerializationError(e.to_string()))?;

        self.accounts.insert(key, value)?;
        Ok(())
    }

    /// Gets an account by its address
    ///
    /// # Arguments
    ///
    /// * `address` - The address of the account
    ///
    /// # Returns
    ///
    /// The account if found
    pub fn get_account(&self, address: &Address) -> Result<Account, StorageError> {
        let key = address.0.as_bytes();

        if let Some(value) = self.accounts.get(key)? {
            let account: Account = bincode::deserialize(&value)
                .map_err(|e| StorageError::DeserializationError(e.to_string()))?;

            Ok(account)
        } else {
            // Return a new account with zero balance if not found
            Ok(Account::new(address.clone()))
        }
    }

    /// Gets the latest block hash
    ///
    /// # Returns
    ///
    /// The latest block hash if found
    pub fn get_latest_block_hash(&self) -> Result<String, StorageError> {
        if let Some(value) = self.metadata.get("latest_block_hash")? {
            Ok(String::from_utf8_lossy(&value).to_string())
        } else {
            Err(StorageError::NotFound("Latest block hash not found".to_string()))
        }
    }

    /// Gets the current block height
    ///
    /// # Returns
    ///
    /// The current block height
    pub fn get_block_height(&self) -> Result<u64, StorageError> {
        if let Some(value) = self.metadata.get("block_height")? {
            let height: u64 = bincode::deserialize(&value)
                .map_err(|e| StorageError::DeserializationError(e.to_string()))?;

            Ok(height)
        } else {
            Ok(0) // Return 0 if not found (empty blockchain)
        }
    }

    /// Flushes all pending writes to disk
    pub fn flush(&self) -> Result<(), StorageError> {
        self.db.flush()?;
        Ok(())
    }

    /// Gets all accounts from storage
    ///
    /// # Returns
    ///
    /// A vector of all accounts
    pub fn get_all_accounts(&self) -> Result<Vec<Account>, StorageError> {
        let mut accounts = Vec::new();
        let mut deserialization_errors = Vec::new();

        for result in self.accounts.iter() {
            match result {
                Ok((key, value)) => {
                    match bincode::deserialize::<Account>(&value) {
                        Ok(account) => {
                            accounts.push(account);
                        },
                        Err(e) => {
                            // Log the error but continue processing other accounts
                            let key_str = String::from_utf8_lossy(key.as_ref()).to_string();
                            deserialization_errors.push(format!("Failed to deserialize account {}: {}", key_str, e));
                        }
                    }
                },
                Err(e) => {
                    return Err(StorageError::DatabaseError(e));
                }
            }
        }

        // If we have deserialization errors but also some valid accounts, log the errors but continue
        if !deserialization_errors.is_empty() {
            if accounts.is_empty() {
                // If we have no valid accounts, return an error
                return Err(StorageError::DeserializationError(
                    format!("Failed to deserialize any accounts: {}", deserialization_errors.join(", "))
                ));
            } else {
                // Log the errors but continue with the valid accounts
                warn!("Some accounts could not be deserialized: {}", deserialization_errors.join(", "));
            }
        }

        Ok(accounts)
    }
}
