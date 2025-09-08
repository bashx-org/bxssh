use anyhow::{Context, Result};
use ssh2::{Channel, Session};
use std::io::Read;
use std::net::TcpStream;

pub struct SshClient {
    session: Session,
    _stream: TcpStream,
}

impl SshClient {
    pub fn connect(host: &str, port: u16, _username: &str) -> Result<Self> {
        let tcp = TcpStream::connect(format!("{}:{}", host, port))
            .context("Failed to connect to host")?;
        
        let mut session = Session::new().context("Failed to create SSH session")?;
        session.set_tcp_stream(tcp.try_clone().context("Failed to clone TCP stream")?);
        session.handshake().context("SSH handshake failed")?;

        Ok(Self {
            session,
            _stream: tcp,
        })
    }

    pub fn authenticate_with_key(&mut self, username: &str, private_key_path: &str) -> Result<()> {
        self.session
            .userauth_pubkey_file(username, None, std::path::Path::new(private_key_path), None)
            .context("SSH key authentication failed")
    }

    pub fn authenticate_with_password(&mut self, username: &str, password: &str) -> Result<()> {
        self.session
            .userauth_password(username, password)
            .context("SSH password authentication failed")
    }

    pub fn execute_command(&self, command: &str) -> Result<String> {
        let mut channel = self.session.channel_session().context("Failed to create channel")?;
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

    pub fn start_shell(&self) -> Result<Channel> {
        let mut channel = self.session.channel_session().context("Failed to create channel")?;
        channel.request_pty("xterm", None, None).context("Failed to request PTY")?;
        channel.shell().context("Failed to start shell")?;
        Ok(channel)
    }

    pub fn is_authenticated(&self) -> bool {
        self.session.authenticated()
    }
}