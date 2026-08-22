// Excel 交换：题库导入向导（预览/导入/模板）与成绩单导出
use calamine::Reader;
use rusqlite::{params, Connection};
use serde::Serialize;
use std::collections::HashMap;

pub const TEMPLATE_HEADERS: [&str; 15] = ["题干", "题型", "选项A", "选项B", "选项C", "选项D", "选项E", "选项F",
    "答案", "解析", "出处", "一级章节", "二级章节", "难度(1-5)", "置信度"];

#[derive(Serialize, Clone)]
pub struct ParsedQ {
    pub stem: String, pub qtype: String, pub options: Vec<String>,
    pub answer: String, pub explain: String, pub source: String,
    pub topic1: String, pub topic2: String, pub difficulty: i64, pub conf: String,
}
#[derive(Serialize)]
pub struct RowError { pub row: usize, pub msg: String }
#[derive(Serialize)]
pub struct ExcelPreview {
    pub total: usize, pub valid: usize,
    pub errors: Vec<RowError>,
    pub sample: Vec<ParsedQ>,
}
#[derive(Serialize)]
pub struct ExcelImportReport {
    pub bank_id: String, pub bank_name: String,
    pub imported: usize, pub skipped: usize, pub topics: usize,
    pub errors: Vec<RowError>,
}

fn cell(r: &[String], i: usize) -> String {
    r.get(i).map(|c| c.trim().to_string()).unwrap_or_default()
}

fn norm_type(s: &str) -> Option<String> {
    match s.trim() {
        "" | "单选" | "single" | "单选题" => Some("single".into()),
        "多选" | "multi" | "多选题" => Some("multi".into()),
        "判断" | "judge" | "判断题" => Some("judge".into()),
        "填空" | "fill" | "填空题" => Some("fill".into()),
        _ => None,
    }
}

/// 答案归一化：判断 对/错/√/×/T/F → T/F；多选 "A,B,D"/"A、B、D"/"ABD" → "ABD"
fn norm_answer(raw: &str, qtype: &str, opt_count: usize) -> Result<String, String> {
    let t = raw.trim().to_uppercase();
    if qtype == "fill" { return Ok(raw.trim().to_string()); }
    if qtype == "judge" {
        return match t.as_str() {
            "对" | "T" | "TRUE" | "√" | "正确" => Ok("T".into()),
            "错" | "F" | "FALSE" | "×" | "X" | "错误" => Ok("F".into()),
            _ => Err("判断题答案须为 对/错/√/×/T/F".into()),
        };
    }
    let letters: Vec<char> = t.chars().filter(|c| ('A'..='H').contains(c)).collect();
    if letters.is_empty() { return Err(format!("无法从「{raw}」解析答案字母")); }
    let mut s = letters.clone();
    s.sort(); s.dedup();
    for c in &s {
        let idx = *c as usize - 'A' as usize;
        if idx >= opt_count { return Err(format!("答案 {c} 超出选项范围（共{opt_count}项）")); }
    }
    if qtype == "single" && s.len() != 1 { return Err("单选题答案只能一个字母".into()); }
    if qtype == "multi" && s.len() < 2 { return Err("多选题答案至少两个字母".into()); }
    Ok(s.into_iter().collect())
}

fn read_rows(path: &str) -> Result<Vec<Vec<String>>, String> {
    let mut wb = calamine::open_workbook_auto(path)
        .map_err(|e| format!("无法打开 Excel：{e}"))?;
    let names = wb.sheet_names().to_vec();
    let mut out = Vec::new();
    for name in names {
        let range = wb.worksheet_range(&name).map_err(|e| e.to_string())?;
        for row in range.rows() {
            out.push(row.iter().map(|c| c.to_string()).collect());
        }
        break; // 只读第一个工作表
    }
    Ok(out)
}

fn parse_workbook(rows: Vec<Vec<String>>) -> (Vec<ParsedQ>, Vec<RowError>) {
    let mut qs = Vec::new();
    let mut errs = Vec::new();
    for (ri, r) in rows.iter().enumerate().skip(1) { // 跳过表头
        if r.iter().all(|c| c.trim().is_empty()) { continue; }
        let row_no = ri + 1;
        let stem = cell(r, 0);
        if stem.is_empty() { errs.push(RowError { row: row_no, msg: "题干为空".into() }); continue; }
        let qtype = match norm_type(&cell(r, 1)) {
            Some(t) => t,
            None => { errs.push(RowError { row: row_no, msg: format!("题型「{}」无法识别（单选/多选/判断/填空）", cell(r, 1)) }); continue; }
        };
        let mut options: Vec<String> = (2..8).map(|i| cell(r, i)).take_while(|s| !s.is_empty()).collect();
        // 补齐中间空档：遇到空档后还有内容则报错
        let raw_opts: Vec<String> = (2..8).map(|i| cell(r, i)).collect();
        if let Some(last) = raw_opts.iter().rposition(|s| !s.is_empty()) {
            if raw_opts[..=last].iter().any(|s| s.is_empty()) {
                errs.push(RowError { row: row_no, msg: "选项中间存在空档（如A、C有值B为空）".into() });
            }
            options = raw_opts[..=last].to_vec();
        }
        let answer_raw = cell(r, 8);
        if answer_raw.is_empty() && qtype != "fill" {
            errs.push(RowError { row: row_no, msg: "答案为空".into() }); continue;
        }
        if qtype == "judge" { options = vec!["对".into(), "错".into()]; }
        let answer = match norm_answer(&answer_raw, &qtype, if qtype == "fill" { 8 } else { options.len() }) {
            Ok(a) => a,
            Err(e) => { errs.push(RowError { row: row_no, msg: format!("答案错误：{e}") }); continue; }
        };
        if (qtype == "single" || qtype == "multi") && options.len() < 2 {
            errs.push(RowError { row: row_no, msg: "选择题至少需要2个选项".into() }); continue;
        }
        let difficulty = cell(r, 13).parse::<i64>().unwrap_or(3).clamp(1, 5);
        let conf = match cell(r, 14).as_str() { "" | "high" | "高" => "high", "medium" | "中" => "medium", "low" | "低" => "low", _ => "high" };
        qs.push(ParsedQ {
            stem, qtype, options: options.iter().enumerate().map(|(i, o)| format!("{}、{}", (b'A' + i as u8) as char, o)).collect(),
            answer, explain: cell(r, 9), source: cell(r, 10),
            topic1: { let t = cell(r, 11); if t.is_empty() { "未分类".into() } else { t } },
            topic2: cell(r, 12), difficulty, conf: conf.into(),
        });
    }
    (qs, errs)
}

pub fn excel_preview(path: &str) -> Result<ExcelPreview, String> {
    let rows = read_rows(path)?;
    let (qs, errs) = parse_workbook(rows);
    Ok(ExcelPreview { total: qs.len() + errs.len(), valid: qs.len(), errors: errs, sample: qs.into_iter().take(5).collect() })
}

pub fn export_template(path: &str) -> Result<String, String> {
    let mut wb = rust_xlsxwriter::Workbook::new();
    let ws = wb.add_worksheet();
    let bold = rust_xlsxwriter::Format::new().set_bold();
    let hint = rust_xlsxwriter::Format::new().set_font_color(rust_xlsxwriter::Color::RGB(0x888888));
    for (c, h) in TEMPLATE_HEADERS.iter().enumerate() {
        let _ = ws.write_with_format(0, c as u16, *h, &bold);
    }
    let example = ["S7-200 SMART 标准型CPU最多扩展几个模块？", "单选", "3个", "6个", "8个", "", "", "",
        "B", "SR/ST最多6个EM+1个SB", "选型手册v2.8", "硬件与选型", "", "3", "high"];
    for (c, v) in example.iter().enumerate() {
        let _ = ws.write_with_format(1, c as u16, *v, &hint);
    }
    let _ = ws.write(3, 0, "说明：题型填 单选/多选/判断/填空；多选答案支持 A,B,D 或 ABD；判断答案支持 对/错/√/×/T/F；置信度 high/medium/low（低置信度题不参与判分）");
    for c in 0..TEMPLATE_HEADERS.len() {
        let _ = ws.set_column_width(c as u16, 16);
    }
    let _ = ws.set_column_width(0, 50);
    wb.save(path).map_err(|e| e.to_string())?;
    Ok(path.to_string())
}

pub fn excel_import_into(bankconn: &Connection, path: &str, bank_name: &str) -> Result<ExcelImportReport, String> {
    let rows = read_rows(path)?;
    let (qs, errs) = parse_workbook(rows);
    if qs.is_empty() { return Err("没有可导入的有效题目（请查看预览中的错误行）".into()); }
    let bank_id = format!("xlsx-{}", uuid::Uuid::new_v4().simple().to_string()[..8].to_string());
    let now = chrono::Utc::now().to_rfc3339();
    let tx = bankconn.unchecked_transaction().map_err(|e| e.to_string())?;
    tx.execute("INSERT INTO banks(bank_id,name,version,schema_ver,description,is_builtin,is_enabled,imported_at) VALUES(?,?,1,1,'Excel导入',0,1,?)",
        params![bank_id, bank_name, now]).map_err(|e| e.to_string())?;
    // 章节树
    let mut topic_ids: HashMap<(String, String), i64> = HashMap::new();
    for q in &qs {
        let key = (q.topic1.clone(), q.topic2.clone());
        if topic_ids.contains_key(&key) { continue; }
        let parent = if !q.topic2.is_empty() {
            let pkey = (q.topic1.clone(), String::new());
            if !topic_ids.contains_key(&pkey) {
                tx.execute("INSERT INTO topics(bank_id,parent_id,name,sort_order) VALUES(?,NULL,?,?)",
                    params![bank_id, q.topic1, topic_ids.len() as i64]).map_err(|e| e.to_string())?;
                topic_ids.insert(pkey, tx.last_insert_rowid());
            }
            Some(topic_ids[&(q.topic1.clone(), String::new())])
        } else { None };
        let name = if q.topic2.is_empty() { q.topic1.clone() } else { q.topic2.clone() };
        tx.execute("INSERT INTO topics(bank_id,parent_id,name,sort_order) VALUES(?,?,?,?)",
            params![bank_id, parent, name, topic_ids.len() as i64]).map_err(|e| e.to_string())?;
        topic_ids.insert(key, tx.last_insert_rowid());
    }
    let n_topics = topic_ids.len();
    for (i, q) in qs.iter().enumerate() {
        let qid = format!("X{i:04}");
        let status = if q.conf == "low" { "pending_review" } else { "active" };
        tx.execute("INSERT INTO questions(bank_id,qid,version,type,stem,img_path,options,answer,answer_conf,explain,source,difficulty,status,created_at,updated_at)
                    VALUES(?,?,1,?,?,NULL,?,?,?,?,?,?,?,?,?)",
            params![bank_id, qid, q.qtype, q.stem, serde_json::to_string(&q.options).unwrap(),
                    q.answer, q.conf, q.explain, q.source, q.difficulty, status, now, now]).map_err(|e| e.to_string())?;
        if let Some(tid) = topic_ids.get(&(q.topic1.clone(), q.topic2.clone())) {
            tx.execute("INSERT INTO question_topics(bank_id,qid,topic_id) VALUES(?,?,?)",
                params![bank_id, qid, tid]).map_err(|e| e.to_string())?;
        }
    }
    tx.commit().map_err(|e| e.to_string())?;
    // 隐私：不记题库名（用户自由输入，日志随诊断包外发）
    log::info!(target: "excel", "Excel导入：成功{}题 跳过{}题", qs.len(), errs.len());
    Ok(ExcelImportReport { bank_id, bank_name: bank_name.into(), imported: qs.len(), skipped: errs.len(), topics: n_topics, errors: errs })
}

// ---------- 成绩单导出 ----------
pub fn export_session_excel(user: &Connection, bankconn: &Connection, session_id: i64, path: &str) -> Result<String, String> {
    let detail = crate::session::session_detail(user, bankconn, session_id)?;
    let s = &detail.session;
    let mut wb = rust_xlsxwriter::Workbook::new();
    let ws = wb.add_worksheet();
    let _ = ws.set_name("成绩单");

    let title = rust_xlsxwriter::Format::new().set_bold().set_font_size(14);
    let bold = rust_xlsxwriter::Format::new().set_bold();
    let _ = ws.write_with_format(0, 0, s.title.as_str(), &title);
    let _ = ws.write(1, 0, format!("得分：{}", s.score.map(|x| format!("{:.1}", x)).unwrap_or_else(|| "—".into())));
    let _ = ws.write(1, 2, format!("答对 {}/{}（计分题）", s.correct_qty, s.scored_qty));
    let _ = ws.write(1, 4, format!("完成时间：{}", s.finished_at.clone().unwrap_or_default()));

    let headers = ["#", "题型", "主题", "题干", "你的答案", "正确答案", "结果", "解析", "出处"];
    for (c, h) in headers.iter().enumerate() {
        let _ = ws.write_with_format(3, c as u16, *h, &bold);
    }
    for (i, r) in detail.records.iter().enumerate() {
        let q = r.question.as_ref();
        let row = (i + 4) as u32;
        let _ = ws.write(row, 0, i as i32 + 1);
        let _ = ws.write(row, 1, if q.map(|q| q.qtype == "multi").unwrap_or(false) { "多选" } else { "单选" });
        let _ = ws.write(row, 2, q.map(|q| q.topics.join("/")).unwrap_or_default());
        let _ = ws.write(row, 3, q.map(|q| q.stem.as_str()).unwrap_or(""));
        let _ = ws.write(row, 4, r.picked.clone().unwrap_or_default());
        let _ = ws.write(row, 5, q.map(|q| q.answer.as_str()).unwrap_or(""));
        let _ = ws.write(row, 6, match r.is_correct { Some(true) => "✓", Some(false) => "✗", None => "不计分" });
        let _ = ws.write(row, 7, q.map(|q| q.explain.as_str()).unwrap_or(""));
        let _ = ws.write(row, 8, q.map(|q| q.source.as_str()).unwrap_or(""));
    }
    let _ = ws.set_column_width(3, 50); // 题干列宽
    let _ = ws.set_column_width(7, 50);
    wb.save(path).map_err(|e| e.to_string())?;
    Ok(path.to_string())
}

// ---------- Tauri 命令 ----------
pub mod commands {
    use super::{excel_preview as excel_preview_impl, export_session_excel as export_session_excel_impl,
                excel_import_into, export_template, ExcelPreview, ExcelImportReport};
    use crate::telemetry::timed;
    use crate::AppState;

    #[tauri::command]
    pub async fn excel_preview(path: String) -> Result<ExcelPreview, String> {
        timed("excel_preview", false, || excel_preview_impl(&path))
    }

    #[tauri::command]
    pub async fn excel_import(state: tauri::State<'_, AppState>, path: String, bank_name: String) -> Result<ExcelImportReport, String> {
        timed("excel_import", false, || {
            let b = state.bank.lock().map_err(|e| e.to_string())?;
            excel_import_into(&b, &path, &bank_name)
        })
    }

    #[tauri::command]
    pub async fn export_excel_template(path: String) -> Result<String, String> {
        timed("export_excel_template", false, || export_template(&path))
    }

    #[tauri::command]
    pub async fn export_session_excel(state: tauri::State<'_, AppState>, session_id: i64, path: String) -> Result<String, String> {
        timed("export_session_excel", false, || {
            let b = state.bank.lock().map_err(|e| e.to_string())?;
            let u = state.user.lock().map_err(|e| e.to_string())?;
            export_session_excel_impl(&u, &b, session_id, &path)
        })
    }
}
