use bdk::{
    bitcoin::{Network, Address},
    blockchain::esplora::EsploraBlockchain,
};
use clap::Parser;
use std::collections::{HashMap, HashSet};
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

/// Balance information for an address
#[derive(Debug, PartialEq, Eq)]
struct BalanceInfo {
    confirmed: u64,
    unconfirmed: u64,
}

impl BalanceInfo {
    fn total(&self) -> u64 {
        self.confirmed + self.unconfirmed
    }
}

/// Validates and parses the network name
fn parse_network(network: &str) -> Result<Network, String> {
    match network {
        "testnet" => Ok(Network::Testnet),
        "bitcoin" | "mainnet" => Ok(Network::Bitcoin),
        _ => Err(format!("Invalid network: '{}'. Use 'testnet' or 'bitcoin'", network)),
    }
}

/// Returns the Esplora API URL for the given network
fn get_esplora_url(network: Network) -> &'static str {
    if network == Network::Bitcoin {
        "https://blockstream.info/api"
    } else {
        "https://blockstream.info/testnet/api"
    }
}

/// Converts satoshis to BTC
fn sats_to_btc(sats: u64) -> f64 {
    sats as f64 / 100_000_000.0
}

/// Calculates balance from outputs and spent outputs
fn calculate_balance(
    outputs: &HashMap<(String, u32), (u64, bool)>,
    spent_outputs: &HashSet<(String, u32)>
) -> BalanceInfo {
    let mut confirmed = 0u64;
    let mut unconfirmed = 0u64;

    for (outpoint, (value, is_confirmed)) in outputs {
        if !spent_outputs.contains(outpoint) {
            // This output is unspent
            if *is_confirmed {
                confirmed += value;
            } else {
                unconfirmed += value;
            }
        }
    }

    BalanceInfo { confirmed, unconfirmed }
}

fn main() {
    let args = Args::parse();

    // Parse network
    let network = match parse_network(&args.network) {
        Ok(net) => net,
        Err(err) => {
            eprintln!("{}", err);
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
    let esplora_url = get_esplora_url(network);

    println!("Connecting to {}...", esplora_url);
    let blockchain = EsploraBlockchain::new(esplora_url, 20);

    println!("Fetching address information...\n");

    // Get script from address
    let script = address.script_pubkey();

    // Get all transactions for this address (with pagination)
    let mut txs = Vec::new();
    let mut last_seen = None;

    loop {
        let batch = match blockchain.scripthash_txs(&script, last_seen) {
            Ok(batch) => batch,
            Err(e) => {
                eprintln!("Error fetching transactions: {}", e);
                eprintln!("\nNote: This tool requires internet access to query the blockchain.");
                return;
            }
        };

        if batch.is_empty() {
            break;
        }

        last_seen = Some(batch.last().unwrap().txid);
        let batch_len = batch.len();
        txs.extend(batch);

        // If we got fewer than the page size, we're done
        if batch_len < 25 {
            break;
        }
    }

    eprintln!("DEBUG: Fetched {} total transactions", txs.len());

    // Track all outputs and which ones are spent
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
            }
        }
    }

    // Second pass: mark spent outputs
    for tx in &txs {
        for input in &tx.vin {
            if let Some(prevout) = &input.prevout {
                if prevout.scriptpubkey == script {
                    let key = (input.txid.to_string(), input.vout);
                    spent_outputs.insert(key);
                }
            }
        }
    }

    let unspent_count = outputs.iter().filter(|(k, _)| !spent_outputs.contains(k)).count();
    eprintln!("DEBUG: Total outputs: {}, Spent: {}, Unspent UTXOs: {}",
              outputs.len(), spent_outputs.len(), unspent_count);

    // Calculate balance using helper function
    let balance = calculate_balance(&outputs, &spent_outputs);

    println!("Balance Summary:");
    println!("  Confirmed:   {} sats", balance.confirmed);
    println!("  Unconfirmed: {} sats", balance.unconfirmed);
    println!("  Total:       {} sats", balance.total());

    // Convert to BTC
    println!("  Total:       {:.8} BTC", sats_to_btc(balance.total()));

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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_network_testnet() {
        assert_eq!(parse_network("testnet").unwrap(), Network::Testnet);
    }

    #[test]
    fn test_parse_network_bitcoin() {
        assert_eq!(parse_network("bitcoin").unwrap(), Network::Bitcoin);
    }

    #[test]
    fn test_parse_network_mainnet() {
        assert_eq!(parse_network("mainnet").unwrap(), Network::Bitcoin);
    }

    #[test]
    fn test_parse_network_invalid() {
        assert!(parse_network("invalid").is_err());
        assert!(parse_network("regtest").is_err());
        assert!(parse_network("").is_err());
    }

    #[test]
    fn test_get_esplora_url_mainnet() {
        assert_eq!(get_esplora_url(Network::Bitcoin), "https://blockstream.info/api");
    }

    #[test]
    fn test_get_esplora_url_testnet() {
        assert_eq!(get_esplora_url(Network::Testnet), "https://blockstream.info/testnet/api");
    }

    #[test]
    fn test_sats_to_btc() {
        assert_eq!(sats_to_btc(100_000_000), 1.0);
        assert_eq!(sats_to_btc(50_000_000), 0.5);
        assert_eq!(sats_to_btc(0), 0.0);
        assert_eq!(sats_to_btc(1), 0.00000001);
        assert_eq!(sats_to_btc(1_500_000), 0.015);
    }

    #[test]
    fn test_balance_info_total() {
        let balance = BalanceInfo {
            confirmed: 1_000_000,
            unconfirmed: 500_000,
        };
        assert_eq!(balance.total(), 1_500_000);
    }

    #[test]
    fn test_calculate_balance_no_outputs() {
        let outputs = HashMap::new();
        let spent = HashSet::new();

        let balance = calculate_balance(&outputs, &spent);

        assert_eq!(balance.confirmed, 0);
        assert_eq!(balance.unconfirmed, 0);
        assert_eq!(balance.total(), 0);
    }

    #[test]
    fn test_calculate_balance_all_confirmed_unspent() {
        let mut outputs = HashMap::new();
        outputs.insert(("tx1".to_string(), 0), (100_000, true));
        outputs.insert(("tx2".to_string(), 0), (200_000, true));
        outputs.insert(("tx3".to_string(), 1), (300_000, true));

        let spent = HashSet::new();

        let balance = calculate_balance(&outputs, &spent);

        assert_eq!(balance.confirmed, 600_000);
        assert_eq!(balance.unconfirmed, 0);
        assert_eq!(balance.total(), 600_000);
    }

    #[test]
    fn test_calculate_balance_mixed_confirmed_unconfirmed() {
        let mut outputs = HashMap::new();
        outputs.insert(("tx1".to_string(), 0), (100_000, true));  // confirmed
        outputs.insert(("tx2".to_string(), 0), (200_000, false)); // unconfirmed
        outputs.insert(("tx3".to_string(), 1), (300_000, true));  // confirmed

        let spent = HashSet::new();

        let balance = calculate_balance(&outputs, &spent);

        assert_eq!(balance.confirmed, 400_000);
        assert_eq!(balance.unconfirmed, 200_000);
        assert_eq!(balance.total(), 600_000);
    }

    #[test]
    fn test_calculate_balance_with_spent_outputs() {
        let mut outputs = HashMap::new();
        outputs.insert(("tx1".to_string(), 0), (100_000, true));
        outputs.insert(("tx2".to_string(), 0), (200_000, true));
        outputs.insert(("tx3".to_string(), 1), (300_000, true));

        let mut spent = HashSet::new();
        spent.insert(("tx1".to_string(), 0)); // tx1:0 is spent

        let balance = calculate_balance(&outputs, &spent);

        // Only tx2:0 and tx3:1 should be counted (tx1:0 is spent)
        assert_eq!(balance.confirmed, 500_000);
        assert_eq!(balance.unconfirmed, 0);
        assert_eq!(balance.total(), 500_000);
    }

    #[test]
    fn test_calculate_balance_all_spent() {
        let mut outputs = HashMap::new();
        outputs.insert(("tx1".to_string(), 0), (100_000, true));
        outputs.insert(("tx2".to_string(), 0), (200_000, true));

        let mut spent = HashSet::new();
        spent.insert(("tx1".to_string(), 0));
        spent.insert(("tx2".to_string(), 0));

        let balance = calculate_balance(&outputs, &spent);

        assert_eq!(balance.confirmed, 0);
        assert_eq!(balance.unconfirmed, 0);
        assert_eq!(balance.total(), 0);
    }

    #[test]
    fn test_calculate_balance_complex_scenario() {
        let mut outputs = HashMap::new();
        // Received outputs
        outputs.insert(("tx1".to_string(), 0), (100_000, true));   // confirmed, spent
        outputs.insert(("tx2".to_string(), 0), (200_000, true));   // confirmed, unspent
        outputs.insert(("tx3".to_string(), 1), (150_000, false));  // unconfirmed, unspent
        outputs.insert(("tx4".to_string(), 0), (300_000, true));   // confirmed, spent
        outputs.insert(("tx5".to_string(), 2), (50_000, false));   // unconfirmed, spent

        let mut spent = HashSet::new();
        spent.insert(("tx1".to_string(), 0));
        spent.insert(("tx4".to_string(), 0));
        spent.insert(("tx5".to_string(), 2));

        let balance = calculate_balance(&outputs, &spent);

        // Only tx2:0 (confirmed) and tx3:1 (unconfirmed) are unspent
        assert_eq!(balance.confirmed, 200_000);
        assert_eq!(balance.unconfirmed, 150_000);
        assert_eq!(balance.total(), 350_000);
    }
}
