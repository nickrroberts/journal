// Prevents additional console window on Windows in release, DO NOT REMOVE!!
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use rusqlite::{params, Connection, Result};
use serde::Serialize;
use dirs::data_local_dir;
use std::fs;
use chrono::Local;
use tauri::path::BaseDirectory;
use tauri::Manager;
use tauri_plugin_dialog;

fn main() {
    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .invoke_handler(tauri::generate_handler![
            save_entry, 
            get_entries, 
            get_entry, 
            create_entry,
            delete_entry,
            export_database,
            import_database,
            delete_all_entries
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}

fn init_db() -> Result<Connection> {
    let app_dir = data_local_dir()
        .expect("Could not find local data dir");
    
    // Create the app directory if it doesn't exist
    std::fs::create_dir_all(&app_dir).expect("Failed to create app directory");
    
    let db_path = app_dir.join("journal.db");
    let conn = Connection::open(db_path)?;

    conn.execute(
        "CREATE TABLE IF NOT EXISTS journal_entries (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            title TEXT NOT NULL,
            body TEXT NOT NULL,
            created_at TEXT DEFAULT CURRENT_TIMESTAMP
        )",
        [],
    )?;

    Ok(conn)
}

#[tauri::command]
fn export_database(app: tauri::AppHandle) -> Result<String, String> {
    let app_dir = data_local_dir()
        .ok_or_else(|| "Could not find local data directory".to_string())?;
    let db_path = app_dir.join("journal.db");
    
    let downloads_dir = app.path()
        .resolve("", BaseDirectory::Download)
        .map_err(|e| e.to_string())?;
    
    let timestamp = Local::now().format("%Y%m%d_%H%M%S").to_string();
    let export_path = downloads_dir.join(format!("journal_export_{}.db", timestamp));
    
    fs::copy(&db_path, &export_path)
        .map_err(|e| e.to_string())?;
    
    Ok(export_path.to_string_lossy().into_owned())
}

#[tauri::command]
fn import_database(_app: tauri::AppHandle, file_path: String) -> Result<(), String> {
    let app_dir = data_local_dir()
        .ok_or_else(|| "Could not find local data directory".to_string())?;
    let db_path = app_dir.join("journal.db");
    
    // Backup the current database
    if db_path.exists() {
        let backup_path = app_dir.join("journal.db.backup");
        fs::copy(&db_path, &backup_path)
            .map_err(|e| e.to_string())?;
    }
    
    // Copy the imported database
    fs::copy(file_path, &db_path)
        .map_err(|e| e.to_string())?;
    
    Ok(())
}

#[tauri::command]
fn save_entry(id: i32, title: String, body: String) -> Result<(), String> {
    let conn = init_db().map_err(|e| e.to_string())?;
    conn.execute(
        "UPDATE journal_entries SET title = ?1, body = ?2 WHERE id = ?3",
        params![title, body, id],
    ).map_err(|e| e.to_string())?;

    Ok(())
}


#[derive(Serialize)]
struct JournalEntry {
    id: i32,
    title: String,
    created_at: String,
}

#[tauri::command]
fn get_entries() -> Result<Vec<JournalEntry>, String> {
    let conn = init_db().map_err(|e| e.to_string())?;

    let mut stmt = conn.prepare("SELECT id, title, created_at FROM journal_entries ORDER BY created_at DESC")
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

    if entries.is_empty() {
        conn.execute(
            "INSERT INTO journal_entries (title, body) VALUES ('Untitled', '')",
            [],
        ).map_err(|e| e.to_string())?;

        let mut stmt = conn.prepare("SELECT id, title, created_at FROM journal_entries ORDER BY created_at DESC")
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

        return Ok(entries);
    }

    Ok(entries)
}

#[derive(Serialize)]
struct FullJournalEntry {
    id: i32,
    title: String,
    body: String,
    created_at: String,
}

#[tauri::command]
fn get_entry(id: i32) -> Result<FullJournalEntry, String> {
    let conn = init_db().map_err(|e| e.to_string())?;

    let mut stmt = conn
        .prepare("SELECT id, title, body, created_at FROM journal_entries WHERE id = ?1")
        .map_err(|e| e.to_string())?;

    let entry = stmt
        .query_row([id], |row| {
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
fn create_entry() -> Result<i32, String> {
    let conn = init_db().map_err(|e| e.to_string())?;
    conn.execute(
        "INSERT INTO journal_entries (title, body) VALUES ('', '')",
        [],
    ).map_err(|e| e.to_string())?;
    
    let id = conn.last_insert_rowid() as i32;
    Ok(id)
}

#[tauri::command]
fn delete_all_entries() -> Result<(), String> {
    let conn = init_db().map_err(|e| e.to_string())?;
    conn.execute("DELETE FROM journal_entries", [])
        .map_err(|e| e.to_string())?;
    Ok(())
}

#[tauri::command]
fn delete_entry(id: i32) -> Result<(), String> {
    let conn = init_db().map_err(|e| e.to_string())?;
    conn.execute(
        "DELETE FROM journal_entries WHERE id = ?1",
        params![id],
    )
    .map_err(|e| e.to_string())?;
    Ok(())
}
