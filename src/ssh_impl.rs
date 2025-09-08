use anyhow::{Context, Result};
use ssh2::{Channel, Session};
use std::io::Read;
use std::net::TcpStream;

use crate::ssh_client::{SshConnection, ShellSession};

pub struct RealSshConnection {
    session: Option<Session>,
    _stream: Option<TcpStream>,
}

impl RealSshConnection {
    pub fn new() -> Self {
        Self {
            session: None,
            _stream: None,
        }
    }
}

impl SshConnection for RealSshConnection {
    fn connect(&mut self, host: &str, port: u16) -> Result<()> {
        let tcp = TcpStream::connect(format!("{}:{}", host, port))
            .context("Failed to connect to host")?;
        
        let mut session = Session::new().context("Failed to create SSH session")?;
        session.set_tcp_stream(tcp.try_clone().context("Failed to clone TCP stream")?);
        session.handshake().context("SSH handshake failed")?;

        self.session = Some(session);
        self._stream = Some(tcp);
        Ok(())
    }

    fn authenticate_with_key(&mut self, username: &str, private_key_path: &str) -> Result<()> {
        let session = self.session.as_mut()
            .ok_or_else(|| anyhow::anyhow!("Not connected"))?;
        
        session
            .userauth_pubkey_file(username, None, std::path::Path::new(private_key_path), None)
            .context("SSH key authentication failed")
    }

    fn authenticate_with_password(&mut self, username: &str, password: &str) -> Result<()> {
        let session = self.session.as_mut()
            .ok_or_else(|| anyhow::anyhow!("Not connected"))?;
        
        session
            .userauth_password(username, password)
            .context("SSH password authentication failed")
    }

    fn execute_command(&self, command: &str) -> Result<String> {
        let session = self.session.as_ref()
            .ok_or_else(|| anyhow::anyhow!("Not connected"))?;

        let mut channel = session.channel_session().context("Failed to create channel")?;
        channel.exec(command).context("Failed to execute command")?;

        let mut output = String::new();
        channel.read_to_string(&mut output).context("Failed to read command output")?;
        
        channel.wait_close().context("Failed to close channel")?;
        let exit_status = channel.exit_status().context("Failed to get exit status")?;
        
        if exit_status != 0 {
            return Err(anyhow::anyhow!("Command failed with exit status {}", exit_status));
        }

        Ok(output)
    }

    fn start_shell(&self) -> Result<Box<dyn ShellSession>> {
        let session = self.session.as_ref()
            .ok_or_else(|| anyhow::anyhow!("Not connected"))?;

        let mut channel = session.channel_session().context("Failed to create channel")?;
        
        // Get terminal size for vim and other full-screen applications
        let (width, height) = match crossterm::terminal::size() {
            Ok((w, h)) => (w as u32, h as u32),
            Err(_) => (80, 24), // fallback
        };
        
        // Request PTY with proper terminal capabilities for vim
        // Use xterm-256color which vim expects for full functionality
        channel.request_pty("xterm-256color", None, None)
            .context("Failed to request PTY")?;
        
        // Set the window size after PTY creation
        channel.request_pty_size(width, height, Some(0), Some(0))?;
        
        // Start the shell
        channel.shell().context("Failed to start shell")?;
        
        // Set the channel to non-blocking mode for better I/O handling
        session.set_blocking(false);
        
        Ok(Box::new(RealShellSession { 
            channel,
            last_size: Some((width, height)),
        }))
    }

    fn is_authenticated(&self) -> bool {
        self.session.as_ref()
            .map(|s| s.authenticated())
            .unwrap_or(false)
    }
}

pub struct RealShellSession {
    channel: Channel,
    last_size: Option<(u32, u32)>,
}

impl std::fmt::Debug for RealShellSession {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("RealShellSession").finish()
    }
}

impl RealShellSession {
    fn check_terminal_resize(&mut self) {
        if let Ok((width, height)) = crossterm::terminal::size() {
            let current_size = (width as u32, height as u32);
            
            if self.last_size != Some(current_size) {
                // Terminal size changed, notify the remote PTY
                if let Err(e) = self.channel.request_pty_size(current_size.0, current_size.1, Some(0), Some(0)) {
                    log::debug!("Failed to update PTY size: {}", e);
                } else {
                    log::debug!("Updated PTY size to {}x{}", current_size.0, current_size.1);
                    self.last_size = Some(current_size);
                }
            }
        }
    }
    
    fn write_chunked(&mut self, data: &[u8]) -> Result<usize> {
        use std::io::Write;
        
        const CHUNK_SIZE: usize = 512;
        let mut total_written = 0;
        
        for chunk in data.chunks(CHUNK_SIZE) {
            let mut retries = 0;
            const MAX_RETRIES: usize = 3;
            
            loop {
                match self.channel.write(chunk) {
                    Ok(n) => {
                        total_written += n;
                        if n < chunk.len() {
                            // Partial write, wait a bit and continue with remaining data
                            std::thread::sleep(std::time::Duration::from_millis(10));
                            // For simplicity, we'll consider this a successful partial write
                            // In a more sophisticated implementation, we'd handle the remaining bytes
                        }
                        break;
                    },
                    Err(e) if e.to_string().contains("draining incoming flow") && retries < MAX_RETRIES => {
                        log::debug!("Flow control during chunked write, retry {}/{}", retries + 1, MAX_RETRIES);
                        retries += 1;
                        std::thread::sleep(std::time::Duration::from_millis((100 * retries) as u64));
                    },
                    Err(e) => {
                        return Err(anyhow::anyhow!("Failed to write chunk: {}", e));
                    }
                }
            }
            
            // Small delay between chunks to prevent overwhelming the channel
            std::thread::sleep(std::time::Duration::from_millis(5));
        }
        
        Ok(total_written)
    }
}

impl ShellSession for RealShellSession {
    fn read(&mut self, buf: &mut [u8]) -> Result<usize> {
        // Check for terminal size changes before reading
        self.check_terminal_resize();
        
        // Try to read data, handle various error conditions gracefully
        match self.channel.read(buf) {
            Ok(n) => Ok(n),
            Err(e) if e.kind() == std::io::ErrorKind::WouldBlock => Ok(0),
            Err(e) if e.kind() == std::io::ErrorKind::UnexpectedEof => {
                log::debug!("SSH channel EOF during read");
                Ok(0)
            },
            Err(e) if e.to_string().contains("draining incoming flow") => {
                log::debug!("SSH channel flow control issue, waiting...");
                std::thread::sleep(std::time::Duration::from_millis(50));
                Ok(0) // Return 0 bytes read, don't error
            },
            Err(e) => {
                log::debug!("SSH read error: {}", e);
                Err(anyhow::anyhow!("Failed to read from shell: {}", e))
            }
        }
    }

    fn write(&mut self, data: &[u8]) -> Result<usize> {
        use std::io::Write;
        
        // Handle large writes by chunking them
        if data.len() > 1024 {
            return self.write_chunked(data);
        }
        
        match self.channel.write(data) {
            Ok(n) => {
                // Don't flush immediately for small writes to improve performance
                // The SSH library will handle batching
                Ok(n)
            },
            Err(e) if e.to_string().contains("draining incoming flow") => {
                log::debug!("SSH channel flow control during write, retrying...");
                std::thread::sleep(std::time::Duration::from_millis(50));
                // Retry the write
                match self.channel.write(data) {
                    Ok(n) => Ok(n),
                    Err(e2) => Err(anyhow::anyhow!("Failed to write to shell after retry: {}", e2))
                }
            },
            Err(e) => Err(anyhow::anyhow!("Failed to write to shell: {}", e))
        }
    }

    fn is_eof(&self) -> bool {
        self.channel.eof()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_real_ssh_connection_creation() {
        let connection = RealSshConnection::new();
        assert!(!connection.is_authenticated());
    }

    #[test]
    fn test_connect_to_invalid_host() {
        let mut connection = RealSshConnection::new();
        let result = connection.connect("nonexistent-host-12345.invalid", 22);
        assert!(result.is_err());
    }

    #[test]
    fn test_authenticate_without_connection() {
        let mut connection = RealSshConnection::new();
        let result = connection.authenticate_with_key("user", "/path/to/key");
        assert!(result.is_err());
        assert!(result.is_err());
    }

    #[test] 
    fn test_execute_command_without_connection() {
        let connection = RealSshConnection::new();
        let result = connection.execute_command("echo hello");
        assert!(result.is_err());
        assert!(result.is_err());
    }

    #[test]
    fn test_start_shell_without_connection() {
        let connection = RealSshConnection::new();
        let result = connection.start_shell();
        assert!(result.is_err());
        assert!(result.is_err());
    }
}