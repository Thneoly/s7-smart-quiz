// 用户数据备份/恢复校验/诊断包 与 应用设置
use rusqlite::{params, Connection};
use serde::Serialize;
use std::io::{Read as _, Write as _};
use std::path::Path;

// ---------- 备份 / 恢复 ----------
#[derive(Serialize)]
pub struct RestoreInfo { pub sessions: i64, pub records: i64, pub created_at: String }

pub fn backup_user(user: &Connection, dest: &str) -> Result<String, String> {
    let p = Path::new(dest);
    if p.extension().and_then(|e| e.to_str()) == Some("zip") {
        // 一致性快照后打 zip：manifest + snapshot.db
        let tmp = std::env::temp_dir().join(format!("sqbackup-{}.db", uuid::Uuid::new_v4()));
        snapshot_into(user, &tmp)?;
        let mf = serde_json::json!({
            "format": "smartquiz-backup", "schema_version": 1,
            "created_at": chrono::Utc::now().to_rfc3339(),
        });
        let f = std::fs::File::create(p).map_err(|e| e.to_string())?;
        let mut z = zip::ZipWriter::new(f);
        let opt = zip::write::SimpleFileOptions::default().compression_method(zip::CompressionMethod::Deflated);
        z.start_file("manifest.json", opt).map_err(|e| e.to_string())?;
        z.write_all(mf.to_string().as_bytes()).map_err(|e| e.to_string())?;
        z.start_file("user.db", opt).map_err(|e| e.to_string())?;
        z.write_all(&std::fs::read(&tmp).map_err(|e| e.to_string())?).map_err(|e| e.to_string())?;
        z.finish().map_err(|e| e.to_string())?;
        let _ = std::fs::remove_file(&tmp);
    } else {
        snapshot_into(user, p)?;
    }
    Ok(dest.to_string())
}

/// VACUUM INTO：WAL 运行态下的一致性快照（V1.1 §3.2）
fn snapshot_into(user: &Connection, dest: &Path) -> Result<(), String> {
    user.execute("VACUUM INTO ?1", params![dest.to_string_lossy()]).map_err(|e| e.to_string())?;
    Ok(())
}

/// 恢复前校验：备份可读、含 sessions 表，返回统计（实际替换由用户在设置页确认后执行）
pub fn restore_check(zip_path: &str) -> Result<RestoreInfo, String> {
    let f = std::fs::File::open(zip_path).map_err(|e| e.to_string())?;
    let mut z = zip::ZipArchive::new(f).map_err(|e| e.to_string())?;
    let mut buf = Vec::new();
    {
        let mut entry = z.by_name("user.db").map_err(|_| "备份文件中无 user.db".to_string())?;
        entry.read_to_end(&mut buf).map_err(|e| e.to_string())?;
    }
    let tmp = std::env::temp_dir().join(format!("sqrestore-{}.db", uuid::Uuid::new_v4()));
    std::fs::write(&tmp, &buf).map_err(|e| e.to_string())?;
    let rc = Connection::open_with_flags(&tmp, rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY).map_err(|e| e.to_string())?;
    let sessions: i64 = rc.query_row("SELECT COUNT(*) FROM sessions", [], |r| r.get(0)).map_err(|_| "备份不是有效的 user.db（缺 sessions 表）".to_string())?;
    let records: i64 = rc.query_row("SELECT COUNT(*) FROM answer_records", [], |r| r.get(0)).map_err(|e| e.to_string())?;
    let created_at = (|| -> Option<String> {
        let mut e = z.by_name("manifest.json").ok()?;
        let mut s = String::new();
        std::io::Read::read_to_string(&mut e, &mut s).ok()?;
        serde_json::from_str::<serde_json::Value>(&s).ok()?["created_at"].as_str().map(String::from)
    })().unwrap_or_default();
    let _ = std::fs::remove_file(&tmp);
    Ok(RestoreInfo { sessions, records, created_at })
}

// ---------- 诊断包 ----------
pub fn diagnostics(user: &Connection, log_dir: Option<&Path>, dest: &str) -> Result<String, String> {
    // 先收集日志文件（保留最近3次启动，单文件≤2MB，总量有界）
    let mut logs: Vec<(String, Vec<u8>)> = Vec::new();
    if let Some(dir) = log_dir {
        if let Ok(rd) = std::fs::read_dir(dir) {
            let mut paths: Vec<std::path::PathBuf> = rd.flatten().map(|e| e.path())
                .filter(|p| p.extension().and_then(|x| x.to_str()) == Some("log")).collect();
            paths.sort();
            for p in paths {
                if let Ok(bytes) = std::fs::read(&p) {
                    let name = p.file_name().map(|n| n.to_string_lossy().into_owned()).unwrap_or_default();
                    logs.push((name, bytes));
                }
            }
        }
    }
    let f = std::fs::File::create(dest).map_err(|e| e.to_string())?;
    let mut z = zip::ZipWriter::new(f);
    let opt = zip::write::SimpleFileOptions::default();
    let info = serde_json::json!({
        "app": "smart-quiz-app", "version": env!("CARGO_PKG_VERSION"),
        "os": std::env::consts::OS, "generated_at": chrono::Utc::now().to_rfc3339(),
        "counts": {
            "sessions": user.query_row("SELECT COUNT(*) FROM sessions", [], |r| r.get::<_, i64>(0)).ok(),
            "answers": user.query_row("SELECT COUNT(*) FROM answer_records", [], |r| r.get::<_, i64>(0)).ok(),
            "wrong": user.query_row("SELECT COUNT(*) FROM wrong_book", [], |r| r.get::<_, i64>(0)).ok(),
        },
        "log_files": logs.len()
    });
    z.start_file("diagnostics.json", opt).map_err(|e| e.to_string())?;
    z.write_all(serde_json::to_string_pretty(&info).unwrap().as_bytes()).map_err(|e| e.to_string())?;
    z.start_file("privacy.txt", opt).map_err(|e| e.to_string())?;
    z.write_all(b"This diagnostics package contains NO personal data.\nContains app version, OS, local counts and log files\n(command names, timings and error messages only - no question content,\nno user input; file paths are redacted to hide the OS account name).\n").map_err(|e| e.to_string())?;
    for (name, bytes) in logs {
        z.start_file(format!("logs/{name}"), opt).map_err(|e| e.to_string())?;
        z.write_all(&bytes).map_err(|e| e.to_string())?;
    }
    z.finish().map_err(|e| e.to_string())?;
    // 隐私：只记文件名，不记用户选择的完整保存路径（含账户名）
    let fname = std::path::Path::new(dest).file_name()
        .map(|n| n.to_string_lossy().into_owned()).unwrap_or_default();
    log::info!(target: "diag", "诊断包已导出：{fname}");
    Ok(dest.to_string())
}

// ---------- 设置 ----------
pub fn setting_get(user: &Connection, key: &str) -> Result<Option<String>, String> {
    Ok(user.query_row("SELECT value FROM settings WHERE key=?1", params![key], |r| r.get(0)).ok())
}
pub fn setting_set(user: &Connection, key: &str, value: &str) -> Result<(), String> {
    user.execute("INSERT INTO settings(key,value) VALUES(?,?) ON CONFLICT(key) DO UPDATE SET value=excluded.value",
        params![key, value]).map_err(|e| e.to_string())?;
    Ok(())
}

// ---------- Tauri 命令 ----------
pub mod commands {
    use super::{backup_user as backup_user_impl, restore_check as restore_check_impl,
                setting_get as setting_get_impl, setting_set as setting_set_impl,
                diagnostics, RestoreInfo};
    use crate::telemetry::timed;
    use crate::AppState;
    use tauri::Manager;

    #[tauri::command]
    pub async fn backup_user(state: tauri::State<'_, AppState>, dest: String) -> Result<String, String> {
        timed("backup_user", false, || {
            let u = state.user.lock().map_err(|e| e.to_string())?;
            backup_user_impl(&u, &dest)
        })
    }

    #[tauri::command]
    pub async fn restore_check(state: tauri::State<'_, AppState>, zip_path: String) -> Result<RestoreInfo, String> {
        timed("restore_check", false, || {
            let _guard = state.user.lock().map_err(|e| e.to_string())?; // 校验期间持锁，防备份/恢复竞争
            restore_check_impl(&zip_path)
        })
    }

    #[tauri::command]
    pub async fn export_diagnostics(state: tauri::State<'_, AppState>, app: tauri::AppHandle, dest: String) -> Result<String, String> {
        timed("export_diagnostics", false, || {
            let u = state.user.lock().map_err(|e| e.to_string())?;
            let log_dir = app.path().app_log_dir().ok();
            diagnostics(&u, log_dir.as_deref(), &dest)
        })
    }

    #[tauri::command]
    pub async fn setting_get(state: tauri::State<'_, AppState>, key: String) -> Result<Option<String>, String> {
        timed("setting_get", true, || {
            let u = state.user.lock().map_err(|e| e.to_string())?;
            setting_get_impl(&u, &key)
        })
    }

    #[tauri::command]
    pub async fn setting_set(state: tauri::State<'_, AppState>, key: String, value: String) -> Result<(), String> {
        timed("setting_set", true, || {
            let u = state.user.lock().map_err(|e| e.to_string())?;
            setting_set_impl(&u, &key, &value)
        })
    }
}
