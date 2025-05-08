// Blockchain module
//
// This module contains the core blockchain implementation including:
// - Block structure
// - Blockchain structure
// - Transaction structure
// - Cryptography utilities
// - Account state
// - Proof of work algorithm

pub mod block;
pub mod chain;
pub mod crypto;
pub mod transaction;
pub mod account;
pub mod storage;

// Re-export main components for easier access
pub use block::Block;
pub use chain::Blockchain;
pub use transaction::Transaction;
pub use crypto::{Address, DigitalSignature, Wallet};
// Account state is used internally by the blockchain
