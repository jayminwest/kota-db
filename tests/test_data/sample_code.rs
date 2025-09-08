//! Sample Rust code for testing and benchmarking code analysis features.
//! This file contains various Rust constructs to test symbol extraction,
//! dependency mapping, and search capabilities.

use std::collections::{HashMap, HashSet};
use std::sync::{Arc, Mutex};
use anyhow::{Result, Context};
use async_trait::async_trait;
use serde::{Serialize, Deserialize};

/// Configuration for the application
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    pub database_url: String,
    pub port: u16,
    pub workers: usize,
    pub timeout_ms: u64,
}

impl Default for Config {
    fn default() -> Self {
        Config {
            database_url: "postgres://localhost/test".to_string(),
            port: 8080,
            workers: 4,
            timeout_ms: 30000,
        }
    }
}

/// Main application state
pub struct AppState {
    config: Config,
    cache: Arc<Mutex<HashMap<String, Vec<u8>>>>,
    connections: HashSet<String>,
}

impl AppState {
    /// Creates a new application state
    pub fn new(config: Config) -> Self {
        AppState {
            config,
            cache: Arc::new(Mutex::new(HashMap::new())),
            connections: HashSet::new(),
        }
    }
    
    /// Validates the configuration
    pub fn validate_config(&self) -> Result<()> {
        if self.config.port == 0 {
            anyhow::bail!("Invalid port number");
        }
        if self.config.workers == 0 {
            anyhow::bail!("Must have at least one worker");
        }
        Ok(())
    }
    
    /// Adds a new connection
    pub fn add_connection(&mut self, conn_id: String) {
        self.connections.insert(conn_id);
    }
    
    /// Removes a connection
    pub fn remove_connection(&mut self, conn_id: &str) -> bool {
        self.connections.remove(conn_id)
    }
}

/// Trait for storage backends
#[async_trait]
pub trait Storage: Send + Sync {
    /// Retrieves data by key
    async fn get(&self, key: &str) -> Result<Option<Vec<u8>>>;
    
    /// Stores data with the given key
    async fn put(&mut self, key: String, value: Vec<u8>) -> Result<()>;
    
    /// Deletes data by key
    async fn delete(&mut self, key: &str) -> Result<bool>;
    
    /// Lists all keys
    async fn list_keys(&self) -> Result<Vec<String>>;
}

/// In-memory storage implementation
pub struct MemoryStorage {
    data: Arc<Mutex<HashMap<String, Vec<u8>>>>,
}

impl MemoryStorage {
    /// Creates a new memory storage instance
    pub fn new() -> Self {
        MemoryStorage {
            data: Arc::new(Mutex::new(HashMap::new())),
        }
    }
    
    /// Clears all data
    pub fn clear(&self) -> Result<()> {
        let mut data = self.data.lock().map_err(|e| anyhow::anyhow!("Lock poisoned: {}", e))?;
        data.clear();
        Ok(())
    }
}

#[async_trait]
impl Storage for MemoryStorage {
    async fn get(&self, key: &str) -> Result<Option<Vec<u8>>> {
        let data = self.data.lock().map_err(|e| anyhow::anyhow!("Lock poisoned: {}", e))?;
        Ok(data.get(key).cloned())
    }
    
    async fn put(&mut self, key: String, value: Vec<u8>) -> Result<()> {
        let mut data = self.data.lock().map_err(|e| anyhow::anyhow!("Lock poisoned: {}", e))?;
        data.insert(key, value);
        Ok(())
    }
    
    async fn delete(&mut self, key: &str) -> Result<bool> {
        let mut data = self.data.lock().map_err(|e| anyhow::anyhow!("Lock poisoned: {}", e))?;
        Ok(data.remove(key).is_some())
    }
    
    async fn list_keys(&self) -> Result<Vec<String>> {
        let data = self.data.lock().map_err(|e| anyhow::anyhow!("Lock poisoned: {}", e))?;
        Ok(data.keys().cloned().collect())
    }
}

/// Error handling utilities
pub mod errors {
    use thiserror::Error;
    
    #[derive(Error, Debug)]
    pub enum AppError {
        #[error("Storage error: {0}")]
        Storage(String),
        
        #[error("Configuration error: {0}")]
        Config(String),
        
        #[error("Connection error: {0}")]
        Connection(String),
        
        #[error("Validation error: {0}")]
        Validation(String),
    }
    
    /// Converts AppError to HTTP status code
    pub fn error_to_status(error: &AppError) -> u16 {
        match error {
            AppError::Storage(_) => 500,
            AppError::Config(_) => 500,
            AppError::Connection(_) => 503,
            AppError::Validation(_) => 400,
        }
    }
}

/// Validation utilities
pub mod validation {
    use super::*;
    
    /// Validates a username
    pub fn validate_username(username: &str) -> Result<()> {
        if username.is_empty() {
            anyhow::bail!("Username cannot be empty");
        }
        if username.len() > 100 {
            anyhow::bail!("Username too long");
        }
        if !username.chars().all(|c| c.is_alphanumeric() || c == '_') {
            anyhow::bail!("Username contains invalid characters");
        }
        Ok(())
    }
    
    /// Validates an email address (simplified)
    pub fn validate_email(email: &str) -> Result<()> {
        if !email.contains('@') {
            anyhow::bail!("Invalid email format");
        }
        Ok(())
    }
    
    /// Validates a password
    pub fn validate_password(password: &str) -> Result<()> {
        if password.len() < 8 {
            anyhow::bail!("Password must be at least 8 characters");
        }
        Ok(())
    }
}

/// Async handler functions
pub mod handlers {
    use super::*;
    
    /// Handles user registration
    pub async fn handle_register(
        username: String,
        email: String,
        password: String,
        storage: &mut impl Storage,
    ) -> Result<String> {
        validation::validate_username(&username)?;
        validation::validate_email(&email)?;
        validation::validate_password(&password)?;
        
        let user_data = format!("{}:{}:{}", username, email, password);
        storage.put(format!("user:{}", username), user_data.into_bytes()).await?;
        
        Ok(format!("User {} registered successfully", username))
    }
    
    /// Handles user login
    pub async fn handle_login(
        username: String,
        password: String,
        storage: &impl Storage,
    ) -> Result<bool> {
        let key = format!("user:{}", username);
        let data = storage.get(&key).await?
            .context("User not found")?;
        
        let user_data = String::from_utf8(data)?;
        let parts: Vec<&str> = user_data.split(':').collect();
        
        if parts.len() == 3 && parts[2] == password {
            Ok(true)
        } else {
            Ok(false)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_config_default() {
        let config = Config::default();
        assert_eq!(config.port, 8080);
        assert_eq!(config.workers, 4);
    }
    
    #[test]
    fn test_app_state_validation() {
        let mut config = Config::default();
        config.port = 0;
        let state = AppState::new(config);
        assert!(state.validate_config().is_err());
    }
    
    #[tokio::test]
    async fn test_memory_storage() {
        let mut storage = MemoryStorage::new();
        
        // Test put and get
        storage.put("key1".to_string(), vec![1, 2, 3]).await.unwrap();
        let value = storage.get("key1").await.unwrap();
        assert_eq!(value, Some(vec![1, 2, 3]));
        
        // Test delete
        let deleted = storage.delete("key1").await.unwrap();
        assert!(deleted);
        
        // Test list keys
        storage.put("key2".to_string(), vec![4, 5, 6]).await.unwrap();
        storage.put("key3".to_string(), vec![7, 8, 9]).await.unwrap();
        let keys = storage.list_keys().await.unwrap();
        assert_eq!(keys.len(), 2);
    }
    
    #[test]
    fn test_validation() {
        assert!(validation::validate_username("valid_user123").is_ok());
        assert!(validation::validate_username("").is_err());
        assert!(validation::validate_username("invalid-user").is_err());
        
        assert!(validation::validate_email("test@example.com").is_ok());
        assert!(validation::validate_email("invalid").is_err());
        
        assert!(validation::validate_password("longpassword").is_ok());
        assert!(validation::validate_password("short").is_err());
    }
    
    #[tokio::test]
    async fn test_handlers() {
        let mut storage = MemoryStorage::new();
        
        // Test registration
        let result = handlers::handle_register(
            "testuser".to_string(),
            "test@example.com".to_string(),
            "password123".to_string(),
            &mut storage,
        ).await;
        assert!(result.is_ok());
        
        // Test login
        let success = handlers::handle_login(
            "testuser".to_string(),
            "password123".to_string(),
            &storage,
        ).await.unwrap();
        assert!(success);
        
        // Test wrong password
        let success = handlers::handle_login(
            "testuser".to_string(),
            "wrongpass".to_string(),
            &storage,
        ).await.unwrap();
        assert!(!success);
    }
}