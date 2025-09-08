#[cfg(not(target_arch = "wasm32"))]
use clap::{Arg, Command};
use anyhow::{Context, Result};
use log::info;

mod ssh_client;
mod config;
mod key_manager;

#[cfg(not(target_arch = "wasm32"))]
mod ssh;
#[cfg(not(target_arch = "wasm32"))]
mod ssh_impl;
#[cfg(not(target_arch = "wasm32"))]
mod native;

#[cfg(target_arch = "wasm32")]
mod wasm_ssh;


#[cfg(not(target_arch = "wasm32"))]
fn main() -> Result<()> {
    env_logger::init();
    
    let matches = Command::new("bxssh")
        .version("0.1.0")
        .author("bashx-org")
        .about("A WebAssembly-compatible SSH client CLI")
        .arg(
            Arg::new("host")
                .help("SSH host to connect to")
                .required(false)
                .index(1),
        )
        .arg(
            Arg::new("port")
                .short('p')
                .long("port")
                .help("SSH port (default: 22)")
                .default_value("22"),
        )
        .arg(
            Arg::new("username")
                .short('u')
                .long("user")
                .help("SSH username")
                .required(false),
        )
        .arg(
            Arg::new("identity")
                .short('i')
                .long("identity")
                .help("Path to SSH private key file"),
        )
        .arg(
            Arg::new("command")
                .short('c')
                .long("command")
                .help("Command to execute on remote host"),
        )
        .arg(
            Arg::new("password")
                .long("password")
                .help("Use password authentication instead of keys")
                .action(clap::ArgAction::SetTrue),
        )
        .arg(
            Arg::new("generate-key")
                .long("generate-key")
                .help("Generate a new SSH key pair")
                .value_name("KEY_NAME"),
        )
        .arg(
            Arg::new("list-keys")
                .long("list-keys")
                .help("List all available SSH keys")
                .action(clap::ArgAction::SetTrue),
        )
        .get_matches();

    // Handle key management commands first
    if let Some(key_name) = matches.get_one::<String>("generate-key") {
        return handle_generate_key(key_name);
    }

    if matches.get_flag("list-keys") {
        return handle_list_keys();
    }

    // Regular connection handling
    let host = matches.get_one::<String>("host");
    let username = matches.get_one::<String>("username");
    
    if host.is_none() || username.is_none() {
        eprintln!("Error: Host and username are required for SSH connections");
        eprintln!("Use --help for usage information");
        std::process::exit(1);
    }

    let host = host.unwrap();
    let username = username.unwrap();
    let port = matches
        .get_one::<String>("port")
        .unwrap()
        .parse::<u16>()
        .context("Invalid port number")?;
    let identity = matches.get_one::<String>("identity");
    let command = matches.get_one::<String>("command");
    let use_password = matches.get_flag("password");

    info!("Connecting to {}@{}:{}", username, host, port);

    #[cfg(target_arch = "wasm32")]
    {
        wasm::connect(host, port, username, identity, command)
    }

    #[cfg(not(target_arch = "wasm32"))]
    {
        native::connect(host, port, username, identity, command, use_password)
    }
}

fn handle_generate_key(key_name: &str) -> Result<()> {
    use key_manager::KeyManager;
    
    let mut key_manager = KeyManager::new()
        .context("Failed to initialize key manager")?;
    
    match key_manager.generate_ed25519_key(key_name) {
        Ok(key) => {
            println!("✅ Generated SSH key pair: {}", key.name);
            println!("📋 Public key:\n{}", key.public_key);
            println!("\n💡 Copy the public key above to your server's ~/.ssh/authorized_keys file");
            println!("🔑 Key stored securely in ~/.bxssh/keys.json");
        }
        Err(e) => {
            eprintln!("❌ Failed to generate key: {}", e);
            std::process::exit(1);
        }
    }
    
    Ok(())
}

fn handle_list_keys() -> Result<()> {
    use key_manager::KeyManager;
    
    let key_manager = KeyManager::new()
        .context("Failed to initialize key manager")?;
    
    let keys = key_manager.list_keys();
    
    if keys.is_empty() {
        println!("📭 No SSH keys found");
        println!("💡 Generate a new key with: bxssh --generate-key <name>");
    } else {
        println!("🔑 Available SSH keys:");
        for key in keys {
            println!("  • {} ({})", key.name, format!("{:?}", key.key_type));
        }
        println!("\n💡 Use a key with: bxssh -u username -i <key-name> hostname");
    }
    
    Ok(())
}

#[cfg(target_arch = "wasm32")]
fn main() {
    // WASM doesn't use main, entry point is through wasm-bindgen
}

#[cfg(target_arch = "wasm32")]
use wasm_bindgen::prelude::*;

#[cfg(target_arch = "wasm32")]
#[wasm_bindgen(start)]
pub fn wasm_main() {
    // Initialize logging for WASM
    web_sys::console::log_1(&"bxssh WebAssembly module loaded".into());
}

#[cfg(target_arch = "wasm32")]
#[wasm_bindgen]
pub async fn wasm_connect(host: &str, port: u16, username: &str, password: Option<String>) -> Result<JsValue, JsValue> {
    use wasm_ssh::WasmSshConnection;
    use ssh_client::SshClient;
    
    let connection = WasmSshConnection::new();
    let mut client = SshClient::new(Box::new(connection));
    
    // Connect
    client.connect(host, port)
        .map_err(|e| JsValue::from_str(&e.to_string()))?;
    
    // Authenticate
    if let Some(pass) = password {
        client.authenticate_with_password(username, &pass)
            .map_err(|e| JsValue::from_str(&e.to_string()))?;
    } else {
        return Err(JsValue::from_str("Password is required for WASM SSH"));
    }
    
    Ok(JsValue::from_str("Connected successfully"))
}