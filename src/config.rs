use anyhow::Result;

#[cfg(not(target_arch = "wasm32"))]
use dirs;

#[derive(Debug, Clone)]
pub struct SshConfig {
    pub default_user: Option<String>,
    pub default_port: u16,
    pub identity_file: Option<String>,
}

impl Default for SshConfig {
    fn default() -> Self {
        Self {
            default_user: None,
            default_port: 22,
            identity_file: None,
        }
    }
}

impl SshConfig {
    #[cfg(not(target_arch = "wasm32"))]
    pub fn load() -> Result<Self> {
        let mut config = Self::default();
        
        if let Some(home_dir) = dirs::home_dir() {
            let ssh_dir = home_dir.join(".ssh");
            if ssh_dir.exists() {
                let id_rsa = ssh_dir.join("id_rsa");
                let id_ed25519 = ssh_dir.join("id_ed25519");
                
                if id_ed25519.exists() {
                    config.identity_file = Some(id_ed25519.to_string_lossy().to_string());
                } else if id_rsa.exists() {
                    config.identity_file = Some(id_rsa.to_string_lossy().to_string());
                }
            }
        }
        
        if let Ok(user) = std::env::var("USER") {
            config.default_user = Some(user);
        }
        
        Ok(config)
    }

    #[cfg(target_arch = "wasm32")]
    pub fn load() -> Result<Self> {
        Ok(Self::default())
    }

    pub fn get_identity_file(&self) -> Option<&str> {
        self.identity_file.as_deref()
    }

    pub fn get_default_user(&self) -> Option<&str> {
        self.default_user.as_deref()
    }

    pub fn get_default_port(&self) -> u16 {
        self.default_port
    }

    pub fn set_identity_file(&mut self, path: String) {
        self.identity_file = Some(path);
    }

    pub fn set_default_user(&mut self, user: String) {
        self.default_user = Some(user);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[cfg(not(target_arch = "wasm32"))]
    use tempfile::TempDir;
    #[cfg(not(target_arch = "wasm32"))]
    use std::fs::{self, File};
    #[cfg(not(target_arch = "wasm32"))]
    use std::env;

    #[test]
    fn test_default_config() {
        let config = SshConfig::default();
        
        assert_eq!(config.default_port, 22);
        assert!(config.default_user.is_none());
        assert!(config.identity_file.is_none());
    }

    #[test]
    fn test_get_identity_file() {
        let mut config = SshConfig::default();
        assert!(config.get_identity_file().is_none());
        
        config.set_identity_file("/path/to/key".to_string());
        assert_eq!(config.get_identity_file(), Some("/path/to/key"));
    }

    #[test]
    fn test_get_default_user() {
        let mut config = SshConfig::default();
        assert!(config.get_default_user().is_none());
        
        config.set_default_user("testuser".to_string());
        assert_eq!(config.get_default_user(), Some("testuser"));
    }

    #[test]
    fn test_get_default_port() {
        let config = SshConfig::default();
        assert_eq!(config.get_default_port(), 22);
    }

    #[test]
    fn test_set_identity_file() {
        let mut config = SshConfig::default();
        config.set_identity_file("/new/path/key".to_string());
        
        assert_eq!(config.identity_file, Some("/new/path/key".to_string()));
    }

    #[test]
    fn test_set_default_user() {
        let mut config = SshConfig::default();
        config.set_default_user("newuser".to_string());
        
        assert_eq!(config.default_user, Some("newuser".to_string()));
    }

    #[cfg(not(target_arch = "wasm32"))]
    #[test]
    fn test_load_with_existing_ssh_keys() {
        let temp_dir = TempDir::new().unwrap();
        let ssh_dir = temp_dir.path().join(".ssh");
        fs::create_dir_all(&ssh_dir).unwrap();

        let id_rsa = ssh_dir.join("id_rsa");
        File::create(&id_rsa).unwrap();

        let id_ed25519 = ssh_dir.join("id_ed25519");
        File::create(&id_ed25519).unwrap();

        // Mock HOME directory for test
        let original_home = env::var("HOME").ok();
        env::set_var("HOME", temp_dir.path());

        let config = SshConfig::load().unwrap();

        // Restore original HOME
        if let Some(home) = original_home {
            env::set_var("HOME", home);
        } else {
            env::remove_var("HOME");
        }

        // Should prefer ed25519 over rsa
        assert!(config.identity_file.is_some());
        assert!(config.identity_file.unwrap().contains("id_ed25519"));
    }

    #[cfg(not(target_arch = "wasm32"))]
    #[test]
    fn test_load_with_only_rsa_key() {
        let temp_dir = TempDir::new().unwrap();
        let ssh_dir = temp_dir.path().join(".ssh");
        fs::create_dir_all(&ssh_dir).unwrap();

        let id_rsa = ssh_dir.join("id_rsa");
        File::create(&id_rsa).unwrap();

        let original_home = env::var("HOME").ok();
        env::set_var("HOME", temp_dir.path());

        let config = SshConfig::load().unwrap();

        if let Some(home) = original_home {
            env::set_var("HOME", home);
        } else {
            env::remove_var("HOME");
        }

        assert!(config.identity_file.is_some());
        assert!(config.identity_file.unwrap().contains("id_rsa"));
    }

    #[cfg(not(target_arch = "wasm32"))]
    #[test]
    fn test_load_with_no_ssh_keys() {
        let temp_dir = TempDir::new().unwrap();
        let ssh_dir = temp_dir.path().join(".ssh");
        fs::create_dir_all(&ssh_dir).unwrap();

        let original_home = env::var("HOME").ok();
        env::set_var("HOME", temp_dir.path());

        let config = SshConfig::load().unwrap();

        if let Some(home) = original_home {
            env::set_var("HOME", home);
        } else {
            env::remove_var("HOME");
        }

        assert!(config.identity_file.is_none());
    }

    #[cfg(not(target_arch = "wasm32"))]
    #[test] 
    fn test_load_with_user_env_var() {
        let original_user = env::var("USER").ok();
        env::set_var("USER", "testuser");

        let config = SshConfig::load().unwrap();

        if let Some(user) = original_user {
            env::set_var("USER", user);
        } else {
            env::remove_var("USER");
        }

        assert_eq!(config.default_user, Some("testuser".to_string()));
    }

    #[cfg(target_arch = "wasm32")]
    #[test]
    fn test_load_wasm() {
        let config = SshConfig::load().unwrap();
        
        // WASM should return default config
        assert_eq!(config.default_port, 22);
        assert!(config.default_user.is_none());
        assert!(config.identity_file.is_none());
    }
}