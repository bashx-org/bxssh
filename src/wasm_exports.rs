// WASM exports for JavaScript integration
use wasm_bindgen::prelude::*;
use crate::wasm_ssh::WasmSshConnection;
use crate::ssh_client::SshConnection;

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

    #[wasm_bindgen]
    pub fn execute_command(&self, command: &str) -> Result<String, JsValue> {
        match self.inner.execute_command(command) {
            Ok(output) => Ok(output),
            Err(e) => Err(JsValue::from_str(&format!("Command execution failed: {}", e))),
        }
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

// Initialize function for setting up the WASM module
#[wasm_bindgen]
pub fn initialize_bxssh() -> Result<(), JsValue> {
    // Any initialization code needed
    log("bxssh WASM module initialized");
    emit_event("initialized", "bxssh WASM module ready");
    Ok(())
}