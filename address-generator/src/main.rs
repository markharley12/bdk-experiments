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
    
    // Create descriptor based on address type
    // Using standard BIP44/84/86 derivation paths
    let coin = if network == Network::Bitcoin { 0 } else { 1 };
    
    let descriptor = match args.address_type {
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
    };
    
    // Create change descriptor (uses 1 instead of 0 for internal/change addresses)
    let change_descriptor = match args.address_type {
        AddressType::Legacy => {
            format!("pkh({}/44'/{}'/0'/1/*)", xprv, coin)
        },
        AddressType::Segwit => {
            format!("wpkh({}/84'/{}'/0'/1/*)", xprv, coin)
        },
        AddressType::Taproot => {
            format!("tr({}/86'/{}'/0'/1/*)", xprv, coin)
        },
    };
    
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