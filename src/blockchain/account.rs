use dashmap::DashMap;
use serde::{Deserialize, Serialize};
use thiserror::Error;
use utoipa::ToSchema;

use std::sync::Arc;

use super::crypto::Address;

/// Errors that can occur during account operations
#[derive(Debug, Error)]
pub enum AccountError {
    #[error("Account not found: {0}")]
    AccountNotFound(String),

    #[error("Insufficient funds: required {required}, available {available}")]
    InsufficientFunds { required: f64, available: f64 },

    #[error("Invalid amount: {0}")]
    InvalidAmount(String),

    #[error("Invalid nonce: expected {expected}, got {got}")]
    InvalidNonce { expected: u64, got: u64 },

    #[error("System error: {0}")]
    SystemError(String),
}

/// Represents an account in the blockchain
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct Account {
    /// Version of the account structure
    #[serde(default = "default_version")]
    pub version: u32,

    /// The account's address
    pub address: Address,

    /// The account's balance
    pub balance: f64,

    /// The account's nonce (used to prevent replay attacks)
    pub nonce: u64,
}

/// Default version for accounts
fn default_version() -> u32 {
    1
}

impl Account {
    /// Creates a new account
    ///
    /// # Arguments
    ///
    /// * `address` - The account's address
    ///
    /// # Returns
    ///
    /// A new Account instance
    pub fn new(address: Address) -> Self {
        Account {
            version: default_version(),
            address,
            balance: 0.0,
            nonce: 0,
        }
    }

    /// Increases the account's balance
    ///
    /// # Arguments
    ///
    /// * `amount` - The amount to add
    ///
    /// # Returns
    ///
    /// Result indicating success or failure
    pub fn deposit(&mut self, amount: f64) -> Result<(), AccountError> {
        if amount <= 0.0 {
            return Err(AccountError::InvalidAmount(format!(
                "Amount must be positive: {}",
                amount
            )));
        }

        self.balance += amount;
        Ok(())
    }

    /// Decreases the account's balance
    ///
    /// # Arguments
    ///
    /// * `amount` - The amount to subtract
    ///
    /// # Returns
    ///
    /// Result indicating success or failure
    pub fn withdraw(&mut self, amount: f64) -> Result<(), AccountError> {
        if amount <= 0.0 {
            return Err(AccountError::InvalidAmount(format!(
                "Amount must be positive: {}",
                amount
            )));
        }

        if self.balance < amount {
            return Err(AccountError::InsufficientFunds {
                required: amount,
                available: self.balance,
            });
        }

        self.balance -= amount;
        Ok(())
    }

    /// Increments the account's nonce
    pub fn increment_nonce(&mut self) {
        self.nonce += 1;
    }

    /// Checks if the account has sufficient funds
    ///
    /// # Arguments
    ///
    /// * `amount` - The amount to check
    ///
    /// # Returns
    ///
    /// true if the account has sufficient funds, false otherwise
    pub fn has_sufficient_funds(&self, amount: f64) -> bool {
        self.balance >= amount
    }

    /// Checks if the nonce is valid
    ///
    /// # Arguments
    ///
    /// * `nonce` - The nonce to check
    ///
    /// # Returns
    ///
    /// true if the nonce is valid, false otherwise
    pub fn is_valid_nonce(&self, nonce: u64) -> bool {
        nonce == self.nonce
    }
}

/// Manages the state of all accounts in the blockchain
#[derive(Debug, Clone)]
pub struct AccountState {
    accounts: Arc<DashMap<Address, Account>>,
}

impl AccountState {
    /// Creates a new account state
    ///
    /// # Returns
    ///
    /// A new AccountState instance
    pub fn new() -> Self {
        AccountState {
            accounts: Arc::new(DashMap::new()),
        }
    }

    /// Gets an account by address
    ///
    /// # Arguments
    ///
    /// * `address` - The account's address
    ///
    /// # Returns
    ///
    /// The account if it exists, or a new account if it doesn't
    pub fn get_account(&self, address: &Address) -> Account {
        if let Some(account) = self.accounts.get(address) {
            account.clone()
        } else {
            Account::new(address.clone())
        }
    }

    /// Updates an account
    ///
    /// # Arguments
    ///
    /// * `account` - The account to update
    pub fn update_account(&self, account: Account) {
        self.accounts.insert(account.address.clone(), account);
    }

    /// Transfers funds between accounts
    ///
    /// # Arguments
    ///
    /// * `from` - The sender's address
    /// * `to` - The recipient's address
    /// * `amount` - The amount to transfer
    /// * `fee` - The transaction fee
    /// * `nonce` - The transaction nonce
    ///
    /// # Returns
    ///
    /// Result indicating success or failure
    pub fn transfer(
        &self,
        from: &Address,
        to: &Address,
        amount: f64,
        fee: f64,
        nonce: u64,
    ) -> Result<(), AccountError> {
        // Get the sender's account
        let mut sender = self.get_account(from);

        // Check if the nonce is valid
        if !sender.is_valid_nonce(nonce) {
            return Err(AccountError::InvalidNonce {
                expected: sender.nonce,
                got: nonce,
            });
        }

        // Check if the sender has sufficient funds
        let total_amount = amount + fee;
        if !sender.has_sufficient_funds(total_amount) {
            return Err(AccountError::InsufficientFunds {
                required: total_amount,
                available: sender.balance,
            });
        }

        // Get the recipient's account
        let mut recipient = self.get_account(to);

        // Withdraw from sender
        sender.withdraw(total_amount)?;

        // Deposit to recipient
        recipient.deposit(amount)?;

        // Increment sender's nonce
        sender.increment_nonce();

        // Update accounts
        self.update_account(sender);
        self.update_account(recipient);

        Ok(())
    }

    /// Processes a mining reward
    ///
    /// # Arguments
    ///
    /// * `miner` - The miner's address
    /// * `reward` - The mining reward
    ///
    /// # Returns
    ///
    /// Result indicating success or failure
    pub fn process_mining_reward(&self, miner: &Address, reward: f64) -> Result<(), AccountError> {
        let mut account = self.get_account(miner);
        account.deposit(reward)?;
        self.update_account(account);
        Ok(())
    }

    /// Gets all accounts
    ///
    /// # Returns
    ///
    /// A vector of all accounts
    pub fn get_all_accounts(&self) -> Vec<Account> {
        self.accounts.iter().map(|entry| entry.value().clone()).collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_account_creation() {
        let address = Address("test_address".to_string());
        let account = Account::new(address.clone());

        assert_eq!(account.address, address);
        assert_eq!(account.balance, 0.0);
        assert_eq!(account.nonce, 0);
    }

    #[test]
    fn test_deposit_and_withdraw() {
        let address = Address("test_address".to_string());
        let mut account = Account::new(address);

        // Test deposit
        account.deposit(100.0).unwrap();
        assert_eq!(account.balance, 100.0);

        // Test withdraw
        account.withdraw(50.0).unwrap();
        assert_eq!(account.balance, 50.0);

        // Test insufficient funds
        let result = account.withdraw(100.0);
        assert!(result.is_err());
    }

    #[test]
    fn test_account_state() {
        let state = AccountState::new();
        let sender = Address("sender".to_string());
        let recipient = Address("recipient".to_string());

        // Test initial state
        let sender_account = state.get_account(&sender);
        assert_eq!(sender_account.balance, 0.0);

        // Update sender account with some funds
        let mut updated_sender = sender_account;
        updated_sender.deposit(100.0).unwrap();
        state.update_account(updated_sender);

        // Test transfer
        state.transfer(&sender, &recipient, 50.0, 1.0, 0).unwrap();

        // Check balances after transfer
        let sender_after = state.get_account(&sender);
        let recipient_after = state.get_account(&recipient);

        assert_eq!(sender_after.balance, 49.0);
        assert_eq!(sender_after.nonce, 1);
        assert_eq!(recipient_after.balance, 50.0);
    }
}
