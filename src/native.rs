use anyhow::{Context, Result};
use crossterm::{
    event::{self, Event, KeyCode, KeyEvent},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use log::{debug, error, info};
use std::io::{self, Write};

use crate::config::SshConfig;
use crate::ssh_client::SshClient;
use crate::ssh_impl::RealSshConnection;
use crate::key_manager::KeyManager;

pub fn connect(
    host: &str,
    port: u16,
    username: &str,
    identity: Option<&String>,
    command: Option<&String>,
    use_password: bool,
) -> Result<()> {
    let config = SshConfig::load().context("Failed to load SSH config")?;
    
    info!("Establishing SSH connection to {}@{}:{}", username, host, port);
    
    let connection = RealSshConnection::new();
    let mut client = SshClient::new(Box::new(connection));
    client.connect(host, port).context("Failed to connect to SSH server")?;

    // Authentication logic
    if use_password {
        // Password authentication
        info!("Using password authentication");
        let password = rpassword::prompt_password("Password: ")
            .context("Failed to read password")?;
        
        match client.authenticate_with_password(username, &password) {
            Ok(_) => info!("Password authentication successful"),
            Err(e) => {
                error!("Password authentication failed: {}", e);
                return Err(e);
            }
        }
    } else {
        // Key-based authentication
        let key_to_use = if let Some(identity) = identity {
            // Check if it's a key name from our internal storage or a file path
            if identity.starts_with('/') || identity.starts_with('~') || identity.contains('.') {
                // Treat as file path (legacy support)
                Some(identity.clone())
            } else {
                // Treat as key name from internal storage
                let key_manager = KeyManager::new().context("Failed to initialize key manager")?;
                if let Some(key) = key_manager.get_key(identity) {
                    // Write the private key to a temporary file
                    let temp_file = create_temp_key_file(&key.private_key)?;
                    Some(temp_file)
                } else {
                    return Err(anyhow::anyhow!("Key '{}' not found in internal storage", identity));
                }
            }
        } else {
            // No key specified, try to use default from internal storage or fallback to system
            let mut key_manager = KeyManager::new().context("Failed to initialize key manager")?;
            
            if let Ok(default_key) = key_manager.ensure_default_key() {
                info!("Using internal default key: {}", default_key.name);
                let temp_file = create_temp_key_file(&default_key.private_key)?;
                Some(temp_file)
            } else {
                // Fallback to system keys
                config.get_identity_file().map(|s| s.to_string())
            }
        };

        if let Some(key_path) = key_to_use {
            info!("Attempting key-based authentication with key");
            match client.authenticate_with_key(username, &key_path) {
                Ok(_) => info!("Key authentication successful"),
                Err(e) => {
                    error!("Key authentication failed: {}", e);
                    
                    // Offer password fallback
                    println!("🔐 Key authentication failed. Try password authentication? (y/N)");
                    let mut input = String::new();
                    io::stdin().read_line(&mut input)?;
                    
                    if input.trim().to_lowercase() == "y" || input.trim().to_lowercase() == "yes" {
                        let password = rpassword::prompt_password("Password: ")
                            .context("Failed to read password")?;
                        client.authenticate_with_password(username, &password)
                            .context("Password authentication also failed")?;
                        info!("Password authentication successful");
                    } else {
                        return Err(e);
                    }
                }
            }
        } else {
            return Err(anyhow::anyhow!(
                "No SSH authentication method available. Use --password for password auth or --generate-key to create a key"
            ));
        }
    }

    if !client.is_authenticated() {
        return Err(anyhow::anyhow!("Authentication failed"));
    }

    if let Some(cmd) = command {
        execute_remote_command(&client, cmd)
    } else {
        start_interactive_shell(&client)
    }
}

fn execute_remote_command(client: &SshClient, command: &str) -> Result<()> {
    info!("Executing command: {}", command);
    let output = client.execute_command(command)?;
    print!("{}", output);
    Ok(())
}

fn start_interactive_shell(client: &SshClient) -> Result<()> {
    info!("Starting interactive shell");
    
    let mut session = client.start_shell()?;
    
    enable_raw_mode().context("Failed to enable raw mode")?;
    execute!(io::stdout(), EnterAlternateScreen).context("Failed to enter alternate screen")?;

    let result = run_shell_loop(&mut session);

    execute!(io::stdout(), LeaveAlternateScreen).context("Failed to leave alternate screen")?;
    disable_raw_mode().context("Failed to disable raw mode")?;

    result
}

fn run_shell_loop(session: &mut Box<dyn crate::ssh_client::ShellSession>) -> Result<()> {
    let mut buffer = [0; 1024];
    
    loop {
        if event::poll(std::time::Duration::from_millis(100))? {
            match event::read()? {
                Event::Key(KeyEvent {
                    code: KeyCode::Char('c'),
                    modifiers: event::KeyModifiers::CONTROL,
                    ..
                }) => {
                    debug!("Ctrl+C pressed, exiting");
                    break;
                }
                Event::Key(KeyEvent { code, .. }) => {
                    let input = match code {
                        KeyCode::Enter => "\r\n",
                        KeyCode::Tab => "\t",
                        KeyCode::Backspace => "\x08",
                        KeyCode::Char(c) => {
                            let mut s = String::new();
                            s.push(c);
                            Box::leak(s.into_boxed_str())
                        }
                        _ => continue,
                    };
                    
                    if let Err(e) = session.write(input.as_bytes()) {
                        error!("Failed to write to session: {}", e);
                        break;
                    }
                }
                _ => {}
            }
        }

        match session.read(&mut buffer) {
            Ok(0) => break,
            Ok(n) => {
                let output = String::from_utf8_lossy(&buffer[..n]);
                print!("{}", output);
                io::stdout().flush()?;
            }
            Err(e) => {
                if e.to_string().contains("WouldBlock") {
                    continue;
                }
                error!("Failed to read from session: {}", e);
                break;
            }
        }

        if session.is_eof() {
            debug!("Session EOF reached");
            break;
        }
    }

    Ok(())
}

fn create_temp_key_file(private_key_content: &str) -> Result<String> {
    use std::os::unix::fs::PermissionsExt;
    
    let temp_dir = std::env::temp_dir();
    let key_file_path = temp_dir.join(format!("bxssh_key_{}", std::process::id()));
    
    // Write the private key content to temp file
    std::fs::write(&key_file_path, private_key_content)
        .context("Failed to write temporary key file")?;
    
    // Set proper permissions (600 - owner read/write only)
    let mut perms = std::fs::metadata(&key_file_path)?.permissions();
    perms.set_mode(0o600);
    std::fs::set_permissions(&key_file_path, perms)?;
    
    Ok(key_file_path.to_string_lossy().to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ssh_client::{MockSshConnection, MockShellSession};
    
    fn setup_mock_client() -> SshClient {
        let mut mock_connection = MockSshConnection::new();
        mock_connection
            .expect_is_authenticated()
            .returning(|| true);
        
        SshClient::new(Box::new(mock_connection))
    }

    #[test]
    fn test_execute_remote_command_success() {
        let mut mock_connection = MockSshConnection::new();
        mock_connection
            .expect_is_authenticated()
            .returning(|| true);
        mock_connection
            .expect_execute_command()
            .with(mockall::predicate::eq("echo hello"))
            .returning(|_| Ok("hello\n".to_string()));

        let client = SshClient::new(Box::new(mock_connection));
        let result = execute_remote_command(&client, "echo hello");
        
        assert!(result.is_ok());
    }

    #[test]
    fn test_execute_remote_command_not_authenticated() {
        let mut mock_connection = MockSshConnection::new();
        mock_connection
            .expect_is_authenticated()
            .returning(|| false);

        let client = SshClient::new(Box::new(mock_connection));
        let result = execute_remote_command(&client, "echo hello");
        
        assert!(result.is_err());
    }

    #[test]
    fn test_connect_success_with_key() {
        let temp_key = tempfile::NamedTempFile::new().unwrap();
        let key_path = temp_key.path().to_str().unwrap();
        
        let mut mock_connection = MockSshConnection::new();
        mock_connection
            .expect_connect()
            .returning(|_, _| Ok(()));
        mock_connection
            .expect_authenticate_with_key()
            .returning(|_, _| Ok(()));
        mock_connection
            .expect_is_authenticated()
            .returning(|| true);

        let mut client = SshClient::new(Box::new(mock_connection));
        
        // Test connection logic (would normally be in connect function)
        let connect_result = client.connect("localhost", 22);
        assert!(connect_result.is_ok());
        
        let auth_result = client.authenticate_with_key("testuser", key_path);
        assert!(auth_result.is_ok());
        
        assert!(client.is_authenticated());
    }

    #[test]
    fn test_connect_failure() {
        let mut mock_connection = MockSshConnection::new();
        mock_connection
            .expect_connect()
            .returning(|_, _| Err(anyhow::anyhow!("Connection refused")));

        let mut client = SshClient::new(Box::new(mock_connection));
        let result = client.connect("invalid-host", 22);
        
        assert!(result.is_err());
    }

    #[test]
    fn test_authentication_failure() {
        let mut mock_connection = MockSshConnection::new();
        mock_connection
            .expect_connect()
            .returning(|_, _| Ok(()));
        mock_connection
            .expect_authenticate_with_key()
            .returning(|_, _| Err(anyhow::anyhow!("Authentication failed")));

        let mut client = SshClient::new(Box::new(mock_connection));
        let _ = client.connect("localhost", 22);
        let result = client.authenticate_with_key("testuser", "/fake/key");
        
        assert!(result.is_err());
    }

    #[test]
    fn test_shell_session_creation() {
        let mut mock_connection = MockSshConnection::new();
        mock_connection
            .expect_is_authenticated()
            .returning(|| true);
        mock_connection
            .expect_start_shell()
            .returning(|| {
                let mut mock_session = MockShellSession::new();
                mock_session.expect_is_eof().returning(|| false);
                Ok(Box::new(mock_session))
            });

        let client = SshClient::new(Box::new(mock_connection));
        let result = client.start_shell();
        
        assert!(result.is_ok());
    }
}