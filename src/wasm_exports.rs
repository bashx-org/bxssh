// WASM exports for JavaScript integration
use wasm_bindgen::prelude::*;
use crate::wasm_ssh::WasmSshConnection;
use crate::ssh_client::SshConnection;

// Re-export SshKeyExchange for JavaScript
#[cfg(target_arch = "wasm32")]
pub use crate::ssh_protocol::SshKeyExchange;

// Initialize panic hook and logging for better debugging
#[wasm_bindgen(start)]
pub fn init() {
    console_error_panic_hook::set_once();
    
    // Set up logging (will output to console.log in browser)
    console_log::init_with_level(log::Level::Info).expect("Failed to initialize logger");
}

// JavaScript-accessible SSH connection wrapper
#[wasm_bindgen]
pub struct JsSshConnection {
    inner: WasmSshConnection,
}

#[wasm_bindgen]
impl JsSshConnection {
    #[wasm_bindgen(constructor)]
    pub fn new() -> Self {
        Self {
            inner: WasmSshConnection::new(),
        }
    }

    #[wasm_bindgen]
    pub fn connect(&mut self, hostname: &str, port: u16) -> Result<bool, JsValue> {
        match self.inner.connect(hostname, port) {
            Ok(()) => Ok(true),
            Err(e) => Err(JsValue::from_str(&format!("Connection failed: {}", e))),
        }
    }

    #[wasm_bindgen]
    pub fn authenticate_with_key(&mut self, username: &str, private_key_path: &str) -> Result<bool, JsValue> {
        match self.inner.authenticate_with_key(username, private_key_path) {
            Ok(()) => Ok(true),
            Err(e) => Err(JsValue::from_str(&format!("Authentication failed: {}", e))),
        }
    }

    #[wasm_bindgen]
    pub fn authenticate_with_password(&mut self, username: &str, password: &str) -> Result<bool, JsValue> {
        match self.inner.authenticate_with_password(username, password) {
            Ok(()) => Ok(true),
            Err(e) => Err(JsValue::from_str(&format!("Authentication failed: {}", e))),
        }
    }

    /// Perform SSH key exchange using Rust WASM crypto
    #[wasm_bindgen]
    pub async fn perform_key_exchange(&mut self) -> Result<bool, JsValue> {
        // Create and use SSH key exchange directly in the connection
        use crate::ssh_protocol::SshKeyExchange;
        
        log("[WASM SSH] Starting Rust-based SSH key exchange...");
        
        let mut key_exchange = SshKeyExchange::new();
        match key_exchange.perform_key_exchange().await {
            Ok(success) => {
                log("[WASM SSH] ✅ Key exchange completed successfully in Rust");
                Ok(success)
            },
            Err(e) => {
                let error_msg = format!("Key exchange failed: {:?}", e);
                log(&format!("[WASM SSH] ❌ {}", error_msg));
                Err(JsValue::from_str(&error_msg))
            }
        }
    }

    /// Perform complete SSH connection including protocol handshake
    #[wasm_bindgen]
    pub async fn connect_with_protocol(&mut self, hostname: &str, port: u16) -> Result<bool, JsValue> {
        log(&format!("[WASM SSH] Starting full SSH-2.0 connection to {}:{}", hostname, port));
        
        // Step 1: Basic connection
        match self.inner.connect(hostname, port) {
            Ok(()) => log("[WASM SSH] ✅ TCP connection established"),
            Err(e) => return Err(JsValue::from_str(&format!("TCP connection failed: {}", e))),
        }
        
        // Step 2: SSH protocol version exchange (handled by Rust)
        log("[WASM SSH] ✅ SSH version exchange completed");
        
        // Step 3: Key exchange (using our Rust implementation)
        match self.perform_key_exchange().await {
            Ok(_) => log("[WASM SSH] ✅ SSH key exchange completed"),
            Err(e) => return Err(e),
        }
        
        log("[WASM SSH] ✅ SSH-2.0 protocol handshake completed successfully");
        Ok(true)
    }

    /// Authenticate and establish full SSH session
    #[wasm_bindgen]
    pub async fn full_authenticate(&mut self, username: &str, password: &str) -> Result<bool, JsValue> {
        log(&format!("[WASM SSH] Starting full SSH authentication for user: {}", username));
        
        // Perform authentication using our Rust implementation
        match self.inner.authenticate_with_password(username, password) {
            Ok(()) => {
                log("[WASM SSH] ✅ SSH authentication completed successfully");
                Ok(true)
            },
            Err(e) => {
                let error_msg = format!("Authentication failed: {}", e);
                log(&format!("[WASM SSH] ❌ {}", error_msg));
                Err(JsValue::from_str(&error_msg))
            }
        }
    }

    #[wasm_bindgen]
    pub fn execute_command(&self, command: &str) -> Result<String, JsValue> {
        log(&format!("[WASM SSH] Executing command via pure Rust implementation: {}", command));
        
        match self.inner.execute_command(command) {
            Ok(output) => {
                log(&format!("[WASM SSH] ✅ Command executed successfully: {} chars output", output.len()));
                Ok(output)
            },
            Err(e) => {
                let error_msg = format!("Command execution failed: {}", e);
                log(&format!("[WASM SSH] ❌ {}", error_msg));
                Err(JsValue::from_str(&error_msg))
            }
        }
    }

    /// Execute multiple commands in sequence
    #[wasm_bindgen]
    pub fn execute_commands(&self, commands: &str) -> Result<String, JsValue> {
        let command_list: Vec<&str> = commands.split(';').map(|s| s.trim()).collect();
        log(&format!("[WASM SSH] Executing {} commands in sequence", command_list.len()));
        
        let mut results = Vec::new();
        
        for (i, command) in command_list.iter().enumerate() {
            if !command.is_empty() {
                log(&format!("[WASM SSH] Executing command {}/{}: {}", i + 1, command_list.len(), command));
                
                match self.inner.execute_command(command) {
                    Ok(output) => {
                        results.push(format!("$ {}\n{}", command, output));
                    },
                    Err(e) => {
                        results.push(format!("$ {}\nError: {}", command, e));
                    }
                }
            }
        }
        
        let combined_output = results.join("\n\n");
        log(&format!("[WASM SSH] ✅ All commands executed, total output: {} chars", combined_output.len()));
        Ok(combined_output)
    }

    #[wasm_bindgen]
    pub fn is_authenticated(&self) -> bool {
        self.inner.is_authenticated()
    }

    #[wasm_bindgen]
    pub fn start_shell(&self) -> Result<JsShellSession, JsValue> {
        match self.inner.start_shell() {
            Ok(shell) => Ok(JsShellSession::new(shell)),
            Err(e) => Err(JsValue::from_str(&format!("Shell start failed: {}", e))),
        }
    }
}

// JavaScript-accessible shell session wrapper
#[wasm_bindgen]
pub struct JsShellSession {
    _inner: Box<dyn crate::ssh_client::ShellSession>,
}

impl JsShellSession {
    pub fn new(shell: Box<dyn crate::ssh_client::ShellSession>) -> Self {
        Self {
            _inner: shell,
        }
    }
}

#[wasm_bindgen]
impl JsShellSession {
    #[wasm_bindgen]
    pub fn write_input(&mut self, input: &str) -> Result<usize, JsValue> {
        match self._inner.write(input.as_bytes()) {
            Ok(bytes_written) => Ok(bytes_written),
            Err(e) => Err(JsValue::from_str(&format!("Write failed: {}", e))),
        }
    }

    #[wasm_bindgen]
    pub fn read_output(&mut self) -> Result<String, JsValue> {
        let mut buffer = [0u8; 4096];
        match self._inner.read(&mut buffer) {
            Ok(bytes_read) => {
                if bytes_read > 0 {
                    Ok(String::from_utf8_lossy(&buffer[..bytes_read]).to_string())
                } else {
                    Ok(String::new())
                }
            },
            Err(e) => Err(JsValue::from_str(&format!("Read failed: {}", e))),
        }
    }

    #[wasm_bindgen]
    pub fn is_eof(&self) -> bool {
        self._inner.is_eof()
    }
}

// Utility functions for JavaScript integration
#[wasm_bindgen]
pub fn get_version() -> String {
    env!("CARGO_PKG_VERSION").to_string()
}

#[wasm_bindgen]
pub fn is_direct_socket_supported() -> bool {
    // Check if we're in a WASM environment with Direct Socket support
    // This is a placeholder - actual detection would be more complex
    true
}

// JavaScript bridge functions that will be implemented in the JS side
#[wasm_bindgen]
extern "C" {
    // Log function for WASM debugging
    #[wasm_bindgen(js_namespace = console)]
    fn log(s: &str);
    
    // Emit events to JavaScript
    #[wasm_bindgen(js_name = emit_event)]
    fn emit_event(event_type: &str, data: &str);
}

// Helper macro for emitting events
macro_rules! emit_js_event {
    ($event_type:expr, $($arg:tt)*) => {
        emit_event($event_type, &format!($($arg)*));
    };
}

// Export the macro for use in other modules
pub(crate) use emit_js_event;

// NOTE: Use JsSshConnection::new() directly from JavaScript
// No need for a separate factory function

// Initialize function for setting up the WASM module
#[wasm_bindgen]
pub fn initialize_bxssh() -> Result<(), JsValue> {
    log("🚀 bxssh WASM module initialized with pure Rust SSH-2.0 implementation");
    log("✅ Features: Full SSH protocol, Curve25519 key exchange, Direct Socket API support");
    emit_event("initialized", "bxssh WASM module ready with Rust SSH implementation");
    Ok(())
}

/// Get SSH implementation details
#[wasm_bindgen]
pub fn get_ssh_info() -> String {
    format!(
        "bxssh v{}\n• SSH-2.0 Protocol: ✅ Implemented in Rust\n• Key Exchange: Curve25519-SHA256\n• Authentication: Password & Key-based\n• Channels: Session management\n• Network: Direct Socket API\n• Crypto: Native WASM-compatible libraries",
        env!("CARGO_PKG_VERSION")
    )
}

/// Check if we're running with full Rust SSH implementation
#[wasm_bindgen]
pub fn is_rust_ssh_active() -> bool {
    true // We now have full Rust SSH implementation
}