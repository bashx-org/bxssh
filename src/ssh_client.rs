use anyhow::{Context, Result};

#[cfg_attr(test, mockall::automock)]
pub trait SshConnection {
    fn connect(&mut self, host: &str, port: u16) -> Result<()>;
    fn authenticate_with_key(&mut self, username: &str, private_key_path: &str) -> Result<()>;
    fn authenticate_with_password(&mut self, username: &str, password: &str) -> Result<()>;
    fn execute_command(&self, command: &str) -> Result<String>;
    fn start_shell(&self) -> Result<Box<dyn ShellSession>>;
    fn is_authenticated(&self) -> bool;
}

#[cfg_attr(test, mockall::automock)]
pub trait ShellSession: std::fmt::Debug + Send + Sync {
    fn read(&mut self, buf: &mut [u8]) -> Result<usize>;
    fn write(&mut self, data: &[u8]) -> Result<usize>;
    fn is_eof(&self) -> bool;
}

pub struct SshClient {
    connection: Box<dyn SshConnection>,
}

impl SshClient {
    pub fn new(connection: Box<dyn SshConnection>) -> Self {
        Self { connection }
    }

    pub fn connect(&mut self, host: &str, port: u16) -> Result<()> {
        self.connection.connect(host, port)
            .context("Failed to establish SSH connection")
    }

    pub fn authenticate_with_key(&mut self, username: &str, private_key_path: &str) -> Result<()> {
        if private_key_path.is_empty() {
            return Err(anyhow::anyhow!("Private key path cannot be empty"));
        }
        
        self.connection.authenticate_with_key(username, private_key_path)
            .context("SSH key authentication failed")
    }

    pub fn authenticate_with_password(&mut self, username: &str, password: &str) -> Result<()> {
        if password.is_empty() {
            return Err(anyhow::anyhow!("Password cannot be empty"));
        }
        
        self.connection.authenticate_with_password(username, password)
            .context("SSH password authentication failed")
    }

    pub fn execute_command(&self, command: &str) -> Result<String> {
        if command.trim().is_empty() {
            return Err(anyhow::anyhow!("Command cannot be empty"));
        }

        if !self.connection.is_authenticated() {
            return Err(anyhow::anyhow!("Not authenticated"));
        }

        self.connection.execute_command(command)
            .context("Failed to execute remote command")
    }

    pub fn start_shell(&self) -> Result<Box<dyn ShellSession>> {
        if !self.connection.is_authenticated() {
            return Err(anyhow::anyhow!("Not authenticated"));
        }

        self.connection.start_shell()
            .context("Failed to start interactive shell")
    }

    pub fn is_authenticated(&self) -> bool {
        self.connection.is_authenticated()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use mockall::predicate::*;

    fn setup_mock_connection() -> MockSshConnection {
        MockSshConnection::new()
    }

    #[test]
    fn test_ssh_client_creation() {
        let mut mock_connection = setup_mock_connection();
        mock_connection
            .expect_is_authenticated()
            .times(1)
            .returning(|| false);
        
        let client = SshClient::new(Box::new(mock_connection));
        
        assert!(!client.is_authenticated());
    }

    #[test]
    fn test_connect_success() {
        let mut mock_connection = setup_mock_connection();
        mock_connection
            .expect_connect()
            .with(eq("localhost"), eq(22))
            .times(1)
            .returning(|_, _| Ok(()));

        let mut client = SshClient::new(Box::new(mock_connection));
        let result = client.connect("localhost", 22);
        
        assert!(result.is_ok());
    }

    #[test]
    fn test_connect_failure() {
        let mut mock_connection = setup_mock_connection();
        mock_connection
            .expect_connect()
            .with(eq("invalid-host"), eq(22))
            .times(1)
            .returning(|_, _| Err(anyhow::anyhow!("Connection refused")));

        let mut client = SshClient::new(Box::new(mock_connection));
        let result = client.connect("invalid-host", 22);
        
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("Failed to establish SSH connection"));
    }

    #[test]
    fn test_authenticate_with_key_success() {
        let mut mock_connection = setup_mock_connection();
        mock_connection
            .expect_authenticate_with_key()
            .with(eq("testuser"), eq("/path/to/key"))
            .times(1)
            .returning(|_, _| Ok(()));

        let mut client = SshClient::new(Box::new(mock_connection));
        let result = client.authenticate_with_key("testuser", "/path/to/key");
        
        assert!(result.is_ok());
    }

    #[test]
    fn test_authenticate_with_key_empty_path() {
        let mock_connection = setup_mock_connection();
        let mut client = SshClient::new(Box::new(mock_connection));
        let result = client.authenticate_with_key("testuser", "");
        
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("Private key path cannot be empty"));
    }

    #[test]
    fn test_authenticate_with_key_failure() {
        let mut mock_connection = setup_mock_connection();
        mock_connection
            .expect_authenticate_with_key()
            .with(eq("testuser"), eq("/invalid/key"))
            .times(1)
            .returning(|_, _| Err(anyhow::anyhow!("Key not found")));

        let mut client = SshClient::new(Box::new(mock_connection));
        let result = client.authenticate_with_key("testuser", "/invalid/key");
        
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("SSH key authentication failed"));
    }

    #[test]
    fn test_authenticate_with_password_success() {
        let mut mock_connection = setup_mock_connection();
        mock_connection
            .expect_authenticate_with_password()
            .with(eq("testuser"), eq("password123"))
            .times(1)
            .returning(|_, _| Ok(()));

        let mut client = SshClient::new(Box::new(mock_connection));
        let result = client.authenticate_with_password("testuser", "password123");
        
        assert!(result.is_ok());
    }

    #[test]
    fn test_authenticate_with_password_empty() {
        let mock_connection = setup_mock_connection();
        let mut client = SshClient::new(Box::new(mock_connection));
        let result = client.authenticate_with_password("testuser", "");
        
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("Password cannot be empty"));
    }

    #[test]
    fn test_execute_command_success() {
        let mut mock_connection = setup_mock_connection();
        mock_connection
            .expect_is_authenticated()
            .times(1)
            .returning(|| true);
        mock_connection
            .expect_execute_command()
            .with(eq("ls -la"))
            .times(1)
            .returning(|_| Ok("file1\nfile2\n".to_string()));

        let client = SshClient::new(Box::new(mock_connection));
        let result = client.execute_command("ls -la");
        
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), "file1\nfile2\n");
    }

    #[test]
    fn test_execute_command_not_authenticated() {
        let mut mock_connection = setup_mock_connection();
        mock_connection
            .expect_is_authenticated()
            .times(1)
            .returning(|| false);

        let client = SshClient::new(Box::new(mock_connection));
        let result = client.execute_command("ls -la");
        
        assert!(result.is_err());
        assert!(result.is_err());
    }

    #[test]
    fn test_execute_command_empty_command() {
        let mock_connection = setup_mock_connection();
        let client = SshClient::new(Box::new(mock_connection));
        let result = client.execute_command("");
        
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("Command cannot be empty"));
    }

    #[test]
    fn test_execute_command_whitespace_only() {
        let mock_connection = setup_mock_connection();
        let client = SshClient::new(Box::new(mock_connection));
        let result = client.execute_command("   \n\t  ");
        
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("Command cannot be empty"));
    }

    #[test]
    fn test_start_shell_success() {
        let mut mock_connection = setup_mock_connection();
        let mut mock_session = MockShellSession::new();
        mock_session
            .expect_is_eof()
            .returning(|| false);

        mock_connection
            .expect_is_authenticated()
            .times(1)
            .returning(|| true);
        mock_connection
            .expect_start_shell()
            .times(1)
            .returning(|| {
                let mut mock_session = MockShellSession::new();
                mock_session.expect_is_eof().returning(|| false);
                Ok(Box::new(mock_session))
            });

        let client = SshClient::new(Box::new(mock_connection));
        let result = client.start_shell();
        
        assert!(result.is_ok());
    }

    #[test]
    fn test_start_shell_not_authenticated() {
        let mut mock_connection = setup_mock_connection();
        mock_connection
            .expect_is_authenticated()
            .times(1)
            .returning(|| false);

        let client = SshClient::new(Box::new(mock_connection));
        let result = client.start_shell();
        
        assert!(result.is_err());
        assert!(result.is_err());
    }

    #[test]
    fn test_is_authenticated_true() {
        let mut mock_connection = setup_mock_connection();
        mock_connection
            .expect_is_authenticated()
            .times(1)
            .returning(|| true);

        let client = SshClient::new(Box::new(mock_connection));
        assert!(client.is_authenticated());
    }

    #[test]
    fn test_is_authenticated_false() {
        let mut mock_connection = setup_mock_connection();
        mock_connection
            .expect_is_authenticated()
            .times(1)
            .returning(|| false);

        let client = SshClient::new(Box::new(mock_connection));
        assert!(!client.is_authenticated());
    }
}