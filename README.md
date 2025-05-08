# Advanced Blockchain

A comprehensive blockchain implementation with a REST API built in Rust.

## Features

- Advanced blockchain implementation with proof of work
- Digital signatures using Ed25519
- Transaction validation and balance checking
- Account state management
- Transaction fees and anti-spam measures
- RESTful API for interacting with the blockchain
- OpenAPI documentation with Swagger UI
- Comprehensive unit tests

## Project Structure

```
my_blockchain/
├── src/
│   ├── api/
│   │   ├── handlers.rs    # API request handlers
│   │   ├── mod.rs         # API module definition
│   │   ├── routes.rs      # API route configuration
│   │   └── schema.rs      # API schema definitions
│   ├── blockchain/
│   │   ├── account.rs     # Account state management
│   │   ├── block.rs       # Block structure
│   │   ├── chain.rs       # Blockchain implementation
│   │   ├── crypto.rs      # Cryptographic utilities
│   │   ├── mod.rs         # Blockchain module definition
│   │   └── transaction.rs # Transaction structure
│   └── main.rs            # Application entry point
└── Cargo.toml             # Project dependencies
```

## API Endpoints

| Method | Endpoint                         | Description                      |
| ------ | -------------------------------- | -------------------------------- |
| GET    | /api/v1/accounts                 | Get the full accounts            |
| GET    | /api/v1/chain                    | Get the full blockchain          |
| GET    | /api/v1/transactions/pending     | Get all pending transactions     |
| POST   | /api/v1/transactions/new         | Create a new transaction         |
| POST   | /api/v1/mine                     | Mine a new block                 |
| GET    | /api/v1/validate                 | Check if the blockchain is valid |
| POST   | /api/v1/wallet/new               | Create a new wallet              |
| POST   | /api/v1/wallet/fund              | Fund a wallet                    |
| GET    | /api/v1/wallet/balance/{address} | Get the balance of a wallet      |

## Getting Started

### Prerequisites

- Rust and Cargo (https://www.rust-lang.org/tools/install)

### Installation

1. Clone the repository:

   ```
   git clone https://github.com/anvuive456/my_blockchain.git
   cd my_blockchain
   ```

2. Build the project:

   ```
   cargo build
   ```

3. Run the project:

   ```
   cargo run
   ```

4. Access the API at http://localhost:8080/api/v1

5. Access the Swagger UI documentation at http://localhost:8080/swagger-ui/

## API Usage Examples

### Get the blockchain

```bash
curl -X GET http://localhost:8080/api/v1/chain
```

### Create a new transaction

```bash
curl -X POST http://localhost:8080/api/v1/transactions/new \
  -H "Content-Type: application/json" \
  -d '{
    "sender": "sender_address",
    "recipient": "recipient_address",
    "amount": 5.0,
    "fee": 0.1,
    "private_key": "your_private_key_in_hex"
  }'
```

Note: The private key is used to sign the transaction. You can get a test wallet with funds when you start the server.

### Mine a new block

```bash
curl -X POST http://localhost:8080/api/v1/mine \
  -H "Content-Type: application/json" \
  -d '{
    "miner_address": "miner_address"
  }'
```

## Testing

Run the tests with:

```bash
cargo test
```

## Advanced Features

### Digital Signatures (Ed25519)

All transactions are signed using Ed25519 digital signatures. This ensures that only the owner of a private key can create transactions from their address.

### Transaction Validation

Transactions are validated before being added to the blockchain:

- Signature verification
- Balance checking
- Nonce validation to prevent replay attacks
- Minimum fee requirement

### Account State

The blockchain maintains a state of all accounts, including:

- Account balances
- Transaction nonces
- Transaction history

### Transaction Fees

All transactions require a fee to be included in a block. This prevents spam and rewards miners.

## License

This project is licensed under the MIT License - see the [LICENSE](LICENSE) file for details.
