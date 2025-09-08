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
        channel.request_pty("xterm", None, None).context("Failed to request PTY")?;
        channel.shell().context("Failed to start shell")?;
        
        Ok(Box::new(RealShellSession { channel }))
    }

    fn is_authenticated(&self) -> bool {
        self.session.as_ref()
            .map(|s| s.authenticated())
            .unwrap_or(false)
    }
}

pub struct RealShellSession {
    channel: Channel,
}

impl std::fmt::Debug for RealShellSession {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("RealShellSession").finish()
    }
}

impl ShellSession for RealShellSession {
    fn read(&mut self, buf: &mut [u8]) -> Result<usize> {
        self.channel.read(buf)
            .map_err(|e| anyhow::anyhow!("Failed to read from shell: {}", e))
    }

    fn write(&mut self, data: &[u8]) -> Result<usize> {
        use std::io::Write;
        self.channel.write(data)
            .map_err(|e| anyhow::anyhow!("Failed to write to shell: {}", e))
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