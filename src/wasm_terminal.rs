#[cfg(target_arch = "wasm32")]
use wasm_bindgen::prelude::*;
#[cfg(target_arch = "wasm32")]
use web_sys::console;

use anyhow::Result;
use std::collections::VecDeque;
use std::sync::{Arc, Mutex};

use crate::terminal::TerminalIO;

/// WebAssembly-specific terminal I/O implementation
/// This uses callbacks and queues to manage input/output in the browser
pub struct WasmTerminalIO {
    input_queue: Arc<Mutex<VecDeque<Vec<u8>>>>,
    output_callback: Option<js_sys::Function>,
    should_continue: Arc<Mutex<bool>>,
}

impl WasmTerminalIO {
    pub fn new() -> Self {
        Self {
            input_queue: Arc::new(Mutex::new(VecDeque::new())),
            output_callback: None,
            should_continue: Arc::new(Mutex::new(true)),
        }
    }
    
    /// Set callback function for output (called from JavaScript)
    #[cfg(target_arch = "wasm32")]
    pub fn set_output_callback(&mut self, callback: js_sys::Function) {
        self.output_callback = Some(callback);
    }
    
    /// Add input data from JavaScript
    pub fn add_input(&self, data: Vec<u8>) {
        if let Ok(mut queue) = self.input_queue.lock() {
            queue.push_back(data);
        }
    }
    
    /// Stop the session from JavaScript
    pub fn stop_session(&self) {
        if let Ok(mut should_continue) = self.should_continue.lock() {
            *should_continue = false;
        }
    }
}

impl TerminalIO for WasmTerminalIO {
    fn read_input(&mut self) -> Result<Option<Vec<u8>>> {
        if let Ok(mut queue) = self.input_queue.lock() {
            Ok(queue.pop_front())
        } else {
            Ok(None)
        }
    }
    
    fn write_output(&mut self, data: &[u8]) -> Result<()> {
        #[cfg(target_arch = "wasm32")]
        {
            if let Some(ref callback) = self.output_callback {
                // Convert bytes to string for JavaScript
                let output_str = String::from_utf8_lossy(data);
                let js_str = JsValue::from_str(&output_str);
                
                if let Err(e) = callback.call1(&JsValue::NULL, &js_str) {
                    console::error_1(&format!("Output callback error: {:?}", e).into());
                }
            } else {
                // Fallback to console output
                let output_str = String::from_utf8_lossy(data);
                console::log_1(&output_str.into());
            }
        }
        
        #[cfg(not(target_arch = "wasm32"))]
        {
            // For testing on non-WASM platforms
            print!("{}", String::from_utf8_lossy(data));
        }
        
        Ok(())
    }
    
    fn should_continue(&self) -> bool {
        self.should_continue
            .lock()
            .map(|guard| *guard)
            .unwrap_or(false)
    }
    
    fn initialize(&mut self) -> Result<()> {
        #[cfg(target_arch = "wasm32")]
        console::log_1(&"🔗 WebAssembly SSH session started".into());
        
        Ok(())
    }
    
    fn cleanup(&mut self) -> Result<()> {
        #[cfg(target_arch = "wasm32")]
        console::log_1(&"🔌 WebAssembly SSH session ended".into());
        
        Ok(())
    }
}

// WebAssembly exports for JavaScript integration
#[cfg(target_arch = "wasm32")]
#[wasm_bindgen]
pub struct WasmSSHSession {
    terminal_io: Arc<Mutex<WasmTerminalIO>>,
    session_manager: Option<crate::terminal::SessionManager>,
}

#[cfg(target_arch = "wasm32")]
#[wasm_bindgen]
impl WasmSSHSession {
    #[wasm_bindgen(constructor)]
    pub fn new() -> WasmSSHSession {
        Self {
            terminal_io: Arc::new(Mutex::new(WasmTerminalIO::new())),
            session_manager: None,
        }
    }
    
    /// Set output callback from JavaScript
    #[wasm_bindgen(js_name = setOutputCallback)]
    pub fn set_output_callback(&mut self, callback: js_sys::Function) {
        if let Ok(mut terminal) = self.terminal_io.lock() {
            terminal.set_output_callback(callback);
        }
    }
    
    /// Send input from JavaScript
    #[wasm_bindgen(js_name = sendInput)]
    pub fn send_input(&self, data: &[u8]) {
        if let Ok(terminal) = self.terminal_io.lock() {
            terminal.add_input(data.to_vec());
        }
    }
    
    /// Stop the session from JavaScript
    #[wasm_bindgen(js_name = stopSession)]
    pub fn stop_session(&self) {
        if let Ok(terminal) = self.terminal_io.lock() {
            terminal.stop_session();
        }
    }
    
    /// Start SSH session with WebSocket tunnel (placeholder)
    #[wasm_bindgen(js_name = startSession)]
    pub async fn start_session(&mut self, host: &str, username: &str, password: &str) -> Result<(), JsValue> {
        // This would integrate with the WebSocket SSH implementation
        // For now, just log the attempt
        console::log_1(&format!("Starting WASM SSH session to {}@{}", username, host).into());
        
        // TODO: Create actual SSH session using WASM SSH implementation
        // let ssh_session = create_wasm_ssh_session(host, username, password).await?;
        // let terminal_io = self.terminal_io.lock().unwrap().clone();
        // let mut manager = SessionManager::new(ssh_session, Box::new(terminal_io));
        // manager.run_session().map_err(|e| JsValue::from_str(&e.to_string()))?;
        
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_wasm_terminal_creation() {
        let terminal = WasmTerminalIO::new();
        assert!(terminal.should_continue());
    }
    
    #[test]
    fn test_input_queue() {
        let terminal = WasmTerminalIO::new();
        terminal.add_input(b"test input".to_vec());
        
        let mut terminal_mut = terminal;
        let input = terminal_mut.read_input().unwrap();
        assert!(input.is_some());
        assert_eq!(input.unwrap(), b"test input");
    }
    
    #[test]
    fn test_stop_session() {
        let terminal = WasmTerminalIO::new();
        assert!(terminal.should_continue());
        
        terminal.stop_session();
        assert!(!terminal.should_continue());
    }
}