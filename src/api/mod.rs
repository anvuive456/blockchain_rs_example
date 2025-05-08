// API module
//
// This module contains the API implementation for the blockchain

pub mod handlers;
pub mod routes;
pub mod schema;

// Re-export main components for easier access
pub use routes::configure_routes;
