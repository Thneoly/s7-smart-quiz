// 会话域：作答/判分/SM-2/错题本/收藏/笔记/仪表盘/活动日历（设计方案 V1.1 §3.2/§4）
// 判分与 SM-2 的唯一权威实现在 Rust（交卷重算 = 信任边界；TS 端只做即时反馈展示）
use crate::bank;
use rusqlite::{params, Connection};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

const SRS_MODES: &[&str] = &["practice", "random", "review", "wrong", "fav"]; // 考试/背诵不驱动状态机

// ---------- 类型 ----------
#[derive(Serialize)]
pub struct SessionInfo {
    pub session_id: i64, pub mode: String, pub title: String,
    pub time_limit_sec: Option<i64>, pub total_qty: i64,
    pub started_at: String, pub finished_at: Option<String>,
    pub score: Option<f64>, pub correct_qty: i64, pub scored_qty: i64,
    pub qid_list: Vec<(String, String)>,
    pub draft: Option<serde_json::Value>,
}

#[derive(Serialize)]
pub struct SessionDetail {
    pub session: SessionInfo,
    pub records: Vec<AnswerRow>,
}

#[derive(Serialize)]
pub struct AnswerRow {
    pub bank_id: String, pub qid: String, pub picked: Option<String>,
    pub is_correct: Option<bool>, pub time_cost_ms: Option<i64>,
    pub question: Option<bank::QuestionRow>,
}

#[derive(Serialize)]
pub struct Dashboard {
    pub answered: i64, pub correct: i64, pub sessions_done: i64,
    pub streak_days: i64, pub due_count: i64, pub wrong_active: i64,
    pub by_topic: Vec<TopicStat>, pub recent: Vec<SessionBrief>,
}
#[derive(Serialize)]
pub struct TopicStat { pub topic: String, pub a: i64, pub c: i64 }
#[derive(Serialize)]
pub struct SessionBrief { pub session_id: i64, pub mode: String, pub title: String, pub score: Option<f64>, pub correct_qty: i64, pub scored_qty: i64, pub finished_at: Option<String>, pub duration_ms: Option<i64> }

#[derive(Serialize)]
pub struct WrongRow {
    pub bank_id: String, pub qid: String, pub wrong_count: i64,
    pub last_wrong_at: String, pub repetitions: i64, pub due_date: Option<String>,
    pub question: bank::QuestionRow,
}

// ---------- 判分（all_or_nothing，决策附录A-1） ----------
pub fn norm_letters(s: &str) -> String {
    let mut v: Vec<char> = s.chars().filter(|c| ('A'..='E').contains(c)).collect();
    v.sort(); v.dedup(); v.into_iter().collect()
}

pub fn grade(qtype: &str, answer: &str, picked: &str, policy: &str) -> Option<bool> {
    let (a, p) = (norm_letters(answer), norm_letters(picked));
    if a.is_empty() { return None; }                    // 无答案/低置信 → 不计分
    if p.is_empty() { return Some(false); }             // 未作答 → 错
    Some(match qtype {
        "multi" => match policy {
            "partial" => { /* 漏选得半：is_correct 仍为全对布尔，分数由 finish 调整 */ p == a }
            _ => p == a,                                // all_or_nothing（默认）
        },
        _ => p == a,
    })
}

// ---------- SM-2（修正版状态机，V1.1 §4.2；消灭判据 repetitions>=2） ----------
#[derive(Clone, Copy)]
struct SrsState { ease: f64, interval: f64, rep: i64 }

fn srs_step(s: SrsState, correct: bool) -> SrsState {
    if correct {
        let rep = s.rep + 1;
        let interval = match rep { 1 => 1.0, 2 => 6.0, _ => s.interval.max(1.0) * s.ease };
        SrsState { ease: s.ease, interval, rep }
    } else {
        SrsState { ease: (s.ease - 0.2).max(1.3), interval: 1.0, rep: 0 }
    }
}

fn srs_apply(conn: &Connection, bank_id: &str, qid: &str, correct: bool) -> Result<(), String> {
    let cur = conn.query_row(
        "SELECT COALESCE(ease,2.5),COALESCE(interval_days,0),COALESCE(repetitions,0) FROM review_queue WHERE bank_id=?1 AND qid=?2",
        params![bank_id, qid],
        |r| Ok(SrsState { ease: r.get(0)?, interval: r.get(1)?, rep: r.get(2)? }),
    ).unwrap_or(SrsState { ease: 2.5, interval: 0.0, rep: 0 });
    let nx = srs_step(cur, correct);
    let due = (chrono::Utc::now() + chrono::Duration::seconds((nx.interval * 86400.0) as i64)).to_rfc3339();
    conn.execute("INSERT INTO review_queue(bank_id,qid,ease,interval_days,repetitions,due_date) VALUES(?,?,?,?,?,?)
                  ON CONFLICT(bank_id,qid) DO UPDATE SET ease=excluded.ease,interval_days=excluded.interval_days,repetitions=excluded.repetitions,due_date=excluded.due_date",
        params![bank_id, qid, nx.ease, nx.interval, nx.rep, due]).map_err(|e| e.to_string())?;
    Ok(())
}

// ---------- 会话 ----------
#[derive(Deserialize, Serialize, Clone)]
pub struct DraftPick { pub picked: String, pub t: Option<i64> }
#[derive(Deserialize, Serialize, Clone, Default)]
pub struct Draft { pub picks: HashMap<String, DraftPick>, pub marks: HashMap<String, bool>, pub remaining_sec: Option<i64>, pub idx: Option<i64> }

fn draft_key(bank_id: &str, qid: &str) -> String { format!("{bank_id}::{qid}") }

pub fn start_session(user: &Connection, mode: &str, title: &str, bank_id: &str,
                     paper_id: Option<i64>, qids: &[(String, String)], time_limit_sec: Option<i64>) -> Result<SessionInfo, String> {
    if !["practice", "random", "recite", "review", "wrong", "fav", "exam"].contains(&mode) {
        return Err(format!("非法模式 {mode}"));
    }
    let now = chrono::Utc::now().to_rfc3339();
    let qid_json = serde_json::to_string(qids).unwrap();
    user.execute("INSERT INTO sessions(mode,title,bank_id,paper_id,time_limit_sec,total_qty,started_at,qid_list,draft)
                  VALUES(?,?,?,?,?,?,?,?,?)",
        params![mode, title, bank_id, paper_id, time_limit_sec, qids.len() as i64, now, qid_json, "{}"])
        .map_err(|e| e.to_string())?;
    // 隐私：不记会话标题（组卷卷名属用户自由输入，日志随诊断包外发）
    log::info!(target: "session", "开始会话#{} {mode} {}题", user.last_insert_rowid(), qids.len());
    get_session(user, user.last_insert_rowid())
}

pub fn save_draft(user: &Connection, session_id: i64, draft: &Draft) -> Result<(), String> {
    let j = serde_json::to_string(draft).unwrap();
    user.execute("UPDATE sessions SET draft=?1 WHERE session_id=?2 AND finished_at IS NULL",
        params![j, session_id]).map_err(|e| e.to_string())?;
    Ok(())
}

fn get_session(user: &Connection, id: i64) -> Result<SessionInfo, String> {
    user.query_row("SELECT session_id,mode,title,time_limit_sec,total_qty,started_at,finished_at,score,correct_qty,scored_qty,qid_list,draft FROM sessions WHERE session_id=?1",
        params![id], |r| {
            let ql: String = r.get(10)?;
            Ok(SessionInfo {
                session_id: r.get(0)?, mode: r.get(1)?, title: r.get(2)?,
                time_limit_sec: r.get(3)?, total_qty: r.get(4)?, started_at: r.get(5)?,
                finished_at: r.get(6)?, score: r.get(7)?, correct_qty: r.get(8)?, scored_qty: r.get(9)?,
                qid_list: serde_json::from_str(&ql).unwrap_or_default(),
                draft: r.get::<_, Option<String>>(11)?.and_then(|d| serde_json::from_str(&d).ok()),
            })
        }).map_err(|e| e.to_string())
}

pub fn unfinished_sessions(user: &Connection) -> Result<Vec<SessionInfo>, String> {
    let mut stmt = user.prepare("SELECT session_id FROM sessions WHERE finished_at IS NULL ORDER BY session_id DESC").map_err(|e| e.to_string())?;
    let ids: Vec<i64> = stmt.query_map([], |r| r.get(0)).map_err(|e| e.to_string())?.filter_map(|x| x.ok()).collect();
    Ok(ids.iter().filter_map(|i| get_session(user, *i).ok()).collect())
}

/// 交卷：Rust 按 bank.db 答案重算成绩、落 answer_records、更新错题本与 SM-2（单一事务）
pub fn finish_session(user: &Connection, bankconn: &Connection, session_id: i64) -> Result<SessionInfo, String> {
    let s = get_session(user, session_id)?;
    if s.finished_at.is_some() { return Ok(s); }
    let draft: Draft = serde_json::from_value(s.draft.clone().unwrap_or(serde_json::json!({}))).unwrap_or_default();
    let policy: String = user.query_row("SELECT multi_score_policy FROM sessions WHERE session_id=?1", params![session_id], |r| r.get(0)).unwrap_or("all_or_nothing".into());

    let questions = bank::get_questions_by_ids(bankconn, &s.qid_list)?;
    let now = chrono::Utc::now().to_rfc3339();
    let tx = user.unchecked_transaction().map_err(|e| e.to_string())?;
    tx.execute("DELETE FROM answer_records WHERE session_id=?1", params![session_id]).map_err(|e| e.to_string())?;
    let mut scored = 0i64; let mut correct = 0i64;
    for q in &questions {
        let key = draft_key(&q.bank_id, &q.qid);
        let pick = draft.picks.get(&key);
        let (picked, t) = pick.map(|p| (p.picked.clone(), p.t)).unwrap_or((String::new(), None));
        // 无答案或低置信度 → 不计分（null）；未作答 → 按错计（grade 内处理）
        let graded = if q.answer.is_empty() || q.answer_conf != "high" { None }
            else { grade(&q.qtype, &q.answer, &picked, &policy) };
        let is_c: Option<i64> = graded.map(|b| b as i64);
        if let Some(g) = graded { scored += 1; if g { correct += 1; } }
        tx.execute("INSERT INTO answer_records(bank_id,qid,q_version,session_id,picked,is_correct,time_cost_ms,answered_at) VALUES(?,?,1,?,?,?,?,?)",
            params![q.bank_id, q.qid, session_id, picked, is_c, t, now]).map_err(|e| e.to_string())?;
        // 错题本 + SM-2（仅练习类模式驱动）
        if let Some(g) = graded {
            if !g {
                tx.execute("INSERT INTO wrong_book(bank_id,qid,wrong_count,last_wrong_at) VALUES(?,?,1,?)
                            ON CONFLICT(bank_id,qid) DO UPDATE SET wrong_count=wrong_count+1,last_wrong_at=excluded.last_wrong_at",
                    params![q.bank_id, q.qid, now]).map_err(|e| e.to_string())?;
            }
            if SRS_MODES.contains(&s.mode.as_str()) {
                let _ = srs_apply(&tx, &q.bank_id, &q.qid, g);
            }
        }
    }
    let score = if scored > 0 { Some((correct as f64 / scored as f64) * 100.0) } else { None };
    let duration = (chrono::Utc::now() - chrono::DateTime::parse_from_rfc3339(&s.started_at).map_err(|e| e.to_string())?
        .with_timezone(&chrono::Utc)).num_milliseconds();
    tx.execute("UPDATE sessions SET finished_at=?1,scored_qty=?2,correct_qty=?3,score=?4,duration_ms=?5 WHERE session_id=?6",
        params![now, scored, correct, score, duration, session_id]).map_err(|e| e.to_string())?;
    tx.commit().map_err(|e| e.to_string())?;
    log::info!(target: "session", "会话#{session_id} 完成：得分{} 对{correct}/{scored}计分题",
        score.map(|s| format!("{s:.1}")).unwrap_or_else(|| "—".into()));
    get_session(user, session_id)
}

pub fn session_detail(user: &Connection, bankconn: &Connection, session_id: i64) -> Result<SessionDetail, String> {
    let s = get_session(user, session_id)?;
    let qs = bank::get_questions_by_ids(bankconn, &s.qid_list)?;
    let mut stmt = user.prepare("SELECT bank_id,qid,picked,is_correct,time_cost_ms FROM answer_records WHERE session_id=?1").map_err(|e| e.to_string())?;
    let mut recs: Vec<AnswerRow> = stmt.query_map(params![session_id], |r| {
        Ok(AnswerRow { bank_id: r.get(0)?, qid: r.get(1)?, picked: r.get(2)?,
            is_correct: r.get::<_, Option<i64>>(3)?.map(|v| v != 0), time_cost_ms: r.get(4)?, question: None })
    }).map_err(|e| e.to_string())?.filter_map(|x| x.ok()).collect();
    for rec in &mut recs {
        rec.question = qs.iter().find(|q| q.bank_id == rec.bank_id && q.qid == rec.qid).cloned();
    }
    Ok(SessionDetail { session: s, records: recs })
}

pub fn list_sessions(user: &Connection, limit: i64) -> Result<Vec<SessionBrief>, String> {
    let mut stmt = user.prepare("SELECT session_id,mode,title,score,correct_qty,scored_qty,finished_at,duration_ms FROM sessions WHERE finished_at IS NOT NULL ORDER BY session_id DESC LIMIT ?1")
        .map_err(|e| e.to_string())?;
    let rows = stmt.query_map(params![limit], |r| Ok(SessionBrief {
        session_id: r.get(0)?, mode: r.get(1)?, title: r.get(2)?, score: r.get(3)?,
        correct_qty: r.get(4)?, scored_qty: r.get(5)?, finished_at: r.get(6)?, duration_ms: r.get(7)?,
    })).map_err(|e| e.to_string())?;
    Ok(rows.filter_map(|x| x.ok()).collect())
}

// ---------- 仪表盘 ----------
pub fn dashboard(user: &Connection, bankconn: &Connection) -> Result<Dashboard, String> {
    let (answered, correct): (i64, i64) = user.query_row(
        "SELECT COUNT(*), COALESCE(SUM(is_correct),0) FROM answer_records WHERE is_correct IS NOT NULL", [],
        |r| Ok((r.get(0)?, r.get(1)?))).map_err(|e| e.to_string())?;
    let sessions_done: i64 = user.query_row("SELECT COUNT(*) FROM sessions WHERE finished_at IS NOT NULL", [], |r| r.get(0)).map_err(|e| e.to_string())?;
    let streak: i64 = user.query_row(
        "SELECT COUNT(DISTINCT date(started_at)) FROM sessions", [], |r| r.get(0)).map_err(|e| e.to_string())?;
    let now = chrono::Utc::now().to_rfc3339();
    let due_count: i64 = user.query_row(
        "SELECT COUNT(*) FROM review_queue WHERE due_date IS NOT NULL AND due_date<=?1 AND repetitions<2", params![now], |r| r.get(0)).map_err(|e| e.to_string())?;
    let wrong_active: i64 = user.query_row(
        "SELECT COUNT(*) FROM wrong_book w LEFT JOIN review_queue r ON r.bank_id=w.bank_id AND r.qid=w.qid
         WHERE COALESCE(r.repetitions,0)<2", [], |r| r.get(0)).map_err(|e| e.to_string())?;

    // 按主题统计：answer_records → question_topics（bank库两步查询）
    let mut per_q: HashMap<(String, String), (i64, i64)> = HashMap::new();
    {
        let mut stmt = user.prepare("SELECT bank_id,qid,is_correct FROM answer_records WHERE is_correct IS NOT NULL").map_err(|e| e.to_string())?;
        let rows = stmt.query_map([], |r| Ok((r.get::<_, String>(0)?, r.get::<_, String>(1)?, r.get::<_, Option<i64>>(2)?)))
            .map_err(|e| e.to_string())?;
        for (b, q, c) in rows.filter_map(|x| x.ok()) {
            if let Some(c) = c {
                let e = per_q.entry((b, q)).or_insert((0, 0));
                e.0 += 1; if c != 0 { e.1 += 1; }
            }
        }
    }
    let mut topic_agg: HashMap<String, (i64, i64)> = HashMap::new();
    {
        let mut stmt = bankconn.prepare(
            "SELECT qt.bank_id, qt.qid, t.name FROM question_topics qt JOIN topics t ON t.topic_id=qt.topic_id")
            .map_err(|e| e.to_string())?;
        let rows = stmt.query_map([], |r| Ok((r.get::<_, String>(0)?, r.get::<_, String>(1)?, r.get::<_, String>(2)?)))
            .map_err(|e| e.to_string())?;
        for (b, q, t) in rows.filter_map(|x| x.ok()) {
            if let Some((a, c)) = per_q.get(&(b.clone(), q.clone())) {
                let e = topic_agg.entry(t).or_insert((0, 0));
                e.0 += a; e.1 += c;
            }
        }
    }
    let mut by_topic: Vec<TopicStat> = topic_agg.into_iter().map(|(topic, (a, c))| TopicStat { topic, a, c }).collect();
    by_topic.sort_by(|x, y| y.a.cmp(&x.a));

    let recent = list_sessions(user, 10)?;
    Ok(Dashboard { answered, correct, sessions_done, streak_days: streak, due_count, wrong_active, by_topic, recent })
}

// ---------- 错题本 / 到期复习 / 收藏 / 笔记 ----------
pub fn wrong_list(user: &Connection, bankconn: &Connection, only_active: bool) -> Result<Vec<WrongRow>, String> {
    let mut stmt = user.prepare(
        "SELECT w.bank_id,w.qid,w.wrong_count,COALESCE(w.last_wrong_at,''),COALESCE(r.repetitions,0),r.due_date
         FROM wrong_book w LEFT JOIN review_queue r ON r.bank_id=w.bank_id AND r.qid=w.qid
         ORDER BY w.last_wrong_at DESC").map_err(|e| e.to_string())?;
    let rows: Vec<(String, String, i64, String, i64, Option<String>)> = stmt.query_map([], |r|
        Ok((r.get(0)?, r.get(1)?, r.get(2)?, r.get(3)?, r.get(4)?, r.get(5)?)))
        .map_err(|e| e.to_string())?.filter_map(|x| x.ok()).collect();
    let mut out = Vec::new();
    for (b, q, wc, lw, rep, due) in rows {
        if only_active && rep >= 2 { continue; }
        if let Some(question) = bank::get_questions_by_ids(bankconn, &[(b.clone(), q.clone())]).ok().and_then(|v| v.into_iter().next()) {
            out.push(WrongRow { bank_id: b, qid: q, wrong_count: wc, last_wrong_at: lw, repetitions: rep, due_date: due, question });
        }
    }
    Ok(out)
}

pub fn due_review(user: &Connection, bankconn: &Connection, limit: i64) -> Result<Vec<bank::QuestionRow>, String> {
    let now = chrono::Utc::now().to_rfc3339();
    let mut stmt = user.prepare(
        "SELECT r.bank_id,r.qid FROM review_queue r JOIN wrong_book w ON w.bank_id=r.bank_id AND w.qid=r.qid
         WHERE r.due_date<=?1 AND r.repetitions<2 ORDER BY r.due_date LIMIT ?2").map_err(|e| e.to_string())?;
    let ids: Vec<(String, String)> = stmt.query_map(params![now, limit], |r| Ok((r.get(0)?, r.get(1)?)))
        .map_err(|e| e.to_string())?.filter_map(|x| x.ok()).collect();
    bank::get_questions_by_ids(bankconn, &ids)
}

pub fn wrong_clear(user: &Connection, bank_id: &str, qid: &str) -> Result<(), String> {
    user.execute("DELETE FROM wrong_book WHERE bank_id=?1 AND qid=?2", params![bank_id, qid]).map_err(|e| e.to_string())?;
    Ok(())
}

pub fn fav_toggle(user: &Connection, bank_id: &str, qid: &str) -> Result<bool, String> {
    let n = user.execute("DELETE FROM favorites WHERE bank_id=?1 AND qid=?2", params![bank_id, qid]).map_err(|e| e.to_string())?;
    if n > 0 { return Ok(false); }
    user.execute("INSERT INTO favorites(bank_id,qid,created_at) VALUES(?,?,?)",
        params![bank_id, qid, chrono::Utc::now().to_rfc3339()]).map_err(|e| e.to_string())?;
    Ok(true)
}

pub fn fav_list(user: &Connection, bankconn: &Connection) -> Result<Vec<bank::QuestionRow>, String> {
    let mut stmt = user.prepare("SELECT bank_id,qid FROM favorites ORDER BY created_at DESC").map_err(|e| e.to_string())?;
    let ids: Vec<(String, String)> = stmt.query_map([], |r| Ok((r.get(0)?, r.get(1)?)))
        .map_err(|e| e.to_string())?.filter_map(|x| x.ok()).collect();
    bank::get_questions_by_ids(bankconn, &ids)
}

pub fn note_set(user: &Connection, bank_id: &str, qid: &str, content: &str) -> Result<(), String> {
    user.execute("INSERT INTO notes(bank_id,qid,content,updated_at) VALUES(?,?,?,?)
                  ON CONFLICT(bank_id,qid) DO UPDATE SET content=excluded.content,updated_at=excluded.updated_at",
        params![bank_id, qid, content, chrono::Utc::now().to_rfc3339()]).map_err(|e| e.to_string())?;
    Ok(())
}

pub fn note_get(user: &Connection, bank_id: &str, qid: &str) -> Result<Option<String>, String> {
    Ok(user.query_row("SELECT content FROM notes WHERE bank_id=?1 AND qid=?2", params![bank_id, qid],
        |r| r.get::<_, String>(0)).ok())
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

// ---------- Tauri 命令 ----------
pub mod commands {
    use super::{start_session as start_session_impl, save_draft as save_draft_impl,
                finish_session as finish_session_impl, session_detail as session_detail_impl,
                unfinished_sessions as unfinished_sessions_impl, list_sessions as list_sessions_impl,
                dashboard as dashboard_impl, due_review as due_review_impl,
                wrong_list as wrong_list_impl, wrong_clear as wrong_clear_impl,
                fav_toggle as fav_toggle_impl, fav_list as fav_list_impl,
                note_get as note_get_impl, note_set as note_set_impl, activity as activity_impl};
    use super::{SessionInfo, SessionDetail, SessionBrief, Dashboard, WrongRow, DayCount, Draft};
    use crate::bank;
    use crate::telemetry::timed;
    use crate::AppState;

    #[tauri::command]
    pub async fn start_session(state: tauri::State<'_, AppState>, mode: String, title: String, bank_id: String,
                               paper_id: Option<i64>, qids: Vec<(String, String)>, time_limit_sec: Option<i64>)
        -> Result<SessionInfo, String> {
        timed("start_session", false, || {
            let u = state.user.lock().map_err(|e| e.to_string())?;
            start_session_impl(&u, &mode, &title, &bank_id, paper_id, &qids, time_limit_sec)
        })
    }

    #[tauri::command]
    pub async fn save_draft(state: tauri::State<'_, AppState>, session_id: i64, draft: serde_json::Value) -> Result<(), String> {
        // 做题页每 1.5s 自动保存一次：成功降为 trace，避免刷屏
        timed("save_draft", true, || {
            let u = state.user.lock().map_err(|e| e.to_string())?;
            let d: Draft = serde_json::from_value(draft).map_err(|e| e.to_string())?;
            save_draft_impl(&u, session_id, &d)
        })
    }

    #[tauri::command]
    pub async fn finish_session(state: tauri::State<'_, AppState>, session_id: i64) -> Result<SessionInfo, String> {
        timed("finish_session", false, || {
            let b = state.bank.lock().map_err(|e| e.to_string())?;
            let u = state.user.lock().map_err(|e| e.to_string())?;
            finish_session_impl(&u, &b, session_id)
        })
    }

    #[tauri::command]
    pub async fn session_detail(state: tauri::State<'_, AppState>, session_id: i64) -> Result<SessionDetail, String> {
        timed("session_detail", false, || {
            let b = state.bank.lock().map_err(|e| e.to_string())?;
            let u = state.user.lock().map_err(|e| e.to_string())?;
            session_detail_impl(&u, &b, session_id)
        })
    }

    #[tauri::command]
    pub async fn unfinished_sessions(state: tauri::State<'_, AppState>) -> Result<Vec<SessionInfo>, String> {
        timed("unfinished_sessions", false, || {
            let u = state.user.lock().map_err(|e| e.to_string())?;
            unfinished_sessions_impl(&u)
        })
    }

    #[tauri::command]
    pub async fn list_sessions(state: tauri::State<'_, AppState>) -> Result<Vec<SessionBrief>, String> {
        timed("list_sessions", false, || {
            let u = state.user.lock().map_err(|e| e.to_string())?;
            list_sessions_impl(&u, 30)
        })
    }

    #[tauri::command]
    pub async fn dashboard(state: tauri::State<'_, AppState>) -> Result<Dashboard, String> {
        timed("dashboard", false, || {
            let b = state.bank.lock().map_err(|e| e.to_string())?;
            let u = state.user.lock().map_err(|e| e.to_string())?;
            dashboard_impl(&u, &b)
        })
    }

    #[tauri::command]
    pub async fn due_review(state: tauri::State<'_, AppState>, limit: Option<i64>) -> Result<Vec<bank::QuestionRow>, String> {
        timed("due_review", false, || {
            let b = state.bank.lock().map_err(|e| e.to_string())?;
            let u = state.user.lock().map_err(|e| e.to_string())?;
            due_review_impl(&u, &b, limit.unwrap_or(20))
        })
    }

    #[tauri::command]
    pub async fn wrong_list(state: tauri::State<'_, AppState>) -> Result<Vec<WrongRow>, String> {
        timed("wrong_list", false, || {
            let b = state.bank.lock().map_err(|e| e.to_string())?;
            let u = state.user.lock().map_err(|e| e.to_string())?;
            wrong_list_impl(&u, &b, true)
        })
    }

    #[tauri::command]
    pub async fn wrong_clear(state: tauri::State<'_, AppState>, bank_id: String, qid: String) -> Result<(), String> {
        timed("wrong_clear", false, || {
            let u = state.user.lock().map_err(|e| e.to_string())?;
            wrong_clear_impl(&u, &bank_id, &qid)
        })
    }

    #[tauri::command]
    pub async fn fav_toggle(state: tauri::State<'_, AppState>, bank_id: String, qid: String) -> Result<bool, String> {
        timed("fav_toggle", true, || {
            let u = state.user.lock().map_err(|e| e.to_string())?;
            fav_toggle_impl(&u, &bank_id, &qid)
        })
    }

    #[tauri::command]
    pub async fn fav_list(state: tauri::State<'_, AppState>) -> Result<Vec<bank::QuestionRow>, String> {
        timed("fav_list", false, || {
            let b = state.bank.lock().map_err(|e| e.to_string())?;
            let u = state.user.lock().map_err(|e| e.to_string())?;
            fav_list_impl(&u, &b)
        })
    }

    #[tauri::command]
    pub async fn note_get(state: tauri::State<'_, AppState>, bank_id: String, qid: String) -> Result<Option<String>, String> {
        // 每道题进入视图都会取一次笔记：成功降为 trace
        timed("note_get", true, || {
            let u = state.user.lock().map_err(|e| e.to_string())?;
            note_get_impl(&u, &bank_id, &qid)
        })
    }

    #[tauri::command]
    pub async fn note_set(state: tauri::State<'_, AppState>, bank_id: String, qid: String, content: String) -> Result<(), String> {
        timed("note_set", true, || {
            let u = state.user.lock().map_err(|e| e.to_string())?;
            note_set_impl(&u, &bank_id, &qid, &content)
        })
    }

    #[tauri::command]
    pub async fn activity_calendar(state: tauri::State<'_, AppState>, days: Option<i64>) -> Result<Vec<DayCount>, String> {
        timed("activity_calendar", false, || {
            let u = state.user.lock().map_err(|e| e.to_string())?;
            activity_impl(&u, days.unwrap_or(120))
        })
    }
}

// ---------- 单测 ----------
#[cfg(test)]
mod tests {
    use super::*;

    /// 公开仓无数据包时返回 None——依赖种子的用例整体跳过（而不是空库硬跑）
    fn env() -> Option<(Connection, Connection, std::path::PathBuf)> {
        let seed = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("resources/seed/smart-core.smartbank");
        if !seed.exists() { eprintln!("种子不存在，跳过（公开仓无数据包，先运行 30_pack_seed.py）"); return None; }
        let tmp = std::env::temp_dir().join(format!("squser-{}", uuid::Uuid::new_v4()));
        std::fs::create_dir_all(&tmp).unwrap();
        let bank = crate::db::open(&tmp.join("bank.db")).unwrap();
        let user = crate::db::open_user(&tmp.join("user.db")).unwrap();
        bank::import(&bank, &seed, &tmp.join("banks"), true).unwrap();
        Some((bank, user, tmp))
    }
    fn pick_qids(bankconn: &Connection, n: usize) -> Vec<(String, String)> {
        bank::list_questions(bankconn, None, None, Some("active".into()), None, n as i64, 0).unwrap()
            .iter().map(|q| (q.bank_id.clone(), q.qid.clone())).collect()
    }

    #[test]
    fn srs_state_machine() {
        // V1.1 修正版：rep1→1天 rep2→6天 之后×ease；答错重置1天（首要单测用例）
        let s0 = SrsState { ease: 2.5, interval: 0.0, rep: 0 };
        let s1 = srs_step(s0, true);
        assert_eq!((s1.rep, s1.interval), (1, 1.0));
        let s2 = srs_step(s1, true);
        assert_eq!((s2.rep, s2.interval), (2, 6.0));
        let s3 = srs_step(s2, true);
        assert_eq!((s3.rep, s3.interval), (3, 15.0)); // 6×2.5
        let s4 = srs_step(s3, false);
        assert_eq!((s4.rep, s4.interval), (0, 1.0));
        assert!((s4.ease - 2.3).abs() < 1e-9);         // 答错 ease-0.2
        let s5 = srs_step(s4, true);
        assert_eq!((s5.rep, s5.interval), (1, 1.0));    // 重新从1天开始
    }

    #[test]
    fn grading_rules() {
        assert_eq!(grade("single", "B", "B", "all_or_nothing"), Some(true));
        assert_eq!(grade("single", "B", "C", "all_or_nothing"), Some(false));
        assert_eq!(grade("single", "B", "", "all_or_nothing"), Some(false)); // 未答=错
        assert_eq!(grade("multi", "ABD", "ABD", "all_or_nothing"), Some(true));
        assert_eq!(grade("multi", "ABD", "AB", "all_or_nothing"), Some(false));  // 漏选=错（决策：全对才得分）
        assert_eq!(grade("multi", "ABD", "ABDE", "all_or_nothing"), Some(false));
        assert_eq!(grade("multi", "DBA", "ABD", "all_or_nothing"), Some(true));  // 乱序容错
        assert_eq!(grade("single", "", "B", "all_or_nothing"), None);            // 无答案不计分
    }

    #[test]
    fn session_finish_flow() {
        let Some((bankconn, user, tmp)) = env() else { return };
        let qids = pick_qids(&bankconn, 3);
        let qs = bank::get_questions_by_ids(&bankconn, &qids).unwrap();
        let sid = start_session(&user, "practice", "测试练习", "smart-core", None, &qids, None).unwrap().session_id;

        // 草稿：第1题答对，第2题答错（取第一题正确答案/第二题给错答案），第3题不答
        let mut d = Draft::default();
        d.picks.insert(draft_key(&qs[0].bank_id, &qs[0].qid), DraftPick { picked: qs[0].answer.clone(), t: Some(1200) });
        d.picks.insert(draft_key(&qs[1].bank_id, &qs[1].qid), DraftPick { picked: if qs[1].answer == "A" { "B" } else { "A" }.to_string(), t: None });
        save_draft(&user, sid, &d).unwrap();

        let fin = finish_session(&user, &bankconn, sid).unwrap();
        assert_eq!(fin.total_qty, 3);
        assert_eq!(fin.scored_qty, 3);
        assert_eq!(fin.correct_qty, 1);
        assert!((fin.score.unwrap() - 100.0 / 3.0).abs() < 0.01);

        // 记录落库
        let n: i64 = user.query_row("SELECT COUNT(*) FROM answer_records WHERE session_id=?1", params![sid], |r| r.get(0)).unwrap();
        assert_eq!(n, 3);

        // 错题本：第2题（答错）+ 第3题（未答）入本；SM-2 推进
        let wrongs = wrong_list(&user, &bankconn, true).unwrap();
        assert_eq!(wrongs.len(), 2);
        assert!(wrongs.iter().all(|w| w.repetitions == 0));
        // 再做一次错题重练全对 → 消灭进度 rep=1（还需1次）
        let sid2 = start_session(&user, "wrong", "错题重练", "smart-core", None,
            &wrongs.iter().map(|w| (w.bank_id.clone(), w.qid.clone())).collect::<Vec<_>>(), None).unwrap().session_id;
        let mut d2 = Draft::default();
        for w in wrong_list(&user, &bankconn, true).unwrap() {
            let q = &bank::get_questions_by_ids(&bankconn, &[(w.bank_id.clone(), w.qid.clone())]).unwrap()[0];
            d2.picks.insert(draft_key(&w.bank_id, &w.qid), DraftPick { picked: q.answer.clone(), t: None });
        }
        save_draft(&user, sid2, &d2).unwrap();
        finish_session(&user, &bankconn, sid2).unwrap();
        let wrongs2 = wrong_list(&user, &bankconn, true).unwrap();
        assert_eq!(wrongs2.len(), 2, "rep=1 未达消灭线2，仍为活跃错题");
        let reps: Vec<i64> = wrongs2.iter().map(|w| w.repetitions).collect();
        assert!(reps.iter().all(|&r| r == 1));

        // 仪表盘
        let dash = dashboard(&user, &bankconn).unwrap();
        assert_eq!(dash.answered, 5); // 会话1三题 + 会话2两道错题
        assert_eq!(dash.sessions_done, 2);
        std::fs::remove_dir_all(&tmp).ok();
    }

    #[test]
    fn draft_resume_persistence() {
        let Some((bankconn, user, tmp)) = env() else { return };
        let qids = pick_qids(&bankconn, 2);
        let s = start_session(&user, "exam", "A卷", "smart-core", Some(1), &qids, Some(5400)).unwrap();
        let mut d = Draft::default();
        d.picks.insert(draft_key(&qids[0].0, &qids[0].1), DraftPick { picked: "A".into(), t: None });
        d.marks.insert(draft_key(&qids[0].0, &qids[0].1), true);
        d.remaining_sec = Some(4300);
        save_draft(&user, s.session_id, &d).unwrap();
        let unfinished = unfinished_sessions(&user).unwrap();
        assert_eq!(unfinished.len(), 1);
        let d2: Draft = serde_json::from_value(unfinished[0].draft.clone().unwrap()).unwrap();
        assert_eq!(d2.remaining_sec, Some(4300));
        assert!(d2.marks.contains_key(&draft_key(&qids[0].0, &qids[0].1)));
        std::fs::remove_dir_all(&tmp).ok();
    }
}
