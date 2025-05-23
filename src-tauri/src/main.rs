// Prevents additional console window on Windows in release, DO NOT REMOVE!!
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use chrono::Local;
use dirs::data_local_dir;
use rusqlite::params;
use serde::Serialize;
use std::fs;
use std::path::PathBuf;
use tauri::menu::{AboutMetadata, MenuBuilder, MenuItemBuilder, SubmenuBuilder};
use tauri::path::BaseDirectory;
use tauri::Emitter;
use tauri::Manager;
use tauri_plugin_dialog;
use uuid::Uuid;

fn app_support_dir() -> Result<PathBuf, String> {
    // Use a separate db in development vs. production
    let base = data_local_dir().ok_or("Could not find local data dir")?;
    let folder_name = if cfg!(debug_assertions) {
        "Journal-dev"
    } else {
        "Journal"
    };
    Ok(base.join(folder_name))
}

fn init_db() -> Result<rusqlite::Connection, String> {
    let app_dir = app_support_dir()?;

    std::fs::create_dir_all(&app_dir).map_err(|e| e.to_string())?;

    let db_path = app_dir.join("journal.db");
    let key_path = app_dir.join("journal.key");

    // Generate or read the encryption key
    let encryption_key = if key_path.exists() {
        fs::read_to_string(&key_path).map_err(|e| e.to_string())?
    } else {
        let new_key = Uuid::new_v4().to_string();
        fs::write(&key_path, &new_key).map_err(|e| e.to_string())?;
        new_key
    };
    let conn = rusqlite::Connection::open(db_path).map_err(|e| e.to_string())?;
    conn.pragma_update(None, "key", &encryption_key)
        .map_err(|e| e.to_string())?;

    conn.execute(
        "CREATE TABLE IF NOT EXISTS journal_entries (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            title TEXT NOT NULL,
            body TEXT NOT NULL,
            created_at TEXT DEFAULT CURRENT_TIMESTAMP
        )",
        [],
    )
    .map_err(|e| e.to_string())?;

    Ok(conn)
}

#[tauri::command]
fn export_database(app: tauri::AppHandle) -> Result<String, String> {
    let app_dir = app_support_dir()?;
    let db_path = app_dir.join("journal.db");

    let downloads_dir = app
        .path()
        .resolve("", BaseDirectory::Download)
        .map_err(|e| e.to_string())?;

    let timestamp = Local::now().format("%Y%m%d_%H%M%S").to_string();
    let export_path = downloads_dir.join(format!("journal_export_{}.db", timestamp));

    fs::copy(&db_path, &export_path).map_err(|e| e.to_string())?;
    Ok(export_path.to_string_lossy().into_owned())
}

#[tauri::command]
fn import_database(_app: tauri::AppHandle, file_path: String) -> Result<(), String> {
    let app_dir = app_support_dir()?;
    let db_path = app_dir.join("journal.db");

    if db_path.exists() {
        let backup_path = app_dir.join("journal.db.backup");
        fs::copy(&db_path, &backup_path).map_err(|e| e.to_string())?;
    }

    fs::copy(file_path, &db_path).map_err(|e| e.to_string())?;
    Ok(())
}

#[tauri::command]
fn save_entry(id: i32, title: String, body: String) -> Result<(), String> {
    let conn = init_db()?;
    conn.execute(
        "UPDATE journal_entries SET title = ?1, body = ?2 WHERE id = ?3",
        params![title, body, id],
    )
    .map_err(|e| e.to_string())?;
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
    let conn = init_db()?;

    let mut stmt = conn
        .prepare("SELECT id, title, created_at FROM journal_entries ORDER BY created_at DESC")
        .map_err(|e| e.to_string())?;

    let rows = stmt
        .query_map([], |row| {
            Ok(JournalEntry {
                id: row.get(0)?,
                title: row.get(1)?,
                created_at: row.get(2)?,
            })
        })
        .map_err(|e| e.to_string())?;

    let mut entries = Vec::new();
    for row in rows {
        entries.push(row.map_err(|e| e.to_string())?);
    }

    if entries.is_empty() {
        conn.execute(
            "INSERT INTO journal_entries (title, body) VALUES ('', '')",
            [],
        )
        .map_err(|e| e.to_string())?;

        let mut stmt = conn
            .prepare("SELECT id, title, created_at FROM journal_entries ORDER BY created_at DESC")
            .map_err(|e| e.to_string())?;

        let rows = stmt
            .query_map([], |row| {
                Ok(JournalEntry {
                    id: row.get(0)?,
                    title: row.get(1)?,
                    created_at: row.get(2)?,
                })
            })
            .map_err(|e| e.to_string())?;

        let mut entries = Vec::new();
        for row in rows {
            entries.push(row.map_err(|e| e.to_string())?);
        }
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
    let conn = init_db()?;

    let mut stmt = conn
        .prepare("SELECT id, title, body, created_at FROM journal_entries WHERE id = ?1")
        .map_err(|e| e.to_string())?;

    let mut rows = stmt.query(params![id]).map_err(|e| e.to_string())?;

    if let Some(row) = rows.next().map_err(|e| e.to_string())? {
        Ok(FullJournalEntry {
            id: row.get(0).map_err(|e| e.to_string())?,
            title: row.get(1).map_err(|e| e.to_string())?,
            body: row.get(2).map_err(|e| e.to_string())?,
            created_at: row.get(3).map_err(|e| e.to_string())?,
        })
    } else {
        Err("Entry not found".to_string())
    }
}

#[tauri::command]
fn create_entry() -> Result<i32, String> {
    let conn = init_db()?;
    conn.execute(
        "INSERT INTO journal_entries (title, body) VALUES ('', '')",
        [],
    )
    .map_err(|e| e.to_string())?;
    Ok(conn.last_insert_rowid() as i32)
}

#[tauri::command]
fn delete_all_entries() -> Result<(), String> {
    let conn = init_db()?;
    conn.execute("DELETE FROM journal_entries", [])
        .map_err(|e| e.to_string())?;
    Ok(())
}

#[tauri::command]
fn delete_entry(id: i32) -> Result<(), String> {
    let conn = init_db()?;
    conn.execute("DELETE FROM journal_entries WHERE id = ?1", params![id])
        .map_err(|e| e.to_string())?;
    Ok(())
}

fn main() {
    tauri::Builder::default()
        .plugin(tauri_plugin_clipboard_manager::init())
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_process::init())
        .setup(|app| {
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

            // --- File menu: New Entry ---
            let new_entry = MenuItemBuilder::new("New Entry")
                .id("new_entry")
                .accelerator("CmdOrCtrl+N")
                .build(app)?;
            let file_menu = SubmenuBuilder::new(app, "File").item(&new_entry).build()?;

            // --- Edit menu: custom with MenuItemBuilder ---------------------------
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

            // --- Window menu: Blur ---
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
        .expect("error while running Journal");
}
