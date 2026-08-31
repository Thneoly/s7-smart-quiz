// .smartbank 导入、题库查询、FTS 中文分词 spike（M0）
use rusqlite::{params, Connection};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::io::Read;
use std::path::Path;
use std::time::Instant;

// ---------- .smartbank manifest 结构 ----------
#[derive(Deserialize)]
pub struct Manifest {
    pub format: String,
    pub schema_ver: i64,
    pub bank: BankMeta,
    #[serde(default)]
    pub topics: Vec<TopicIn>,
    #[serde(default)]
    pub questions: Vec<QuestionIn>,
    #[serde(default)]
    pub papers: Vec<PaperIn>,
    #[serde(default)]
    #[allow(dead_code)] // 预留字段（决策方案Ⅰ：不防盗版，后续可选启用）
    pub license: Option<serde_json::Value>,
}

#[derive(Deserialize)]
pub struct BankMeta {
    pub bank_id: String,
    pub name: String,
    pub version: i64,
    #[serde(default)]
    pub description: String,
}

#[derive(Deserialize)]
pub struct TopicIn {
    pub topic_key: String,
    pub name: String,
    #[serde(default)]
    pub parent: Option<String>, // 父 topic_key
}

#[derive(Deserialize)]
pub struct QuestionIn {
    pub qid: String,
    #[serde(rename = "type")]
    pub qtype: String,
    pub stem: String,
    #[serde(default)]
    pub img_path: Option<String>,
    pub options: Vec<String>,
    pub answer: String,
    #[serde(default)]
    pub answer_conf: String,
    #[serde(default)]
    pub explain: String,
    #[serde(default)]
    pub source: String,
    #[serde(default)]
    pub difficulty: i64,
    #[serde(default)]
    pub topics: Vec<String>,
    #[serde(default)]
    pub status: String,
}

#[derive(Deserialize)]
pub struct PaperIn {
    pub name: String,
    #[serde(default)]
    pub title: String,
    #[serde(default)]
    pub source_url: String,
    #[serde(default)]
    pub items: Vec<PaperItemIn>,
}

#[derive(Deserialize)]
pub struct PaperItemIn {
    pub qid: String,
    pub sort_no: i64,
    #[serde(default)]
    pub score: f64,
}

#[derive(Serialize)]
pub struct ImportReport {
    pub bank_id: String,
    pub bank_name: String,
    pub questions: usize,
    pub papers: usize,
    pub images: usize,
    pub skipped: bool, // 已存在同版本，跳过
}

// ---------- 导入 ----------
pub fn import(conn: &Connection, file: &Path, banks_root: &Path, is_builtin: bool) -> Result<ImportReport, String> {
    let zf = zip::ZipArchive::new(std::fs::File::open(file).map_err(|e| e.to_string())?).map_err(|e| e.to_string())?;
    let mut zf = zf;
    let mut manifest_str = String::new();
    zf.by_name("manifest.json").map_err(|e| format!("manifest.json 缺失: {e}"))?
        .read_to_string(&mut manifest_str).map_err(|e| e.to_string())?;
    let mf: Manifest = serde_json::from_str(&manifest_str).map_err(|e| format!("manifest 解析失败: {e}"))?;
    if mf.format != "smartbank" || mf.schema_ver != 1 {
        return Err(format!("不支持的格式 {} v{}", mf.format, mf.schema_ver));
    }

    // 幂等：同 bank 同版本已导入则跳过
    let exists: Option<i64> = conn.query_row(
        "SELECT 1 FROM banks WHERE bank_id=?1 AND version=?2", params![mf.bank.bank_id, mf.bank.version],
        |_| Ok(1)).ok();
    if exists.is_some() {
        return Ok(ImportReport { bank_id: mf.bank.bank_id, bank_name: mf.bank.name,
            questions: 0, papers: 0, images: 0, skipped: true});
    }

    // 解包资产到 banks_root/<bank_id>/assets/
    let asset_root = banks_root.join(&mf.bank.bank_id).join("assets");
    std::fs::create_dir_all(&asset_root).map_err(|e| e.to_string())?;
    let mut images = 0usize;
    for i in 0..zf.len() {
        let mut entry = zf.by_index(i).map_err(|e| e.to_string())?;
        let name = entry.name().to_string();
        if let Some(rel) = name.strip_prefix("assets/") {
            if rel.is_empty() { continue; }
            // 防路径穿越
            if rel.split(['/','\\']).any(|s| s == "..") { continue; }
            let out = asset_root.join(rel);
            if let Some(p) = out.parent() { std::fs::create_dir_all(p).map_err(|e| e.to_string())?; }
            let mut buf = Vec::new();
            entry.read_to_end(&mut buf).map_err(|e| e.to_string())?;
            std::fs::write(&out, &buf).map_err(|e| e.to_string())?;
            images += 1;
        }
    }

    let now = chrono::Utc::now().to_rfc3339();
    let tx = conn.unchecked_transaction().map_err(|e| e.to_string())?;
    tx.execute("INSERT OR REPLACE INTO banks(bank_id,name,version,schema_ver,description,is_builtin,is_enabled,asset_root,imported_at)
                VALUES(?,?,?,?,?,?,1,?,?)",
        params![mf.bank.bank_id, mf.bank.name, mf.bank.version, mf.schema_ver, mf.bank.description,
                is_builtin as i64, asset_root.to_string_lossy(), now]).map_err(|e| e.to_string())?;

    // 主题（先父后子插入，key->id 映射）
    let mut topic_ids: HashMap<String, i64> = HashMap::new();
    let mut sorted: Vec<&TopicIn> = mf.topics.iter().collect();
    sorted.sort_by_key(|t| t.parent.is_some()); // 无父的先插
    for t in sorted {
        let pid = t.parent.as_ref().and_then(|k| topic_ids.get(k).copied());
        tx.execute("INSERT INTO topics(bank_id,parent_id,name,sort_order) VALUES(?,?,?,?)",
            params![mf.bank.bank_id, pid, t.name, topic_ids.len() as i64]).map_err(|e| e.to_string())?;
        let id = tx.last_insert_rowid();
        topic_ids.insert(t.topic_key.clone(), id);
    }

    for q in &mf.questions {
        let status = if q.status.is_empty() { "active".into() } else { q.status.clone() };
        tx.execute("INSERT OR REPLACE INTO questions(bank_id,qid,version,type,stem,img_path,options,answer,answer_conf,explain,source,difficulty,status,created_at,updated_at)
                    VALUES(?,?,1,?,?,?,?,?,?,?,?,?,?,?,?)",
            params![mf.bank.bank_id, q.qid, q.qtype, q.stem, q.img_path,
                    serde_json::to_string(&q.options).unwrap(), q.answer,
                    if q.answer_conf.is_empty() {"high".into()} else {q.answer_conf.clone()},
                    q.explain, q.source, q.difficulty, status, now, now]).map_err(|e| e.to_string())?;
        for tk in &q.topics {
            if let Some(tid) = topic_ids.get(tk) {
                tx.execute("INSERT OR REPLACE INTO question_topics(bank_id,qid,topic_id) VALUES(?,?,?)",
                    params![mf.bank.bank_id, q.qid, tid]).map_err(|e| e.to_string())?;
            }
        }
    }

    // 试卷幂等：重导入（版本升级）前先清本库旧试卷及其题目关联，防止同名试卷重复堆积。
    // sessions.paper_id 无外键仅作来源记录，断链不影响续做（resume 走 qid_list）。
    tx.execute("DELETE FROM paper_questions WHERE bank_id=?1", params![mf.bank.bank_id]).map_err(|e| e.to_string())?;
    tx.execute("DELETE FROM papers WHERE bank_id=?1", params![mf.bank.bank_id]).map_err(|e| e.to_string())?;
    for p in &mf.papers {
        tx.execute("INSERT INTO papers(bank_id,name,source_url,description,is_builtin) VALUES(?,?,?,?,1)",
            params![mf.bank.bank_id, p.name, p.source_url, p.title]).map_err(|e| e.to_string())?;
        let pid = tx.last_insert_rowid();
        for it in &p.items {
            tx.execute("INSERT OR REPLACE INTO paper_questions(paper_id,bank_id,qid,sort_no,score) VALUES(?,?,?,?,?)",
                params![pid, mf.bank.bank_id, it.qid, it.sort_no, it.score]).map_err(|e| e.to_string())?;
        }
    }
    tx.commit().map_err(|e| e.to_string())?;

    Ok(ImportReport { bank_id: mf.bank.bank_id, bank_name: mf.bank.name,
        questions: mf.questions.len(), papers: mf.papers.len(), images, skipped: false })
}

// ---------- 查询 ----------
#[derive(Serialize)]
pub struct Overview {
    pub banks: Vec<BankStat>,
    pub topics: Vec<TopicStat>,
}
#[derive(Serialize)]
pub struct BankStat { pub bank_id: String, pub name: String, pub version: i64, pub total: i64, pub active: i64, pub pending: i64, pub papers: i64 }
#[derive(Serialize)]
pub struct TopicStat { pub topic_id: i64, pub name: String, pub total: i64, pub active: i64 }

pub fn overview(conn: &Connection) -> Result<Overview, String> {
    let mut banks = Vec::new();
    let mut stmt = conn.prepare(
        "SELECT b.bank_id,b.name,b.version,
            (SELECT COUNT(*) FROM questions q WHERE q.bank_id=b.bank_id),
            (SELECT COUNT(*) FROM questions q WHERE q.bank_id=b.bank_id AND q.status='active'),
            (SELECT COUNT(*) FROM questions q WHERE q.bank_id=b.bank_id AND q.status='pending_review'),
            (SELECT COUNT(*) FROM papers p WHERE p.bank_id=b.bank_id)
         FROM banks b WHERE b.is_enabled=1 ORDER BY b.is_builtin DESC, b.bank_id")
        .map_err(|e| e.to_string())?;
    let rows = stmt.query_map([], |r| Ok(BankStat {
        bank_id: r.get(0)?, name: r.get(1)?, version: r.get(2)?,
        total: r.get(3)?, active: r.get(4)?, pending: r.get(5)?, papers: r.get(6)?,
    })).map_err(|e| e.to_string())?;
    for b in rows { banks.push(b.map_err(|e| e.to_string())?); }

    let mut topics = Vec::new();
    let mut stmt = conn.prepare(
        "SELECT t.topic_id,t.name,
            (SELECT COUNT(*) FROM question_topics qt JOIN questions q ON q.bank_id=qt.bank_id AND q.qid=qt.qid WHERE qt.topic_id=t.topic_id),
            (SELECT COUNT(*) FROM question_topics qt JOIN questions q ON q.bank_id=qt.bank_id AND q.qid=qt.qid WHERE qt.topic_id=t.topic_id AND q.status='active')
         FROM topics t ORDER BY t.sort_order").map_err(|e| e.to_string())?;
    let rows = stmt.query_map([], |r| Ok(TopicStat { topic_id: r.get(0)?, name: r.get(1)?, total: r.get(2)?, active: r.get(3)? }))
        .map_err(|e| e.to_string())?;
    for t in rows { topics.push(t.map_err(|e| e.to_string())?); }
    Ok(Overview { banks, topics })
}

#[derive(Serialize, Clone)]
pub struct QuestionRow {
    pub bank_id: String, pub qid: String, pub qtype: String, pub stem: String,
    pub img_path: Option<String>, pub options: Vec<String>,
    pub answer: String, pub answer_conf: String, pub explain: String,
    pub source: String, pub status: String, pub topics: Vec<String>,
}

const Q_SELECT: &str = "SELECT q.bank_id,q.qid,q.type,q.stem,q.img_path,q.options,q.answer,q.answer_conf,q.explain,q.source,q.status,
    (SELECT group_concat(t.name,'/') FROM question_topics qt JOIN topics t ON t.topic_id=qt.topic_id WHERE qt.bank_id=q.bank_id AND qt.qid=q.qid)";

fn row_to_question(r: &rusqlite::Row) -> rusqlite::Result<QuestionRow> {
    let opts: String = r.get(5)?;
    Ok(QuestionRow {
        bank_id: r.get(0)?, qid: r.get(1)?, qtype: r.get(2)?, stem: r.get(3)?, img_path: r.get(4)?,
        options: serde_json::from_str(&opts).unwrap_or_default(),
        answer: r.get(6)?, answer_conf: r.get(7)?, explain: r.get(8)?,
        source: r.get(9)?, status: r.get(10)?,
        topics: r.get::<_, Option<String>>(11)?.map(|s| s.split('/').map(String::from).collect()).unwrap_or_default(),
    })
}

/// 按 (bank_id, qid) 列表取题，保持传入顺序
pub fn get_questions_by_ids(conn: &Connection, qids: &[(String, String)]) -> Result<Vec<QuestionRow>, String> {
    let mut out: Vec<QuestionRow> = Vec::new();
    let mut stmt = conn.prepare(&format!("{Q_SELECT} FROM questions q WHERE q.bank_id=?1 AND q.qid=?2"))
        .map_err(|e| e.to_string())?;
    for (bank_id, qid) in qids {
        let mut rows = stmt.query_map(params![bank_id, qid], row_to_question).map_err(|e| e.to_string())?;
        if let Some(r) = rows.next() { out.push(r.map_err(|e| e.to_string())?); }
    }
    Ok(out)
}

#[derive(Serialize)]
pub struct PaperInfo { pub paper_id: i64, pub bank_id: String, pub name: String, pub title: String, pub count: i64 }

pub fn list_papers(conn: &Connection) -> Result<Vec<PaperInfo>, String> {
    let mut stmt = conn.prepare(
        "SELECT p.paper_id,p.bank_id,p.name,COALESCE(p.description,''),COUNT(pq.qid)
         FROM papers p LEFT JOIN paper_questions pq ON pq.paper_id=p.paper_id
         GROUP BY p.paper_id ORDER BY p.paper_id").map_err(|e| e.to_string())?;
    let rows = stmt.query_map([], |r| Ok(PaperInfo {
        paper_id: r.get(0)?, bank_id: r.get(1)?, name: r.get(2)?, title: r.get(3)?, count: r.get(4)?,
    })).map_err(|e| e.to_string())?;
    Ok(rows.filter_map(|x| x.ok()).collect())
}

pub fn paper_qids(conn: &Connection, paper_id: i64) -> Result<Vec<(String, String)>, String> {
    let mut stmt = conn.prepare(
        "SELECT bank_id,qid FROM paper_questions WHERE paper_id=? ORDER BY sort_no").map_err(|e| e.to_string())?;
    let rows = stmt.query_map(params![paper_id], |r| Ok((r.get::<_, String>(0)?, r.get::<_, String>(1)?)))
        .map_err(|e| e.to_string())?;
    Ok(rows.filter_map(|x| x.ok()).collect())
}

pub fn list_questions(conn: &Connection, topic_id: Option<i64>, qtype: Option<String>,
                      status: Option<String>, search: Option<String>, limit: i64, offset: i64) -> Result<Vec<QuestionRow>, String> {
    let mut sql = format!("{Q_SELECT} FROM questions q WHERE 1=1");
    let mut args: Vec<Box<dyn rusqlite::types::ToSql>> = Vec::new();
    if let Some(t) = topic_id {
        sql.push_str(" AND q.qid IN (SELECT qid FROM question_topics WHERE topic_id=?)");
        args.push(Box::new(t));
    }
    if let Some(t) = &qtype { sql.push_str(" AND q.type=?"); args.push(Box::new(t)); }
    if let Some(s) = &status { sql.push_str(" AND q.status=?"); args.push(Box::new(s)); }
    else { sql.push_str(" AND q.status!='retired'"); }
    if let Some(s) = &search { sql.push_str(" AND (q.stem LIKE ? OR q.explain LIKE ?)"); args.push(Box::new(format!("%{s}%"))); args.push(Box::new(format!("%{s}%"))); }
    sql.push_str(" ORDER BY q.qid LIMIT ? OFFSET ?");
    args.push(Box::new(limit)); args.push(Box::new(offset));

    let mut stmt = conn.prepare(&sql).map_err(|e| e.to_string())?;
    let params_ref: Vec<&dyn rusqlite::types::ToSql> = args.iter().map(|b| b.as_ref()).collect();
    let rows = stmt.query_map(params_ref.as_slice(), row_to_question).map_err(|e| e.to_string())?;
    let mut out = Vec::new();
    for q in rows { out.push(q.map_err(|e| e.to_string())?); }
    Ok(out)
}

// ---------- FTS5 中文分词 spike（M0 验收项） ----------
// 验证：FTS5 默认 unicode61 对中文不可用；jieba-rs 预分词列方案可行且有性能优势
#[derive(Serialize)]
pub struct SpikeResult {
    pub rows: usize,              // 基准行数（含合成扩量）
    pub seg_build_ms: u128,       // jieba 分词 + 建 FTS 索引耗时
    pub like_avg_ms: f64,         // LIKE '%kw%' 平均耗时
    pub fts_avg_ms: f64,          // FTS5 MATCH 平均耗时
    pub fts_hits: usize,          // FTS 命中数（校验）
    pub like_hits: usize,
}

pub fn fts_spike(conn: &Connection, scale_to: usize, queries: &[&str]) -> Result<SpikeResult, String> {
    let jb = jieba_rs::Jieba::new();
    let t0 = Instant::now();

    conn.execute_batch("DROP TABLE IF EXISTS spike_src; DROP TABLE IF EXISTS spike_fts;
        CREATE TABLE spike_src(id INTEGER PRIMARY KEY, seg TEXT);").map_err(|e| e.to_string())?;
    // 取全部题干，不足 scale_to 则循环扩量（模拟大题库）
    let mut stmt = conn.prepare("SELECT stem FROM questions").map_err(|e| e.to_string())?;
    let stems: Vec<String> = stmt.query_map([], |r| r.get::<_, String>(0)).map_err(|e| e.to_string())?
        .filter_map(|x| x.ok()).collect();
    drop(stmt);
    let mut n = 0usize;
    conn.execute_batch("BEGIN").map_err(|e| e.to_string())?;
    {
        let mut ins = conn.prepare("INSERT INTO spike_src(seg) VALUES(?)").map_err(|e| e.to_string())?;
        while n < scale_to {
            for s in &stems {
                let seg: String = jb.cut(s, true).iter().map(|t| t.word).collect::<Vec<&str>>().join(" ");
                ins.execute(params![seg]).map_err(|e| e.to_string())?;
                n += 1;
                if n >= scale_to { break; }
            }
        }
    }
    conn.execute_batch("COMMIT").map_err(|e| e.to_string())?;
    conn.execute_batch(
        "CREATE VIRTUAL TABLE spike_fts USING fts5(seg, content='spike_src', content_rowid='id');
         INSERT INTO spike_fts(rowid, seg) SELECT id, seg FROM spike_src;")
        .map_err(|e| format!("FTS5 建表失败（bundled SQLite 可能未启用FTS5）: {e}"))?;
    let seg_build_ms = t0.elapsed().as_millis();

    // 对比查询
    let mut like_total = 0f64; let mut fts_total = 0f64;
    let mut fts_hits = 0usize; let mut like_hits = 0usize;
    for q in queries {
        let pat = format!("%{q}%");
        let t = Instant::now();
        let lh: i64 = conn.query_row("SELECT COUNT(*) FROM spike_src WHERE seg LIKE ?", params![pat], |r| r.get(0))
            .map_err(|e| e.to_string())?;
        like_total += t.elapsed().as_secs_f64() * 1000.0;
        like_hits += lh as usize;

        // 查询词也过 jieba，多词 AND
        let toks: Vec<String> = jb.cut(q, true).iter().map(|t| t.word)
            .filter(|s| !s.trim().is_empty()).map(|s| s.to_string()).collect();
        let match_expr = if toks.is_empty() { format!("\"{q}\"") } else {
            toks.iter().map(|t| format!("\"{t}\"")).collect::<Vec<_>>().join(" AND ")
        };
        let t = Instant::now();
        let fh: i64 = conn.query_row("SELECT COUNT(*) FROM spike_fts WHERE spike_fts MATCH ?", params![match_expr], |r| r.get(0))
            .map_err(|e| e.to_string())?;
        fts_total += t.elapsed().as_secs_f64() * 1000.0;
        fts_hits += fh as usize;
    }
    let rounds = queries.len().max(1) as f64;
    conn.execute_batch("DROP TABLE IF EXISTS spike_src; DROP TABLE IF EXISTS spike_fts;").map_err(|e| e.to_string())?;
    Ok(SpikeResult { rows: n, seg_build_ms, like_avg_ms: like_total / rounds, fts_avg_ms: fts_total / rounds, fts_hits, like_hits })
}

// ---------- 测试 ----------
#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    fn test_env() -> (Connection, PathBuf) {
        let tmp = std::env::temp_dir().join(format!("sqatest-{}", uuid::Uuid::new_v4()));
        std::fs::create_dir_all(&tmp).unwrap();
        let conn = crate::db::open(&tmp.join("bank.db")).unwrap();
        (conn, tmp)
    }

    #[test]
    fn import_seed_and_query() {
        let (conn, tmp) = test_env();
        let seed = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("resources/seed/smart-core.smartbank");
        if !seed.exists() { eprintln!("种子不存在，跳过（先运行 pack_seed.py）"); return; }
        let r = import(&conn, &seed, &tmp.join("banks"), true).unwrap();
        assert!(!r.skipped);
        assert_eq!(r.questions, 694);
        assert_eq!(r.papers, 5);
        assert_eq!(r.images, 11);

        let ov = overview(&conn).unwrap();
        assert_eq!(ov.banks.len(), 1);
        assert_eq!(ov.banks[0].total, 694);
        // v2 种子：A~E 卷答案经官方满分卷校准，原 57 道待复审题全部转正
        assert_eq!(ov.banks[0].active, 694);
        assert_eq!(ov.banks[0].pending, 0);

        // 复索验证 pending_review 隔离
        let act = list_questions(&conn, None, None, Some("active".into()), None, 1000, 0).unwrap();
        assert_eq!(act.len(), 694);
        assert!(act.iter().all(|q| q.status == "active"));

        // 图片题存在且指向 bank 内相对路径
        let with_img = list_questions(&conn, None, None, None, Some("VW0".into()), 10, 0).unwrap();
        assert!(with_img.iter().any(|q| q.img_path.is_some()));

        // 幂等：同版本重导入 skipped
        let r2 = import(&conn, &seed, &tmp.join("banks"), true).unwrap();
        assert!(r2.skipped);

        // 幂等：版本升级触发重导入时试卷不得重复堆积（曾出现 5 套变 10 套）
        conn.execute("UPDATE banks SET version=1 WHERE bank_id='smart-core'", []).unwrap();
        let r3 = import(&conn, &seed, &tmp.join("banks"), true).unwrap();
        assert!(!r3.skipped);
        let n_papers: i64 = conn.query_row("SELECT COUNT(*) FROM papers", [], |r| r.get(0)).unwrap();
        let n_pq: i64 = conn.query_row("SELECT COUNT(*) FROM paper_questions", [], |r| r.get(0)).unwrap();
        assert_eq!(n_papers, 5, "重导入后试卷应仍为 5 套");
        assert_eq!(n_pq, 350, "重导入后试卷-题目关联应仍为 350 条");
        std::fs::remove_dir_all(&tmp).ok();
    }

    #[test]
    fn fts_spike_benchmark() {
        let (conn, tmp) = test_env();
        let seed = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("resources/seed/smart-core.smartbank");
        if !seed.exists() { eprintln!("种子不存在，跳过"); return; }
        import(&conn, &seed, &tmp.join("banks"), true).unwrap();
        let r = fts_spike(&conn, 100_000, &["以太网通信", "高速计数器", "Modbus", "模拟量输入"]).unwrap();
        println!("\n=== FTS spike 基准 ===\nrows={} 建索引={}ms LIKE平均={:.2}ms FTS平均={:.2}ms FTS命中={} LIKE命中={}",
            r.rows, r.seg_build_ms, r.like_avg_ms, r.fts_avg_ms, r.fts_hits, r.like_hits);
        assert!(r.rows >= 100_000);
        assert!(r.fts_hits > 0);
        std::fs::remove_dir_all(&tmp).ok();
    }
}

// ---------- Tauri 命令 ----------
pub mod commands {
    use super::{overview as overview_impl, list_papers as list_papers_impl,
                paper_qids as paper_qids_impl, list_questions as list_questions_impl,
                get_questions_by_ids as get_questions_by_ids_impl, fts_spike as fts_spike_impl,
                import, Overview, PaperInfo, QuestionRow, SpikeResult, ImportReport};
    use crate::telemetry::timed;
    use crate::AppState;
    use tauri::Manager;

    #[tauri::command]
    pub async fn bank_overview(state: tauri::State<'_, AppState>) -> Result<Overview, String> {
        timed("bank_overview", false, || {
            let b = state.bank.lock().map_err(|e| e.to_string())?;
            overview_impl(&b)
        })
    }

    #[tauri::command]
    pub async fn list_papers(state: tauri::State<'_, AppState>) -> Result<Vec<PaperInfo>, String> {
        timed("list_papers", false, || {
            let b = state.bank.lock().map_err(|e| e.to_string())?;
            list_papers_impl(&b)
        })
    }

    #[tauri::command]
    pub async fn paper_questions(state: tauri::State<'_, AppState>, paper_id: i64) -> Result<Vec<(String, String)>, String> {
        timed("paper_questions", false, || {
            let b = state.bank.lock().map_err(|e| e.to_string())?;
            paper_qids_impl(&b, paper_id)
        })
    }

    #[tauri::command]
    pub async fn list_questions(state: tauri::State<'_, AppState>,
        topic_id: Option<i64>, qtype: Option<String>, status: Option<String>,
        search: Option<String>, limit: Option<i64>, offset: Option<i64>) -> Result<Vec<QuestionRow>, String> {
        timed("list_questions", false, || {
            let conn = state.bank.lock().map_err(|e| e.to_string())?;
            list_questions_impl(&conn, topic_id, qtype, status, search,
                limit.unwrap_or(50).min(500), offset.unwrap_or(0))
        })
    }

    #[tauri::command]
    pub async fn get_questions_by_ids(state: tauri::State<'_, AppState>, qids: Vec<(String, String)>)
        -> Result<Vec<QuestionRow>, String> {
        timed("get_questions_by_ids", false, || {
            let conn = state.bank.lock().map_err(|e| e.to_string())?;
            get_questions_by_ids_impl(&conn, &qids)
        })
    }

    #[tauri::command]
    pub async fn fts_spike(state: tauri::State<'_, AppState>, scale_to: Option<usize>) -> Result<SpikeResult, String> {
        timed("fts_spike", false, || {
            let conn = state.bank.lock().map_err(|e| e.to_string())?;
            fts_spike_impl(&conn, scale_to.unwrap_or(100_000).min(200_000),
                &["以太网通信", "高速计数器", "Modbus", "模拟量输入"])
        })
    }

    /// 数据包导入（公开仓不含版权数据，用户自备 .smartbank）
    #[tauri::command]
    pub async fn import_bank_file(state: tauri::State<'_, AppState>, app: tauri::AppHandle, path: String)
        -> Result<ImportReport, String> {
        timed("import_bank_file", false, || {
            if !path.to_lowercase().ends_with(".smartbank") {
                return Err("仅支持 .smartbank 题库包".into());
            }
            let b = state.bank.lock().map_err(|e| e.to_string())?;
            let banks_root = app.path().app_data_dir().map_err(|e| e.to_string())?.join("banks");
            std::fs::create_dir_all(&banks_root).map_err(|e| e.to_string())?;
            import(&b, std::path::Path::new(&path), &banks_root, false)
        })
    }
}
