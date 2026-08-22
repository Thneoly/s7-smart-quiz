// 题库去重：归一化精确重复 + 64位 SimHash 相似聚类，合并时全链路数据迁移
use rusqlite::{params, Connection};
use serde::Serialize;
use std::collections::HashMap;

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

// ---------- Tauri 命令 ----------
pub mod commands {
    use super::{dedup_scan as dedup_scan_impl, dedup_merge as dedup_merge_impl, DupGroup};
    use crate::telemetry::timed;
    use crate::AppState;

    #[tauri::command]
    pub async fn dedup_scan(state: tauri::State<'_, AppState>, bank_id: String) -> Result<Vec<DupGroup>, String> {
        timed("dedup_scan", false, || {
            let b = state.bank.lock().map_err(|e| e.to_string())?;
            dedup_scan_impl(&b, &bank_id)
        })
    }

    #[tauri::command]
    pub async fn dedup_merge(state: tauri::State<'_, AppState>, bank_id: String, keep: String, removes: Vec<String>) -> Result<usize, String> {
        timed("dedup_merge", false, || {
            let b = state.bank.lock().map_err(|e| e.to_string())?;
            let u = state.user.lock().map_err(|e| e.to_string())?;
            dedup_merge_impl(&b, &u, &bank_id, &keep, &removes)
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn simhash_sanity() {
        // 与 dedup_scan 一致：先归一化再 simhash
        let a = simhash64(&norm_stem("s7200smart标准型cpu最多扩展6个模块"));
        let b = simhash64(&norm_stem("s7200smart标准型CPU最多扩展6个模块！"));
        let c = simhash64(&norm_stem("modbusrtu通信需要设置波特率与校验位"));
        assert_eq!(a, b, "归一化后相同文本应完全一致");
        assert!(hamming(a, c) > 10, "无关文本不应相似");
    }
}
