use bdk_wallet::bitcoin::Network;
use bdk_wallet::keys::bip39::Mnemonic;
use bdk_wallet::keys::{DerivableKey, ExtendedKey};
use bdk_wallet::{KeychainKind, Wallet};
use clap::{Parser, ValueEnum};

#[derive(Debug, Clone, ValueEnum)]
enum AddressType {
    Legacy,      // P2PKH
    Segwit,      // P2WPKH (native segwit)
    Taproot,     // P2TR
}

#[derive(Parser, Debug)]
#[command(name = "address-generator")]
#[command(about = "Generate Bitcoin addresses from a seed", long_about = None)]
struct Args {
    /// Address type to generate
    #[arg(short, long, value_enum, default_value = "segwit")]
    address_type: AddressType,
    
    /// Network (testnet or bitcoin)
    #[arg(short, long, default_value = "testnet")]
    network: String,
    
    /// Number of addresses to generate
    #[arg(short = 'c', long, default_value = "1")]
    count: u32,
    
    /// Optional mnemonic seed phrase (generates random if not provided)
    #[arg(short, long)]
    seed: Option<String>,
}

/// Validates and parses the network name
fn parse_network(network: &str) -> Result<Network, String> {
    match network {
        "testnet" => Ok(Network::Testnet),
        "bitcoin" | "mainnet" => Ok(Network::Bitcoin),
        _ => Err(format!("Invalid network: '{}'. Use 'testnet' or 'bitcoin'", network)),
    }
}

/// Returns the coin type for BIP44/84/86 derivation paths
fn get_coin_type(network: Network) -> u32 {
    if network == Network::Bitcoin { 0 } else { 1 }
}

/// Creates the descriptor string for a given address type and network
fn create_descriptor(address_type: &AddressType, xprv: &str, network: Network) -> String {
    let coin = get_coin_type(network);

    match address_type {
        AddressType::Legacy => {
            // BIP44: m/44'/coin'/0'/0/*
            format!("pkh({}/44'/{}'/0'/0/*)", xprv, coin)
        },
        AddressType::Segwit => {
            // BIP84: m/84'/coin'/0'/0/*
            format!("wpkh({}/84'/{}'/0'/0/*)", xprv, coin)
        },
        AddressType::Taproot => {
            // BIP86: m/86'/coin'/0'/0/*
            format!("tr({}/86'/{}'/0'/0/*)", xprv, coin)
        },
    }
}

/// Creates the change descriptor string for a given address type and network
fn create_change_descriptor(address_type: &AddressType, xprv: &str, network: Network) -> String {
    let coin = get_coin_type(network);

    match address_type {
        AddressType::Legacy => {
            format!("pkh({}/44'/{}'/0'/1/*)", xprv, coin)
        },
        AddressType::Segwit => {
            format!("wpkh({}/84'/{}'/0'/1/*)", xprv, coin)
        },
        AddressType::Taproot => {
            format!("tr({}/86'/{}'/0'/1/*)", xprv, coin)
        },
    }
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
    
    // Generate or parse mnemonic
    let mnemonic = if let Some(seed_phrase) = args.seed {
        Mnemonic::parse(&seed_phrase).expect("Invalid mnemonic")
    } else {
        // Generate random mnemonic
        let mut entropy = [0u8; 16]; // 16 bytes = 128 bits = 12 words
        use rand::RngCore;
        rand::thread_rng().fill_bytes(&mut entropy);
        Mnemonic::from_entropy(&entropy).expect("Failed to generate mnemonic")
    };
    
    println!("=== Bitcoin Address Generator ===\n");
    println!("Network: {:?}", network);
    println!("Address Type: {:?}", args.address_type);
    println!("Mnemonic: {}\n", mnemonic);
    
    // Create extended key from mnemonic
    let xkey: ExtendedKey = mnemonic
        .into_extended_key()
        .expect("Failed to create extended key");
    let xprv = xkey.into_xprv(network).expect("Failed to create xprv");

    // Create descriptors using helper functions
    let xprv_str = xprv.to_string();
    let descriptor = create_descriptor(&args.address_type, &xprv_str, network);
    let change_descriptor = create_change_descriptor(&args.address_type, &xprv_str, network);
    
    let mut wallet = Wallet::create(descriptor, change_descriptor)
        .network(network)
        .create_wallet_no_persist()
        .expect("Failed to create wallet");
    
    // Generate addresses
    println!("Generated Addresses:");
    for i in 0..args.count {
        let address = wallet.reveal_next_address(KeychainKind::External);
        println!("  {}: {}", i, address.address);
    }
    
    if network == Network::Bitcoin {
        println!("\n⚠️  WARNING: These are REAL Bitcoin addresses!");
        println!("⚠️  Keep your seed phrase secure!");
    } else {
        println!("\n✓ Testnet addresses - safe to experiment with");
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
    fn test_get_coin_type_mainnet() {
        assert_eq!(get_coin_type(Network::Bitcoin), 0);
    }

    #[test]
    fn test_get_coin_type_testnet() {
        assert_eq!(get_coin_type(Network::Testnet), 1);
    }

    #[test]
    fn test_create_descriptor_legacy_mainnet() {
        let xprv = "test_xprv";
        let desc = create_descriptor(&AddressType::Legacy, xprv, Network::Bitcoin);
        assert_eq!(desc, "pkh(test_xprv/44'/0'/0'/0/*)");
    }

    #[test]
    fn test_create_descriptor_legacy_testnet() {
        let xprv = "test_xprv";
        let desc = create_descriptor(&AddressType::Legacy, xprv, Network::Testnet);
        assert_eq!(desc, "pkh(test_xprv/44'/1'/0'/0/*)");
    }

    #[test]
    fn test_create_descriptor_segwit_mainnet() {
        let xprv = "test_xprv";
        let desc = create_descriptor(&AddressType::Segwit, xprv, Network::Bitcoin);
        assert_eq!(desc, "wpkh(test_xprv/84'/0'/0'/0/*)");
    }

    #[test]
    fn test_create_descriptor_segwit_testnet() {
        let xprv = "test_xprv";
        let desc = create_descriptor(&AddressType::Segwit, xprv, Network::Testnet);
        assert_eq!(desc, "wpkh(test_xprv/84'/1'/0'/0/*)");
    }

    #[test]
    fn test_create_descriptor_taproot_mainnet() {
        let xprv = "test_xprv";
        let desc = create_descriptor(&AddressType::Taproot, xprv, Network::Bitcoin);
        assert_eq!(desc, "tr(test_xprv/86'/0'/0'/0/*)");
    }

    #[test]
    fn test_create_descriptor_taproot_testnet() {
        let xprv = "test_xprv";
        let desc = create_descriptor(&AddressType::Taproot, xprv, Network::Testnet);
        assert_eq!(desc, "tr(test_xprv/86'/1'/0'/0/*)");
    }

    #[test]
    fn test_create_change_descriptor_legacy_mainnet() {
        let xprv = "test_xprv";
        let desc = create_change_descriptor(&AddressType::Legacy, xprv, Network::Bitcoin);
        assert_eq!(desc, "pkh(test_xprv/44'/0'/0'/1/*)");
    }

    #[test]
    fn test_create_change_descriptor_segwit_testnet() {
        let xprv = "test_xprv";
        let desc = create_change_descriptor(&AddressType::Segwit, xprv, Network::Testnet);
        assert_eq!(desc, "wpkh(test_xprv/84'/1'/0'/1/*)");
    }

    #[test]
    fn test_create_change_descriptor_taproot_mainnet() {
        let xprv = "test_xprv";
        let desc = create_change_descriptor(&AddressType::Taproot, xprv, Network::Bitcoin);
        assert_eq!(desc, "tr(test_xprv/86'/0'/0'/1/*)");
    }

    #[test]
    fn test_mnemonic_parsing() {
        // Test with a valid 12-word mnemonic
        let mnemonic_str = "abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon about";
        let mnemonic = Mnemonic::parse(mnemonic_str);
        assert!(mnemonic.is_ok());
    }

    #[test]
    fn test_mnemonic_parsing_invalid() {
        // Test with invalid mnemonic
        let mnemonic_str = "invalid invalid invalid";
        let mnemonic = Mnemonic::parse(mnemonic_str);
        assert!(mnemonic.is_err());
    }

    #[test]
    fn test_address_type_descriptor_consistency() {
        // Ensure all address types produce valid descriptors
        let xprv = "test";
        let types = vec![AddressType::Legacy, AddressType::Segwit, AddressType::Taproot];

        for addr_type in types {
            let desc = create_descriptor(&addr_type, xprv, Network::Bitcoin);
            let change_desc = create_change_descriptor(&addr_type, xprv, Network::Bitcoin);

            // Verify descriptors are not empty and contain expected patterns
            assert!(!desc.is_empty());
            assert!(!change_desc.is_empty());
            assert!(desc.contains(xprv));
            assert!(change_desc.contains(xprv));

            // Verify external path ends with /0/*
            assert!(desc.ends_with("/0'/0/*)"));
            // Verify change path ends with /1/*
            assert!(change_desc.ends_with("/0'/1/*)"));
        }
    }
}