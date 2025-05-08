use actix_web::web;

use super::handlers;

/// Configures the API routes
///
/// # Arguments
///
/// * `cfg` - The service configuration
pub fn configure_routes(cfg: &mut web::ServiceConfig) {
    cfg.service(
        web::scope("/api/v1")
            .route("/chain", web::get().to(handlers::get_chain))
            .route("/transactions/pending", web::get().to(handlers::get_pending_transactions))
            .route("/transactions/new", web::post().to(handlers::new_transaction))
            .route("/mine", web::post().to(handlers::mine_block))
            .route("/validate", web::get().to(handlers::validate_chain))
            .route("/wallet/new", web::post().to(handlers::create_wallet))
            .route("/wallet/fund", web::post().to(handlers::fund_wallet))
            .route("/wallet/balance/{address}", web::get().to(handlers::get_wallet_balance))
            .route("/accounts", web::get().to(handlers::get_all_accounts))
    );
}
