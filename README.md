# BDK Experiments

A collection of Bitcoin command-line tools built with the [Bitcoin Development Kit (BDK)](https://bitcoindevkit.org/). This workspace contains three standalone tools for working with Bitcoin addresses, balances, and blockchain data.

## Tools

### 1. Address Generator
Generate Bitcoin addresses from mnemonic seed phrases with support for multiple address types.

**Features:**
- Generate random BIP39 mnemonic seeds or use existing ones
- Support for Legacy (P2PKH), SegWit (P2WPKH), and Taproot (P2TR) addresses
- Standard BIP44/84/86 derivation paths
- Generate multiple addresses at once
- Works with both testnet and mainnet

### 2. Balance Checker
Check the balance of any Bitcoin address by querying the blockchain via Esplora API.

**Features:**
- Query any Bitcoin address balance
- Shows confirmed and unconfirmed balances
- Display transaction history
- Accurate UTXO tracking
- Supports both testnet and mainnet

### 3. Block Explorer
Explore Bitcoin blocks by height or hash with detailed information display.

**Features:**
- Query blocks by height (number) or hash
- Comprehensive block information (hash, height, timestamp, difficulty, etc.)
- View transactions with configurable limits
- Coinbase transaction detection
- Human-readable timestamps
- Supports both testnet and mainnet

## Installation

### Prerequisites
- [Rust](https://rustup.rs/) 1.70 or later
- Internet connection (required for blockchain queries)

### Building from Source

```bash
# Clone the repository
git clone https://github.com/markharley12/bdk-experiments.git
cd bdk-experiments

# Build all tools
cargo build --release

# Or build a specific tool
cargo build --release --package address-generator
cargo build --release --package balance-checker
cargo build --release --package block-explorer
```

The compiled binaries will be in `target/release/`.

## Usage

### Address Generator

Generate Bitcoin addresses with various options:

```bash
# Generate a SegWit address with random seed (testnet)
cargo run --package address-generator

# Generate 5 Taproot addresses (testnet)
cargo run --package address-generator -- --address-type taproot --count 5

# Generate Legacy address on mainnet with specific seed
cargo run --package address-generator -- \
  --address-type legacy \
  --network bitcoin \
  --seed "your twelve word seed phrase here..."

# Generate SegWit addresses (default)
cargo run --package address-generator -- --count 3
```

**Options:**
- `-a, --address-type <TYPE>` - Address type: `legacy`, `segwit` (default), or `taproot`
- `-n, --network <NETWORK>` - Network: `testnet` (default) or `bitcoin`
- `-c, --count <COUNT>` - Number of addresses to generate (default: 1)
- `-s, --seed <SEED>` - Optional mnemonic seed phrase (generates random if not provided)

### Balance Checker

Check Bitcoin address balances:

```bash
# Check testnet address balance
cargo run --package balance-checker -- tb1qw508d6qejxtdg4y5r3zarvary0c5xw7kxpjzsx

# Check mainnet address balance
cargo run --package balance-checker -- \
  --network bitcoin \
  bc1qxy2kgdygjrsqtzq2n0yrf2493p83kkfjhx0wlh

# Check balance with transaction history
cargo run --package balance-checker -- \
  --txs \
  tb1qw508d6qejxtdg4y5r3zarvary0c5xw7kxpjzsx
```

**Options:**
- `-n, --network <NETWORK>` - Network: `testnet` (default) or `bitcoin`
- `-t, --txs` - Show transaction history

### Block Explorer

Explore Bitcoin blocks:

```bash
# Query testnet block by height
cargo run --package block-explorer -- 2500000

# Query block by hash
cargo run --package block-explorer -- \
  0000000000000093bcb68c03a9a168ae252572d348a2eaeba2cdf9231d73206f

# Query block with transactions (limit to 5)
cargo run --package block-explorer -- 2500000 --txs --limit 5

# Query mainnet genesis block
cargo run --package block-explorer -- --network bitcoin 0

# Query mainnet block with all transactions
cargo run --package block-explorer -- \
  --network bitcoin \
  --txs \
  --limit 100 \
  800000
```

**Options:**
- `-n, --network <NETWORK>` - Network: `testnet` (default) or `bitcoin`
- `-t, --txs` - Show transactions in the block
- `-l, --limit <LIMIT>` - Limit number of transactions to display (default: 10)

## Examples

### Generate Testnet Addresses

```bash
$ cargo run --package address-generator -- --count 3

=== Bitcoin Address Generator ===

Network: Testnet
Address Type: Segwit
Mnemonic: abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon about

Generated Addresses:
  0: tb1qw508d6qejxtdg4y5r3zarvary0c5xw7kxpjzsx
  1: tb1qrp33g0q5c5txsp9arysrx4k6zdkfs4nce4xj0gdcccefvpysxf3q0sl5k7
  2: tb1qftyhqwtwl7kx9ghyt8eaqutjn3wwzn29tq6s0m

✓ Testnet addresses - safe to experiment with
```

### Check Address Balance

```bash
$ cargo run --package balance-checker -- tb1qw508d6qejxtdg4y5r3zarvary0c5xw7kxpjzsx

=== Bitcoin Balance Checker ===

Network: Testnet
Checking: tb1qw508d6qejxtdg4y5r3zarvary0c5xw7kxpjzsx

Connecting to https://blockstream.info/testnet/api...
Fetching address information...

Balance Summary:
  Confirmed:   1500000 sats
  Unconfirmed: 0 sats
  Total:       1500000 sats
  Total:       0.01500000 BTC
```

### Explore a Block

```bash
$ cargo run --package block-explorer -- 0

=== Bitcoin Block Explorer ===

Network: Bitcoin Testnet
API: https://blockstream.info/testnet/api

Querying block at height 0...
Block hash: 000000000933ea01ad0ee984209779baaec3ced90fa3f408719526f8d77f4943

╔════════════════════════════════════════════════════════════════════
║ BLOCK INFORMATION
╠════════════════════════════════════════════════════════════════════
║ Hash:        000000000933ea01ad0ee984209779baaec3ced90fa3f408719526f8d77f4943
║ Height:      0
║ Version:     1
║ Previous:    None (Genesis Block)
║ Merkle Root: 4a5e1e4baab89f3a32518a88c31bc87f618f76673e2cc77ab2127b7afdeda33b
║ Timestamp:   1296688602 (2011-02-02 23:16:42 UTC)
║ Median Time: 1296688602 (2011-02-02 23:16:42 UTC)
║ Bits:        486604799
║ Nonce:       414098458
║ Difficulty:  1.00
║ Size:        285 bytes
║ Weight:      1140 WU
║ Transactions: 1
╚════════════════════════════════════════════════════════════════════

✓ Query completed successfully!
```

## Testing

Each tool includes comprehensive unit tests:

```bash
# Run all tests
cargo test

# Test a specific tool
cargo test --package address-generator
cargo test --package balance-checker
cargo test --package block-explorer

# Run tests with output
cargo test -- --nocapture
```

## Project Structure

```
bdk-experiments/
├── Cargo.toml              # Workspace configuration
├── Cargo.lock
├── address-generator/      # Address generation tool
│   ├── Cargo.toml
│   └── src/
│       └── main.rs
├── balance-checker/        # Balance checking tool
│   ├── Cargo.toml
│   └── src/
│       └── main.rs
└── block-explorer/         # Block exploration tool
    ├── Cargo.toml
    └── src/
        └── main.rs
```

## Dependencies

- **BDK** - Bitcoin Development Kit for wallet functionality
- **clap** - Command-line argument parsing
- **ureq** - HTTP client for API requests
- **serde/serde_json** - JSON serialization
- **chrono** - Date and time handling

## Network Support

All tools support both Bitcoin networks:
- **Testnet** (default) - Safe for experimentation, uses test coins
- **Mainnet** - Real Bitcoin network (use with caution)

## API Usage

The balance-checker and block-explorer tools use the [Blockstream Esplora API](https://github.com/Blockstream/esplora/blob/master/API.md):
- Mainnet: `https://blockstream.info/api`
- Testnet: `https://blockstream.info/testnet/api`

## Security Notes

- **Never share your seed phrase** - Anyone with your seed can access your funds
- **Use testnet for experimentation** - Testnet coins have no value
- **Verify addresses** - Always double-check addresses before sending real Bitcoin
- **Backup your seeds** - Store seed phrases securely offline

## License

MIT OR Apache-2.0

## Contributing

Contributions are welcome! Please feel free to submit issues or pull requests.

## Acknowledgments

Built with the [Bitcoin Development Kit (BDK)](https://bitcoindevkit.org/) and powered by [Blockstream's Esplora API](https://blockstream.info/).
