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
struct Output {
    value: u64,
    scriptpubkey: String,
}

#[derive(Debug, Deserialize)]
struct TxStatus {
    confirmed: bool,
    #[serde(default)]
    block_height: Option<u32>,
}

fn main() {
    let args = Args::parse();

    // Validate network
    let network = match args.network.as_str() {
        "testnet" => "testnet",
        "bitcoin" | "mainnet" => "mainnet",
        _ => {
            eprintln!("Invalid network. Use 'testnet' or 'bitcoin'");
            return;
        }
    };

    println!("=== Bitcoin Block Explorer ===\n");
    println!("Network: {}", if network == "mainnet" { "Bitcoin Mainnet" } else { "Bitcoin Testnet" });

    // Build Esplora URL
    let esplora_url = if network == "mainnet" {
        "https://blockstream.info/api"
    } else {
        "https://blockstream.info/testnet/api"
    };

    println!("API: {}\n", esplora_url);

    // Determine if input is a height (number) or hash (hex string)
    let block_hash = if let Ok(height) = args.block.parse::<u32>() {
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
    } else {
        // Input is assumed to be a block hash
        println!("Querying block with hash {}...\n", args.block);
        args.block.clone()
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
            let is_coinbase = tx.vin.iter().any(|input| input.is_coinbase);
            if is_coinbase {
                println!("    Type:     Coinbase (Block Reward)");
            }

            // Calculate total output value
            let total_out: u64 = tx.vout.iter().map(|o| o.value).sum();
            println!("    Total Out: {} sats ({:.8} BTC)", total_out, total_out as f64 / 100_000_000.0);

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
