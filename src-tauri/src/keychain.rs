use keyring::Entry;
use log::{debug, error, info, warn};
use std::error::Error;
use std::fmt;
use std::fs;
use std::path::PathBuf;
use dirs::data_local_dir;
use uuid::Uuid;
use tauri::command;
use once_cell::sync::OnceCell;

const SERVICE_NAME: &str = "com.journal.app";
const ACCOUNT_NAME: &str = "journal_encryption_key";
const KEY_FILE_NAME: &str = "journal.key";

// Static in-memory cache for the encryption key
static IN_MEMORY_KEY: OnceCell<String> = OnceCell::new();

#[derive(Debug)]
pub enum KeychainError {
    KeychainAccess(String),
    KeyNotFound,
    KeyStorage(String),
    KeyRetrieval(String),
    KeyDeletion(String),
    AuthenticationRequired,
    FileIO(String),
    MigrationError(String),
    AppSupportDirNotFound,
    KeyGeneration(String),
    AuthenticationFailed,
    KeychainAccessDenied,
    KeychainError(String),
    FileError(String),
}

impl fmt::Display for KeychainError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            KeychainError::KeychainAccess(msg) => write!(f, "Keychain access error: {}", msg),
            KeychainError::KeyNotFound => write!(f, "Key not found in keychain"),
            KeychainError::KeyStorage(msg) => write!(f, "Failed to store key: {}", msg),
            KeychainError::KeyRetrieval(msg) => write!(f, "Failed to retrieve key: {}", msg),
            KeychainError::KeyDeletion(msg) => write!(f, "Failed to delete key: {}", msg),
            KeychainError::AuthenticationRequired => write!(f, "Authentication required to access keychain"),
            KeychainError::FileIO(msg) => write!(f, "File I/O error: {}", msg),
            KeychainError::MigrationError(msg) => write!(f, "Migration error: {}", msg),
            KeychainError::AppSupportDirNotFound => write!(f, "Application Support directory not found"),
            KeychainError::KeyGeneration(msg) => write!(f, "Failed to generate key: {}", msg),
            KeychainError::AuthenticationFailed => write!(f, "Authentication failed to access the system keychain"),
            KeychainError::KeychainAccessDenied => write!(f, "Access denied to the system keychain"),
            KeychainError::KeychainError(msg) => write!(f, "Keychain error: {}", msg),
            KeychainError::FileError(msg) => write!(f, "File error: {}", msg),
        }
    }
}

impl Error for KeychainError {}

impl KeychainError {
    /// Converts the error into a user-friendly message
    pub fn to_user_message(&self) -> String {
        match self {
            // Authentication errors
            KeychainError::AuthenticationFailed => 
                "Unable to access the system keychain. Please check your system permissions.".to_string(),
            
            // Keychain access errors
            KeychainError::KeychainAccessDenied |
            KeychainError::KeychainError(_) |
            KeychainError::KeyNotFound => 
                "There was a problem accessing the system keychain. Please ensure you have granted the necessary permissions.".to_string(),
            
            // File system errors
            KeychainError::FileError(_) |
            KeychainError::AppSupportDirNotFound => 
                "There was a problem accessing the application data. Please ensure you have the necessary permissions.".to_string(),
            
            // Migration errors
            KeychainError::MigrationError(_) => 
                "There was a problem migrating your encryption key. Please try restarting the application.".to_string(),
            
            // Key generation errors
            KeychainError::KeyGeneration(_) => 
                "There was a problem generating a new encryption key. Please try restarting the application.".to_string(),
            
            // Generic error fallback
            _ => 
                "An unexpected error occurred. Please try restarting the application.".to_string(),
        }
    }
}

pub struct KeychainManager {
    keyring: Entry,
}

impl KeychainManager {
    pub fn new() -> Result<Self, KeychainError> {
        debug!("Initializing KeychainManager");
        Ok(Self {
            keyring: Entry::new(SERVICE_NAME, ACCOUNT_NAME)
                .map_err(|e| KeychainError::KeychainError(e.to_string()))?,
        })
    }

    fn get_app_support_dir() -> Result<PathBuf, KeychainError> {
        let base = data_local_dir().ok_or(KeychainError::AppSupportDirNotFound)?;
        let folder_name = if cfg!(debug_assertions) {
            "Journal-dev"
        } else {
            "Journal"
        };
        Ok(base.join(folder_name))
    }

    fn get_key_file_path() -> Result<PathBuf, KeychainError> {
        let app_dir = Self::get_app_support_dir()?;
        Ok(app_dir.join(KEY_FILE_NAME))
    }

    pub fn detect_existing_key_file() -> Result<Option<PathBuf>, KeychainError> {
        debug!("Checking for existing key file");

        // Current (expected) location
        let current_path = Self::get_key_file_path()?;
        if current_path.exists() {
            debug!("Found key file at expected location: {:?}", current_path);
            return Ok(Some(current_path));
        }

        // Look for a legacy key file in the *other* support directory
        let base = data_local_dir().ok_or(KeychainError::AppSupportDirNotFound)?;
        let legacy_folder = if cfg!(debug_assertions) { "Journal" } else { "Journal-dev" };
        let legacy_path = base.join(legacy_folder).join(KEY_FILE_NAME);

        if legacy_path.exists() {
            debug!(
                "Found legacy key file in alternate support directory: {:?}",
                legacy_path
            );
            Ok(Some(legacy_path))
        } else {
            debug!("No key file found in any known location");
            Ok(None)
        }
    }

    pub fn generate_and_store_new_key(&self) -> Result<String, KeychainError> {
        debug!("Generating new encryption key");
        
        // Generate a new UUID v4 as the encryption key
        let new_key = Uuid::new_v4().to_string();
        debug!("Generated new key");
        
        // Store the key in the keychain
        self.store_key(&new_key)?;
        info!("Successfully stored new key in keychain");
        
        Ok(new_key)
    }

    pub fn initialize_key(&self) -> Result<String, KeychainError> {
        debug!("Initializing encryption key");

        // 1️⃣ Try retrieving a key directly from the keychain
        match self.get_key() {
            Ok(key) => {
                debug!("Found existing key in keychain");
                // Clean up any stale key file once keychain is confirmed
                let _ = self.cleanup_stale_key_file();
                Ok(key)
            }
            Err(KeychainError::KeyNotFound) => {
                debug!("Key not found in keychain, will check for and migrate any existing key file");
                // If a legacy key file exists, migrate it
                if let Some(key_file_path) = Self::detect_existing_key_file()? {
                    debug!("Migrating existing key file: {:?}", key_file_path);
                    self.migrate_existing_key(&key_file_path)?;
                    // After migration, cleanup any remaining key file
                    let _ = self.cleanup_stale_key_file();
                    // Retrieve the migrated key from keychain
                    let key = self.get_key()?;
                    Ok(key)
                } else {
                    // No key file: generate a new key and store in keychain
                    debug!("No existing key file, generating new key");
                    let new_key = self.generate_and_store_new_key()?;
                    // Clean up if a key file somehow exists
                    let _ = self.cleanup_stale_key_file();
                    Ok(new_key)
                }
            }
            Err(e) => Err(e),
        }
    }

    /// Creates a backup of the key file before migration
    fn backup_key_file(&self, key_file_path: &PathBuf) -> Result<PathBuf, KeychainError> {
        let backup_path = key_file_path.with_extension("key.backup");
        debug!("Creating backup of key file at {:?}", backup_path);
        
        fs::copy(key_file_path, &backup_path)
            .map_err(|e| KeychainError::FileError(format!("Failed to create backup: {}", e)))?;
        
        debug!("Successfully created backup at {:?}", backup_path);
        Ok(backup_path)
    }

    /// Restores the key file from backup
    fn restore_from_backup(&self, backup_path: &PathBuf, original_path: &PathBuf) -> Result<(), KeychainError> {
        debug!("Restoring key file from backup at {:?}", backup_path);
        
        fs::copy(backup_path, original_path)
            .map_err(|e| KeychainError::FileError(format!("Failed to restore from backup: {}", e)))?;
        
        debug!("Successfully restored key file from backup");
        Ok(())
    }

    /// Attempts to recover from a failed migration
    fn recover_from_failed_migration(&self, key_file_path: &PathBuf) -> Result<(), KeychainError> {
        debug!("Attempting to recover from failed migration");
        
        let backup_path = key_file_path.with_extension("key.backup");
        if backup_path.exists() {
            debug!("Found backup file, attempting restoration");
            self.restore_from_backup(&backup_path, key_file_path)?;
            
            // Clean up the backup file after successful restoration
            fs::remove_file(&backup_path)
                .map_err(|e| KeychainError::FileError(format!("Failed to clean up backup: {}", e)))?;
            
            debug!("Successfully recovered from failed migration");
            Ok(())
        } else {
            debug!("No backup file found for recovery");
            Err(KeychainError::MigrationError("No backup available for recovery".to_string()))
        }
    }

    pub fn migrate_existing_key(&self, key_file_path: &PathBuf) -> Result<(), KeychainError> {
        debug!("Starting migration of existing key file: {:?}", key_file_path);
        
        // Check if key file exists
        if !key_file_path.exists() {
            debug!("No existing key file found at {:?}", key_file_path);
            return Ok(());
        }

        // Create a backup before proceeding
        let backup_path = self.backup_key_file(key_file_path)?;

        // Read the key from the file
        let key = match fs::read_to_string(key_file_path) {
            Ok(key) => {
                debug!("Successfully read key from file");
                key
            }
            Err(e) => {
                error!("Failed to read key file: {}", e);
                // Attempt to recover from backup
                self.recover_from_failed_migration(key_file_path)?;
                return Err(KeychainError::FileError(format!("Failed to read key file: {}", e)));
            }
        };

        // Store the key in the keychain
        if let Err(e) = self.store_key(&key) {
            error!("Failed to store key in keychain: {}", e);
            // Attempt to recover from backup
            self.recover_from_failed_migration(key_file_path)?;
            return Err(e);
        }
        
        debug!("Successfully stored key in keychain");

        // Delete the local key file
        if let Err(e) = fs::remove_file(key_file_path) {
            error!("Failed to delete key file: {}", e);
            // Attempt to recover from backup
            self.recover_from_failed_migration(key_file_path)?;
            return Err(KeychainError::FileError(format!("Failed to delete key file: {}", e)));
        }
        
        // Clean up the backup file after successful migration
        if let Err(e) = fs::remove_file(&backup_path) {
            warn!("Failed to clean up backup file: {}", e);
            // This is not critical, so we don't return an error
        }
        
        info!("Successfully migrated key to keychain and removed local file");
        Ok(())
    }

    /// Attempts to retrieve a key from the keychain, with specific handling for access denied scenarios
    pub fn get_key(&self) -> Result<String, KeychainError> {
        // First check the in-memory cache
        if let Some(key) = IN_MEMORY_KEY.get() {
            debug!("Retrieved key from in-memory cache");
            return Ok(key.clone());
        }

        // If not in cache, try to get from keychain
        match self.keyring.get_password() {
            Ok(key) => {
                debug!("Successfully retrieved key from keychain");
                // Store in cache for future use
                let _ = IN_MEMORY_KEY.set(key.clone());
                Ok(key)
            }
            Err(e) => {
                // Check for specific error messages that indicate access denied
                let error_msg = e.to_string().to_lowercase();
                if error_msg.contains("denied") || 
                   error_msg.contains("access") || 
                   error_msg.contains("permission") {
                    log::error!("Keychain access denied: {}", e);
                    Err(KeychainError::KeychainAccessDenied)
                } else if error_msg.contains("not found") {
                    log::error!("Key not found in keychain");
                    Err(KeychainError::KeyNotFound)
                } else {
                    log::error!("Failed to retrieve key from keychain: {}", e);
                    Err(KeychainError::KeychainError(e.to_string()))
                }
            }
        }
    }

    /// Attempts to store a key in the keychain, with specific handling for access denied scenarios
    fn store_key(&self, key: &str) -> Result<(), KeychainError> {
        match self.keyring.set_password(key) {
            Ok(_) => {
                log::info!("Successfully stored key in keychain");
                // Update the in-memory cache
                let _ = IN_MEMORY_KEY.set(key.to_string());
                Ok(())
            }
            Err(e) => {
                // Check for specific error messages that indicate access denied
                let error_msg = e.to_string().to_lowercase();
                if error_msg.contains("denied") || 
                   error_msg.contains("access") || 
                   error_msg.contains("permission") {
                    log::error!("Keychain access denied: {}", e);
                    Err(KeychainError::KeychainAccessDenied)
                } else {
                    log::error!("Failed to store key in keychain: {}", e);
                    Err(KeychainError::KeychainError(e.to_string()))
                }
            }
        }
    }


    /// Deletes any leftover on‑disk `journal.key` once the key is safely stored in
    /// the macOS Keychain.  It is a no‑op if no file is found.
    fn cleanup_stale_key_file(&self) -> Result<(), KeychainError> {
        if let Some(path) = Self::detect_existing_key_file()? {
            if path.exists() {
                debug!("Deleting stale key file at {:?}", path);
                fs::remove_file(&path).map_err(|e| {
                    KeychainError::FileError(format!("Failed to delete key file: {}", e))
                })?;
            }
        }
        Ok(())
    }

    /// Ensures we have a usable encryption key, prompting the user only once.
    ///
    /// Strategy:
    /// 1. If we have already cached a key for this process (`IN_MEMORY_KEY`),
    ///    return immediately – no keychain I/O and therefore no prompt.
    /// 2. Try to *read* the existing key from the Keychain (`get_key()`).
    ///    • Success ⇒ key is now cached, we're done (one “use item” prompt
    ///      unless the user chose “Always Allow” previously).
    ///    • `KeyNotFound` ⇒ first launch. Generate & store a brand‑new key,
    ///      which triggers exactly one “add item” prompt.
    ///    • Any other error (access‑denied, etc.) bubbles up.
    pub fn authorize_keychain(&self) -> Result<(), KeychainError> {
        // ──────────────────────────────────────────────────────────────
        // 1️⃣ Fast path: key is already cached for this process.
        // We **do not** delete any on‑disk key file yet; the database may
        // still depend on it. Cleanup happens after the DB opens.
        // ──────────────────────────────────────────────────────────────
        if IN_MEMORY_KEY.get().is_some() {
            return Ok(());
        }

        // ──────────────────────────────────────────────────────────────
        // 2️⃣ Try the key already stored in the macOS Keychain.  
        // If that works, again leave any legacy key file alone for now.
        // ──────────────────────────────────────────────────────────────
        match self.get_key() {
            Ok(_) => {
                // Key read & cached – we'll delete any legacy key file later,
                // once the database has opened successfully.
                Ok(())
            }, // key read & cached
            Err(KeychainError::KeyNotFound) => {
                // First look for a legacy on‑disk `journal.key` and migrate it.
                if let Some(path) = Self::detect_existing_key_file()? {
                    // One‑time migration (single “add item” prompt).
                    self.migrate_existing_key(&path)?;
                    self.cleanup_stale_key_file()?;
                    // Re‑read so the key is cached for this process.
                    self.get_key().map(|_| ())
                } else {
                    // Brand‑new install: generate a fresh key (single prompt).
                    self.generate_and_store_new_key().and_then(|_| {
                        self.cleanup_stale_key_file()?;
                        Ok(())
                    })
                }
            }
            Err(e) => Err(e), // propagate access‑denied or other errors
        }
    }
}

#[command]
pub fn authorize_keychain_command() -> Result<(), String> {
    let manager = KeychainManager::new().map_err(|e| e.to_user_message())?;
    manager.authorize_keychain().map_err(|e| e.to_user_message())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::NamedTempFile;

    #[test]
    fn test_keychain_operations() {
        let manager = KeychainManager::new().unwrap();
        let test_key = "test_key_123";

        // Test storing key
        assert!(manager.store_key(test_key).is_ok());

        // Test retrieving key
        let retrieved_key = manager.get_key().unwrap();
        assert_eq!(retrieved_key, test_key);

        // Test deleting key
        assert!(manager.delete_key().is_ok());

        // Verify key is deleted
        assert!(matches!(manager.get_key(), Err(KeychainError::KeyNotFound)));
    }

    #[test]
    fn test_migration() {
        let manager = KeychainManager::new().unwrap();
        let test_key = "test_key_456";

        // Create a temporary key file
        let temp_file = NamedTempFile::new().unwrap();
        fs::write(&temp_file, test_key).unwrap();

        // Test migration
        assert!(manager.migrate_existing_key(&temp_file.path().to_path_buf()).is_ok());

        // Verify key was stored in keychain
        let retrieved_key = manager.get_key().unwrap();
        assert_eq!(retrieved_key, test_key);

        // Verify file was deleted
        assert!(!temp_file.path().exists());

        // Cleanup
        manager.delete_key().unwrap();
    }

    #[test]
    fn test_key_file_detection() {
        // Test with non-existent file
        let result = KeychainManager::detect_existing_key_file().unwrap();
        assert!(result.is_none());

        // Create a temporary key file in the app support directory
        let app_dir = KeychainManager::get_app_support_dir().unwrap();
        fs::create_dir_all(&app_dir).unwrap();
        let key_file_path = app_dir.join(KEY_FILE_NAME);
        fs::write(&key_file_path, "test_key").unwrap();

        // Test detection
        let result = KeychainManager::detect_existing_key_file().unwrap();
        assert!(result.is_some());
        assert_eq!(result.unwrap(), key_file_path);

        // Cleanup
        fs::remove_file(key_file_path).unwrap();
    }

    #[test]
    fn test_new_key_generation() {
        let manager = KeychainManager::new().unwrap();
        
        // Generate and store a new key
        let new_key = manager.generate_and_store_new_key().unwrap();
        
        // Verify the key was stored
        let retrieved_key = manager.get_key().unwrap();
        assert_eq!(retrieved_key, new_key);
        
        // Cleanup
        manager.delete_key().unwrap();
    }

    #[test]
    fn test_key_initialization() {
        let manager = KeychainManager::new().unwrap();
        
        // Test initialization with no existing key
        let key = manager.initialize_key().unwrap();
        assert!(!key.is_empty());
        
        // Cleanup
        manager.delete_key().unwrap();
        
        // Test initialization with existing key file
        let app_dir = KeychainManager::get_app_support_dir().unwrap();
        fs::create_dir_all(&app_dir).unwrap();
        let key_file_path = app_dir.join(KEY_FILE_NAME);
        let test_key = "test_key_789";
        fs::write(&key_file_path, test_key).unwrap();
        
        let key = manager.initialize_key().unwrap();
        assert_eq!(key, test_key);
        assert!(!key_file_path.exists());
        
        // Cleanup
        manager.delete_key().unwrap();
    }
} 