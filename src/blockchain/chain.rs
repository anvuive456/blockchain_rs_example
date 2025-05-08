use std::sync::{Arc, Mutex};
use thiserror::Error;
use log::{error, info, warn};

use super::account::{AccountState, AccountError};
use super::block::Block;
use super::crypto::Address;
use super::transaction::{Transaction, TransactionError};
use super::storage::{BlockchainStorage, StorageError};

/// Errors that can occur during blockchain operations
#[derive(Debug, Error)]
pub enum BlockchainError {
    #[error("Transaction error: {0}")]
    TransactionError(#[from] TransactionError),

    #[error("Account error: {0}")]
    AccountError(#[from] AccountError),

    #[error("Storage error: {0}")]
    StorageError(#[from] StorageError),

    #[error("Invalid block: {0}")]
    InvalidBlock(String),

    #[error("Invalid chain: {0}")]
    InvalidChain(String),

    #[error("System error: {0}")]
    SystemError(String),
}

/// Represents the blockchain
#[derive(Debug, Clone)]
pub struct Blockchain {
    /// The chain of blocks
    chain: Arc<Mutex<Vec<Block>>>,

    /// Pending transactions to be included in the next block
    pending_transactions: Arc<Mutex<Vec<Transaction>>>,

    /// Account state
    account_state: Arc<AccountState>,

    /// Mining difficulty (number of leading zeros required in hash)
    difficulty: u8,

    /// Mining reward
    mining_reward: f64,

    /// Minimum transaction fee
    minimum_fee: f64,

    /// Storage for blockchain data
    storage: Option<Arc<BlockchainStorage>>,
}

impl Blockchain {
    /// Creates a new blockchain with a genesis block
    ///
    /// # Returns
    ///
    /// A new Blockchain instance
    pub fn new() -> Self {
        let mut blockchain = Blockchain {
            chain: Arc::new(Mutex::new(Vec::new())),
            pending_transactions: Arc::new(Mutex::new(Vec::new())),
            account_state: Arc::new(AccountState::new()),
            difficulty: 4,
            mining_reward: 50.0,
            minimum_fee: 0.01,
            storage: None,
        };

        // Create the genesis block
        blockchain.create_genesis_block();

        blockchain
    }

    /// Creates a new blockchain with a genesis block and persistent storage
    ///
    /// # Arguments
    ///
    /// * `storage_path` - The path to the storage directory
    ///
    /// # Returns
    ///
    /// A new Blockchain instance with persistent storage
    pub fn with_storage<P: AsRef<std::path::Path>>(storage_path: P) -> Result<Self, BlockchainError> {
        // Create storage
        let storage = BlockchainStorage::new(storage_path)?;

        let mut blockchain = Blockchain {
            chain: Arc::new(Mutex::new(Vec::new())),
            pending_transactions: Arc::new(Mutex::new(Vec::new())),
            account_state: Arc::new(AccountState::new()),
            difficulty: 4,
            mining_reward: 50.0,
            minimum_fee: 0.01,
            storage: Some(Arc::new(storage)),
        };

        // Try to load existing chain from storage
        match blockchain.load_from_storage() {
            Ok(_) => {
                info!("Loaded blockchain from storage");
            }
            Err(err) => {
                // If storage is empty, create genesis block
                if let BlockchainError::StorageError(StorageError::NotFound(_)) = err {
                    info!("No existing blockchain found in storage, creating genesis block");
                    blockchain.create_genesis_block();
                    blockchain.save_to_storage()?;
                } else {
                    return Err(err);
                }
            }
        }

        Ok(blockchain)
    }

    /// Creates the genesis block (first block in the chain)
    fn create_genesis_block(&mut self) {
        let genesis_block = Block::new(
            0,
            Vec::new(),
            1,
            "0".to_string(),
        );

        self.chain.lock().unwrap().push(genesis_block);
    }

    /// Gets the last block in the chain
    ///
    /// # Returns
    ///
    /// The last block in the chain
    pub fn get_last_block(&self) -> Block {
        let chain = self.chain.lock().unwrap();
        chain.last().unwrap().clone()
    }

    /// Adds a new transaction to the pending transactions
    ///
    /// # Arguments
    ///
    /// * `transaction` - The transaction to add
    ///
    /// # Returns
    ///
    /// Result with the index of the block that will include this transaction
    pub fn add_transaction(&self, transaction: Transaction) -> Result<u64, BlockchainError> {
        // Verify the transaction signature
        if !transaction.is_coinbase() && !transaction.verify_signature()? {
            return Err(BlockchainError::TransactionError(
                TransactionError::InvalidSignature,
            ));
        }

        // Check if the transaction fee is sufficient
        if !transaction.is_coinbase() && transaction.fee < self.minimum_fee {
            return Err(BlockchainError::TransactionError(
                TransactionError::InvalidAmount(format!(
                    "Transaction fee too low: {} (minimum: {})",
                    transaction.fee, self.minimum_fee
                )),
            ));
        }

        // Check if the sender has sufficient funds
        if !transaction.is_coinbase() {
            let sender_account = self.account_state.get_account(&transaction.sender);

            if !sender_account.has_sufficient_funds(transaction.total_amount()) {
                return Err(BlockchainError::AccountError(
                    AccountError::InsufficientFunds {
                        required: transaction.total_amount(),
                        available: sender_account.balance,
                    },
                ));
            }

            // Check if the nonce is valid
            if !sender_account.is_valid_nonce(transaction.nonce) {
                return Err(BlockchainError::AccountError(
                    AccountError::InvalidNonce {
                        expected: sender_account.nonce,
                        got: transaction.nonce,
                    },
                ));
            }
        }

        // Add the transaction to pending transactions
        self.pending_transactions.lock().unwrap().push(transaction);

        Ok(self.get_last_block().index + 1)
    }

    /// Mines a new block with the pending transactions
    ///
    /// # Arguments
    ///
    /// * `miner_address` - The address of the miner (to receive mining reward)
    ///
    /// # Returns
    ///
    /// Result with the newly mined block
    pub fn mine_block(&self, miner_address: &str) -> Result<Block, BlockchainError> {
        // Parse miner address
        let miner_address = Address(miner_address.to_string());

        // Add mining reward transaction
        let reward_transaction = Transaction::new_coinbase(
            miner_address.clone(),
            self.mining_reward,
        );

        // Get pending transactions and add reward
        let mut pending = self.pending_transactions.lock().unwrap();

        // Process all transactions
        for transaction in pending.iter() {
            if !transaction.is_coinbase() {
                // Transfer funds
                self.account_state.transfer(
                    &transaction.sender,
                    &transaction.recipient,
                    transaction.amount,
                    transaction.fee,
                    transaction.nonce,
                )?;
            }
        }

        // Process mining reward
        self.account_state.process_mining_reward(&miner_address, self.mining_reward)?;

        // Add reward transaction to pending transactions
        pending.push(reward_transaction);
        let transactions = pending.clone();

        // Clear pending transactions
        pending.clear();

        // Get the last block
        let last_block = self.get_last_block();

        // Mine the new block
        let new_block = self.proof_of_work(
            last_block.index + 1,
            transactions,
            last_block.hash,
        );

        // Add the new block to the chain
        self.chain.lock().unwrap().push(new_block.clone());

        // Save to storage if available
        if let Some(storage) = &self.storage {
            // Save the block
            storage.save_block(&new_block)?;

            // Save all transactions in the block
            for transaction in &new_block.transactions {
                storage.save_transaction(transaction)?;
            }

            // Save account state
            for account in self.account_state.get_all_accounts() {
                storage.save_account(&account)?;
            }

            // Flush storage to disk
            storage.flush()?;

            info!("Saved block {} to storage", new_block.index);
        }

        Ok(new_block)
    }

    /// Performs proof of work to find a valid hash
    ///
    /// # Arguments
    ///
    /// * `index` - The index of the new block
    /// * `transactions` - The transactions to include in the block
    /// * `previous_hash` - The hash of the previous block
    ///
    /// # Returns
    ///
    /// The newly mined block with a valid proof
    fn proof_of_work(&self, index: u64, transactions: Vec<Transaction>, previous_hash: String) -> Block {
        let mut proof = 0;
        let target = "0".repeat(self.difficulty as usize);

        loop {
            let block = Block::new(index, transactions.clone(), proof, previous_hash.clone());
            let hash = block.calculate_hash();

            if hash.starts_with(&target) {
                return Block {
                    hash,
                    ..block
                };
            }

            proof += 1;
        }
    }

    /// Gets the entire blockchain
    ///
    /// # Returns
    ///
    /// A vector of all blocks in the chain
    pub fn get_chain(&self) -> Vec<Block> {
        self.chain.lock().unwrap().clone()
    }

    /// Gets all pending transactions
    ///
    /// # Returns
    ///
    /// A vector of all pending transactions
    pub fn get_pending_transactions(&self) -> Vec<Transaction> {
        self.pending_transactions.lock().unwrap().clone()
    }

    /// Gets the account state
    ///
    /// # Returns
    ///
    /// The account state
    pub fn get_account_state(&self) -> Arc<AccountState> {
        self.account_state.clone()
    }

    /// Validates the blockchain
    ///
    /// # Returns
    ///
    /// true if the blockchain is valid, false otherwise
    pub fn is_valid(&self) -> bool {
        let chain = self.chain.lock().unwrap();

        for i in 1..chain.len() {
            let current_block = &chain[i];
            let previous_block = &chain[i - 1];

            // Check if the hash is correct
            if current_block.hash != current_block.calculate_hash() {
                return false;
            }

            // Check if the previous hash is correct
            if current_block.previous_hash != previous_block.hash {
                return false;
            }
        }

        true
    }

    /// Loads the blockchain from storage
    ///
    /// # Returns
    ///
    /// Result with () if successful
    fn load_from_storage(&mut self) -> Result<(), BlockchainError> {
        let storage = match &self.storage {
            Some(storage) => storage,
            None => return Err(BlockchainError::SystemError("No storage configured".to_string())),
        };

        // Get all blocks from storage
        let blocks = storage.get_all_blocks()?;

        if blocks.is_empty() {
            return Err(BlockchainError::StorageError(StorageError::NotFound("No blocks found in storage".to_string())));
        }

        // Replace the chain with the loaded blocks
        *self.chain.lock().unwrap() = blocks;

        // Load account state from storage
        info!("Loading account state from storage");
        match storage.get_all_accounts() {
            Ok(accounts) => {
                info!("Loaded {} accounts from storage", accounts.len());
                for account in accounts {
                    info!("Loaded account {} with balance {}", account.address.0, account.balance);
                    self.account_state.update_account(account);
                }
            },
            Err(err) => {
                warn!("Failed to load accounts from storage: {}", err);
                warn!("Account state may be incomplete");

                // Rebuild account state from transactions in the chain
                info!("Rebuilding account state from blockchain transactions");
                self.rebuild_account_state()?;
            }
        }

        Ok(())
    }

    /// Rebuilds the account state from transactions in the chain
    ///
    /// # Returns
    ///
    /// Result with () if successful
    fn rebuild_account_state(&self) -> Result<(), BlockchainError> {
        // Get all blocks
        let chain = self.chain.lock().unwrap();

        // Process all transactions in all blocks
        for block in chain.iter() {
            for transaction in &block.transactions {
                if !transaction.is_coinbase() {
                    // Transfer funds
                    self.account_state.transfer(
                        &transaction.sender,
                        &transaction.recipient,
                        transaction.amount,
                        transaction.fee,
                        transaction.nonce,
                    )?;
                } else {
                    // Process mining reward
                    self.account_state.process_mining_reward(&transaction.recipient, transaction.amount)?;
                }
            }
        }

        info!("Account state rebuilt from {} blocks", chain.len());
        Ok(())
    }

    /// Saves the blockchain to storage
    ///
    /// # Returns
    ///
    /// Result with () if successful
    fn save_to_storage(&self) -> Result<(), BlockchainError> {
        let storage = match &self.storage {
            Some(storage) => storage,
            None => return Err(BlockchainError::SystemError("No storage configured".to_string())),
        };

        // Save all blocks to storage
        for block in self.chain.lock().unwrap().iter() {
            storage.save_block(block)?;

            // Save all transactions in the block
            for transaction in &block.transactions {
                storage.save_transaction(transaction)?;
            }
        }

        // Save account state
        for account in self.account_state.get_all_accounts() {
            storage.save_account(&account)?;
        }

        // Flush storage to disk
        storage.flush()?;

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::blockchain::crypto::Wallet;

    #[test]
    fn test_new_blockchain() {
        let blockchain = Blockchain::new();
        let chain = blockchain.get_chain();

        assert_eq!(chain.len(), 1);
        assert_eq!(chain[0].index, 0);
    }

    #[test]
    fn test_add_transaction() {
        let blockchain = Blockchain::new();
        let sender_wallet = Wallet::new().unwrap();
        let recipient_wallet = Wallet::new().unwrap();

        // Create a transaction
        let mut transaction = Transaction::new(
            sender_wallet.address().clone(),
            recipient_wallet.address().clone(),
            10.0,
            0.1,
            0,
        );

        // Sign the transaction
        transaction.sign(&sender_wallet).unwrap();

        // Add funds to sender's account
        let mut sender_account = blockchain.account_state.get_account(sender_wallet.address());
        sender_account.deposit(100.0).unwrap();
        blockchain.account_state.update_account(sender_account);

        // Add the transaction
        let block_index = blockchain.add_transaction(transaction).unwrap();
        assert_eq!(block_index, 1);

        let pending = blockchain.get_pending_transactions();
        assert_eq!(pending.len(), 1);
    }

    #[test]
    fn test_mine_block() {
        let blockchain = Blockchain::new();
        let sender_wallet = Wallet::new().unwrap();
        let recipient_wallet = Wallet::new().unwrap();

        // Create a transaction
        let mut transaction = Transaction::new(
            sender_wallet.address().clone(),
            recipient_wallet.address().clone(),
            10.0,
            0.1,
            0,
        );

        // Sign the transaction
        transaction.sign(&sender_wallet).unwrap();

        // Add funds to sender's account
        let mut sender_account = blockchain.account_state.get_account(sender_wallet.address());
        sender_account.deposit(100.0).unwrap();
        blockchain.account_state.update_account(sender_account);

        // Add the transaction
        blockchain.add_transaction(transaction).unwrap();

        // Mine a block
        let miner_address = sender_wallet.address().0.clone();
        let block = blockchain.mine_block(&miner_address).unwrap();

        assert_eq!(block.index, 1);
        assert_eq!(block.transactions.len(), 2); // Original transaction + mining reward

        // Check that the pending transactions are cleared
        let pending = blockchain.get_pending_transactions();
        assert_eq!(pending.len(), 0);

        // Check that the recipient received the funds
        let recipient_account = blockchain.account_state.get_account(recipient_wallet.address());
        assert_eq!(recipient_account.balance, 10.0);

        // Check that the miner received the reward
        let miner_account = blockchain.account_state.get_account(sender_wallet.address());
        assert_eq!(miner_account.balance, 139.9); // 100 - 10 - 0.1 + 50 (mining reward)
    }

    #[test]
    fn test_blockchain_validity() {
        let blockchain = Blockchain::new();
        let sender_wallet = Wallet::new().unwrap();
        let recipient_wallet = Wallet::new().unwrap();

        // Create a transaction
        let mut transaction = Transaction::new(
            sender_wallet.address().clone(),
            recipient_wallet.address().clone(),
            10.0,
            0.1,
            0,
        );

        // Sign the transaction
        transaction.sign(&sender_wallet).unwrap();

        // Add funds to sender's account
        let mut sender_account = blockchain.account_state.get_account(sender_wallet.address());
        sender_account.deposit(100.0).unwrap();
        blockchain.account_state.update_account(sender_account);

        // Add the transaction and mine a block
        blockchain.add_transaction(transaction).unwrap();
        blockchain.mine_block(&sender_wallet.address().0).unwrap();

        // The blockchain should be valid
        assert!(blockchain.is_valid());
    }
}
