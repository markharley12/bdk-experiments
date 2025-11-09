use bdk::{
    bitcoin::{Network, Address},
    blockchain::esplora::EsploraBlockchain,
};
use clap::Parser;
use std::str::FromStr;

#[derive(Parser, Debug)]
#[command(name = "balance-checker")]
#[command(about = "Check Bitcoin address or wallet balance", long_about = None)]
struct Args {
    /// Bitcoin address or extended public key (xpub/ypub/zpub)
    address: String,

    /// Network (testnet or bitcoin)
    #[arg(short, long, default_value = "testnet")]
    network: String,

    /// Show transaction history
    #[arg(short, long)]
    txs: bool,
}

fn main() {
    let args = Args::parse();

    // Parse network
    let network = match args.network.as_str() {
        "testnet" => Network::Testnet,
        "bitcoin" | "mainnet" => Network::Bitcoin,
        _ => {
            eprintln!("Invalid network. Use 'testnet' or 'bitcoin'");
            return;
        }
    };

    println!("=== Bitcoin Balance Checker ===\n");
    println!("Network: {:?}", network);
    println!("Checking: {}\n", args.address);

    // Parse the address
    let address = Address::from_str(&args.address)
        .expect("Invalid Bitcoin address");

    // Verify network matches
    if !address.is_valid_for_network(network) {
        eprintln!("Error: Address is not valid for {:?} network", network);
        return;
    }

    // Connect to Esplora
    let esplora_url = if network == Network::Bitcoin {
        "https://blockstream.info/api"
    } else {
        "https://blockstream.info/testnet/api"
    };

    println!("Connecting to {}...", esplora_url);
    let blockchain = EsploraBlockchain::new(esplora_url, 20);

    println!("Fetching address information...\n");

    // Get script from address
    let script = address.script_pubkey();

    // Get all transactions for this address
    let txs = match blockchain.scripthash_txs(&script, None) {
        Ok(txs) => txs,
        Err(e) => {
            eprintln!("Error fetching transactions: {}", e);
            eprintln!("\nNote: This tool requires internet access to query the blockchain.");
            return;
        }
    };

    // Track all outputs and which ones are spent
    use std::collections::{HashMap, HashSet};

    // Map of (txid, vout) -> (value, confirmed)
    let mut outputs: HashMap<(String, u32), (u64, bool)> = HashMap::new();

    // Set of spent outputs (txid, vout)
    let mut spent_outputs: HashSet<(String, u32)> = HashSet::new();

    // First pass: collect all outputs belonging to this address
    for tx in &txs {
        for (vout_index, output) in tx.vout.iter().enumerate() {
            if output.scriptpubkey == script {
                let key = (tx.txid.to_string(), vout_index as u32);
                outputs.insert(key, (output.value, tx.status.confirmed));
                eprintln!("DEBUG: Found output: {}:{} = {} sats", tx.txid, vout_index, output.value);
            }
        }
    }

    eprintln!("DEBUG: Total outputs found: {}", outputs.len());

    // Second pass: mark spent outputs
    for tx in &txs {
        for input in &tx.vin {
            if let Some(prevout) = &input.prevout {
                if prevout.scriptpubkey == script {
                    let key = (input.txid.to_string(), input.vout);
                    spent_outputs.insert(key);
                    eprintln!("DEBUG: Marked as spent: {}:{}", input.txid, input.vout);
                }
            }
        }
    }

    eprintln!("DEBUG: Total spent outputs: {}", spent_outputs.len());
    eprintln!("DEBUG: Unspent outputs: {}", outputs.len() - spent_outputs.len());

    // Calculate balance from unspent outputs
    let mut confirmed_balance: u64 = 0;
    let mut unconfirmed_balance: u64 = 0;

    for (outpoint, (value, is_confirmed)) in &outputs {
        if !spent_outputs.contains(outpoint) {
            // This output is unspent
            eprintln!("DEBUG: Unspent UTXO: {}:{} = {} sats (confirmed: {})", outpoint.0, outpoint.1, value, is_confirmed);
            if *is_confirmed {
                confirmed_balance += value;
            } else {
                unconfirmed_balance += value;
            }
        }
    }

    let total_balance = confirmed_balance + unconfirmed_balance;

    println!("Balance Summary:");
    println!("  Confirmed:   {} sats", confirmed_balance);
    println!("  Unconfirmed: {} sats", unconfirmed_balance);
    println!("  Total:       {} sats", total_balance);

    // Convert to BTC
    let btc = total_balance as f64 / 100_000_000.0;
    println!("  Total:       {:.8} BTC", btc);

    // Show transactions if requested
    if args.txs {
        println!("\nTransaction History ({} transactions):", txs.len());

        if txs.is_empty() {
            println!("  No transactions found");
        } else {
            for tx in &txs {
                println!("\n  TXID: {}", tx.txid);
                if tx.status.confirmed {
                    if let Some(height) = tx.status.block_height {
                        println!("  Confirmed at height: {}", height);
                    }
                } else {
                    println!("  Status: Unconfirmed");
                }
            }
        }
    }
}
