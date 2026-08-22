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

// ---------- 可维护性：命令打点 ----------
/// 命令统一打点：成功按 debug 记录耗时（quiet 命令降为 trace，避免高频刷屏），
/// 失败一律 error 记录错误信息——用户报问题时日志可还原完整操作序列。
/// 注意：绝不记录参数内容（草稿/笔记/题干均含用户数据），只记命令名/耗时/错误串。
fn timed<T>(cmd: &'static str, quiet: bool, f: impl FnOnce() -> Result<T, String>) -> Result<T, String> {
    let t0 = std::time::Instant::now();
    let r = f();
    let ms = t0.elapsed().as_millis();
    match &r {
        Ok(_) => log::log!(target: "cmd", if quiet { log::Level::Trace } else { log::Level::Debug },
            "{cmd} ok {ms}ms"),
        Err(e) => log::error!(target: "cmd", "{cmd} 失败({ms}ms): {e}"),
    }
    r
}

/// panic 钩子：带 backtrace 落日志，release 下也能定位崩溃点（写日志后交回默认钩子走原有流程）
fn install_panic_hook() {
    let default = std::panic::take_hook();
    std::panic::set_hook(Box::new(move |info| {
        let bt = std::backtrace::Backtrace::force_capture();
        log::error!(target: "panic", "线程恐慌: {info}\n{bt}");
        default(info);
    }));
}

/// 路径脱敏：隐藏 Windows 账户名（C:\Users\<账户>\… → C:\Users\~\…）。
/// 日志会随诊断包外发，账户名属个人数据。
fn redact_path(p: &std::path::Path) -> String {
    let s = p.to_string_lossy();
    if let Some(i) = s.find("\\Users\\") {
        let rest = &s[i + "\\Users\\".len()..];
        match rest.find('\\') {
            Some(j) => format!("{}\\Users\\~{}", &s[..i], &rest[j..]),
            None => format!("{}\\Users\\~", &s[..i]),
        }
    } else { s.into_owned() }
}

// ---------- 题库查询 ----------
#[tauri::command]
async fn bank_overview(state: tauri::State<'_, AppState>) -> Result<bank::Overview, String> {
    timed("bank_overview", false, || {
        let b = state.bank.lock().map_err(|e| e.to_string())?;
        bank::overview(&b)
    })
}

#[tauri::command]
async fn list_papers(state: tauri::State<'_, AppState>) -> Result<Vec<bank::PaperInfo>, String> {
    timed("list_papers", false, || {
        let b = state.bank.lock().map_err(|e| e.to_string())?;
        bank::list_papers(&b)
    })
}

#[tauri::command]
async fn paper_questions(state: tauri::State<'_, AppState>, paper_id: i64) -> Result<Vec<(String, String)>, String> {
    timed("paper_questions", false, || {
        let b = state.bank.lock().map_err(|e| e.to_string())?;
        bank::paper_qids(&b, paper_id)
    })
}

#[tauri::command]
async fn dashboard(state: tauri::State<'_, AppState>) -> Result<user::Dashboard, String> {
    timed("dashboard", false, || {
        let b = state.bank.lock().map_err(|e| e.to_string())?;
        let u = state.user.lock().map_err(|e| e.to_string())?;
        user::dashboard(&u, &b)
    })
}

#[tauri::command]
async fn wrong_list(state: tauri::State<'_, AppState>) -> Result<Vec<user::WrongRow>, String> {
    timed("wrong_list", false, || {
        let b = state.bank.lock().map_err(|e| e.to_string())?;
        let u = state.user.lock().map_err(|e| e.to_string())?;
        user::wrong_list(&u, &b, true)
    })
}

#[tauri::command]
async fn fav_list(state: tauri::State<'_, AppState>) -> Result<Vec<bank::QuestionRow>, String> {
    timed("fav_list", false, || {
        let b = state.bank.lock().map_err(|e| e.to_string())?;
        let u = state.user.lock().map_err(|e| e.to_string())?;
        user::fav_list(&u, &b)
    })
}

#[tauri::command]
async fn unfinished_sessions(state: tauri::State<'_, AppState>) -> Result<Vec<user::SessionInfo>, String> {
    timed("unfinished_sessions", false, || {
        let u = state.user.lock().map_err(|e| e.to_string())?;
        user::unfinished_sessions(&u)
    })
}

#[tauri::command]
async fn list_sessions(state: tauri::State<'_, AppState>) -> Result<Vec<user::SessionBrief>, String> {
    timed("list_sessions", false, || {
        let u = state.user.lock().map_err(|e| e.to_string())?;
        user::list_sessions(&u, 30)
    })
}

#[tauri::command]
async fn list_questions(state: tauri::State<'_, AppState>,
    topic_id: Option<i64>, qtype: Option<String>, status: Option<String>,
    search: Option<String>, limit: Option<i64>, offset: Option<i64>) -> Result<Vec<bank::QuestionRow>, String> {
    timed("list_questions", false, || {
        let conn = state.bank.lock().map_err(|e| e.to_string())?;
        bank::list_questions(&conn, topic_id, qtype, status, search,
            limit.unwrap_or(50).min(500), offset.unwrap_or(0))
    })
}

#[tauri::command]
async fn get_questions_by_ids(state: tauri::State<'_, AppState>, qids: Vec<(String, String)>)
    -> Result<Vec<bank::QuestionRow>, String> {
    timed("get_questions_by_ids", false, || {
        let conn = state.bank.lock().map_err(|e| e.to_string())?;
        bank::get_questions_by_ids(&conn, &qids)
    })
}

#[tauri::command]
async fn due_review(state: tauri::State<'_, AppState>, limit: Option<i64>) -> Result<Vec<bank::QuestionRow>, String> {
    timed("due_review", false, || {
        let b = state.bank.lock().map_err(|e| e.to_string())?;
        let u = state.user.lock().map_err(|e| e.to_string())?;
        user::due_review(&u, &b, limit.unwrap_or(20))
    })
}

#[tauri::command]
async fn fts_spike(state: tauri::State<'_, AppState>, scale_to: Option<usize>) -> Result<bank::SpikeResult, String> {
    timed("fts_spike", false, || {
        let conn = state.bank.lock().map_err(|e| e.to_string())?;
        bank::fts_spike(&conn, scale_to.unwrap_or(100_000).min(200_000),
            &["以太网通信", "高速计数器", "Modbus", "模拟量输入"])
    })
}

// ---------- 会话 ----------
#[tauri::command]
async fn start_session(state: tauri::State<'_, AppState>, mode: String, title: String, bank_id: String,
                       paper_id: Option<i64>, qids: Vec<(String, String)>, time_limit_sec: Option<i64>)
    -> Result<user::SessionInfo, String> {
    timed("start_session", false, || {
        let u = state.user.lock().map_err(|e| e.to_string())?;
        user::start_session(&u, &mode, &title, &bank_id, paper_id, &qids, time_limit_sec)
    })
}

#[tauri::command]
async fn save_draft(state: tauri::State<'_, AppState>, session_id: i64, draft: serde_json::Value) -> Result<(), String> {
    // 做题页每 1.5s 自动保存一次：成功降为 trace，避免刷屏
    timed("save_draft", true, || {
        let u = state.user.lock().map_err(|e| e.to_string())?;
        let d: user::Draft = serde_json::from_value(draft).map_err(|e| e.to_string())?;
        user::save_draft(&u, session_id, &d)
    })
}

#[tauri::command]
async fn finish_session(state: tauri::State<'_, AppState>, session_id: i64) -> Result<user::SessionInfo, String> {
    timed("finish_session", false, || {
        let b = state.bank.lock().map_err(|e| e.to_string())?;
        let u = state.user.lock().map_err(|e| e.to_string())?;
        user::finish_session(&u, &b, session_id)
    })
}

#[tauri::command]
async fn session_detail(state: tauri::State<'_, AppState>, session_id: i64) -> Result<user::SessionDetail, String> {
    timed("session_detail", false, || {
        let b = state.bank.lock().map_err(|e| e.to_string())?;
        let u = state.user.lock().map_err(|e| e.to_string())?;
        user::session_detail(&u, &b, session_id)
    })
}

// ---------- 错题/收藏/笔记 ----------
#[tauri::command]
async fn wrong_clear(state: tauri::State<'_, AppState>, bank_id: String, qid: String) -> Result<(), String> {
    timed("wrong_clear", false, || {
        let u = state.user.lock().map_err(|e| e.to_string())?;
        user::wrong_clear(&u, &bank_id, &qid)
    })
}

#[tauri::command]
async fn fav_toggle(state: tauri::State<'_, AppState>, bank_id: String, qid: String) -> Result<bool, String> {
    timed("fav_toggle", true, || {
        let u = state.user.lock().map_err(|e| e.to_string())?;
        user::fav_toggle(&u, &bank_id, &qid)
    })
}

#[tauri::command]
async fn note_get(state: tauri::State<'_, AppState>, bank_id: String, qid: String) -> Result<Option<String>, String> {
    // 每道题进入视图都会取一次笔记：成功降为 trace
    timed("note_get", true, || {
        let u = state.user.lock().map_err(|e| e.to_string())?;
        user::note_get(&u, &bank_id, &qid)
    })
}

#[tauri::command]
async fn note_set(state: tauri::State<'_, AppState>, bank_id: String, qid: String, content: String) -> Result<(), String> {
    timed("note_set", true, || {
        let u = state.user.lock().map_err(|e| e.to_string())?;
        user::note_set(&u, &bank_id, &qid, &content)
    })
}

// ---------- M2 ----------
#[tauri::command]
async fn compose_blueprint(state: tauri::State<'_, AppState>, blueprint: m2::Blueprint) -> Result<m2::ComposeReport, String> {
    timed("compose_blueprint", false, || {
        let b = state.bank.lock().map_err(|e| e.to_string())?;
        m2::compose(&b, &blueprint)
    })
}

#[tauri::command]
async fn activity_calendar(state: tauri::State<'_, AppState>, days: Option<i64>) -> Result<Vec<m2::DayCount>, String> {
    timed("activity_calendar", false, || {
        let u = state.user.lock().map_err(|e| e.to_string())?;
        m2::activity(&u, days.unwrap_or(120))
    })
}

#[tauri::command]
async fn export_session_excel(state: tauri::State<'_, AppState>, session_id: i64, path: String) -> Result<String, String> {
    timed("export_session_excel", false, || {
        let b = state.bank.lock().map_err(|e| e.to_string())?;
        let u = state.user.lock().map_err(|e| e.to_string())?;
        m2::export_session_excel(&u, &b, session_id, &path)
    })
}

#[tauri::command]
async fn backup_user(state: tauri::State<'_, AppState>, dest: String) -> Result<String, String> {
    timed("backup_user", false, || {
        let u = state.user.lock().map_err(|e| e.to_string())?;
        m2::backup_user(&u, &dest)
    })
}

#[tauri::command]
async fn restore_check(state: tauri::State<'_, AppState>, zip_path: String) -> Result<m2::RestoreInfo, String> {
    timed("restore_check", false, || {
        let _guard = state.user.lock().map_err(|e| e.to_string())?; // 校验期间持锁，防备份/恢复竞争
        m2::restore_check(&zip_path)
    })
}

#[tauri::command]
async fn export_diagnostics(state: tauri::State<'_, AppState>, app: tauri::AppHandle, dest: String) -> Result<String, String> {
    timed("export_diagnostics", false, || {
        let u = state.user.lock().map_err(|e| e.to_string())?;
        let log_dir = app.path().app_log_dir().ok();
        m2::diagnostics(&u, log_dir.as_deref(), &dest)
    })
}

#[tauri::command]
async fn setting_get(state: tauri::State<'_, AppState>, key: String) -> Result<Option<String>, String> {
    timed("setting_get", true, || {
        let u = state.user.lock().map_err(|e| e.to_string())?;
        m2::setting_get(&u, &key)
    })
}

#[tauri::command]
async fn setting_set(state: tauri::State<'_, AppState>, key: String, value: String) -> Result<(), String> {
    timed("setting_set", true, || {
        let u = state.user.lock().map_err(|e| e.to_string())?;
        m2::setting_set(&u, &key, &value)
    })
}

// ---------- M3：资料检索 ----------
#[tauri::command]
async fn docs_status(state: tauri::State<'_, AppState>) -> Result<refs::DocsStatus, String> {
    timed("docs_status", false, || {
        let b = state.bank.lock().map_err(|e| e.to_string())?;
        refs::ensure_schema(&b)?;
        Ok(refs::status(&b))
    })
}

#[tauri::command]
async fn docs_build(state: tauri::State<'_, AppState>, app: tauri::AppHandle, force: Option<bool>) -> Result<usize, String> {
    // 索引构建为秒级阻塞任务：用独立连接在阻塞线程执行，不占用应用主连接
    let _ = state;
    let t0 = std::time::Instant::now();
    let r = (|| {
        let data_dir = app.path().app_data_dir().map_err(|e| e.to_string())?;
        let pack = {
            let candidates = [
                // 用户应用内导入的数据包优先于内置资源（公开仓构建无内置）
                data_dir.join("docs/docs.docpack"),
                app.path().resource_dir().map_err(|e| e.to_string())?.join("docs/docs.docpack"),
                std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("resources/docs/docs.docpack"),
            ];
            candidates.into_iter().find(|p| p.exists())
                .ok_or("docs.docpack 未找到——请先在「资料速查」导入语料包，或将其放入 resources/docs/")?
        };
        let docs_dir = data_dir.join("docs");
        let force = force.unwrap_or(false);
        let conn2 = crate::db::open(&data_dir.join("bank.db")).map_err(|e| e.to_string())?;
        Ok((conn2, pack, docs_dir, force))
    })();
    let (conn2, pack, docs_dir, force) = match r {
        Ok(v) => v,
        Err(e) => {
            log::error!(target: "docs", "docs_build 失败: {e}");
            return Err(e);
        }
    };
    let n = tauri::async_runtime::spawn_blocking(move || refs::build_index(&conn2, &pack, &docs_dir, force))
        .await.map_err(|e| e.to_string())?.map_err(|e| e.to_string())?;
    log::info!(target: "docs", "docs_build 完成：{n} 块，{}ms", t0.elapsed().as_millis());
    Ok(n)
}

#[tauri::command]
async fn docs_search(state: tauri::State<'_, AppState>, query: String, limit: Option<i64>) -> Result<Vec<refs::DocHit>, String> {
    // 只记长度不记内容：搜索词属用户输入
    timed("docs_search", false, || {
        let b = state.bank.lock().map_err(|e| e.to_string())?;
        log::debug!(target: "docs", "搜索词长度 {} 命中上限 {}", query.chars().count(), limit.unwrap_or(20));
        refs::search(&b, &query, limit.unwrap_or(20).min(50))
    })
}

// ---------- M3：Excel导入 / 去重 / 打印 ----------
#[tauri::command]
async fn excel_preview(path: String) -> Result<m3::ExcelPreview, String> {
    timed("excel_preview", false, || m3::excel_preview(&path))
}

#[tauri::command]
async fn excel_import(state: tauri::State<'_, AppState>, path: String, bank_name: String) -> Result<m3::ExcelImportReport, String> {
    timed("excel_import", false, || {
        let b = state.bank.lock().map_err(|e| e.to_string())?;
        m3::excel_import_into(&b, &path, &bank_name)
    })
}

#[tauri::command]
async fn export_excel_template(path: String) -> Result<String, String> {
    timed("export_excel_template", false, || m3::export_template(&path))
}

#[tauri::command]
async fn dedup_scan(state: tauri::State<'_, AppState>, bank_id: String) -> Result<Vec<m3::DupGroup>, String> {
    timed("dedup_scan", false, || {
        let b = state.bank.lock().map_err(|e| e.to_string())?;
        m3::dedup_scan(&b, &bank_id)
    })
}

#[tauri::command]
async fn dedup_merge(state: tauri::State<'_, AppState>, bank_id: String, keep: String, removes: Vec<String>) -> Result<usize, String> {
    timed("dedup_merge", false, || {
        let b = state.bank.lock().map_err(|e| e.to_string())?;
        let u = state.user.lock().map_err(|e| e.to_string())?;
        m3::dedup_merge(&b, &u, &bank_id, &keep, &removes)
    })
}

#[tauri::command]
async fn paper_print_data(state: tauri::State<'_, AppState>, paper_id: i64) -> Result<m3::PrintPaper, String> {
    timed("paper_print_data", false, || {
        let b = state.bank.lock().map_err(|e| e.to_string())?;
        m3::paper_print_data(&b, paper_id)
    })
}

// ---------- M3：数据包导入（公开仓不含版权数据，用户自备数据包） ----------
#[tauri::command]
async fn import_bank_file(state: tauri::State<'_, AppState>, app: tauri::AppHandle, path: String)
    -> Result<bank::ImportReport, String> {
    timed("import_bank_file", false, || {
        if !path.to_lowercase().ends_with(".smartbank") {
            return Err("仅支持 .smartbank 题库包".into());
        }
        let b = state.bank.lock().map_err(|e| e.to_string())?;
        let banks_root = app.path().app_data_dir().map_err(|e| e.to_string())?.join("banks");
        std::fs::create_dir_all(&banks_root).map_err(|e| e.to_string())?;
        bank::import(&b, std::path::Path::new(&path), &banks_root, false)
    })
}

#[tauri::command]
async fn import_docpack(app: tauri::AppHandle, path: String) -> Result<u64, String> {
    timed("import_docpack", false, || {
        if !path.to_lowercase().ends_with(".docpack") {
            return Err("仅支持 .docpack 语料包".into());
        }
        let dir = app.path().app_data_dir().map_err(|e| e.to_string())?.join("docs");
        std::fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
        let n = std::fs::copy(&path, dir.join("docs.docpack")).map_err(|e| e.to_string())?;
        log::info!(target: "docs", "语料包已导入（{n} 字节），建议重建索引");
        Ok(n)
    })
}

// ---------- 可维护性：日志查看 ----------
#[tauri::command]
async fn logs_read(app: tauri::AppHandle, tail: Option<usize>) -> Result<m2::LogView, String> {
    timed("logs_read", true, || {
        let dir = app.path().app_log_dir().ok();
        Ok(m2::logs_read(dir.as_deref(), tail.unwrap_or(200).min(1000)))
    })
}

#[tauri::command]
async fn open_log_dir(app: tauri::AppHandle) -> Result<(), String> {
    timed("open_log_dir", false, || {
        let dir = app.path().app_log_dir().map_err(|e| e.to_string())?;
        std::fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
        tauri_plugin_opener::open_path(&dir, None::<&str>).map_err(|e| e.to_string())
    })
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    install_panic_hook();
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_updater::Builder::new().build())
        .plugin(tauri_plugin_process::init())
        .register_uri_scheme_protocol("bankasset", |ctx, request| {
            protocol::handle(ctx.app_handle(), request)
        })
        .setup(|app| {
            // 滚动日志手动装配（split）：文件按次轮转（每次启动新文件，保留最近3次），单文件超 2MB 也轮转；
            // 级别 Debug（trace 级的高频命令成功日志默认不落盘）；本地时区。
            // 关键：日志文件初始化失败（如归档日志被 Excel/WPS 独占锁定导致清理失败）时
            // 降级为仅控制台输出，绝不因日志子系统故障拖垮应用启动。
            let mk = |file: bool| {
                let mut b = tauri_plugin_log::Builder::new()
                    // 注意：Builder 默认自带 [Stdout, LogDir] 两个 target，且 .target() 是追加语义，
                    // 不先 clear 会叠加导致每条日志重复写入
                    .clear_targets()
                    .level(log::LevelFilter::Debug)
                    .max_file_size(2 * 1024 * 1024)
                    .rotation_strategy(tauri_plugin_log::RotationStrategy::KeepSome(3))
                    .file_open_strategy(tauri_plugin_log::FileOpenStrategy::Rotate)
                    .timezone_strategy(tauri_plugin_log::TimezoneStrategy::UseLocal)
                    .target(tauri_plugin_log::Target::new(tauri_plugin_log::TargetKind::Stdout));
                if file {
                    b = b.target(tauri_plugin_log::Target::new(
                        tauri_plugin_log::TargetKind::LogDir { file_name: None }));
                }
                b
            };
            match mk(true).split(app.handle()) {
                Ok((plugin, max_level, logger)) => {
                    let _ = app.handle().plugin(plugin);
                    let _ = tauri_plugin_log::attach_logger(max_level, logger);
                }
                Err(e) => {
                    eprintln!("[log] 文件日志初始化失败（{e}），降级为仅控制台输出");
                    if let Ok((plugin, max_level, logger)) = mk(false).split(app.handle()) {
                        let _ = app.handle().plugin(plugin);
                        let _ = tauri_plugin_log::attach_logger(max_level, logger);
                    }
                }
            }

            let data_dir: PathBuf = app.path().app_data_dir()?;
            std::fs::create_dir_all(&data_dir)?;
            log::info!(target: "app", "启动 smart-quiz-app v{}（数据目录 {}）",
                env!("CARGO_PKG_VERSION"), redact_path(&data_dir));
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
                            Ok(r) if !r.skipped => log::info!(target: "seed", "导入 {} v{}：{}题 {}卷 {}图",
                                r.bank_id, r.bank_name, r.questions, r.papers, r.images),
                            Ok(_) => {}
                            Err(e) => log::error!(target: "seed", "导入失败 {}: {e}", p.display()),
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
            dedup_scan, dedup_merge, paper_print_data,
            logs_read, open_log_dir,
            import_bank_file, import_docpack
        ])
        .build(tauri::generate_context!())
        .expect("error while running tauri application")
        .run(|_handle, event| {
            // 正常退出落日志：与崩溃/被杀区分（崩溃无此行），并强制刷盘防末行丢失
            if let tauri::RunEvent::Exit = event {
                log::info!(target: "app", "应用正常退出");
                log::logger().flush();
            }
        });
}

#[cfg(test)]
mod tests {
    #[test]
    fn redact_path_hides_account() {
        assert_eq!(super::redact_path(std::path::Path::new(
                r"C:\Users\张三\AppData\Roaming\com.smartquiz.app")),
            r"C:\Users\~\AppData\Roaming\com.smartquiz.app");
        assert_eq!(super::redact_path(std::path::Path::new(r"C:\Users\public")), r"C:\Users\~");
        assert_eq!(super::redact_path(std::path::Path::new(r"D:\PLC\data")), r"D:\PLC\data");
    }
}
