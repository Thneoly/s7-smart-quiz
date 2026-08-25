// 资料语料域：docpack 解包 → 分块 → jieba → FTS5 全文检索（RAG 的检索层）
use rusqlite::{params, Connection};
use serde::Serialize;
use std::io::Read;
use std::path::Path;
use std::time::Instant;

#[derive(Serialize)]
pub struct DocHit { pub title: String, pub path: String, pub snippet: String }

#[derive(Serialize)]
pub struct DocsStatus { pub chunks: i64, pub built_at: String }

pub fn ensure_schema(conn: &Connection) -> Result<(), String> {
    conn.execute_batch(r#"
        CREATE TABLE IF NOT EXISTS docs_chunks (
          id INTEGER PRIMARY KEY, path TEXT, title TEXT, raw TEXT, seg TEXT);
        CREATE VIRTUAL TABLE IF NOT EXISTS docs_fts USING fts5(
          seg, content='docs_chunks', content_rowid='id');
        CREATE TABLE IF NOT EXISTS docs_meta (k TEXT PRIMARY KEY, v TEXT);
    "#).map_err(|e| e.to_string())?;
    Ok(())
}

pub fn status(conn: &Connection) -> DocsStatus {
    let chunks: i64 = conn.query_row("SELECT COUNT(*) FROM docs_chunks", [], |r| r.get(0)).unwrap_or(0);
    let built_at: String = conn.query_row("SELECT v FROM docs_meta WHERE k='built_at'", [], |r| r.get(0)).unwrap_or_default();
    DocsStatus { chunks, built_at }
}

/// 解包 docpack（zip）到 docs_dir 并建立索引；已有索引时跳过（force 重建）
pub fn build_index(conn: &Connection, docpack: &Path, docs_dir: &Path, force: bool) -> Result<usize, String> {
    ensure_schema(conn)?;
    if !force && status(conn).chunks > 0 {
        return Ok(status(conn).chunks as usize);
    }
    log::info!(target: "docs", "开始构建检索索引（force={force}）");
    let t0 = Instant::now();
    conn.execute_batch("DELETE FROM docs_fts; DELETE FROM docs_chunks;").map_err(|e| e.to_string())?;

    // 解包
    std::fs::create_dir_all(docs_dir).map_err(|e| e.to_string())?;
    let f = std::fs::File::open(docpack).map_err(|e| format!("docpack 缺失: {e}"))?;
    let mut zf = zip::ZipArchive::new(f).map_err(|e| e.to_string())?;
    let mut texts: Vec<(String, String)> = Vec::new(); // (path, content)
    for i in 0..zf.len() {
        let mut ent = zf.by_index(i).map_err(|e| e.to_string())?;
        if !ent.name().ends_with(".txt") { continue; }
        let mut s = String::new();
        ent.read_to_string(&mut s).map_err(|e| e.to_string())?;
        texts.push((ent.name().to_string(), s));
    }

    let jb = jieba_rs::Jieba::new();
    let tx = conn.unchecked_transaction().map_err(|e| e.to_string())?;
    let mut n = 0usize;
    for (path, content) in &texts {
        let title = content.lines().next().unwrap_or("").trim().trim_start_matches('#').to_string();
        let title = if title.is_empty() { path.rsplit('/').next().unwrap_or(path).to_string() } else { title };
        // 分块：约1200字符，段落边界
        let mut chunk = String::new();
        for para in content.split("\n\n") {
            chunk.push_str(para.trim());
            chunk.push('\n');
            if chunk.chars().count() >= 1200 {
                insert_chunk(&tx, &jb, path, &title, &chunk, &mut n)?;
                chunk.clear();
            }
        }
        if chunk.trim().chars().count() > 80 {
            insert_chunk(&tx, &jb, path, &title, &chunk, &mut n)?;
        }
    }
    tx.commit().map_err(|e| e.to_string())?;
    conn.execute("INSERT OR REPLACE INTO docs_meta(k,v) VALUES('built_at',?1)",
        params![chrono::Utc::now().to_rfc3339()]).map_err(|e| e.to_string())?;
    log::info!(target: "docs", "索引构建完成：{n} 块，{:.1}s", t0.elapsed().as_secs_f32());
    Ok(n)
}

fn insert_chunk(tx: &rusqlite::Transaction, jb: &jieba_rs::Jieba, path: &str, title: &str, raw: &str, n: &mut usize) -> Result<(), String> {
    let seg: String = jb.cut(raw, true).iter().map(|t| t.word).collect::<Vec<&str>>().join(" ");
    tx.execute("INSERT INTO docs_chunks(path,title,raw,seg) VALUES(?,?,?,?)", params![path, title, raw.trim(), seg])
        .map_err(|e| e.to_string())?;
    let id = tx.last_insert_rowid();
    tx.execute("INSERT INTO docs_fts(rowid, seg) VALUES(?,?)", params![id, seg]).map_err(|e| e.to_string())?;
    *n += 1;
    Ok(())
}

pub fn search(conn: &Connection, query: &str, limit: i64) -> Result<Vec<DocHit>, String> {
    ensure_schema(conn)?;
    let jb = jieba_rs::Jieba::new();
    let toks: Vec<String> = jb.cut(query, true).iter().map(|t| t.word)
        .filter(|s| !s.trim().is_empty()).map(|s| s.to_string()).collect();
    if toks.is_empty() { return Ok(vec![]); }
    let expr = toks.iter().map(|t| format!("\"{t}\"")).collect::<Vec<_>>().join(" AND ");
    let sql = format!(
        "SELECT c.title, c.path, c.raw FROM docs_fts f JOIN docs_chunks c ON c.id=f.rowid
         WHERE docs_fts MATCH ?1 ORDER BY rank LIMIT ?2");
    let mut stmt = conn.prepare(&sql).map_err(|e| e.to_string())?;
    let rows = stmt.query_map(params![expr, limit], |r| {
        Ok(DocHit { title: r.get(0)?, path: r.get(1)?, snippet: r.get(2)? })
    }).map_err(|e| e.to_string())?;
    let mut out = Vec::new();
    for h in rows {
        if let Ok(mut h) = h {
            // 片段：找第一个命中词位置截取窗口
            let raw = h.snippet.clone();
            let chars: Vec<char> = raw.chars().collect();
            let pos = toks.iter().filter_map(|t| raw.find(t.as_str())).min().unwrap_or(0);
            let cpos = raw[..pos].chars().count();
            let start = cpos.saturating_sub(30);
            let end = (start + 160).min(chars.len());
            h.snippet = chars[start..end].iter().collect::<String>();
            if start > 0 { h.snippet = format!("…{}", h.snippet); }
            if end < chars.len() { h.snippet.push('…'); }
            h.snippet = h.snippet.replace('\n', " ");
            out.push(h);
        }
    }
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn build_and_search() {
        let tmp = std::env::temp_dir().join(format!("sqdocs-{}", uuid::Uuid::new_v4()));
        std::fs::create_dir_all(&tmp).unwrap();
        let conn = crate::db::open(&tmp.join("bank.db")).unwrap();
        let pack = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("resources/docs/docs.docpack");
        if !pack.exists() { eprintln!("docpack 不存在，跳过"); return; }
        let n = build_index(&conn, &pack, &tmp.join("docs"), false).unwrap();
        assert!(n > 500, "块数异常: {n}");
        // 幂等
        let n2 = build_index(&conn, &pack, &tmp.join("docs"), false).unwrap();
        assert_eq!(n, n2);
        // 中文搜索
        let hits = search(&conn, "Modbus 地址", 5).unwrap();
        assert!(!hits.is_empty());
        assert!(hits.iter().any(|h| h.snippet.contains("Modbus") || h.title.contains("Modbus")));
        let hits2 = search(&conn, "模拟量 换算", 5).unwrap();
        assert!(!hits2.is_empty());
        let hits3 = search(&conn, "PID 回路", 5).unwrap();
        assert!(!hits3.is_empty());
        std::fs::remove_dir_all(&tmp).ok();
    }

    #[test]
    fn resolve_docpack_priority() {
        let tmp = std::env::temp_dir().join(format!("sqres-{}", uuid::Uuid::new_v4()));
        let user_dir = tmp.join("user");
        let res_dir = tmp.join("res");
        std::fs::create_dir_all(user_dir.join("docs")).unwrap();
        std::fs::create_dir_all(res_dir.join("docs")).unwrap();
        // 仅内置存在 → 取内置（打包布局：<resource>/resources/docs/）
        std::fs::create_dir_all(res_dir.join("resources/docs")).unwrap();
        std::fs::write(res_dir.join("resources/docs/docs.docpack"), b"r").unwrap();
        assert_eq!(resolve_docpack(&user_dir, Some(&res_dir)).unwrap(), res_dir.join("resources/docs/docs.docpack"));
        // 用户导入优先于内置
        std::fs::write(user_dir.join("docs/docs.docpack"), b"u").unwrap();
        assert_eq!(resolve_docpack(&user_dir, Some(&res_dir)).unwrap(), user_dir.join("docs/docs.docpack"));
        // resource_dir 缺失（None）时用户导入仍可命中
        assert_eq!(resolve_docpack(&user_dir, None).unwrap(), user_dir.join("docs/docs.docpack"));
        std::fs::remove_dir_all(&tmp).ok();
    }

    /// 学习页"手册原文"块的检索词表（与前端 src/study/chapter_links.ts 同步维护）。
    /// FTS 为 AND 语义：检索词必须精简，且每个词对真实语料非空命中——
    /// 此测试防止词表改动后章节原文块静默变空。
    #[test]
    fn study_doc_queries_hit() {
        let queries = [
            "扩展模块", "存储区", "接线", "编程软件", "定时器", "电机", "状态图表",
            "子程序", "中断", "PWM", "高速计数器", "PID", "Modbus", "CRC", "运动控制",
            "存储卡", "字符串", "PUT", "PROFINET", "自由口", "UDP", "TCP",
        ];
        let tmp = std::env::temp_dir().join(format!("sqhit-{}", uuid::Uuid::new_v4()));
        std::fs::create_dir_all(&tmp).unwrap();
        let conn = crate::db::open(&tmp.join("bank.db")).unwrap();
        let pack = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("resources/docs/docs.docpack");
        if !pack.exists() { eprintln!("docpack 不存在，跳过"); return; }
        build_index(&conn, &pack, &tmp.join("docs"), false).unwrap();
        let mut misses: Vec<&str> = Vec::new();
        for q in queries {
            if search(&conn, q, 3).unwrap().is_empty() { misses.push(q); }
        }
        assert!(misses.is_empty(), "以下检索词零命中（章节原文块会空）：{misses:?}");
        std::fs::remove_dir_all(&tmp).ok();
    }
}

/// 解析语料包位置：用户导入（app_data_dir/docs）优先于安装包内置资源，最后回退开发目录。
/// 公开仓克隆者无内置资源，靠第一级或最后一级。
pub fn resolve_docpack(data_dir: &Path, resource_dir: Option<&Path>) -> Result<std::path::PathBuf, String> {
    let mut candidates = vec![data_dir.join("docs/docs.docpack")];
    if let Some(r) = resource_dir { candidates.push(r.join("resources/docs/docs.docpack")); }
    candidates.push(std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("resources/docs/docs.docpack"));
    candidates.into_iter().find(|p| p.exists())
        .ok_or_else(|| "docs.docpack 未找到——请先在「资料速查」导入语料包，或将其放入 resources/docs/".into())
}

/// 构建索引（阻塞，秒级）：开独立连接执行，避免占用应用主连接
pub fn build_docs(data_dir: &Path, resource_dir: Option<&Path>, force: bool) -> Result<usize, String> {
    let pack = resolve_docpack(data_dir, resource_dir)?;
    let conn = crate::db::open(&data_dir.join("bank.db")).map_err(|e| e.to_string())?;
    build_index(&conn, &pack, &data_dir.join("docs"), force)
}

// ---------- Tauri 命令 ----------
pub mod commands {
    use super::{search as search_impl, status as status_impl, ensure_schema, build_docs, DocHit, DocsStatus};
    use crate::telemetry::timed;
    use crate::AppState;
    use tauri::Manager;

    #[tauri::command]
    pub async fn docs_status(state: tauri::State<'_, AppState>) -> Result<DocsStatus, String> {
        timed("docs_status", false, || {
            let b = state.bank.lock().map_err(|e| e.to_string())?;
            ensure_schema(&b)?;
            Ok(status_impl(&b))
        })
    }

    #[tauri::command]
    pub async fn docs_build(app: tauri::AppHandle, force: Option<bool>) -> Result<usize, String> {
        // 索引构建为秒级阻塞任务：spawn_blocking 走领域函数 build_docs（内部开独立连接）
        let data_dir = app.path().app_data_dir().map_err(|e| e.to_string())?;
        let resource_dir = app.path().resource_dir().ok();
        let f = force.unwrap_or(false);
        crate::telemetry::timed_async("docs_build", false, async move {
            tauri::async_runtime::spawn_blocking(move || build_docs(&data_dir, resource_dir.as_deref(), f))
                .await.map_err(|e| e.to_string())?
        }).await
    }

    #[tauri::command]
    pub async fn docs_search(state: tauri::State<'_, AppState>, query: String, limit: Option<i64>) -> Result<Vec<DocHit>, String> {
        // 只记长度不记内容：搜索词属用户输入
        timed("docs_search", false, || {
            let b = state.bank.lock().map_err(|e| e.to_string())?;
            log::debug!(target: "docs", "搜索词长度 {} 命中上限 {}", query.chars().count(), limit.unwrap_or(20));
            search_impl(&b, &query, limit.unwrap_or(20).min(50))
        })
    }

    #[tauri::command]
    pub async fn import_docpack(app: tauri::AppHandle, path: String) -> Result<u64, String> {
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
}
