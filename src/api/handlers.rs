use actix_web::{web, HttpResponse, Responder};
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

use crate::blockchain::{Block, Blockchain, Transaction, Address, Wallet};
use crate::blockchain::account::Account;

/// Data structure for the blockchain state
pub type BlockchainData = web::Data<Blockchain>;

/// Response for the chain endpoint
#[derive(Serialize, Deserialize, ToSchema)]
pub struct ChainResponse {
    /// The length of the chain
    pub length: usize,

    /// The blocks in the chain
    pub chain: Vec<Block>,

    /// Whether the chain is valid
    pub is_valid: bool,
}

/// Request for the transaction endpoint
#[derive(Serialize, Deserialize, ToSchema)]
pub struct TransactionRequest {
    /// The sender's address
    pub sender: String,

    /// The recipient's address
    pub recipient: String,

    /// The amount to transfer
    pub amount: f64,

    /// The transaction fee
    pub fee: f64,

    /// The sender's private key (for signing)
    pub private_key: String,
}

/// Response for the transaction endpoint
#[derive(Serialize, Deserialize, ToSchema)]
pub struct TransactionResponse {
    /// The message
    pub message: String,

    /// The index of the block that will include this transaction
    pub block_index: u64,
}

/// Request for the mine endpoint
#[derive(Serialize, Deserialize, ToSchema)]
pub struct MineRequest {
    /// The miner's address
    pub miner_address: String,
}

/// Response for the mine endpoint
#[derive(Serialize, Deserialize, ToSchema)]
pub struct MineResponse {
    /// The message
    pub message: String,

    /// The newly mined block
    pub block: Block,
}

/// Get the full blockchain
///
/// Returns the entire blockchain and its validity status
#[utoipa::path(
    get,
    path = "/api/v1/chain",
    responses(
        (status = 200, description = "Blockchain retrieved successfully", body = ChainResponse)
    )
)]
pub async fn get_chain(blockchain: BlockchainData) -> impl Responder {
    let chain = blockchain.get_chain();
    let is_valid = blockchain.is_valid();

    let response = ChainResponse {
        length: chain.len(),
        chain,
        is_valid,
    };

    HttpResponse::Ok().json(response)
}

/// Get all pending transactions
///
/// Returns all transactions waiting to be included in a block
#[utoipa::path(
    get,
    path = "/api/v1/transactions/pending",
    responses(
        (status = 200, description = "Pending transactions retrieved successfully", body = Vec<Transaction>)
    )
)]
pub async fn get_pending_transactions(blockchain: BlockchainData) -> impl Responder {
    let transactions = blockchain.get_pending_transactions();
    HttpResponse::Ok().json(transactions)
}

/// Create a new transaction
///
/// Adds a new transaction to the pending transactions
#[utoipa::path(
    post,
    path = "/api/v1/transactions/new",
    request_body = TransactionRequest,
    responses(
        (status = 201, description = "Transaction created successfully", body = TransactionResponse),
        (status = 400, description = "Invalid transaction data"),
        (status = 500, description = "Internal server error")
    )
)]
pub async fn new_transaction(
    blockchain: BlockchainData,
    transaction_req: web::Json<TransactionRequest>,
) -> impl Responder {
    // Create addresses from strings
    let sender_address = Address(transaction_req.sender.clone());
    let recipient_address = Address(transaction_req.recipient.clone());

    // Get the sender's account to get the current nonce and check balance
    let sender_account = blockchain.get_account_state().get_account(&sender_address);
    let nonce = sender_account.nonce;

    // Check if the sender has enough balance for the transaction
    let total_amount = transaction_req.amount + transaction_req.fee;
    if sender_account.balance < total_amount {
        return HttpResponse::BadRequest().json(serde_json::json!({
            "error": format!("Insufficient funds: required {}, available {}", total_amount, sender_account.balance),
            "required": total_amount,
            "available": sender_account.balance
        }));
    }

    // Create the transaction
    let mut transaction = Transaction::new(
        sender_address,
        recipient_address,
        transaction_req.amount,
        transaction_req.fee,
        nonce,
    );

    // Create a wallet from the private key
    let private_key_bytes = match hex::decode(&transaction_req.private_key) {
        Ok(bytes) => bytes,
        Err(_) => {
            return HttpResponse::BadRequest().json(serde_json::json!({
                "error": "Invalid private key format. Must be a hex string."
            }));
        }
    };

    let wallet = match Wallet::from_secret_key(&private_key_bytes) {
        Ok(wallet) => wallet,
        Err(err) => {
            return HttpResponse::BadRequest().json(serde_json::json!({
                "error": format!("Invalid private key: {}", err)
            }));
        }
    };

    // Check if the wallet address matches the sender address
    if wallet.address().0 != transaction_req.sender {
        return HttpResponse::BadRequest().json(serde_json::json!({
            "error": "Private key does not match sender address"
        }));
    }

    // Sign the transaction
    if let Err(err) = transaction.sign(&wallet) {
        return HttpResponse::BadRequest().json(serde_json::json!({
            "error": format!("Failed to sign transaction: {}", err)
        }));
    }

    // Add the transaction to the blockchain
    match blockchain.add_transaction(transaction) {
        Ok(block_index) => {
            let response = TransactionResponse {
                message: "Transaction will be added to Block".to_string(),
                block_index,
            };

            HttpResponse::Created().json(response)
        }
        Err(err) => {
            HttpResponse::BadRequest().json(serde_json::json!({
                "error": format!("Failed to add transaction: {}", err)
            }))
        }
    }
}

/// Mine a new block
///
/// Creates a new block with all pending transactions
#[utoipa::path(
    post,
    path = "/api/v1/mine",
    request_body = MineRequest,
    responses(
        (status = 200, description = "Block mined successfully", body = MineResponse),
        (status = 400, description = "Invalid mining request"),
        (status = 500, description = "Internal server error")
    )
)]
pub async fn mine_block(
    blockchain: BlockchainData,
    mine_req: web::Json<MineRequest>,
) -> impl Responder {
    match blockchain.mine_block(&mine_req.miner_address) {
        Ok(block) => {
            let response = MineResponse {
                message: "New Block Mined".to_string(),
                block,
            };

            HttpResponse::Ok().json(response)
        }
        Err(err) => {
            HttpResponse::BadRequest().json(serde_json::json!({
                "error": format!("Failed to mine block: {}", err)
            }))
        }
    }
}

/// Check if the blockchain is valid
///
/// Validates the entire blockchain
#[utoipa::path(
    get,
    path = "/api/v1/validate",
    responses(
        (status = 200, description = "Blockchain validation status", body = bool)
    )
)]
pub async fn validate_chain(blockchain: BlockchainData) -> impl Responder {
    let is_valid = blockchain.is_valid();
    HttpResponse::Ok().json(is_valid)
}

/// Response for the create wallet endpoint
#[derive(Serialize, Deserialize, ToSchema)]
pub struct WalletResponse {
    /// The wallet's address
    pub address: String,

    /// The wallet's private key (hex encoded)
    pub private_key: String,
}

/// Create a new wallet
///
/// Creates a new wallet with a random keypair
/// 
/// The private key must be stored by your own
#[utoipa::path(
    post,
    path = "/api/v1/wallet/new",
    responses(
        (status = 201, description = "Wallet created successfully", body = WalletResponse),
        (status = 500, description = "Internal server error")
    )
)]
pub async fn create_wallet() -> impl Responder {
    match crate::blockchain::Wallet::new() {
        Ok(wallet) => {
            let address = wallet.address().0.clone();
            let private_key = hex::encode(wallet.export_secret_key());

            let response = WalletResponse {
                address,
                private_key,
            };

            HttpResponse::Created().json(response)
        },
        Err(err) => {
            HttpResponse::InternalServerError().json(serde_json::json!({
                "error": format!("Failed to create wallet: {}", err)
            }))
        }
    }
}

/// Request for the fund wallet endpoint
#[derive(Serialize, Deserialize, ToSchema)]
pub struct FundWalletRequest {
    /// The address to fund
    pub address: String,

    /// The amount to fund
    pub amount: f64,
}

/// Fund a wallet
///
/// Adds funds to a wallet for testing
#[utoipa::path(
    post,
    path = "/api/v1/wallet/fund",
    request_body = FundWalletRequest,
    responses(
        (status = 200, description = "Wallet funded successfully"),
        (status = 400, description = "Invalid address"),
        (status = 500, description = "Internal server error")
    )
)]
pub async fn fund_wallet(
    blockchain: BlockchainData,
    fund_req: web::Json<FundWalletRequest>,
) -> impl Responder {
    // Create address from string
    let address = Address(fund_req.address.clone());

    // Get the account
    let mut account = blockchain.get_account_state().get_account(&address);

    // Add funds
    match account.deposit(fund_req.amount) {
        Ok(_) => {
            // Update the account
            blockchain.get_account_state().update_account(account);

            HttpResponse::Ok().json(serde_json::json!({
                "message": format!("Added {} coins to wallet {}", fund_req.amount, fund_req.address),
                "new_balance": blockchain.get_account_state().get_account(&address).balance
            }))
        },
        Err(err) => {
            HttpResponse::BadRequest().json(serde_json::json!({
                "error": format!("Failed to fund wallet: {}", err)
            }))
        }
    }
}

/// Get wallet balance
///
/// Returns the balance of a wallet
#[utoipa::path(
    get,
    path = "/api/v1/wallet/balance/{address}",
    responses(
        (status = 200, description = "Wallet balance retrieved successfully"),
        (status = 400, description = "Invalid address"),
        (status = 500, description = "Internal server error")
    )
)]
pub async fn get_wallet_balance(
    blockchain: BlockchainData,
    address: web::Path<String>,
) -> impl Responder {
    // Create address from string
    let wallet_address = Address(address.into_inner());

    // Get the account
    let account = blockchain.get_account_state().get_account(&wallet_address);

    HttpResponse::Ok().json(serde_json::json!({
        "address": wallet_address.0,
        "balance": account.balance,
        "nonce": account.nonce
    }))
}

/// Response for the get accounts endpoint
#[derive(Serialize, Deserialize, ToSchema)]
pub struct AccountResponse {
    /// The address of the account
    pub address: String,

    /// The balance of the account
    pub balance: f64,

    /// The nonce of the account
    pub nonce: u64,

}

/// Get all accounts
///
/// Returns all accounts in the blockchain
#[utoipa::path(
    get,
    path = "/api/v1/accounts",
    responses(
        (status = 200, description = "Accounts retrieved successfully", body = Vec<AccountResponse>)
    )
)]
pub async fn get_all_accounts(blockchain: BlockchainData) -> impl Responder {
    let accounts = blockchain.get_account_state().get_all_accounts();

    let account_responses: Vec<AccountResponse> = accounts.into_iter()
        .map(|account| AccountResponse {
            address: account.address.0,
            balance: account.balance,
            nonce: account.nonce,
        })
        .collect();

    HttpResponse::Ok().json(account_responses)
}
