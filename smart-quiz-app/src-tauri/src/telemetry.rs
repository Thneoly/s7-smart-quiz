// 遥测与可维护性：滚动日志装配、命令打点、panic 捕获、日志查看
use serde::Serialize;
use tauri::Manager;

// ---------- 命令打点 ----------
fn log_cmd_result<T>(cmd: &str, quiet: bool, ms: u128, r: &Result<T, String>) {
    match r {
        Ok(_) => log::log!(target: "cmd", if quiet { log::Level::Trace } else { log::Level::Debug },
            "{cmd} ok {ms}ms"),
        Err(e) => log::error!(target: "cmd", "{cmd} 失败({ms}ms): {e}"),
    }
}

/// 命令统一打点：成功按 debug 记录耗时（quiet 命令降为 trace，避免高频刷屏），
/// 失败一律 error 记录错误信息——用户报问题时日志可还原完整操作序列。
/// 注意：绝不记录参数内容（草稿/笔记/题干均含用户数据），只记命令名/耗时/错误串。
pub fn timed<T>(cmd: &'static str, quiet: bool, f: impl FnOnce() -> Result<T, String>) -> Result<T, String> {
    let t0 = std::time::Instant::now();
    let r = f();
    log_cmd_result(cmd, quiet, t0.elapsed().as_millis(), &r);
    r
}

/// 异步命令打点（如 docs_build 的 spawn_blocking 装配），语义与 timed 一致
pub async fn timed_async<T>(cmd: &'static str, quiet: bool, f: impl std::future::Future<Output = Result<T, String>>) -> Result<T, String> {
    let t0 = std::time::Instant::now();
    let r = f.await;
    log_cmd_result(cmd, quiet, t0.elapsed().as_millis(), &r);
    r
}

/// 路径脱敏：隐藏 Windows 账户名（C:\Users\<账户>\… → C:\Users\~\…）。
/// 日志会随诊断包外发，账户名属个人数据。
pub fn redact_path(p: &std::path::Path) -> String {
    let s = p.to_string_lossy();
    if let Some(i) = s.find("\\Users\\") {
        let rest = &s[i + "\\Users\\".len()..];
        match rest.find('\\') {
            Some(j) => format!("{}\\Users\\~{}", &s[..i], &rest[j..]),
            None => format!("{}\\Users\\~", &s[..i]),
        }
    } else { s.into_owned() }
}

/// panic 钩子：带 backtrace 落日志，release 下也能定位崩溃点（写日志后交回默认钩子走原有流程）
pub fn install_panic_hook() {
    let default = std::panic::take_hook();
    std::panic::set_hook(Box::new(move |info| {
        let bt = std::backtrace::Backtrace::force_capture();
        log::error!(target: "panic", "线程恐慌: {info}\n{bt}");
        default(info);
    }));
}

/// 滚动日志手动装配（split）：文件按次轮转（每次启动新文件，保留最近3次），单文件超 2MB 也轮转；
/// 级别 Debug（trace 级的高频命令成功日志默认不落盘）；本地时区。
/// 关键：日志文件初始化失败（如归档日志被 Excel/WPS 独占锁定导致清理失败）时
/// 降级为仅控制台输出，绝不因日志子系统故障拖垮应用启动。
pub fn assemble_logging(app: &tauri::AppHandle) {
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
    match mk(true).split(app) {
        Ok((plugin, max_level, logger)) => {
            let _ = app.plugin(plugin);
            let _ = tauri_plugin_log::attach_logger(max_level, logger);
        }
        Err(e) => {
            eprintln!("[log] 文件日志初始化失败（{e}），降级为仅控制台输出");
            if let Ok((plugin, max_level, logger)) = mk(false).split(app) {
                let _ = app.plugin(plugin);
                let _ = tauri_plugin_log::attach_logger(max_level, logger);
            }
        }
    }
}

// ---------- 日志查看 ----------
#[derive(Serialize)]
pub struct LogView {
    pub path: Option<String>,
    pub lines: Vec<String>,
}

/// 读最近一次启动的日志（目录内 mtime 最新的 .log），返回末尾 tail 行。
/// 文件≤2MB 直接整读后截尾，无性能问题。
pub fn read_latest_log(dir: Option<&std::path::Path>, tail: usize) -> LogView {
    let none = LogView { path: None, lines: Vec::new() };
    let Some(dir) = dir else { return none };
    let Some(file) = std::fs::read_dir(dir).ok().and_then(|rd| {
        rd.flatten().map(|e| e.path())
            .filter(|p| p.extension().and_then(|x| x.to_str()) == Some("log"))
            .max_by_key(|p| p.metadata().and_then(|m| m.modified()).unwrap_or(std::time::UNIX_EPOCH))
    }) else { return none };
    let content = std::fs::read_to_string(&file).unwrap_or_default();
    let all: Vec<&str> = content.lines().collect();
    let lines: Vec<String> = all.iter().rev().take(tail).rev().map(|s| s.to_string()).collect();
    LogView { path: Some(file.to_string_lossy().into_owned()), lines }
}

// ---------- Tauri 命令 ----------
pub mod commands {
    use super::*;

    #[tauri::command]
    pub async fn logs_read(app: tauri::AppHandle, tail: Option<usize>) -> Result<LogView, String> {
        timed("logs_read", true, || {
            let dir = app.path().app_log_dir().ok();
            Ok(read_latest_log(dir.as_deref(), tail.unwrap_or(200).min(1000)))
        })
    }

    #[tauri::command]
    pub async fn open_log_dir(app: tauri::AppHandle) -> Result<(), String> {
        timed("open_log_dir", false, || {
            let dir = app.path().app_log_dir().map_err(|e| e.to_string())?;
            std::fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
            tauri_plugin_opener::open_path(&dir, None::<&str>).map_err(|e| e.to_string())
        })
    }
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
