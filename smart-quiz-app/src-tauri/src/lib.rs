// S7-200 SMART 题库平台 —— M3
mod bank;
mod db;
mod m2;
mod m3;
mod protocol;
mod refs;
mod user;

use rusqlite::Connection;
use std::path::PathBuf;
use std::sync::Mutex;
use tauri::Manager;

pub struct AppState {
    pub bank: Mutex<Connection>,
    pub user: Mutex<Connection>,
}

// ---------- 题库查询 ----------
#[tauri::command]
async fn bank_overview(state: tauri::State<'_, AppState>) -> Result<bank::Overview, String> {
    let b = state.bank.lock().map_err(|e| e.to_string())?;
    bank::overview(&b)
}

#[tauri::command]
async fn list_papers(state: tauri::State<'_, AppState>) -> Result<Vec<bank::PaperInfo>, String> {
    let b = state.bank.lock().map_err(|e| e.to_string())?;
    bank::list_papers(&b)
}

#[tauri::command]
async fn paper_questions(state: tauri::State<'_, AppState>, paper_id: i64) -> Result<Vec<(String, String)>, String> {
    let b = state.bank.lock().map_err(|e| e.to_string())?;
    bank::paper_qids(&b, paper_id)
}

#[tauri::command]
async fn dashboard(state: tauri::State<'_, AppState>) -> Result<user::Dashboard, String> {
    let b = state.bank.lock().map_err(|e| e.to_string())?;
    let u = state.user.lock().map_err(|e| e.to_string())?;
    user::dashboard(&u, &b)
}

#[tauri::command]
async fn wrong_list(state: tauri::State<'_, AppState>) -> Result<Vec<user::WrongRow>, String> {
    let b = state.bank.lock().map_err(|e| e.to_string())?;
    let u = state.user.lock().map_err(|e| e.to_string())?;
    user::wrong_list(&u, &b, true)
}

#[tauri::command]
async fn fav_list(state: tauri::State<'_, AppState>) -> Result<Vec<bank::QuestionRow>, String> {
    let b = state.bank.lock().map_err(|e| e.to_string())?;
    let u = state.user.lock().map_err(|e| e.to_string())?;
    user::fav_list(&u, &b)
}

#[tauri::command]
async fn unfinished_sessions(state: tauri::State<'_, AppState>) -> Result<Vec<user::SessionInfo>, String> {
    let u = state.user.lock().map_err(|e| e.to_string())?;
    user::unfinished_sessions(&u)
}

#[tauri::command]
async fn list_sessions(state: tauri::State<'_, AppState>) -> Result<Vec<user::SessionBrief>, String> {
    let u = state.user.lock().map_err(|e| e.to_string())?;
    user::list_sessions(&u, 30)
}

#[tauri::command]
async fn list_questions(state: tauri::State<'_, AppState>,
    topic_id: Option<i64>, qtype: Option<String>, status: Option<String>,
    search: Option<String>, limit: Option<i64>, offset: Option<i64>) -> Result<Vec<bank::QuestionRow>, String> {
    let conn = state.bank.lock().map_err(|e| e.to_string())?;
    bank::list_questions(&conn, topic_id, qtype, status, search,
        limit.unwrap_or(50).min(500), offset.unwrap_or(0))
}

#[tauri::command]
async fn get_questions_by_ids(state: tauri::State<'_, AppState>, qids: Vec<(String, String)>)
    -> Result<Vec<bank::QuestionRow>, String> {
    let conn = state.bank.lock().map_err(|e| e.to_string())?;
    bank::get_questions_by_ids(&conn, &qids)
}

#[tauri::command]
async fn due_review(state: tauri::State<'_, AppState>, limit: Option<i64>) -> Result<Vec<bank::QuestionRow>, String> {
    let b = state.bank.lock().map_err(|e| e.to_string())?;
    let u = state.user.lock().map_err(|e| e.to_string())?;
    user::due_review(&u, &b, limit.unwrap_or(20))
}

#[tauri::command]
async fn fts_spike(state: tauri::State<'_, AppState>, scale_to: Option<usize>) -> Result<bank::SpikeResult, String> {
    let conn = state.bank.lock().map_err(|e| e.to_string())?;
    bank::fts_spike(&conn, scale_to.unwrap_or(100_000).min(200_000),
        &["以太网通信", "高速计数器", "Modbus", "模拟量输入"])
}

// ---------- 会话 ----------
#[tauri::command]
async fn start_session(state: tauri::State<'_, AppState>, mode: String, title: String, bank_id: String,
                       paper_id: Option<i64>, qids: Vec<(String, String)>, time_limit_sec: Option<i64>)
    -> Result<user::SessionInfo, String> {
    let u = state.user.lock().map_err(|e| e.to_string())?;
    user::start_session(&u, &mode, &title, &bank_id, paper_id, &qids, time_limit_sec)
}

#[tauri::command]
async fn save_draft(state: tauri::State<'_, AppState>, session_id: i64, draft: serde_json::Value) -> Result<(), String> {
    let u = state.user.lock().map_err(|e| e.to_string())?;
    let d: user::Draft = serde_json::from_value(draft).map_err(|e| e.to_string())?;
    user::save_draft(&u, session_id, &d)
}

#[tauri::command]
async fn finish_session(state: tauri::State<'_, AppState>, session_id: i64) -> Result<user::SessionInfo, String> {
    let b = state.bank.lock().map_err(|e| e.to_string())?;
    let u = state.user.lock().map_err(|e| e.to_string())?;
    user::finish_session(&u, &b, session_id)
}

#[tauri::command]
async fn session_detail(state: tauri::State<'_, AppState>, session_id: i64) -> Result<user::SessionDetail, String> {
    let b = state.bank.lock().map_err(|e| e.to_string())?;
    let u = state.user.lock().map_err(|e| e.to_string())?;
    user::session_detail(&u, &b, session_id)
}

// ---------- 错题/收藏/笔记 ----------
#[tauri::command]
async fn wrong_clear(state: tauri::State<'_, AppState>, bank_id: String, qid: String) -> Result<(), String> {
    let u = state.user.lock().map_err(|e| e.to_string())?;
    user::wrong_clear(&u, &bank_id, &qid)
}

#[tauri::command]
async fn fav_toggle(state: tauri::State<'_, AppState>, bank_id: String, qid: String) -> Result<bool, String> {
    let u = state.user.lock().map_err(|e| e.to_string())?;
    user::fav_toggle(&u, &bank_id, &qid)
}

#[tauri::command]
async fn note_get(state: tauri::State<'_, AppState>, bank_id: String, qid: String) -> Result<Option<String>, String> {
    let u = state.user.lock().map_err(|e| e.to_string())?;
    user::note_get(&u, &bank_id, &qid)
}

#[tauri::command]
async fn note_set(state: tauri::State<'_, AppState>, bank_id: String, qid: String, content: String) -> Result<(), String> {
    let u = state.user.lock().map_err(|e| e.to_string())?;
    user::note_set(&u, &bank_id, &qid, &content)
}

// ---------- M2 ----------
#[tauri::command]
async fn compose_blueprint(state: tauri::State<'_, AppState>, blueprint: m2::Blueprint) -> Result<m2::ComposeReport, String> {
    let b = state.bank.lock().map_err(|e| e.to_string())?;
    m2::compose(&b, &blueprint)
}

#[tauri::command]
async fn activity_calendar(state: tauri::State<'_, AppState>, days: Option<i64>) -> Result<Vec<m2::DayCount>, String> {
    let u = state.user.lock().map_err(|e| e.to_string())?;
    m2::activity(&u, days.unwrap_or(120))
}

#[tauri::command]
async fn export_session_excel(state: tauri::State<'_, AppState>, session_id: i64, path: String) -> Result<String, String> {
    let b = state.bank.lock().map_err(|e| e.to_string())?;
    let u = state.user.lock().map_err(|e| e.to_string())?;
    m2::export_session_excel(&u, &b, session_id, &path)
}

#[tauri::command]
async fn backup_user(state: tauri::State<'_, AppState>, dest: String) -> Result<String, String> {
    let u = state.user.lock().map_err(|e| e.to_string())?;
    m2::backup_user(&u, &dest)
}

#[tauri::command]
async fn restore_check(state: tauri::State<'_, AppState>, zip_path: String) -> Result<m2::RestoreInfo, String> {
    let _guard = state.user.lock().map_err(|e| e.to_string())?;
    m2::restore_check(&zip_path)
}

#[tauri::command]
async fn export_diagnostics(state: tauri::State<'_, AppState>, dest: String) -> Result<String, String> {
    let u = state.user.lock().map_err(|e| e.to_string())?;
    m2::diagnostics(&u, &dest)
}

#[tauri::command]
async fn setting_get(state: tauri::State<'_, AppState>, key: String) -> Result<Option<String>, String> {
    let u = state.user.lock().map_err(|e| e.to_string())?;
    m2::setting_get(&u, &key)
}

#[tauri::command]
async fn setting_set(state: tauri::State<'_, AppState>, key: String, value: String) -> Result<(), String> {
    let u = state.user.lock().map_err(|e| e.to_string())?;
    m2::setting_set(&u, &key, &value)
}

// ---------- M3：资料检索 ----------
#[tauri::command]
async fn docs_status(state: tauri::State<'_, AppState>) -> Result<refs::DocsStatus, String> {
    let b = state.bank.lock().map_err(|e| e.to_string())?;
    refs::ensure_schema(&b)?;
    Ok(refs::status(&b))
}

#[tauri::command]
async fn docs_build(state: tauri::State<'_, AppState>, app: tauri::AppHandle, force: Option<bool>) -> Result<usize, String> {
    // 索引构建为秒级阻塞任务：用独立连接在阻塞线程执行，不占用应用主连接
    let _ = state;
    let data_dir = app.path().app_data_dir().map_err(|e| e.to_string())?;
    let pack = {
        let candidates = [
            app.path().resource_dir().map_err(|e| e.to_string())?.join("docs/docs.docpack"),
            std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("resources/docs/docs.docpack"),
        ];
        candidates.into_iter().find(|p| p.exists()).ok_or("docs.docpack 未找到")?
    };
    let docs_dir = data_dir.join("docs");
    let force = force.unwrap_or(false);
    let conn2 = crate::db::open(&data_dir.join("bank.db")).map_err(|e| e.to_string())?;
    let n = tauri::async_runtime::spawn_blocking(move || refs::build_index(&conn2, &pack, &docs_dir, force))
        .await.map_err(|e| e.to_string())?.map_err(|e| e.to_string())?;
    Ok(n)
}

#[tauri::command]
async fn docs_search(state: tauri::State<'_, AppState>, query: String, limit: Option<i64>) -> Result<Vec<refs::DocHit>, String> {
    let b = state.bank.lock().map_err(|e| e.to_string())?;
    refs::search(&b, &query, limit.unwrap_or(20).min(50))
}

// ---------- M3：Excel导入 / 去重 / 打印 ----------
#[tauri::command]
async fn excel_preview(path: String) -> Result<m3::ExcelPreview, String> {
    m3::excel_preview(&path)
}

#[tauri::command]
async fn excel_import(state: tauri::State<'_, AppState>, path: String, bank_name: String) -> Result<m3::ExcelImportReport, String> {
    let b = state.bank.lock().map_err(|e| e.to_string())?;
    m3::excel_import_into(&b, &path, &bank_name)
}

#[tauri::command]
async fn export_excel_template(path: String) -> Result<String, String> {
    m3::export_template(&path)
}

#[tauri::command]
async fn dedup_scan(state: tauri::State<'_, AppState>, bank_id: String) -> Result<Vec<m3::DupGroup>, String> {
    let b = state.bank.lock().map_err(|e| e.to_string())?;
    m3::dedup_scan(&b, &bank_id)
}

#[tauri::command]
async fn dedup_merge(state: tauri::State<'_, AppState>, bank_id: String, keep: String, removes: Vec<String>) -> Result<usize, String> {
    let b = state.bank.lock().map_err(|e| e.to_string())?;
    let u = state.user.lock().map_err(|e| e.to_string())?;
    m3::dedup_merge(&b, &u, &bank_id, &keep, &removes)
}

#[tauri::command]
async fn paper_print_data(state: tauri::State<'_, AppState>, paper_id: i64) -> Result<m3::PrintPaper, String> {
    let b = state.bank.lock().map_err(|e| e.to_string())?;
    m3::paper_print_data(&b, paper_id)
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_updater::Builder::new().build())
        .plugin(tauri_plugin_process::init())
        .register_uri_scheme_protocol("bankasset", |ctx, request| {
            protocol::handle(ctx.app_handle(), request)
        })
        .setup(|app| {
            let data_dir: PathBuf = app.path().app_data_dir()?;
            std::fs::create_dir_all(&data_dir)?;
            let banks_root = data_dir.join("banks");
            let conn = db::open(&data_dir.join("bank.db"))?;
            let userconn = db::open_user(&data_dir.join("user.db"))?;

            // 导入内置种子题库（打包 resources 或开发目录）
            let seed_dirs = [
                app.path().resource_dir()?.join("seed"),
                PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("resources/seed"),
            ];
            for dir in seed_dirs {
                let Ok(rd) = std::fs::read_dir(&dir) else { continue };
                for ent in rd.flatten() {
                    let p = ent.path();
                    if p.extension().and_then(|e| e.to_str()) == Some("smartbank") {
                        match bank::import(&conn, &p, &banks_root, true) {
                            Ok(r) if !r.skipped => println!("[seed] 导入 {} v{}: {}题 {}卷 {}图",
                                r.bank_id, r.bank_name, r.questions, r.papers, r.images),
                            Ok(_) => {}
                            Err(e) => eprintln!("[seed] 导入失败 {p:?}: {e}"),
                        }
                    }
                }
            }
            app.manage(AppState { bank: Mutex::new(conn), user: Mutex::new(userconn) });
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            bank_overview, list_questions, get_questions_by_ids, list_papers, paper_questions, fts_spike,
            start_session, save_draft, finish_session, session_detail, unfinished_sessions, list_sessions,
            dashboard, due_review, wrong_list, wrong_clear, fav_toggle, fav_list, note_get, note_set,
            compose_blueprint, activity_calendar, export_session_excel,
            backup_user, restore_check, export_diagnostics, setting_get, setting_set,
            docs_status, docs_build, docs_search,
            excel_preview, excel_import, export_excel_template,
            dedup_scan, dedup_merge, paper_print_data
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
