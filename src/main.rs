use actix_cors::Cors;
use actix_web::{middleware, web, App, HttpServer};
use log::{info, warn};
use utoipa::OpenApi;
use utoipa_swagger_ui::SwaggerUi;
use std::path::Path;

mod api;
mod blockchain;

// Initialize the blockchain with a genesis block and some initial accounts
fn initialize_blockchain() -> blockchain::Blockchain {
    // Try to initialize blockchain with storage
    let data_dir = "data/blockchain";

    // Create data directory if it doesn't exist
    std::fs::create_dir_all(data_dir).unwrap_or_else(|e| {
        warn!("Failed to create data directory: {}", e);
    });

    // Try to load blockchain from storage
    match blockchain::Blockchain::with_storage(data_dir) {
        Ok(blockchain) => {
            info!("Loaded blockchain from storage at {}", data_dir);

            // Create a wallet for testing if not already created
            create_test_wallet(&blockchain);

            blockchain
        },
        Err(err) => {
            warn!("Failed to load blockchain from storage: {}", err);
            warn!("Creating in-memory blockchain instead");

            // Create in-memory blockchain
            let blockchain = blockchain::Blockchain::new();

            // Create a wallet for testing
            create_test_wallet(&blockchain);

            blockchain
        }
    }
}

// Create a test wallet with initial funds
fn create_test_wallet(blockchain: &blockchain::Blockchain) -> Option<blockchain::Wallet> {
    match blockchain::Wallet::new() {
        Ok(wallet) => {
            info!("Created test wallet with address: {}", wallet.address());

            // Export the private key for testing
            let private_key = wallet.export_secret_key();
            let private_key_hex = hex::encode(&private_key);
            info!("Test wallet private key: {}", private_key_hex);

            // Add some initial funds to the wallet
            let mut account = blockchain.get_account_state().get_account(wallet.address());
            if let Ok(_) = account.deposit(1000.0) {
                blockchain.get_account_state().update_account(account);
                info!("Added 1000 coins to test wallet");
            }

            Some(wallet)
        },
        Err(err) => {
            warn!("Failed to create test wallet: {}", err);
            None
        }
    }
}

#[derive(OpenApi)]
#[openapi(
    paths(
        api::handlers::get_chain,
        api::handlers::get_pending_transactions,
        api::handlers::new_transaction,
        api::handlers::mine_block,
        api::handlers::validate_chain,
        api::handlers::create_wallet,
        api::handlers::fund_wallet,
        api::handlers::get_wallet_balance,
        api::handlers::get_all_accounts
    ),
    components(
        schemas(
            blockchain::Block,
            blockchain::Transaction,
            blockchain::crypto::Address,
            blockchain::crypto::DigitalSignature,
            api::schema::DateTimeUtc,
            api::handlers::ChainResponse,
            api::handlers::TransactionRequest,
            api::handlers::TransactionResponse,
            api::handlers::MineRequest,
            api::handlers::MineResponse,
            api::handlers::WalletResponse,
            api::handlers::FundWalletRequest,
            api::handlers::AccountResponse
        )
    ),
    tags(
        (name = "blockchain", description = "Blockchain API endpoints")
    ),
    info(
        title = "Blockchain API",
        version = "1.0.0",
        description = "A simple blockchain API",
        license(
            name = "MIT",
            url = "https://opensource.org/licenses/MIT"
        ),
        contact(
            name = "API Support",
            email = "support@example.com"
        )
    )
)]
struct ApiDoc;

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    // Initialize logger
    env_logger::init_from_env(env_logger::Env::new().default_filter_or("info"));

    // Create a new blockchain with initial data
    let blockchain = web::Data::new(initialize_blockchain());

    info!("Starting HTTP server at http://localhost:8080");

    // Start HTTP server
    HttpServer::new(move || {
        // Configure CORS
        let cors = Cors::default()
            .allow_any_origin()
            .allow_any_method()
            .allow_any_header()
            .max_age(3600);

        // Configure OpenAPI documentation
        let openapi = ApiDoc::openapi();

        App::new()
            .wrap(middleware::Logger::default())
            .wrap(cors)
            .app_data(blockchain.clone())
            // API routes
            .configure(api::configure_routes)
            // Swagger UI
            .service(
                SwaggerUi::new("/swagger-ui/{_:.*}")
                    .url("/api-docs/openapi.json", openapi.clone())
            )
    })
    .bind(("127.0.0.1", 8080))?
    .run()
    .await
}
