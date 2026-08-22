// M3：Excel题库导入向导 / 去重扫描与合并 / 试卷打印数据
use crate::bank;
use calamine::Reader;
use rusqlite::{params, Connection};
use serde::Serialize;
use std::collections::HashMap;

// ==================== Excel 导入 ====================
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
    Ok(ExcelImportReport { bank_id, bank_name: bank_name.into(), imported: qs.len(), skipped: errs.len(), topics: n_topics, errors: errs })
}

// ==================== 去重扫描与合并 ====================
#[derive(Serialize)]
pub struct DupItem { pub qid: String, pub stem: String, pub status: String }
#[derive(Serialize)]
pub struct DupGroup { pub kind: String /* exact|similar */, pub items: Vec<DupItem> }

fn norm_stem(s: &str) -> String {
    s.chars().filter(|c| c.is_alphanumeric()).collect::<String>().to_lowercase()
}

/// 64位 SimHash（字符二元组特征 + FNV-1a）
fn simhash64(text: &str) -> u64 {
    let cs: Vec<char> = text.chars().collect();
    if cs.is_empty() { return 0; }
    let mut feats: Vec<u64> = Vec::new();
    for i in 0..cs.len().saturating_sub(1) {
        let g: u64 = (cs[i] as u64) << 32 | cs[i + 1] as u64;
        feats.push(fnv1a(g));
    }
    if cs.len() == 1 { feats.push(fnv1a(cs[0] as u64)); }
    let mut bits = [0i64; 64];
    for f in feats {
        for b in 0..64 { bits[b] += if (f >> b) & 1 == 1 { 1 } else { -1 }; }
    }
    let mut h: u64 = 0;
    for b in 0..64 { if bits[b] > 0 { h |= 1 << b; } }
    h
}
fn fnv1a(mut x: u64) -> u64 {
    let mut hash: u64 = 0xcbf29ce484222325;
    for _ in 0..8 { hash ^= x & 0xFF; hash = hash.wrapping_mul(0x100000001b3); x >>= 8; }
    hash
}
fn hamming(a: u64, b: u64) -> u32 { (a ^ b).count_ones() }

pub fn dedup_scan(bankconn: &Connection, bank_id: &str) -> Result<Vec<DupGroup>, String> {
    let mut stmt = bankconn.prepare("SELECT qid,stem,status FROM questions WHERE bank_id=?1").map_err(|e| e.to_string())?;
    let rows: Vec<(String, String, String)> = stmt.query_map(params![bank_id], |r|
        Ok((r.get(0)?, r.get(1)?, r.get(2)?))).map_err(|e| e.to_string())?.filter_map(|x| x.ok()).collect();
    drop(stmt);
    // 精确重复
    let mut by_norm: HashMap<String, Vec<usize>> = HashMap::new();
    for (i, (_, s, _)) in rows.iter().enumerate() {
        by_norm.entry(norm_stem(s)).or_default().push(i);
    }
    let mut groups = Vec::new();
    let mut grouped = vec![false; rows.len()];
    for idxs in by_norm.values() {
        if idxs.len() > 1 {
            idxs.iter().for_each(|i| grouped[*i] = true);
            groups.push(DupGroup { kind: "exact".into(),
                items: idxs.iter().map(|i| DupItem { qid: rows[*i].0.clone(), stem: rows[*i].1.clone(), status: rows[*i].2.clone() }).collect() });
        }
    }
    // 相似（SimHash 海明距 ≤3），排除已分组的
    let hashes: Vec<(usize, u64)> = rows.iter().enumerate()
        .filter(|(i, _)| !grouped[*i])
        .map(|(i, (_, s, _))| (i, simhash64(&norm_stem(s)))).collect();
    let mut used = vec![false; hashes.len()];
    for i in 0..hashes.len() {
        if used[i] { continue; }
        let mut cluster = vec![i];
        for j in (i + 1)..hashes.len() {
            if !used[j] && hamming(hashes[i].1, hashes[j].1) <= 3 { cluster.push(j); used[j] = true; }
        }
        if cluster.len() > 1 {
            used[i] = true;
            groups.push(DupGroup { kind: "similar".into(),
                items: cluster.iter().map(|k| { let r = hashes[*k].0; DupItem { qid: rows[r].0.clone(), stem: rows[r].1.clone(), status: rows[r].2.clone() } }).collect() });
        }
    }
    Ok(groups)
}

/// 合并：保留 keep，其余 remove——试卷/章节/用户数据重指向 + qid_mapping 留痕
pub fn dedup_merge(bankconn: &Connection, userconn: &Connection, bank_id: &str, keep: &str, removes: &[String]) -> Result<usize, String> {
    let now = chrono::Utc::now().to_rfc3339();
    let tx = bankconn.unchecked_transaction().map_err(|e| e.to_string())?;
    let utx = userconn.unchecked_transaction().map_err(|e| e.to_string())?;
    for rm in removes {
        // 试卷引用迁移（OR REPLACE 自动处理同卷同题唯一键冲突）
        tx.execute("UPDATE OR REPLACE paper_questions SET qid=?1 WHERE bank_id=?2 AND qid=?3", params![keep, bank_id, rm]).map_err(|e| e.to_string())?;
        // 章节关联迁移
        tx.execute("INSERT OR IGNORE INTO question_topics(bank_id,qid,topic_id) SELECT bank_id,?1,topic_id FROM question_topics WHERE bank_id=?2 AND qid=?3",
            params![keep, bank_id, rm]).map_err(|e| e.to_string())?;
        tx.execute("DELETE FROM question_topics WHERE bank_id=?1 AND qid=?2", params![bank_id, rm]).map_err(|e| e.to_string())?;
        // 用户数据迁移（错题次数累加）
        utx.execute("INSERT INTO wrong_book(bank_id,qid,wrong_count,last_wrong_at)
                     SELECT bank_id,?1,COALESCE((SELECT wrong_count FROM wrong_book WHERE bank_id=?2 AND qid=?1),0)+wrong_count,MAX(last_wrong_at) FROM wrong_book WHERE bank_id=?2 AND qid=?3
                     ON CONFLICT(bank_id,qid) DO UPDATE SET wrong_count=excluded.wrong_count",
            params![keep, bank_id, rm]).map_err(|e| e.to_string())?;
        utx.execute("DELETE FROM wrong_book WHERE bank_id=?1 AND qid=?2", params![bank_id, rm]).map_err(|e| e.to_string())?;
        utx.execute("INSERT OR IGNORE INTO favorites(bank_id,qid,created_at) SELECT bank_id,?1,created_at FROM favorites WHERE bank_id=?2 AND qid=?3",
            params![keep, bank_id, rm]).map_err(|e| e.to_string())?;
        utx.execute("DELETE FROM favorites WHERE bank_id=?1 AND qid=?2", params![bank_id, rm]).map_err(|e| e.to_string())?;
        // review_queue：保留进度较新的
        let rm_due: Option<String> = utx.query_row("SELECT due_date FROM review_queue WHERE bank_id=?1 AND qid=?2", params![bank_id, rm], |r| r.get(0)).ok();
        if rm_due.is_some() {
            let keep_exists: bool = utx.query_row("SELECT 1 FROM review_queue WHERE bank_id=?1 AND qid=?2", params![bank_id, keep], |_| Ok(true)).unwrap_or(false);
            if !keep_exists {
                utx.execute("INSERT INTO review_queue(bank_id,qid,ease,interval_days,repetitions,due_date) SELECT bank_id,?1,ease,interval_days,repetitions,due_date FROM review_queue WHERE bank_id=?2 AND qid=?3",
                    params![keep, bank_id, rm]).map_err(|e| e.to_string())?;
            }
            utx.execute("DELETE FROM review_queue WHERE bank_id=?1 AND qid=?2", params![bank_id, rm]).map_err(|e| e.to_string())?;
        }
        utx.execute("INSERT OR IGNORE INTO notes(bank_id,qid,content,updated_at) SELECT bank_id,?1,content,updated_at FROM notes WHERE bank_id=?2 AND qid=?3",
            params![keep, bank_id, rm]).map_err(|e| e.to_string())?;
        utx.execute("DELETE FROM notes WHERE bank_id=?1 AND qid=?2", params![bank_id, rm]).map_err(|e| e.to_string())?;
        // 作答记录重指向（保留统计连续性；qid_mapping 留痕可追溯）
        utx.execute("UPDATE answer_records SET qid=?1 WHERE bank_id=?2 AND qid=?3", params![keep, bank_id, rm]).map_err(|e| e.to_string())?;
        // 留痕
        tx.execute("INSERT OR REPLACE INTO qid_mapping(old_bank,old_qid,new_bank,new_qid,mapped_at) VALUES(?,?,?,?,?)",
            params![bank_id, rm, bank_id, keep, now]).map_err(|e| e.to_string())?;
        tx.execute("DELETE FROM questions WHERE bank_id=?1 AND qid=?2", params![bank_id, rm]).map_err(|e| e.to_string())?;
    }
    tx.commit().map_err(|e| e.to_string())?;
    utx.commit().map_err(|e| e.to_string())?;
    Ok(removes.len())
}

// ==================== 试卷打印数据 ====================
#[derive(Serialize)]
pub struct PrintPaper {
    pub name: String, pub title: String, pub total_score: f64, pub total_count: usize,
    pub sections: Vec<PrintSection>,
}
#[derive(Serialize)]
pub struct PrintSection {
    pub qtype: String, pub score_each: f64,
    pub questions: Vec<bank::QuestionRow>,
}

pub fn paper_print_data(bankconn: &Connection, paper_id: i64) -> Result<PrintPaper, String> {
    let (name, title): (String, String) = bankconn.query_row(
        "SELECT name, COALESCE(description,'') FROM papers WHERE paper_id=?1", params![paper_id],
        |r| Ok((r.get(0)?, r.get(1)?))).map_err(|e| e.to_string())?;
    let qids = bank::paper_qids(bankconn, paper_id)?;
    let questions = bank::get_questions_by_ids(bankconn, &qids)?;
    let mut sections: Vec<PrintSection> = Vec::new();
    for q in questions {
        if let Some(s) = sections.iter_mut().find(|s| s.qtype == q.qtype) { s.questions.push(q); }
        else { sections.push(PrintSection { qtype: q.qtype.clone(), score_each: 1.0, questions: vec![q] }); }
    }
    let total_count = sections.iter().map(|s| s.questions.len()).sum();
    let total_score: f64 = sections.iter().map(|s| s.questions.len() as f64 * s.score_each).sum();
    Ok(PrintPaper { name, title, total_score, total_count, sections })
}

// ==================== 测试 ====================
#[cfg(test)]
mod tests {
    use super::*;

    fn write_test_xlsx(path: &str) {
        let mut wb = rust_xlsxwriter::Workbook::new();
        let ws = wb.add_worksheet();
        for (c, h) in TEMPLATE_HEADERS.iter().enumerate() { let _ = ws.write(0, c as u16, *h); }
        let rows: Vec<Vec<&str>> = vec![
            vec!["标准型CPU最多几个EM？", "单选", "3", "6", "", "", "", "", "B", "SR/ST最多6个", "选型手册", "硬件", "", "3", "high"],
            vec!["通信需要设置哪些参数？", "多选", "波特率", "校验", "IP", "", "", "", "A,B,C", "IP属于以太网", "技术参考", "通信", "串口", "3", "high"],
            vec!["此题答案越界", "单选", "x", "y", "", "", "", "", "D", "", "", "", "", "", ""],
            vec!["此题题干为空", "", "", "", "", "", "", "", "", "", "", "", "", "", ""],
            vec!["SR是继电器输出", "判断", "", "", "", "", "", "", "√", "", "", "硬件", "", "2", "medium"],
            vec!["标准型cpu最多几个EM?", "单选", "3", "6", "", "", "", "", "B", "", "", "硬件", "", "3", "high"], // 大小写/标点不同→归一化后与第1题精确重复
        ];
        for (r, row) in rows.iter().enumerate() {
            for (c, v) in row.iter().enumerate() { let _ = ws.write((r + 1) as u32, c as u16, *v); }
        }
        wb.save(path).unwrap();
    }

    #[test]
    fn excel_import_and_dedup() {
        let tmp = std::env::temp_dir().join(format!("sqm3-{}", uuid::Uuid::new_v4()));
        std::fs::create_dir_all(&tmp).unwrap();
        let bankconn = crate::db::open(&tmp.join("bank.db")).unwrap();
        let userconn = crate::db::open_user(&tmp.join("user.db")).unwrap();
        let xlsx = tmp.join("t.xlsx");
        write_test_xlsx(xlsx.to_str().unwrap());
        export_template(tmp.join("模板.xlsx").to_str().unwrap()).unwrap();

        // 预览
        let pv = excel_preview(xlsx.to_str().unwrap()).unwrap();
        assert_eq!(pv.valid, 4); // 4有效（1/2/5/6行）
        assert_eq!(pv.errors.len(), 2); // 越界+空题干
        assert!(pv.errors.iter().any(|e| e.msg.contains("超出选项范围")));

        // 导入
        let rep = excel_import_into(&bankconn, xlsx.to_str().unwrap(), "我的Excel题库").unwrap();
        assert_eq!(rep.imported, 4);
        assert!(rep.bank_id.starts_with("xlsx-"));
        let ov = bank::overview(&bankconn).unwrap();
        assert_eq!(ov.banks.len(), 1);
        // 判断题选项自动生成
        let judge = bank::list_questions(&bankconn, None, Some("judge".into()), None, None, 10, 0).unwrap();
        assert_eq!(judge.len(), 1);
        assert_eq!(judge[0].answer, "T");

        // 去重扫描：第6行与第1行归一化后精确重复
        let groups = dedup_scan(&bankconn, &rep.bank_id).unwrap();
        assert!(!groups.is_empty(), "应检出重复组");
        assert_eq!(groups[0].kind, "exact");

        // 合并（保留 X0000）
        let similar: Vec<&DupGroup> = groups.iter().collect();
        let g = similar[0];
        let keep = &g.items[0].qid;
        let removes: Vec<String> = g.items[1..].iter().map(|i| i.qid.clone()).collect();
        if !removes.is_empty() {
            let n = dedup_merge(&bankconn, &userconn, &rep.bank_id, keep, &removes).unwrap();
            assert_eq!(n, removes.len());
            let after = bank::list_questions(&bankconn, None, None, None, None, 100, 0).unwrap();
            assert_eq!(after.len(), 4 - removes.len());
        }
        std::fs::remove_dir_all(&tmp).ok();
    }

    #[test]
    fn simhash_sanity() {
        // 与 dedup_scan 一致：先归一化再 simhash
        let a = simhash64(&norm_stem("s7200smart标准型cpu最多扩展6个模块"));
        let b = simhash64(&norm_stem("s7200smart标准型CPU最多扩展6个模块！"));
        let c = simhash64(&norm_stem("modbusrtu通信需要设置波特率与校验位"));
        assert_eq!(a, b, "归一化后相同文本应完全一致");
        assert!(hamming(a, c) > 10, "无关文本不应相似");
    }

    #[test]
    fn print_data() {
        let tmp = std::env::temp_dir().join(format!("sqprint-{}", uuid::Uuid::new_v4()));
        std::fs::create_dir_all(&tmp).unwrap();
        let bankconn = crate::db::open(&tmp.join("bank.db")).unwrap();
        let seed = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("resources/seed/smart-core.smartbank");
        bank::import(&bankconn, &seed, &tmp.join("banks"), true).unwrap();
        let p = paper_print_data(&bankconn, 1).unwrap();
        assert!(p.total_count > 0);
        assert_eq!(p.sections.len(), 2); // 单选+多选
        assert!(p.sections[0].questions.len() >= 40);
        std::fs::remove_dir_all(&tmp).ok();
    }
}
