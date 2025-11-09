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

    // Calculate balance
    let mut confirmed_balance: u64 = 0;
    let mut unconfirmed_balance: u64 = 0;

    for tx in &txs {
        let tx_confirmed = tx.status.confirmed;

        for output in &tx.vout {
            if output.scriptpubkey == script {
                if tx_confirmed {
                    confirmed_balance += output.value;
                } else {
                    unconfirmed_balance += output.value;
                }
            }
        }

        for input in &tx.vin {
            if let Some(prevout) = &input.prevout {
                if prevout.scriptpubkey == script {
                    if tx_confirmed {
                        confirmed_balance = confirmed_balance.saturating_sub(prevout.value);
                    } else {
                        unconfirmed_balance = unconfirmed_balance.saturating_sub(prevout.value);
                    }
                }
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
            for tx in txs {
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
