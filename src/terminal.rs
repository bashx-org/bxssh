use anyhow::Result;

/// Abstraction for terminal input/output handling
/// This allows different implementations for CLI vs WebAssembly
pub trait TerminalIO: Send + Sync {
    /// Read input from the user (non-blocking)
    /// Returns None if no input is available
    fn read_input(&mut self) -> Result<Option<Vec<u8>>>;
    
    /// Write output to the user's display
    fn write_output(&mut self, data: &[u8]) -> Result<()>;
    
    /// Check if the session should continue
    fn should_continue(&self) -> bool;
    
    /// Initialize the terminal for interactive use
    fn initialize(&mut self) -> Result<()>;
    
    /// Cleanup and restore terminal state
    fn cleanup(&mut self) -> Result<()>;
}

/// Session manager that coordinates between SSH and Terminal I/O
pub struct SessionManager {
    ssh_session: Box<dyn crate::ssh_client::ShellSession>,
    terminal_io: Box<dyn TerminalIO>,
}

impl SessionManager {
    pub fn new(
        ssh_session: Box<dyn crate::ssh_client::ShellSession>, 
        terminal_io: Box<dyn TerminalIO>
    ) -> Self {
        Self {
            ssh_session,
            terminal_io,
        }
    }
    
    /// Run the interactive session loop
    pub fn run_session(&mut self) -> Result<()> {
        self.terminal_io.initialize()?;
        
        let result = self.session_loop();
        
        self.terminal_io.cleanup()?;
        result
    }
    
    fn session_loop(&mut self) -> Result<()> {
        use log::{debug, info};
        use std::time::{Duration, Instant};
        
        let mut ssh_buffer = [0u8; 8192];
        info!("Starting session loop");
        
        // Wait for initial prompt/output from SSH server
        info!("Waiting for initial SSH output...");
        let start_time = Instant::now();
        let mut got_initial_output = false;
        
        while start_time.elapsed() < Duration::from_secs(5) && !got_initial_output {
            match self.ssh_session.read(&mut ssh_buffer) {
                Ok(0) => {
                    std::thread::sleep(Duration::from_millis(50));
                }
                Ok(n) => {
                    info!("Received initial SSH output: {} bytes", n);
                    debug!("Initial output: {:?}", String::from_utf8_lossy(&ssh_buffer[..n]));
                    self.terminal_io.write_output(&ssh_buffer[..n])?;
                    got_initial_output = true;
                    break;
                }
                Err(e) => {
                    let error_msg = e.to_string();
                    if error_msg.contains("WouldBlock") || 
                       error_msg.contains("Resource temporarily unavailable") {
                        std::thread::sleep(Duration::from_millis(50));
                        continue;
                    } else {
                        debug!("SSH read error during initial wait: {}", e);
                        break; // Continue to main loop anyway
                    }
                }
            }
        }
        
        if !got_initial_output {
            info!("No initial output received, continuing anyway");
        }
        
        // Main session loop
        while self.terminal_io.should_continue() {
            // Handle user input -> SSH
            if let Some(input_data) = self.terminal_io.read_input()? {
                if !input_data.is_empty() {
                    debug!("Sending input to SSH: {:?}", String::from_utf8_lossy(&input_data));
                    match self.ssh_session.write(&input_data) {
                        Ok(bytes_written) => {
                            debug!("Wrote {} bytes to SSH session", bytes_written);
                        }
                        Err(e) => {
                            debug!("Failed to write to SSH session: {}", e);
                            return Err(e);
                        }
                    }
                }
            }
            
            // Handle SSH output -> user display
            match self.ssh_session.read(&mut ssh_buffer) {
                Ok(0) => {
                    // No data from SSH, continue
                    std::thread::sleep(std::time::Duration::from_millis(10));
                }
                Ok(n) => {
                    // Got data from SSH, display to user
                    debug!("Received {} bytes from SSH", n);
                    
                    // Check for vim crash indicators in output
                    let output_str = String::from_utf8_lossy(&ssh_buffer[..n]);
                    if output_str.contains("Vim: Error reading input") || 
                       output_str.contains("terminal too small") ||
                       output_str.contains("E558") { // Vim error codes
                        debug!("Detected vim terminal issue: {}", output_str);
                    }
                    
                    match self.terminal_io.write_output(&ssh_buffer[..n]) {
                        Ok(_) => {},
                        Err(e) => {
                            debug!("Failed to write output to terminal: {}", e);
                            // Don't return error, just log and continue
                        }
                    }
                }
                Err(e) => {
                    // Check if it's a would-block error (non-blocking I/O)
                    let error_msg = e.to_string();
                    if error_msg.contains("WouldBlock") || 
                       error_msg.contains("Resource temporarily unavailable") {
                        std::thread::sleep(std::time::Duration::from_millis(10));
                        continue;
                    } else {
                        debug!("SSH read error: {}", e);
                        return Err(e);
                    }
                }
            }
            
            // Check if SSH session ended
            if self.ssh_session.is_eof() {
                info!("SSH session ended");
                break;
            }
        }
        
        info!("Session loop completed");
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ssh_client::MockShellSession;
    use std::sync::{Arc, Mutex};
    
    struct MockTerminalIO {
        input_data: Arc<Mutex<Vec<Vec<u8>>>>,
        output_data: Arc<Mutex<Vec<u8>>>,
        should_continue: Arc<Mutex<bool>>,
    }
    
    impl MockTerminalIO {
        fn new() -> Self {
            Self {
                input_data: Arc::new(Mutex::new(vec![])),
                output_data: Arc::new(Mutex::new(vec![])),
                should_continue: Arc::new(Mutex::new(true)),
            }
        }
        
        fn add_input(&self, data: Vec<u8>) {
            self.input_data.lock().unwrap().push(data);
        }
        
        fn get_output(&self) -> Vec<u8> {
            self.output_data.lock().unwrap().clone()
        }
        
        fn stop(&self) {
            *self.should_continue.lock().unwrap() = false;
        }
    }
    
    impl TerminalIO for MockTerminalIO {
        fn read_input(&mut self) -> Result<Option<Vec<u8>>> {
            let mut input = self.input_data.lock().unwrap();
            if input.is_empty() {
                Ok(None)
            } else {
                Ok(Some(input.remove(0)))
            }
        }
        
        fn write_output(&mut self, data: &[u8]) -> Result<()> {
            self.output_data.lock().unwrap().extend_from_slice(data);
            Ok(())
        }
        
        fn should_continue(&self) -> bool {
            *self.should_continue.lock().unwrap()
        }
        
        fn initialize(&mut self) -> Result<()> {
            Ok(())
        }
        
        fn cleanup(&mut self) -> Result<()> {
            Ok(())
        }
    }
    
    #[test]
    fn test_session_manager_creation() {
        let mut mock_session = MockShellSession::new();
        mock_session.expect_is_eof().returning(|| true);
        
        let mock_terminal = MockTerminalIO::new();
        
        let _manager = SessionManager::new(
            Box::new(mock_session),
            Box::new(mock_terminal)
        );
    }
    
    #[test]
    fn test_session_basic_flow() {
        let mut mock_session = MockShellSession::new();
        
        // First call returns EOF immediately to end the session
        mock_session
            .expect_read()
            .times(1..)
            .returning(|_| Ok(0)); // No data available
            
        mock_session
            .expect_write()
            .times(0..=1) // May or may not be called
            .returning(|_| Ok(5));
        
        mock_session
            .expect_is_eof()
            .times(1..)
            .returning(|| true); // End session immediately
        
        let mock_terminal = MockTerminalIO::new();
        // Don't add any input to make test simpler
        
        let mut manager = SessionManager::new(
            Box::new(mock_session),
            Box::new(mock_terminal)
        );
        
        let result = manager.run_session();
        assert!(result.is_ok());
    }
}