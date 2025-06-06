// Prevents additional console window on Windows in release, DO NOT REMOVE!!
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;
use tauri::State;
use crate::keychain::{KeychainManager, KeychainError, authorize_keychain_command};
use tauri_plugin_updater;
use log::{debug, warn};
use std::sync::Mutex;
use chrono::Utc;
use once_cell::sync::OnceCell;
use std::fmt;

mod keychain;

struct DatabaseManager {
    conn: rusqlite::Connection,
    keychain: KeychainManager,
}

impl DatabaseManager {
    fn new() -> Result<Self, ErrorResponse> {
        debug!("Initializing database manager");
        let db_dir = app_support_dir()?;
        fs::create_dir_all(&db_dir).map_err(|e| ErrorResponse {
            message: format!("Failed to create database directory: {}", e),
            error_type: "file_error".to_string(),
        })?;
        let db_path = db_dir.join("journal.db");
        debug!("Database path: {:?}", db_path);
        let keychain = KeychainManager::new()
            .map_err(|e| ErrorResponse {
                message: e.to_string(),
                error_type: "keychain_error".to_string(),
            })?;
        let encryption_key = match keychain.get_key() {
            Ok(key) => {
                debug!("Retrieved encryption key from keychain");
                key
            },
            Err(KeychainError::KeyNotFound) => {
                debug!("No key found in keychain, generating new key");
                keychain.generate_and_store_new_key().map_err(|e| ErrorResponse {
                    message: e.to_string(),
                    error_type: "keychain_error".to_string(),
                })?
            },
            Err(e) => {
                return Err(ErrorResponse {
                    message: e.to_string(),
                    error_type: "keychain_error".to_string(),
                });
            }
        };
        let db_exists = db_path.exists();
        let mut conn_result = rusqlite::Connection::open(&db_path);
        let mut conn = match conn_result {
            Ok(c) => c,
            Err(e) => {
                #[cfg(debug_assertions)]
                {
                    warn!("Failed to open DB in dev mode: {}. Attempting to reset DB.", e);
                    let _ = fs::remove_file(&db_path);
                    rusqlite::Connection::open(&db_path).map_err(|e| ErrorResponse {
                        message: format!("Failed to create new database after reset: {}", e),
                        error_type: "database_error".to_string(),
                    })?
                }
                #[cfg(not(debug_assertions))]
                {
                    return Err(ErrorResponse {
                        message: format!("Failed to open database: {}", e),
                        error_type: "database_error".to_string(),
                    });
                }
            }
        };
        debug!("Setting database encryption key");
        let set_key_result = conn.pragma_update(None, "key", &encryption_key);
        if let Err(e) = set_key_result {
            #[cfg(debug_assertions)]
            {
                warn!("Failed to set key in dev mode: {}. Attempting to reset DB.", e);
                let _ = fs::remove_file(&db_path);
                conn = rusqlite::Connection::open(&db_path).map_err(|e| ErrorResponse {
                    message: format!("Failed to create new database after reset: {}", e),
                    error_type: "database_error".to_string(),
                })?;
                conn.pragma_update(None, "key", &encryption_key).map_err(|e| ErrorResponse {
                    message: format!("Failed to set key after reset: {}", e),
                    error_type: "database_error".to_string(),
                })?;
            }
            #[cfg(not(debug_assertions))]
            {
                return Err(ErrorResponse {
                    message: format!("Failed to set database encryption key: {}", e),
                    error_type: "database_error".to_string(),
                });
            }
        }
        if db_exists {
            debug!("Checking for journal_entries table in existing database");
            let schema_missing_or_error = {
                let check = conn.prepare("SELECT name FROM sqlite_master WHERE type='table' AND name='journal_entries'");
                match check {
                    Ok(mut stmt) => {
                        let mut rows = stmt.query([]).map_err(|e| ErrorResponse {
                            message: format!("Failed to query schema: {}", e),
                            error_type: "database_error".to_string(),
                        })?;
                        rows.next()?.is_none()
                    }
                    Err(_) => true
                }
            };
            if schema_missing_or_error {
                #[cfg(debug_assertions)]
                {
                    warn!("Schema missing or error in dev mode. Resetting DB.");
                    let _ = fs::remove_file(&db_path);
                    conn = rusqlite::Connection::open(&db_path).map_err(|e| ErrorResponse {
                        message: format!("Failed to create new database after reset: {}", e),
                        error_type: "database_error".to_string(),
                    })?;
                    conn.pragma_update(None, "key", &encryption_key).map_err(|e| ErrorResponse {
                        message: format!("Failed to set key after reset: {}", e),
                        error_type: "database_error".to_string(),
                    })?;
                    debug!("Creating database schema after reset");
                    conn.execute(
                        "CREATE TABLE IF NOT EXISTS journal_entries (
                            id INTEGER PRIMARY KEY,
                            title TEXT NOT NULL,
                            body TEXT NOT NULL,
                            created_at TEXT NOT NULL
                        )",
                        [],
                    ).map_err(|e| ErrorResponse {
                        message: format!("Failed to create database schema after reset: {}", e),
                        error_type: "database_error".to_string(),
                    })?;
                }
                #[cfg(not(debug_assertions))]
                {
                    return Err(ErrorResponse {
                        message: "Database exists but schema is missing or corrupt. Please reset or migrate your database.".to_string(),
                        error_type: "database_error".to_string(),
                    });
                }
            }
        } else {
            debug!("Creating database schema");
            conn.execute(
                "CREATE TABLE IF NOT EXISTS journal_entries (
                    id INTEGER PRIMARY KEY,
                    title TEXT NOT NULL,
                    body TEXT NOT NULL,
                    created_at TEXT NOT NULL
                )",
                [],
            ).map_err(|e| ErrorResponse {
                message: format!("Failed to create database schema: {}", e),
                error_type: "database_error".to_string(),
            })?;
        }
        Ok(Self { conn, keychain })
    }

    fn export_database(&self, export_path: &PathBuf) -> Result<(), ErrorResponse> {
        debug!("Exporting database to {:?}", export_path);
        fs::copy(self.conn.path().unwrap(), export_path)
            .map_err(|e| ErrorResponse {
                message: format!("Failed to export database: {}", e),
                error_type: "file_error".to_string(),
            })?;
        Ok(())
    }

    fn import_database(&self, import_path: &PathBuf) -> Result<(), ErrorResponse> {
        debug!("Importing database from {:?}", import_path);
        fs::copy(import_path, self.conn.path().unwrap())
            .map_err(|e| ErrorResponse {
                message: format!("Failed to import database: {}", e),
                error_type: "file_error".to_string(),
            })?;
        Ok(())
    }
}

#[derive(Debug, Serialize)]
pub struct ErrorResponse {
    message: String,
    error_type: String,
}

impl From<String> for ErrorResponse {
    fn from(error: String) -> Self {
        ErrorResponse { 
            message: error,
            error_type: "unknown_error".to_string() 
        }
    }
}

impl From<rusqlite::Error> for ErrorResponse {
    fn from(error: rusqlite::Error) -> Self {
        ErrorResponse { 
            message: error.to_string(),
            error_type: "database_error".to_string() 
        }
    }
}

impl fmt::Display for ErrorResponse {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}: {}", self.error_type, self.message)
    }
}

fn app_support_dir() -> Result<PathBuf, ErrorResponse> {
    let base = dirs::data_local_dir().ok_or_else(|| ErrorResponse {
        message: "Could not determine application support directory".to_string(),
        error_type: "app_support_error".to_string(),
    })?;
    
    let folder_name = if cfg!(debug_assertions) {
        "Journal-dev"
    } else {
        "Journal"
    };
    
    Ok(base.join(folder_name))
}

#[derive(Debug, Serialize, Deserialize)]
struct JournalEntry {
    id: i32,
    title: String,
    created_at: String,
}

#[derive(Debug, Serialize, Deserialize)]
struct FullJournalEntry {
    id: i32,
    title: String,
    body: String,
    created_at: String,
}

#[derive(Debug, Deserialize)]
struct CreateEntryRequest {
    title: String,
    body: String,
}

#[tauri::command]
fn get_entries() -> Result<Vec<JournalEntry>, String> {
    let db = DatabaseManager::new().map_err(|e| e.to_string())?;
    let mut stmt = db.conn
        .prepare("SELECT id, title, created_at FROM journal_entries ORDER BY created_at DESC")
        .map_err(|e| e.to_string())?;
    let entries = stmt
        .query_map([], |row| {
            Ok(JournalEntry {
                id: row.get(0)?,
                title: row.get(1)?,
                created_at: row.get(2)?,
            })
        })
        .map_err(|e| e.to_string())?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|e| e.to_string())?;
    Ok(entries)
}

#[tauri::command]
fn get_entry(id: i32) -> Result<FullJournalEntry, String> {
    let db = DatabaseManager::new().map_err(|e| e.to_string())?;
    let mut stmt = db.conn
        .prepare("SELECT id, title, body, created_at FROM journal_entries WHERE id = ?1")
        .map_err(|e| e.to_string())?;
    let entry = stmt
        .query_row(rusqlite::params![id], |row| {
            Ok(FullJournalEntry {
                id: row.get(0)?,
                title: row.get(1)?,
                body: row.get(2)?,
                created_at: row.get(3)?,
            })
        })
        .map_err(|e| e.to_string())?;
    Ok(entry)
}

#[tauri::command]
fn create_entry(request: CreateEntryRequest) -> Result<i32, String> {
    let db = DatabaseManager::new().map_err(|e| e.to_string())?;
    let now = Utc::now().to_rfc3339();
    db.conn.execute(
        "INSERT INTO journal_entries (title, body, created_at) VALUES (?1, ?2, ?3)",
        rusqlite::params![request.title, request.body, now],
    )
    .map_err(|e| e.to_string())?;
    Ok(db.conn.last_insert_rowid() as i32)
}

#[tauri::command]
fn save_entry(id: i32, title: String, body: String) -> Result<(), String> {
    let db = DatabaseManager::new().map_err(|e| e.to_string())?;
    db.conn.execute(
        "UPDATE journal_entries SET title = ?1, body = ?2 WHERE id = ?3",
        rusqlite::params![title, body, id],
    )
    .map_err(|e| e.to_string())?;
    Ok(())
}

#[tauri::command]
fn delete_all_entries() -> Result<(), String> {
    let db = DatabaseManager::new().map_err(|e| e.to_string())?;
    db.conn.execute("DELETE FROM journal_entries", [])
        .map_err(|e| e.to_string())?;
    Ok(())
}

#[tauri::command]
fn delete_entry(id: i32) -> Result<(), String> {
    let db = DatabaseManager::new().map_err(|e| e.to_string())?;
    db.conn.execute("DELETE FROM journal_entries WHERE id = ?1", rusqlite::params![id])
        .map_err(|e| e.to_string())?;
    Ok(())
}

#[tauri::command]
fn export_database(path: String) -> Result<(), String> {
    let db = DatabaseManager::new().map_err(|e| e.to_string())?;
    db.export_database(&PathBuf::from(path)).map_err(|e| e.to_string())
}

#[tauri::command]
fn import_database(path: String) -> Result<(), String> {
    let db = DatabaseManager::new().map_err(|e| e.to_string())?;
    db.import_database(&PathBuf::from(path)).map_err(|e| e.to_string())
}

fn main() {
    env_logger::init();
    debug!("Starting application");

    tauri::Builder::default()
        .plugin(tauri_plugin_updater::Builder::new().build())
        .invoke_handler(tauri::generate_handler![
            get_entries,
            get_entry,
            create_entry,
            save_entry,
            delete_all_entries,
            delete_entry,
            export_database,
            import_database,
            authorize_keychain_command,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
