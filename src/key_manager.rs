use anyhow::{Context, Result};
use ed25519_dalek::{SigningKey, VerifyingKey};
use rand::{rngs::OsRng, RngCore};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;
use base64::{Engine as _, engine::general_purpose};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KeyPair {
    pub name: String,
    pub public_key: String,
    pub private_key: String,
    pub key_type: KeyType,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum KeyType {
    Ed25519,
}

#[derive(Debug)]
pub struct KeyManager {
    keys: HashMap<String, KeyPair>,
    storage_path: PathBuf,
}

impl KeyManager {
    pub fn new() -> Result<Self> {
        let storage_path = Self::get_storage_path()?;
        let keys = Self::load_keys(&storage_path)?;
        
        Ok(Self {
            keys,
            storage_path,
        })
    }

    fn get_storage_path() -> Result<PathBuf> {
        #[cfg(not(target_arch = "wasm32"))]
        {
            let mut path = dirs::home_dir()
                .ok_or_else(|| anyhow::anyhow!("Could not find home directory"))?;
            path.push(".bxssh");
            if !path.exists() {
                fs::create_dir_all(&path)
                    .context("Failed to create bxssh directory")?;
            }
            path.push("keys.json");
            Ok(path)
        }
        
        #[cfg(target_arch = "wasm32")]
        {
            // For WASM, we'll use browser local storage later
            Ok(PathBuf::from("keys.json"))
        }
    }

    fn load_keys(path: &PathBuf) -> Result<HashMap<String, KeyPair>> {
        if path.exists() {
            let content = fs::read_to_string(path)
                .context("Failed to read keys file")?;
            let keys: HashMap<String, KeyPair> = serde_json::from_str(&content)
                .context("Failed to parse keys file")?;
            Ok(keys)
        } else {
            Ok(HashMap::new())
        }
    }

    fn save_keys(&self) -> Result<()> {
        let content = serde_json::to_string_pretty(&self.keys)
            .context("Failed to serialize keys")?;
        fs::write(&self.storage_path, content)
            .context("Failed to write keys file")?;
        Ok(())
    }

    pub fn generate_ed25519_key(&mut self, name: &str) -> Result<&KeyPair> {
        if self.keys.contains_key(name) {
            return Err(anyhow::anyhow!("Key '{}' already exists", name));
        }

        // Generate random 32 bytes for Ed25519 private key
        let mut secret_bytes = [0u8; 32];
        OsRng.fill_bytes(&mut secret_bytes);
        let signing_key = SigningKey::from_bytes(&secret_bytes);
        let verifying_key = signing_key.verifying_key();

        let private_pem = self.signing_key_to_pem(&signing_key)?;
        let public_pem = self.verifying_key_to_pem(&verifying_key)?;

        let key_pair = KeyPair {
            name: name.to_string(),
            public_key: public_pem,
            private_key: private_pem,
            key_type: KeyType::Ed25519,
        };

        self.keys.insert(name.to_string(), key_pair);
        self.save_keys()?;

        Ok(self.keys.get(name).unwrap())
    }

    fn signing_key_to_pem(&self, key: &SigningKey) -> Result<String> {
        // For now, store raw key bytes in a simple format
        // In production, we'd want proper OpenSSH private key format
        let key_bytes = key.to_bytes();
        let base64_key = general_purpose::STANDARD.encode(&key_bytes);
        
        Ok(format!(
            "-----BEGIN BXSSH PRIVATE KEY-----\n{}\n-----END BXSSH PRIVATE KEY-----",
            base64_key
        ))
    }

    fn verifying_key_to_pem(&self, key: &VerifyingKey) -> Result<String> {
        // Create proper SSH public key format
        let key_bytes = key.to_bytes();
        let base64_key = general_purpose::STANDARD.encode(&key_bytes);
        
        Ok(format!("ssh-ed25519 {} bxssh-generated", base64_key))
    }
    
    pub fn extract_private_key_bytes(&self, private_key_pem: &str) -> Result<Vec<u8>> {
        // Extract the base64 content between the markers
        let lines: Vec<&str> = private_key_pem.lines().collect();
        let mut base64_content = String::new();
        
        let mut in_key = false;
        for line in lines {
            if line.contains("BEGIN BXSSH PRIVATE KEY") {
                in_key = true;
                continue;
            }
            if line.contains("END BXSSH PRIVATE KEY") {
                break;
            }
            if in_key {
                base64_content.push_str(line.trim());
            }
        }
        
        general_purpose::STANDARD.decode(base64_content)
            .context("Failed to decode private key")
    }

    pub fn get_key(&self, name: &str) -> Option<&KeyPair> {
        self.keys.get(name)
    }

    pub fn list_keys(&self) -> Vec<&KeyPair> {
        self.keys.values().collect()
    }

    pub fn delete_key(&mut self, name: &str) -> Result<()> {
        if self.keys.remove(name).is_some() {
            self.save_keys()?;
            Ok(())
        } else {
            Err(anyhow::anyhow!("Key '{}' not found", name))
        }
    }

    pub fn get_default_key(&self) -> Option<&KeyPair> {
        // Return the first key if available, or create a default one
        self.keys.values().next()
    }

    pub fn ensure_default_key(&mut self) -> Result<&KeyPair> {
        if self.keys.is_empty() {
            log::info!("No SSH keys found, generating default key");
            self.generate_ed25519_key("default")
        } else {
            Ok(self.get_default_key().unwrap())
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;
    use std::env;

    fn setup_test_key_manager() -> Result<(KeyManager, TempDir)> {
        let temp_dir = TempDir::new()?;
        let original_home = env::var("HOME").ok();
        env::set_var("HOME", temp_dir.path());
        
        let key_manager = KeyManager::new()?;
        
        if let Some(home) = original_home {
            env::set_var("HOME", home);
        } else {
            env::remove_var("HOME");
        }
        
        Ok((key_manager, temp_dir))
    }

    #[test]
    fn test_key_manager_creation() {
        let result = setup_test_key_manager();
        assert!(result.is_ok());
        let (key_manager, _temp_dir) = result.unwrap();
        assert_eq!(key_manager.list_keys().len(), 0);
    }

    #[test]
    fn test_generate_ed25519_key() {
        let (mut key_manager, _temp_dir) = setup_test_key_manager().unwrap();
        
        let result = key_manager.generate_ed25519_key("test-key");
        assert!(result.is_ok());
        
        let key = result.unwrap();
        assert_eq!(key.name, "test-key");
        assert!(matches!(key.key_type, KeyType::Ed25519));
        assert!(key.private_key.contains("BEGIN BXSSH PRIVATE KEY"));
        assert!(key.public_key.starts_with("ssh-ed25519"));
    }

    #[test]
    fn test_duplicate_key_generation() {
        let (mut key_manager, _temp_dir) = setup_test_key_manager().unwrap();
        
        key_manager.generate_ed25519_key("test-key").unwrap();
        let result = key_manager.generate_ed25519_key("test-key");
        
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("already exists"));
    }

    #[test]
    fn test_get_key() {
        let (mut key_manager, _temp_dir) = setup_test_key_manager().unwrap();
        
        key_manager.generate_ed25519_key("test-key").unwrap();
        
        let key = key_manager.get_key("test-key");
        assert!(key.is_some());
        assert_eq!(key.unwrap().name, "test-key");
        
        let missing_key = key_manager.get_key("missing-key");
        assert!(missing_key.is_none());
    }

    #[test]
    fn test_list_keys() {
        let (mut key_manager, _temp_dir) = setup_test_key_manager().unwrap();
        
        key_manager.generate_ed25519_key("key1").unwrap();
        key_manager.generate_ed25519_key("key2").unwrap();
        
        let keys = key_manager.list_keys();
        assert_eq!(keys.len(), 2);
        
        let key_names: Vec<&String> = keys.iter().map(|k| &k.name).collect();
        assert!(key_names.contains(&&"key1".to_string()));
        assert!(key_names.contains(&&"key2".to_string()));
    }

    #[test]
    fn test_delete_key() {
        let (mut key_manager, _temp_dir) = setup_test_key_manager().unwrap();
        
        key_manager.generate_ed25519_key("test-key").unwrap();
        assert_eq!(key_manager.list_keys().len(), 1);
        
        let result = key_manager.delete_key("test-key");
        assert!(result.is_ok());
        assert_eq!(key_manager.list_keys().len(), 0);
        
        let result = key_manager.delete_key("missing-key");
        assert!(result.is_err());
    }

    #[test]
    fn test_ensure_default_key() {
        let (mut key_manager, _temp_dir) = setup_test_key_manager().unwrap();
        
        assert_eq!(key_manager.list_keys().len(), 0);
        
        let result = key_manager.ensure_default_key();
        assert!(result.is_ok());
        let key = result.unwrap();
        assert_eq!(key.name, "default");
        
        // Check count separately after key is used
        assert_eq!(key_manager.list_keys().len(), 1);
    }

    #[test]
    fn test_get_default_key() {
        let (mut key_manager, _temp_dir) = setup_test_key_manager().unwrap();
        
        assert!(key_manager.get_default_key().is_none());
        
        key_manager.generate_ed25519_key("first-key").unwrap();
        
        let default_key = key_manager.get_default_key();
        assert!(default_key.is_some());
        assert_eq!(default_key.unwrap().name, "first-key");
    }

    #[test]
    fn test_key_persistence() {
        let (mut key_manager, temp_dir) = setup_test_key_manager().unwrap();
        
        // Generate a key
        let result = key_manager.generate_ed25519_key("persistent-key");
        assert!(result.is_ok());
        
        // Create new key manager with same directory path to test persistence
        let keys_path = temp_dir.path().join(".bxssh").join("keys.json");
        let loaded_keys = KeyManager::load_keys(&keys_path).unwrap();
        assert_eq!(loaded_keys.len(), 1);
        assert!(loaded_keys.contains_key("persistent-key"));
    }
}