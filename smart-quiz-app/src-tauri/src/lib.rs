// S7-200 SMART 题库平台 —— 组合根：模块装配、插件、启动流程
// 各领域模块自带 commands 子模块（#[tauri::command]），领域实现不依赖 Tauri
mod backup;
mod bank;
mod compose;
mod db;
mod dedup;
mod docs;
mod excel;
mod print;
mod protocol;
mod session;
mod telemetry;
#[cfg(test)]
mod tests;

use rusqlite::Connection;
use std::path::PathBuf;
use std::sync::Mutex;
use tauri::Manager;

/// 全局共享状态：bank.db（题库）与 user.db（用户数据）各一个连接，
/// 命令内短临界区加锁（连接非线程安全，锁是唯一出入口）
pub struct AppState {
    pub bank: Mutex<Connection>,
    pub user: Mutex<Connection>,
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    telemetry::install_panic_hook();
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_updater::Builder::new().build())
        .plugin(tauri_plugin_process::init())
        .register_uri_scheme_protocol("bankasset", |ctx, request| {
            protocol::handle(ctx.app_handle(), request)
        })
        .setup(|app| {
            telemetry::assemble_logging(app.handle());

            let data_dir: PathBuf = app.path().app_data_dir()?;
            std::fs::create_dir_all(&data_dir)?;
            log::info!(target: "app", "启动 smart-quiz-app v{}（数据目录 {}）",
                env!("CARGO_PKG_VERSION"), telemetry::redact_path(&data_dir));
            let banks_root = data_dir.join("banks");
            let conn = db::open(&data_dir.join("bank.db"))?;
            let userconn = db::open_user(&data_dir.join("user.db"))?;

            // 导入内置种子题库（打包 resources 或开发目录；公开仓克隆者无种子，引导应用内导入）
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
            bank::commands::bank_overview, bank::commands::list_questions, bank::commands::get_questions_by_ids,
            bank::commands::list_papers, bank::commands::paper_questions, bank::commands::fts_spike,
            bank::commands::import_bank_file,
            session::commands::start_session, session::commands::save_draft, session::commands::finish_session,
            session::commands::session_detail, session::commands::unfinished_sessions, session::commands::list_sessions,
            session::commands::dashboard, session::commands::due_review,
            session::commands::wrong_list, session::commands::wrong_clear,
            session::commands::fav_toggle, session::commands::fav_list,
            session::commands::note_get, session::commands::note_set, session::commands::activity_calendar,
            compose::commands::compose_blueprint,
            excel::commands::excel_preview, excel::commands::excel_import,
            excel::commands::export_excel_template, excel::commands::export_session_excel,
            dedup::commands::dedup_scan, dedup::commands::dedup_merge,
            print::commands::paper_print_data,
            backup::commands::backup_user, backup::commands::restore_check, backup::commands::export_diagnostics,
            backup::commands::setting_get, backup::commands::setting_set,
            docs::commands::docs_status, docs::commands::docs_build, docs::commands::docs_search,
            docs::commands::import_docpack,
            telemetry::commands::logs_read, telemetry::commands::open_log_dir
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
