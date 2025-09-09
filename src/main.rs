#[cfg(not(target_arch = "wasm32"))]
use clap::{Arg, Command};
use anyhow::{Context, Result};
use log::info;

mod ssh_client;
mod config;
mod key_manager;
mod terminal;

#[cfg(not(target_arch = "wasm32"))]
mod ssh;
#[cfg(not(target_arch = "wasm32"))]
mod ssh_impl;
#[cfg(not(target_arch = "wasm32"))]
mod native;
#[cfg(not(target_arch = "wasm32"))]
mod cli_terminal;

#[cfg(target_arch = "wasm32")]
mod wasm_ssh;
#[cfg(target_arch = "wasm32")]
mod wasm_terminal;


#[cfg(not(target_arch = "wasm32"))]
fn main() -> Result<()> {
    env_logger::init();
    
    let matches = Command::new("bxssh")
        .version("0.1.0")
        .author("bashx-org")
        .about("A WebAssembly-compatible SSH client CLI")
        .arg(
            Arg::new("target")
                .help("SSH target in format 'user@host' or just 'host' (requires -u)")
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

    // Parse connection target (user@host or host)
    let target = matches.get_one::<String>("target");
    let username_arg = matches.get_one::<String>("username");
    
    // Check for common mistake: using -u with user@host format
    if target.is_none() {
        if let Some(username) = username_arg {
            if username.contains('@') {
                eprintln!("Error: Invalid usage detected");
                eprintln!("You used: bxssh -u '{}'", username);
                eprintln!("Correct usage:");
                eprintln!("  bxssh {}           (new format)", username);
                eprintln!("  bxssh -u user host  (old format)");
                std::process::exit(1);
            }
        }
        
        eprintln!("Error: Target host is required for SSH connections");
        eprintln!("Usage: bxssh user@host  OR  bxssh -u user host");
        eprintln!("Use --help for more information");
        std::process::exit(1);
    }
    
    let target = target.unwrap();
    let (username, host) = parse_target(target, username_arg)?;
    
    // Debug log to show what was parsed
    log::info!("Parsed target: username='{}', host='{}'", username, host);
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
        native::connect(&host, port, &username, identity, command, use_password)
    }
}

/// Parse target string to extract username and host
/// Supports both "user@host" and just "host" (with -u flag)
fn parse_target(target: &str, username_arg: Option<&String>) -> Result<(String, String)> {
    if target.contains('@') {
        // Parse user@host format
        let parts: Vec<&str> = target.split('@').collect();
        if parts.len() != 2 {
            return Err(anyhow::anyhow!("Invalid target format. Use 'user@host'"));
        }
        
        let username = parts[0].to_string();
        let host = parts[1].to_string();
        
        if username.is_empty() || host.is_empty() {
            return Err(anyhow::anyhow!("Username and host cannot be empty"));
        }
        
        // If -u flag was also provided, check for common mistakes
        if let Some(flag_user) = username_arg {
            if flag_user.contains('@') {
                return Err(anyhow::anyhow!(
                    "Invalid usage: Don't use -u flag with user@host format.\n\
                     Use either: 'bxssh {}' OR 'bxssh -u {} {}'", 
                    target, username, host
                ));
            }
            if flag_user != &username {
                eprintln!("⚠️  Warning: Username from target '{}' overrides -u flag '{}'", username, flag_user);
            }
        }
        
        Ok((username, host))
    } else {
        // Just host provided, username must come from -u flag
        if let Some(username) = username_arg {
            Ok((username.clone(), target.to_string()))
        } else {
            Err(anyhow::anyhow!(
                "Username is required. Use 'user@host' format or provide -u flag with hostname"
            ))
        }
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
        println!("\n💡 Use a key with: bxssh -i <key-name> user@hostname");
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
    use terminal::SessionManager;
    use wasm_terminal::WasmTerminalIO;
    
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
    
    // Start session with WASM terminal I/O
    let ssh_session = client.start_shell()
        .map_err(|e| JsValue::from_str(&e.to_string()))?;
    let terminal_io = WasmTerminalIO::new();
    
    let mut session_manager = SessionManager::new(
        ssh_session,
        Box::new(terminal_io)
    );
    
    session_manager.run_session()
        .map_err(|e| JsValue::from_str(&e.to_string()))?;
    
    Ok(JsValue::from_str("Session completed"))
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_parse_target_user_at_host() {
        let result = parse_target("alice@example.com", None).unwrap();
        assert_eq!(result.0, "alice");
        assert_eq!(result.1, "example.com");
    }
    
    #[test]
    fn test_parse_target_host_only_with_username_flag() {
        let username = "bob".to_string();
        let result = parse_target("server.local", Some(&username)).unwrap();
        assert_eq!(result.0, "bob");
        assert_eq!(result.1, "server.local");
    }
    
    #[test]
    fn test_parse_target_user_at_host_overrides_flag() {
        let username_flag = "bob".to_string();
        let result = parse_target("alice@example.com", Some(&username_flag)).unwrap();
        assert_eq!(result.0, "alice"); // Should use alice from target, not bob from flag
        assert_eq!(result.1, "example.com");
    }
    
    #[test]
    fn test_parse_target_host_only_without_username_fails() {
        let result = parse_target("example.com", None);
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("Username is required"));
    }
    
    #[test]
    fn test_parse_target_invalid_format() {
        let result = parse_target("user@host@extra", None);
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("Invalid target format"));
    }
    
    #[test]
    fn test_parse_target_empty_username_or_host() {
        let result = parse_target("@host", None);
        assert!(result.is_err());
        
        let result = parse_target("user@", None);
        assert!(result.is_err());
    }
}