use clap::Parser;
use chrono::{DateTime, Utc};
use serde::Deserialize;

#[derive(Parser, Debug)]
#[command(name = "block-explorer")]
#[command(about = "Explore Bitcoin blocks by height or hash", long_about = None)]
struct Args {
    /// Block height or block hash to query
    block: String,

    /// Network (testnet or bitcoin)
    #[arg(short, long, default_value = "testnet")]
    network: String,

    /// Show transactions in the block
    #[arg(short, long)]
    txs: bool,

    /// Limit number of transactions to display (default: 10)
    #[arg(short, long, default_value = "10")]
    limit: usize,
}

#[derive(Debug, Deserialize)]
struct BlockInfo {
    id: String,
    height: u32,
    version: u32,
    timestamp: u64,
    tx_count: usize,
    size: usize,
    weight: usize,
    merkle_root: String,
    previousblockhash: Option<String>,
    #[serde(default)]
    mediantime: u64,
    nonce: u32,
    bits: u32,
    difficulty: f64,
}

#[derive(Debug, Deserialize)]
#[allow(dead_code)]
struct Transaction {
    txid: String,
    version: u32,
    locktime: u32,
    vin: Vec<Input>,
    vout: Vec<Output>,
    size: usize,
    weight: usize,
    #[serde(default)]
    fee: u64,
    status: TxStatus,
}

#[derive(Debug, Deserialize)]
#[allow(dead_code)]
struct Input {
    txid: String,
    vout: u32,
    #[serde(default)]
    is_coinbase: bool,
    scriptsig: String,
    #[serde(default)]
    sequence: u32,
}

#[derive(Debug, Deserialize)]
#[allow(dead_code)]
struct Output {
    value: u64,
    scriptpubkey: String,
}

#[derive(Debug, Deserialize)]
#[allow(dead_code)]
struct TxStatus {
    confirmed: bool,
    #[serde(default)]
    block_height: Option<u32>,
}

/// Validates and normalizes the network name
fn validate_network(network: &str) -> Result<&'static str, String> {
    match network {
        "testnet" => Ok("testnet"),
        "bitcoin" | "mainnet" => Ok("mainnet"),
        _ => Err(format!("Invalid network: '{}'. Use 'testnet' or 'bitcoin'", network)),
    }
}

/// Returns the Esplora API URL for the given network
fn get_esplora_url(network: &str) -> &'static str {
    if network == "mainnet" {
        "https://blockstream.info/api"
    } else {
        "https://blockstream.info/testnet/api"
    }
}

/// Determines if the input is a block height (number) or hash
fn parse_block_identifier(input: &str) -> BlockIdentifier {
    if let Ok(height) = input.parse::<u32>() {
        BlockIdentifier::Height(height)
    } else {
        BlockIdentifier::Hash(input.to_string())
    }
}

/// Represents either a block height or hash
#[derive(Debug, PartialEq, Eq)]
enum BlockIdentifier {
    Height(u32),
    Hash(String),
}

/// Formats a network name for display
fn format_network_name(network: &str) -> &'static str {
    if network == "mainnet" {
        "Bitcoin Mainnet"
    } else {
        "Bitcoin Testnet"
    }
}

/// Checks if a transaction is a coinbase transaction
fn is_coinbase_tx(tx: &Transaction) -> bool {
    tx.vin.iter().any(|input| input.is_coinbase)
}

/// Calculates total output value in satoshis
fn calculate_total_output(tx: &Transaction) -> u64 {
    tx.vout.iter().map(|o| o.value).sum()
}

/// Converts satoshis to BTC
fn sats_to_btc(sats: u64) -> f64 {
    sats as f64 / 100_000_000.0
}

fn main() {
    let args = Args::parse();

    // Validate network
    let network = match validate_network(&args.network) {
        Ok(net) => net,
        Err(err) => {
            eprintln!("{}", err);
            return;
        }
    };

    println!("=== Bitcoin Block Explorer ===\n");
    println!("Network: {}", format_network_name(network));

    // Build Esplora URL
    let esplora_url = get_esplora_url(network);

    println!("API: {}\n", esplora_url);

    // Determine if input is a height (number) or hash (hex string)
    let block_hash = match parse_block_identifier(&args.block) {
        BlockIdentifier::Height(height) => {
            // Input is a block height - get the hash first
            println!("Querying block at height {}...", height);
            let url = format!("{}/block-height/{}", esplora_url, height);

            match ureq::get(&url).call() {
                Ok(response) => {
                    match response.into_string() {
                        Ok(hash) => {
                            println!("Block hash: {}\n", hash);
                            hash
                        }
                        Err(e) => {
                            eprintln!("Error reading response: {}", e);
                            return;
                        }
                    }
                }
                Err(e) => {
                    eprintln!("Error fetching block hash: {}", e);
                    eprintln!("\nNote: This tool requires internet access to query the blockchain.");
                    return;
                }
            }
        }
        BlockIdentifier::Hash(hash) => {
            // Input is assumed to be a block hash
            println!("Querying block with hash {}...\n", hash);
            hash
        }
    };

    // Fetch block information
    let url = format!("{}/block/{}", esplora_url, block_hash);
    let block: BlockInfo = match ureq::get(&url).call() {
        Ok(response) => {
            match response.into_json() {
                Ok(data) => data,
                Err(e) => {
                    eprintln!("Error parsing block data: {}", e);
                    return;
                }
            }
        }
        Err(e) => {
            eprintln!("Error fetching block: {}", e);
            return;
        }
    };

    // Display block information
    println!("╔════════════════════════════════════════════════════════════════════");
    println!("║ BLOCK INFORMATION");
    println!("╠════════════════════════════════════════════════════════════════════");
    println!("║ Hash:        {}", block.id);
    println!("║ Height:      {}", block.height);
    println!("║ Version:     {}", block.version);

    if let Some(prev) = &block.previousblockhash {
        println!("║ Previous:    {}", prev);
    } else {
        println!("║ Previous:    None (Genesis Block)");
    }

    println!("║ Merkle Root: {}", block.merkle_root);

    // Convert timestamp to human-readable format
    let datetime = DateTime::<Utc>::from_timestamp(block.timestamp as i64, 0)
        .unwrap_or_else(|| Utc::now());
    println!("║ Timestamp:   {} ({})", block.timestamp, datetime.format("%Y-%m-%d %H:%M:%S UTC"));

    if block.mediantime > 0 {
        let median_dt = DateTime::<Utc>::from_timestamp(block.mediantime as i64, 0)
            .unwrap_or_else(|| Utc::now());
        println!("║ Median Time: {} ({})", block.mediantime, median_dt.format("%Y-%m-%d %H:%M:%S UTC"));
    }

    println!("║ Bits:        {}", block.bits);
    println!("║ Nonce:       {}", block.nonce);
    println!("║ Difficulty:  {:.2}", block.difficulty);
    println!("║ Size:        {} bytes", block.size);
    println!("║ Weight:      {} WU", block.weight);
    println!("║ Transactions: {}", block.tx_count);
    println!("╚════════════════════════════════════════════════════════════════════");

    // Show transactions if requested
    if args.txs && block.tx_count > 0 {
        println!("\n╔════════════════════════════════════════════════════════════════════");
        println!("║ TRANSACTIONS (showing {} of {})",
                 std::cmp::min(args.limit, block.tx_count), block.tx_count);
        println!("╠════════════════════════════════════════════════════════════════════");

        // Fetch transaction IDs
        let txids_url = format!("{}/block/{}/txids", esplora_url, block_hash);
        let txids: Vec<String> = match ureq::get(&txids_url).call() {
            Ok(response) => {
                match response.into_json() {
                    Ok(data) => data,
                    Err(e) => {
                        eprintln!("Error parsing transaction IDs: {}", e);
                        return;
                    }
                }
            }
            Err(e) => {
                eprintln!("Error fetching transaction IDs: {}", e);
                return;
            }
        };

        // Fetch details for each transaction (up to limit)
        for (i, txid) in txids.iter().take(args.limit).enumerate() {
            let tx_url = format!("{}/tx/{}", esplora_url, txid);
            let tx: Transaction = match ureq::get(&tx_url).call() {
                Ok(response) => {
                    match response.into_json() {
                        Ok(data) => data,
                        Err(e) => {
                            eprintln!("Warning: Could not parse transaction {}: {}", txid, e);
                            continue;
                        }
                    }
                }
                Err(e) => {
                    eprintln!("Warning: Could not fetch transaction {}: {}", txid, e);
                    continue;
                }
            };

            println!("\n[{}] TXID: {}", i + 1, tx.txid);
            println!("    Version:  {}", tx.version);
            println!("    Inputs:   {}", tx.vin.len());
            println!("    Outputs:  {}", tx.vout.len());
            println!("    Size:     {} bytes", tx.size);
            println!("    Weight:   {} WU", tx.weight);
            println!("    Locktime: {}", tx.locktime);

            // Check if coinbase
            if is_coinbase_tx(&tx) {
                println!("    Type:     Coinbase (Block Reward)");
            }

            // Calculate total output value
            let total_out = calculate_total_output(&tx);
            println!("    Total Out: {} sats ({:.8} BTC)", total_out, sats_to_btc(total_out));

            if tx.fee > 0 {
                println!("    Fee:      {} sats", tx.fee);
            }
        }

        if block.tx_count > args.limit {
            println!("\n... and {} more transactions", block.tx_count - args.limit);
            println!("(use --limit to show more)");
        }

        println!("\n╚════════════════════════════════════════════════════════════════════");
    }

    println!("\n✓ Query completed successfully!");
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_validate_network_testnet() {
        assert_eq!(validate_network("testnet"), Ok("testnet"));
    }

    #[test]
    fn test_validate_network_bitcoin() {
        assert_eq!(validate_network("bitcoin"), Ok("mainnet"));
    }

    #[test]
    fn test_validate_network_mainnet() {
        assert_eq!(validate_network("mainnet"), Ok("mainnet"));
    }

    #[test]
    fn test_validate_network_invalid() {
        assert!(validate_network("invalid").is_err());
        assert!(validate_network("regtest").is_err());
        assert!(validate_network("").is_err());
    }

    #[test]
    fn test_get_esplora_url_mainnet() {
        assert_eq!(get_esplora_url("mainnet"), "https://blockstream.info/api");
    }

    #[test]
    fn test_get_esplora_url_testnet() {
        assert_eq!(get_esplora_url("testnet"), "https://blockstream.info/testnet/api");
    }

    #[test]
    fn test_parse_block_identifier_height() {
        assert_eq!(
            parse_block_identifier("123456"),
            BlockIdentifier::Height(123456)
        );
        assert_eq!(
            parse_block_identifier("0"),
            BlockIdentifier::Height(0)
        );
        assert_eq!(
            parse_block_identifier("2500000"),
            BlockIdentifier::Height(2500000)
        );
    }

    #[test]
    fn test_parse_block_identifier_hash() {
        let hash = "0000000000000093bcb68c03a9a168ae252572d348a2eaeba2cdf9231d73206f";
        assert_eq!(
            parse_block_identifier(hash),
            BlockIdentifier::Hash(hash.to_string())
        );

        // Test with shorter hash
        let short_hash = "abc123";
        assert_eq!(
            parse_block_identifier(short_hash),
            BlockIdentifier::Hash(short_hash.to_string())
        );
    }

    #[test]
    fn test_format_network_name() {
        assert_eq!(format_network_name("mainnet"), "Bitcoin Mainnet");
        assert_eq!(format_network_name("testnet"), "Bitcoin Testnet");
    }

    #[test]
    fn test_sats_to_btc() {
        assert_eq!(sats_to_btc(100_000_000), 1.0);
        assert_eq!(sats_to_btc(50_000_000), 0.5);
        assert_eq!(sats_to_btc(0), 0.0);
        assert_eq!(sats_to_btc(1), 0.00000001);
        assert_eq!(sats_to_btc(2_449_190), 0.02449190);
    }

    #[test]
    fn test_is_coinbase_tx() {
        // Test coinbase transaction
        let coinbase_tx = Transaction {
            txid: "test".to_string(),
            version: 1,
            locktime: 0,
            vin: vec![Input {
                txid: "0000000000000000000000000000000000000000000000000000000000000000".to_string(),
                vout: 0,
                is_coinbase: true,
                scriptsig: "".to_string(),
                sequence: 0,
            }],
            vout: vec![],
            size: 0,
            weight: 0,
            fee: 0,
            status: TxStatus {
                confirmed: true,
                block_height: Some(123),
            },
        };
        assert!(is_coinbase_tx(&coinbase_tx));

        // Test non-coinbase transaction
        let regular_tx = Transaction {
            txid: "test".to_string(),
            version: 1,
            locktime: 0,
            vin: vec![Input {
                txid: "abc123".to_string(),
                vout: 0,
                is_coinbase: false,
                scriptsig: "".to_string(),
                sequence: 0,
            }],
            vout: vec![],
            size: 0,
            weight: 0,
            fee: 0,
            status: TxStatus {
                confirmed: true,
                block_height: Some(123),
            },
        };
        assert!(!is_coinbase_tx(&regular_tx));
    }

    #[test]
    fn test_calculate_total_output() {
        let tx = Transaction {
            txid: "test".to_string(),
            version: 1,
            locktime: 0,
            vin: vec![],
            vout: vec![
                Output {
                    value: 100_000,
                    scriptpubkey: "".to_string(),
                },
                Output {
                    value: 200_000,
                    scriptpubkey: "".to_string(),
                },
                Output {
                    value: 50_000,
                    scriptpubkey: "".to_string(),
                },
            ],
            size: 0,
            weight: 0,
            fee: 0,
            status: TxStatus {
                confirmed: true,
                block_height: Some(123),
            },
        };
        assert_eq!(calculate_total_output(&tx), 350_000);

        // Test with no outputs
        let empty_tx = Transaction {
            txid: "test".to_string(),
            version: 1,
            locktime: 0,
            vin: vec![],
            vout: vec![],
            size: 0,
            weight: 0,
            fee: 0,
            status: TxStatus {
                confirmed: true,
                block_height: Some(123),
            },
        };
        assert_eq!(calculate_total_output(&empty_tx), 0);
    }

    #[test]
    fn test_blockinfo_deserialization() {
        let json = r#"{
            "id": "0000000000000093bcb68c03a9a168ae252572d348a2eaeba2cdf9231d73206f",
            "height": 2500000,
            "version": 869416960,
            "timestamp": 1694733634,
            "tx_count": 6,
            "size": 1650,
            "weight": 5109,
            "merkle_root": "4f39919cc7a1553dc6ea43a1f46888bccd0dfbb0a492ed1b30f00d785f555eb4",
            "previousblockhash": "000000000000019405302299e109d3513edcc61b72b0007fd2599c0a485698f1",
            "mediantime": 1694733257,
            "nonce": 2655522930,
            "bits": 436469756,
            "difficulty": 4194304.0
        }"#;

        let block: Result<BlockInfo, _> = serde_json::from_str(json);
        assert!(block.is_ok());
        let block = block.unwrap();
        assert_eq!(block.height, 2500000);
        assert_eq!(block.tx_count, 6);
        assert_eq!(block.size, 1650);
    }

    #[test]
    fn test_blockinfo_deserialization_genesis() {
        // Genesis block has no previousblockhash
        let json = r#"{
            "id": "000000000933ea01ad0ee984209779baaec3ced90fa3f408719526f8d77f4943",
            "height": 0,
            "version": 1,
            "timestamp": 1296688602,
            "tx_count": 1,
            "size": 285,
            "weight": 1140,
            "merkle_root": "4a5e1e4baab89f3a32518a88c31bc87f618f76673e2cc77ab2127b7afdeda33b",
            "mediantime": 1296688602,
            "nonce": 414098458,
            "bits": 486604799,
            "difficulty": 1.0
        }"#;

        let block: Result<BlockInfo, _> = serde_json::from_str(json);
        assert!(block.is_ok());
        let block = block.unwrap();
        assert_eq!(block.height, 0);
        assert!(block.previousblockhash.is_none());
    }

    #[test]
    fn test_transaction_deserialization() {
        let json = r#"{
            "txid": "c9f85816f7f106f4ecd75ea8d3ba1cacbebd8a9cafb86a35d193024733f98988",
            "version": 1,
            "locktime": 0,
            "vin": [
                {
                    "txid": "0000000000000000000000000000000000000000000000000000000000000000",
                    "vout": 4294967295,
                    "is_coinbase": true,
                    "scriptsig": "03402826",
                    "sequence": 4294967295
                }
            ],
            "vout": [
                {
                    "value": 2449190,
                    "scriptpubkey": "76a914"
                }
            ],
            "size": 197,
            "weight": 680,
            "fee": 0,
            "status": {
                "confirmed": true,
                "block_height": 2500000
            }
        }"#;

        let tx: Result<Transaction, _> = serde_json::from_str(json);
        assert!(tx.is_ok());
        let tx = tx.unwrap();
        assert_eq!(tx.version, 1);
        assert_eq!(tx.vin.len(), 1);
        assert_eq!(tx.vout.len(), 1);
        assert!(tx.vin[0].is_coinbase);
    }
}
