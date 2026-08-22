// 蓝图组卷（设计方案 V1.1 §4.1：候选池校验 → 降级 → 跨section去重）
use rusqlite::Connection;
use serde::{Deserialize, Serialize};

// JSON 里另有 name/time_limit_min 字段，由前端自行使用（会话标题/限时），serde 默认忽略未知字段
#[derive(Deserialize)]
pub struct Blueprint {
    #[serde(default)]
    pub sections: Vec<SectionSpec>,
    #[serde(default)]
    pub allow_fallback: bool, // false=不足即报错；true=按策略降级
}

#[derive(Deserialize)]
pub struct SectionSpec {
    #[serde(rename = "type")]
    pub qtype: String,               // single/multi
    pub qty: i64,
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
    let sql = format!(
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

// ---------- Tauri 命令 ----------
pub mod commands {
    use super::*;
    use crate::telemetry::timed;
    use crate::AppState;

    #[tauri::command]
    pub async fn compose_blueprint(state: tauri::State<'_, AppState>, blueprint: Blueprint) -> Result<ComposeReport, String> {
        timed("compose_blueprint", false, || {
            let b = state.bank.lock().map_err(|e| e.to_string())?;
            compose(&b, &blueprint)
        })
    }
}
