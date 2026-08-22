// 试卷打印数据：按题型分节（A4 双栏排版由前端 PrintPaperView 完成）
use rusqlite::{params, Connection};
use serde::Serialize;

use crate::bank;

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

// ---------- Tauri 命令 ----------
pub mod commands {
    use super::paper_print_data as paper_print_data_impl;
    use super::PrintPaper;
    use crate::telemetry::timed;
    use crate::AppState;

    #[tauri::command]
    pub async fn paper_print_data(state: tauri::State<'_, AppState>, paper_id: i64) -> Result<PrintPaper, String> {
        timed("paper_print_data", false, || {
            let b = state.bank.lock().map_err(|e| e.to_string())?;
            paper_print_data_impl(&b, paper_id)
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn print_data() {
        let tmp = std::env::temp_dir().join(format!("sqprint-{}", uuid::Uuid::new_v4()));
        std::fs::create_dir_all(&tmp).unwrap();
        let bankconn = crate::db::open(&tmp.join("bank.db")).unwrap();
        let seed = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("resources/seed/smart-core.smartbank");
        if !seed.exists() { eprintln!("种子不存在，跳过（公开仓无数据包，先运行 30_pack_seed.py）"); return; }
        bank::import(&bankconn, &seed, &tmp.join("banks"), true).unwrap();
        let p = paper_print_data(&bankconn, 1).unwrap();
        assert!(p.total_count > 0);
        assert_eq!(p.sections.len(), 2); // 单选+多选
        assert!(p.sections[0].questions.len() >= 40);
        std::fs::remove_dir_all(&tmp).ok();
    }
}
