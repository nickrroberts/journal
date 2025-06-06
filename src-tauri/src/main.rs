// Prevents additional console window on Windows in release, DO NOT REMOVE!!
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use chrono::Local;
use dirs::data_local_dir;
use rusqlite::{Connection, Result as SqliteResult, params};
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;
use tauri::menu::{AboutMetadata, MenuBuilder, MenuItemBuilder, SubmenuBuilder};
use tauri::path::BaseDirectory;
use tauri::Emitter;
use tauri::Manager;
use tauri_plugin_dialog;
use uuid::Uuid;
use log::{debug, error, info};
use std::sync::Mutex;
use tauri::State;
use chrono::{DateTime, Utc};
use crate::keychain::{KeychainManager, KeychainError};

mod keychain;

struct DatabaseManager {
    conn: Connection,
    keychain: KeychainManager,
}

impl DatabaseManager {
    fn new() -> Result<Self, ErrorResponse> {
        debug!("Initializing database manager");
        
        // Create the database directory if it doesn't exist
        let db_dir = app_support_dir()?;
        fs::create_dir_all(&db_dir).map_err(|e| ErrorResponse {
            message: format!("Failed to create database directory: {}", e),
            error_type: "file_error".to_string(),
        })?;
        
        let db_path = db_dir.join("journal.db");
        debug!("Database path: {:?}", db_path);
        
        // Initialize the keychain manager
        let keychain = KeychainManager::new()
            .map_err(|e| ErrorResponse {
                message: e.to_user_message(),
                error_type: "keychain_error".to_string(),
            })?;
        
        // Get or generate the encryption key
        let encryption_key = keychain.initialize_key()
            .map_err(|e| ErrorResponse {
                message: e.to_user_message(),
                error_type: "keychain_error".to_string(),
            })?;
        
        let conn = rusqlite::Connection::open(db_path).map_err(|e| ErrorResponse {
            message: format!("Failed to open database: {}", e),
            error_type: "database_error".to_string(),
        })?;

        // Set the encryption key for the database
        conn.pragma_update(None, "key", &encryption_key)
            .map_err(|e| ErrorResponse {
                message: format!("Failed to set database encryption key: {}", e),
                error_type: "database_error".to_string(),
            })?;
        
        // Initialize the database schema
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

struct AppState {
    db: Mutex<DatabaseManager>,
}

#[tauri::command]
fn get_entries(state: State<AppState>) -> Result<Vec<JournalEntry>, ErrorResponse> {
    let db = state.db.lock().unwrap();
    let mut stmt = db.conn
        .prepare("SELECT id, title, created_at FROM journal_entries ORDER BY created_at DESC")
        .map_err(|e| ErrorResponse {
            message: format!("Failed to prepare query: {}", e),
            error_type: "database_error".to_string(),
        })?;

    let entries = stmt
        .query_map([], |row| {
            Ok(JournalEntry {
                id: row.get(0)?,
                title: row.get(1)?,
                created_at: row.get(2)?,
            })
        })
        .map_err(|e| ErrorResponse {
            message: format!("Failed to execute query: {}", e),
            error_type: "database_error".to_string(),
        })?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|e| ErrorResponse {
            message: format!("Failed to collect results: {}", e),
            error_type: "database_error".to_string(),
        })?;

    Ok(entries)
}

#[tauri::command]
fn get_entry(id: i32, state: State<AppState>) -> Result<FullJournalEntry, ErrorResponse> {
    let db = state.db.lock().unwrap();
    let mut stmt = db.conn
        .prepare("SELECT id, title, body, created_at FROM journal_entries WHERE id = ?1")
        .map_err(|e| ErrorResponse {
            message: format!("Failed to prepare query: {}", e),
            error_type: "database_error".to_string(),
        })?;

    let entry = stmt
        .query_row(params![id], |row| {
            Ok(FullJournalEntry {
                id: row.get(0)?,
                title: row.get(1)?,
                body: row.get(2)?,
                created_at: row.get(3)?,
            })
        })
        .map_err(|e| ErrorResponse {
            message: format!("Failed to get entry: {}", e),
            error_type: "database_error".to_string(),
        })?;

    Ok(entry)
}

#[tauri::command]
fn create_entry(
    request: CreateEntryRequest,
    state: State<AppState>,
) -> Result<i32, ErrorResponse> {
    let db = state.db.lock().unwrap();
    let now = Utc::now().to_rfc3339();

    db.conn.execute(
        "INSERT INTO journal_entries (title, body, created_at) VALUES (?1, ?2, ?3)",
        params![request.title, request.body, now],
    )
    .map_err(|e| ErrorResponse {
        message: format!("Failed to create entry: {}", e),
        error_type: "database_error".to_string(),
    })?;

    Ok(db.conn.last_insert_rowid() as i32)
}

#[tauri::command]
fn delete_all_entries(state: State<AppState>) -> Result<(), ErrorResponse> {
    let db = state.db.lock().unwrap();
    db.conn.execute("DELETE FROM journal_entries", [])
        .map_err(|e| ErrorResponse {
            message: format!("Failed to delete entries: {}", e),
            error_type: "database_error".to_string(),
        })?;
    Ok(())
}

#[tauri::command]
fn delete_entry(id: i32, state: State<AppState>) -> Result<(), ErrorResponse> {
    let db = state.db.lock().unwrap();
    db.conn.execute("DELETE FROM journal_entries WHERE id = ?1", params![id])
        .map_err(|e| ErrorResponse {
            message: format!("Failed to delete entry: {}", e),
            error_type: "database_error".to_string(),
        })?;
    Ok(())
}

#[tauri::command]
fn export_database(path: String, state: State<AppState>) -> Result<(), ErrorResponse> {
    let db = state.db.lock().unwrap();
    db.export_database(&PathBuf::from(path))
}

#[tauri::command]
fn import_database(path: String, state: State<AppState>) -> Result<(), ErrorResponse> {
    let db = state.db.lock().unwrap();
    db.import_database(&PathBuf::from(path))
}

fn main() {
    env_logger::init();
    debug!("Starting application");

    let db_manager = DatabaseManager::new().expect("Failed to initialize database");
    let app_state = AppState {
        db: Mutex::new(db_manager),
    };

    tauri::Builder::default()
        .manage(app_state)
        .invoke_handler(tauri::generate_handler![
            get_entries,
            get_entry,
            create_entry,
            delete_all_entries,
            delete_entry,
            export_database,
            import_database,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
