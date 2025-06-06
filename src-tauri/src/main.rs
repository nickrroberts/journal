// Prevents additional console window on Windows in release, DO NOT REMOVE!!
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;
use crate::keychain::{KeychainManager, authorize_keychain_command};
use tauri_plugin_updater;
use log::{debug, warn};
use chrono::Utc;
use std::fmt;
use tauri::menu::{AboutMetadata, MenuBuilder, MenuItemBuilder, SubmenuBuilder};
use tauri_plugin_clipboard_manager;
use tauri_plugin_opener;
use tauri_plugin_process;
use tauri_plugin_dialog;
use tauri::{Emitter, Manager};

mod keychain;

struct DatabaseManager {
    conn: rusqlite::Connection,
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
        // Track whether a database already exists before we open it or copy one in
        let mut db_exists = db_path.exists();
        // ------------------------------------------------------------------
        // Legacy migration: copy an existing database from the *alternate*
        // application‑support folder (e.g. "Journal" ↔ "Journal‑dev") if the
        // current location is empty. This prevents data loss when users move
        // between release and dev builds.
        // ------------------------------------------------------------------
        if !db_path.exists() {
            let base = dirs::data_local_dir().ok_or_else(|| ErrorResponse {
                message: "Could not determine application support directory".to_string(),
                error_type: "app_support_error".to_string(),
            })?;
            // The folder we're NOT currently using
            let alt_folder = if cfg!(debug_assertions) { "Journal" } else { "Journal-dev" };
            let alt_db_path = base.join(alt_folder).join("journal.db");
            if alt_db_path.exists() {
                debug!("Found legacy database at {:?}, migrating…", alt_db_path);
                // Ensure destination directory exists (already created above, but be safe)
                fs::create_dir_all(&db_dir).map_err(|e| ErrorResponse {
                    message: format!("Failed to create database directory: {}", e),
                    error_type: "file_error".to_string(),
                })?;
                fs::copy(&alt_db_path, &db_path).map_err(|e| ErrorResponse {
                    message: format!("Failed to migrate legacy database: {}", e),
                    error_type: "file_error".to_string(),
                })?;
                // Mark that a database now exists in the current location
                db_exists = true;
            }
        }
        debug!("Database path: {:?}", db_path);
        let keychain = KeychainManager::new()
            .map_err(|e| ErrorResponse {
                message: e.to_string(),
                error_type: "keychain_error".to_string(),
            })?;
        // Ensure we have a key in the Keychain (handles legacy file migration too)
        keychain.authorize_keychain().map_err(|e| ErrorResponse {
            message: e.to_string(),
            error_type: "keychain_error".to_string(),
        })?;

        // After authorization, migrate any legacy key-file and get the correct key
        let mut encryption_key = keychain
            .initialize_key()
            .map_err(|e| ErrorResponse {
                message: e.to_string(),
                error_type: "keychain_error".to_string(),
            })?;
        let conn_result = rusqlite::Connection::open(&db_path);
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
                warn!(
                    "Failed to set key in dev mode: {}. Trying to migrate legacy key before resetting DB.",
                    e
                );

                // Assume a full reset is needed unless migration succeeds
                let mut must_reset = true;

                // 1️⃣ Try migrating any legacy on‑disk key first
                if let Ok(Some(key_path)) = KeychainManager::detect_existing_key_file() {
                    warn!("Attempting key migration from {:?}", key_path);
                    if keychain.migrate_existing_key(&key_path).is_ok() {
                        if let Ok(new_key) = keychain.get_key() {
                            // Use the migrated key going forward
                            encryption_key = new_key;

                            // Re‑open the connection and retry with the migrated key
                            conn = rusqlite::Connection::open(&db_path).map_err(|e| ErrorResponse {
                                message: format!("Failed to reopen database after key migration: {}", e),
                                error_type: "database_error".to_string(),
                            })?;

                            if conn.pragma_update(None, "key", &encryption_key).is_ok() {
                                debug!("Key migration succeeded – no data loss 🎉");
                                must_reset = false;
                            }
                        }
                    }
                }

                // 2️⃣ Last resort: wipe and recreate the DB (old behaviour)
                if must_reset {
                    warn!("Resetting database because it could not be opened with any key.");
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
                    warn!(
                        "Schema not found or unreadable – possible key mismatch. \
                         Attempting last‑chance key migration before wiping."
                    );

                    // Flag to decide whether we really need to reset the DB
                    let mut recovered = false;

                    // 👉 Try migrating any stray on‑disk key (if one still exists)
                    if let Ok(Some(key_path)) = KeychainManager::detect_existing_key_file() {
                        warn!("Attempting key migration from {:?}", key_path);
                        if keychain.migrate_existing_key(&key_path).is_ok() {
                            if let Ok(new_key) = keychain.get_key() {
                                // Use the migrated key from now on
                                encryption_key = new_key;

                                // Re‑open the connection with the migrated key
                                if let Ok(c) = rusqlite::Connection::open(&db_path) {
                                    if c.pragma_update(None, "key", &encryption_key).is_ok() {
                                        // Quick sanity‑check: does the expected table exist now?
                                        let table_ok = c
                                            .query_row(
                                                "SELECT 1 FROM sqlite_master \
                                                 WHERE type='table' AND name='journal_entries' \
                                                 LIMIT 1",
                                                [],
                                                |_| Ok::<_, rusqlite::Error>(()),
                                            )
                                            .is_ok();

                                        if table_ok {
                                            debug!(
                                                "Key migration succeeded – keeping existing \
                                                 database intact 🎉"
                                            );
                                            conn = c;
                                            recovered = true;
                                        }
                                    }
                                }
                            }
                        }
                    }

                    // ❌ Migration failed – fall back to the original dev‑mode reset
                    if !recovered {
                        warn!("Resetting database because it could not be opened with any key.");
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
        Ok(Self { conn })
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
        .plugin(tauri_plugin_clipboard_manager::init())
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_process::init())
        .setup(|app| {
            // Build the application menu --------------------------
            let settings = MenuItemBuilder::new("Settings…")
                .id("settings")
                .accelerator("Cmd+,")
                .build(app)?;
            let check_updates = MenuItemBuilder::new("Check for updates…")
                .id("check_updates")
                .build(app)?;
            let app_submenu = SubmenuBuilder::new(app, &app.package_info().name)
                .about(Some(AboutMetadata::default()))
                .separator()
                .item(&settings)
                .item(&check_updates)
                .separator()
                .quit()
                .build()?;

            // File ▸ New Entry
            let new_entry = MenuItemBuilder::new("New Entry")
                .id("new_entry")
                .accelerator("CmdOrCtrl+N")
                .build(app)?;
            let file_menu = SubmenuBuilder::new(app, "File")
                .item(&new_entry)
                .build()?;

            // Edit menu with standard shortcuts
            let undo = MenuItemBuilder::new("Undo")
                .id("undo")
                .accelerator("CmdOrCtrl+Z")
                .build(app)?;
            let redo = MenuItemBuilder::new("Redo")
                .id("redo")
                .accelerator("Shift+CmdOrCtrl+Z")
                .build(app)?;
            let cut = MenuItemBuilder::new("Cut")
                .id("cut")
                .accelerator("CmdOrCtrl+X")
                .build(app)?;
            let copy = MenuItemBuilder::new("Copy")
                .id("copy")
                .accelerator("CmdOrCtrl+C")
                .build(app)?;
            let paste = MenuItemBuilder::new("Paste")
                .id("paste")
                .accelerator("CmdOrCtrl+V")
                .build(app)?;
            let select_all = MenuItemBuilder::new("Select All")
                .id("select_all")
                .accelerator("CmdOrCtrl+A")
                .build(app)?;
            let edit_menu = SubmenuBuilder::new(app, "Edit")
                .item(&undo)
                .item(&redo)
                .separator()
                .item(&cut)
                .item(&copy)
                .item(&paste)
                .separator()
                .item(&select_all)
                .build()?;

            // Window ▸ Blur toggle
            let blur_item = MenuItemBuilder::new("Blur")
                .id("blur")
                .accelerator("Ctrl+B")
                .build(app)?;
            let window_menu = SubmenuBuilder::new(app, "Window")
                .item(&blur_item)
                .build()?;

            let menu = MenuBuilder::new(app)
                .items(&[&app_submenu, &file_menu, &edit_menu, &window_menu])
                .build()?;
            app.set_menu(menu)?;
            Ok(())
        })
        .on_menu_event(|window, menu_event| match menu_event.id().0.as_str() {
            "settings" => window.emit("open-settings", {}).unwrap(),
            "check_updates" => window.emit("check-for-updates", {}).unwrap(),
            "new_entry" => window.emit("new-entry", {}).unwrap(),
            "blur" => window.emit("blur", {}).unwrap(),
            "undo" | "redo" | "cut" | "copy" | "paste" | "select_all" => {
                window
                    .get_webview_window("main")
                    .unwrap()
                    .eval(&format!("document.execCommand('{}')", menu_event.id().0))
                    .unwrap();
            }
            _ => {}
        })
        .plugin(tauri_plugin_updater::Builder::new().build())
        .plugin(tauri_plugin_dialog::init())
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
