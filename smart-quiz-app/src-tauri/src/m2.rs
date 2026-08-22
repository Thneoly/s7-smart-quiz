// M2：蓝图组卷 / Excel导出 / 活动日历 / 备份恢复 / 诊断包 / 设置
use rusqlite::{params, Connection};
use serde::{Deserialize, Serialize};
use std::io::{Read as _, Write as _};
use std::path::Path;

// ---------- 蓝图组卷（V1.1 §4.1：候选池校验 → 降级 → 跨section去重） ----------
#[derive(Deserialize)]
pub struct Blueprint {
    pub name: String,
    #[serde(default = "default_min")]
    pub time_limit_min: i64,
    #[serde(default)]
    pub sections: Vec<SectionSpec>,
    #[serde(default)]
    pub allow_fallback: bool, // false=不足即报错；true=按策略降级
}
fn default_min() -> i64 { 90 }

#[derive(Deserialize)]
pub struct SectionSpec {
    #[serde(rename = "type")]
    pub qtype: String,               // single/multi
    pub qty: i64,
    #[serde(default)]
    pub score_each: f64,
    #[serde(default)]
    pub from_topics: Vec<i64>,       // topic_id，空=全部
    #[serde(default)]
    pub difficulty: Option<(i64, i64)>,
}

#[derive(Serialize)]
pub struct ComposeReport {
    pub sections: Vec<SectionReport>,
    pub total: usize,
    pub qids: Vec<(String, String)>,
}
#[derive(Serialize)]
pub struct SectionReport {
    pub qtype: String, pub requested: i64, pub actual: i64,
    pub fallback: Option<String>,
}

pub fn compose(conn: &Connection, bp: &Blueprint) -> Result<ComposeReport, String> {
    for s in &bp.sections {
        if !["single", "multi"].contains(&s.qtype.as_str()) {
            return Err(format!("非法题型 {}", s.qtype));
        }
    }
    let mut used: Vec<(String, String)> = Vec::new();
    let mut reports = Vec::new();
    for s in &bp.sections {
        // 候选池计数（active 状态、可选主题/难度过滤）
        let (mut sql, mut args): (String, Vec<Box<dyn rusqlite::types::ToSql>>) = (
            "SELECT COUNT(*) FROM questions q WHERE q.bank_id=(SELECT bank_id FROM banks WHERE is_enabled=1 ORDER BY is_builtin DESC LIMIT 1) AND q.status='active' AND q.answer_conf='high' AND q.type=?1".into(),
            vec![Box::new(s.qtype.clone())]);
        if !s.from_topics.is_empty() {
            sql.push_str(&format!(" AND q.qid IN (SELECT qid FROM question_topics WHERE topic_id IN ({}))",
                (0..s.from_topics.len()).map(|i| format!("?{}", i + 2)).collect::<Vec<_>>().join(",")));
            for t in &s.from_topics { args.push(Box::new(*t)); }
        }
        if let Some((lo, hi)) = s.difficulty {
            sql.push_str(&format!(" AND q.difficulty BETWEEN ?{} AND ?{}", args.len() + 1, args.len() + 2));
            args.push(Box::new(lo)); args.push(Box::new(hi));
        }
        let mut st = conn.prepare(&sql).map_err(|e| e.to_string())?;
        let refs: Vec<&dyn rusqlite::types::ToSql> = args.iter().map(|b| b.as_ref()).collect();
        let pool: i64 = st.query_row(refs.as_slice(), |r| r.get(0)).map_err(|e| e.to_string())?;

        let mut fallback = None;
        let mut actual_qty = s.qty;
        let mut relax_diff = false;
        if pool < s.qty {
            if !bp.allow_fallback {
                return Err(format!("候选不足：题型{} 需要{}题，可用{}题。可在组卷设置中开启降级策略", s.qtype, s.qty, pool));
            }
            if pool >= s.qty {
                unreachable!()
            } else {
                // 先放宽难度重试
                if s.difficulty.is_some() {
                    relax_diff = true;
                    let cnt = count_pool(conn, &s.qtype, &s.from_topics, None)?;
                    if cnt >= s.qty { fallback = Some("已放宽难度范围".into()); }
                }
                if fallback.is_none() {
                    actual_qty = pool;
                    fallback = Some(format!("题量 {}→{}", s.qty, pool));
                }
            }
        }
        // 抽题（排除已选）
        let mut sql = format!(
            "SELECT q.bank_id,q.qid FROM questions q WHERE q.bank_id=(SELECT bank_id FROM banks WHERE is_enabled=1 ORDER BY is_builtin DESC LIMIT 1) AND q.status='active' AND q.answer_conf='high' AND q.type=?1{}{}",
            if relax_diff { String::new() } else { diff_clause(s.difficulty) },
            if s.from_topics.is_empty() { String::new() } else {
                format!(" AND q.qid IN (SELECT qid FROM question_topics WHERE topic_id IN ({}))",
                    s.from_topics.iter().enumerate().map(|(i, _)| format!("?{}", i + 2)).collect::<Vec<_>>().join(","))
            });
        sql.push_str(&format!(" AND q.qid NOT IN (SELECT value FROM json_each(?{})) ORDER BY RANDOM() LIMIT ?{}",
            (if s.from_topics.is_empty() { 1 } else { s.from_topics.len() + 1 }) + 1,
            (if s.from_topics.is_empty() { 1 } else { s.from_topics.len() + 1 }) + 2));
        let mut args: Vec<Box<dyn rusqlite::types::ToSql>> = vec![Box::new(s.qtype.clone())];
        for t in &s.from_topics { args.push(Box::new(*t)); }
        let used_json = serde_json::to_string(&used.iter().map(|(_, q)| q).collect::<Vec<_>>()).unwrap();
        args.push(Box::new(used_json));
        args.push(Box::new(actual_qty));
        let mut st = conn.prepare(&sql).map_err(|e| e.to_string())?;
        let refs: Vec<&dyn rusqlite::types::ToSql> = args.iter().map(|b| b.as_ref()).collect();
        let picked: Vec<(String, String)> = st.query_map(refs.as_slice(), |r| Ok((r.get(0)?, r.get(1)?)))
            .map_err(|e| e.to_string())?.filter_map(|x| x.ok()).collect();
        if (picked.len() as i64) < actual_qty {
            return Err(format!("组卷抽题异常：需{}实得{}", actual_qty, picked.len()));
        }
        used.extend(picked.clone());
        reports.push(SectionReport { qtype: s.qtype.clone(), requested: s.qty, actual: picked.len() as i64, fallback });
    }
    Ok(ComposeReport { sections: reports, total: used.len(), qids: used })
}

fn diff_clause(d: Option<(i64, i64)>) -> String {
    match d { Some((lo, hi)) => format!(" AND q.difficulty BETWEEN {lo} AND {hi}"), None => String::new() }
}

fn count_pool(conn: &Connection, qtype: &str, topics: &[i64], diff: Option<(i64, i64)>) -> Result<i64, String> {
    let mut sql = format!(
        "SELECT COUNT(*) FROM questions q WHERE q.bank_id=(SELECT bank_id FROM banks WHERE is_enabled=1 ORDER BY is_builtin DESC LIMIT 1) AND q.status='active' AND q.answer_conf='high' AND q.type=?1{}{}",
        diff_clause(diff),
        if topics.is_empty() { String::new() } else {
            format!(" AND q.qid IN (SELECT qid FROM question_topics WHERE topic_id IN ({}))",
                topics.iter().enumerate().map(|(i, _)| format!("?{}", i + 2)).collect::<Vec<_>>().join(","))
        });
    let mut args: Vec<Box<dyn rusqlite::types::ToSql>> = vec![Box::new(qtype.to_string())];
    for t in topics { args.push(Box::new(*t)); }
    let mut st = conn.prepare(&sql).map_err(|e| e.to_string())?;
    let refs: Vec<&dyn rusqlite::types::ToSql> = args.iter().map(|b| b.as_ref()).collect();
    st.query_row(refs.as_slice(), |r| r.get(0)).map_err(|e| e.to_string())
}

// ---------- 活动日历（打卡热力图数据） ----------
#[derive(Serialize)]
pub struct DayCount { pub date: String, pub count: i64 }

pub fn activity(user: &Connection, days: i64) -> Result<Vec<DayCount>, String> {
    let mut stmt = user.prepare(
        "SELECT date(answered_at) d, COUNT(*) FROM answer_records GROUP BY d ORDER BY d DESC LIMIT ?1")
        .map_err(|e| e.to_string())?;
    let rows: Vec<(String, i64)> = stmt.query_map(params![days], |r| Ok((r.get(0)?, r.get(1)?)))
        .map_err(|e| e.to_string())?.filter_map(|x| x.ok()).collect();
    Ok(rows.into_iter().map(|(date, count)| DayCount { date, count }).collect())
}

// ---------- 成绩单 Excel 导出 ----------
pub fn export_session_excel(user: &Connection, bankconn: &Connection, session_id: i64, path: &str) -> Result<String, String> {
    let detail = crate::user::session_detail(user, bankconn, session_id)?;
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

// ---------- 备份 / 恢复 / 诊断包 ----------
#[derive(Serialize)]
pub struct RestoreInfo { pub sessions: i64, pub records: i64, pub created_at: String }

pub fn backup_user(user: &Connection, dest: &str) -> Result<String, String> {
    let p = Path::new(dest);
    if p.extension().and_then(|e| e.to_str()) == Some("zip") {
        // 一致性快照后打 zip：manifest + snapshot.db
        let tmp = std::env::temp_dir().join(format!("sqbackup-{}.db", uuid::Uuid::new_v4()));
        _backup_into(user, &tmp)?;
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
        _backup_into(user, p)?;
    }
    Ok(dest.to_string())
}
fn _backup_into(user: &Connection, dest: &Path) -> Result<(), String> {
    // VACUUM INTO：WAL 运行态下的一致性快照（V1.1 §3.2）
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

pub fn diagnostics(user: &Connection, dest: &str) -> Result<String, String> {
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
        }
    });
    z.start_file("diagnostics.json", opt).map_err(|e| e.to_string())?;
    z.write_all(serde_json::to_string_pretty(&info).unwrap().as_bytes()).map_err(|e| e.to_string())?;
    z.start_file("privacy.txt", opt).map_err(|e| e.to_string())?;
    z.write_all(b"This diagnostics package contains NO personal data.\nOnly app version, OS and local counts.\n").map_err(|e| e.to_string())?;
    z.finish().map_err(|e| e.to_string())?;
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

// ---------- 单测 ----------
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn compose_with_fallback() {
        let tmp = std::env::temp_dir().join(format!("sqm2-{}", uuid::Uuid::new_v4()));
        std::fs::create_dir_all(&tmp).unwrap();
        let bankconn = crate::db::open(&tmp.join("bank.db")).unwrap();
        let user = crate::db::open_user(&tmp.join("user.db")).unwrap();
        let seed = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("resources/seed/smart-core.smartbank");
        crate::bank::import(&bankconn, &seed, &tmp.join("banks"), true).unwrap();

        // 1) 正常组卷：40单+10多（种子库 active&high 足够）
        let bp = Blueprint { name: "全真".into(), time_limit_min: 90, allow_fallback: false, sections: vec![
            SectionSpec { qtype: "single".into(), qty: 40, score_each: 1.0, from_topics: vec![], difficulty: None },
            SectionSpec { qtype: "multi".into(), qty: 10, score_each: 1.0, from_topics: vec![], difficulty: None }] };
        let r = compose(&bankconn, &bp).unwrap();
        assert_eq!(r.total, 50);
        assert_eq!(r.sections[0].actual, 40);
        assert_eq!(r.sections[1].actual, 10);
        // 跨 section 去重
        let mut set = std::collections::HashSet::new();
        for q in &r.qids { assert!(set.insert(q.clone()), "出现重复题"); }

        // 2) 不足+不允许降级 → 报错
        let bp2 = Blueprint { name: "过量".into(), time_limit_min: 90, allow_fallback: false, sections: vec![
            SectionSpec { qtype: "single".into(), qty: 10000, score_each: 1.0, from_topics: vec![], difficulty: None }] };
        assert!(compose(&bankconn, &bp2).is_err());

        // 3) 不足+允许降级 → 降题量并报告
        let bp3 = Blueprint { name: "降级".into(), time_limit_min: 90, allow_fallback: true, sections: vec![
            SectionSpec { qtype: "single".into(), qty: 10000, score_each: 1.0, from_topics: vec![], difficulty: None }] };
        let r3 = compose(&bankconn, &bp3).unwrap();
        assert!(r3.sections[0].actual < 10000);
        assert!(r3.sections[0].fallback.is_some());

        // 4) 活动日历 & 导出 & 备份链路
        let qids: Vec<(String, String)> = r.qids.clone();
        let s = crate::user::start_session(&user, "exam", "组卷测试", "smart-core", None, &qids, Some(60)).unwrap();
        crate::user::finish_session(&user, &bankconn, s.session_id).unwrap();
        let act = activity(&user, 30).unwrap();
        assert!(!act.is_empty());
        let xlsx = tmp.join("out.xlsx");
        export_session_excel(&user, &bankconn, s.session_id, xlsx.to_str().unwrap()).unwrap();
        assert!(xlsx.exists());
        let bk = tmp.join("bk.zip");
        backup_user(&user, bk.to_str().unwrap()).unwrap();
        let ri = restore_check(bk.to_str().unwrap()).unwrap();
        assert_eq!(ri.sessions, 1);
        std::fs::remove_dir_all(&tmp).ok();
    }
}
