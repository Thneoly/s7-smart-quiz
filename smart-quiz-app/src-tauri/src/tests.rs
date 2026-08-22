// 跨领域集成测试：单测归各领域模块，这里只放贯穿多模块的真实用户流程
#[cfg(test)]
mod tests {
    // 组卷 → 会话 → 活动日历 → Excel成绩导出 → 备份/恢复 → 诊断包（原 m2 全链路用例）
    #[test]
    fn compose_to_diagnosis_flow() {
        use crate::compose::{compose, Blueprint, SectionSpec};

        let tmp = std::env::temp_dir().join(format!("sqflow-{}", uuid::Uuid::new_v4()));
        std::fs::create_dir_all(&tmp).unwrap();
        let bankconn = crate::db::open(&tmp.join("bank.db")).unwrap();
        let user = crate::db::open_user(&tmp.join("user.db")).unwrap();
        let seed = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("resources/seed/smart-core.smartbank");
        if !seed.exists() { eprintln!("种子不存在，跳过（公开仓无数据包，先运行 30_pack_seed.py）"); return; }
        crate::bank::import(&bankconn, &seed, &tmp.join("banks"), true).unwrap();

        // 1) 正常组卷：40单+10多（种子库 active&high 足够）
        let bp = Blueprint { allow_fallback: false, sections: vec![
            SectionSpec { qtype: "single".into(), qty: 40, from_topics: vec![], difficulty: None },
            SectionSpec { qtype: "multi".into(), qty: 10, from_topics: vec![], difficulty: None }] };
        let r = compose(&bankconn, &bp).unwrap();
        assert_eq!(r.total, 50);
        assert_eq!(r.sections[0].actual, 40);
        assert_eq!(r.sections[1].actual, 10);
        // 跨 section 去重
        let mut set = std::collections::HashSet::new();
        for q in &r.qids { assert!(set.insert(q.clone()), "出现重复题"); }

        // 2) 不足+不允许降级 → 报错；允许降级 → 降题量并报告
        let bp2 = Blueprint { allow_fallback: false, sections: vec![
            SectionSpec { qtype: "single".into(), qty: 10000, from_topics: vec![], difficulty: None }] };
        assert!(compose(&bankconn, &bp2).is_err());
        let bp3 = Blueprint { allow_fallback: true, sections: vec![
            SectionSpec { qtype: "single".into(), qty: 10000, from_topics: vec![], difficulty: None }] };
        let r3 = compose(&bankconn, &bp3).unwrap();
        assert!(r3.sections[0].actual < 10000);
        assert!(r3.sections[0].fallback.is_some());

        // 3) 会话 → 活动日历 → 成绩导出
        let qids: Vec<(String, String)> = r.qids.clone();
        let s = crate::session::start_session(&user, "exam", "组卷测试", "smart-core", None, &qids, Some(60)).unwrap();
        crate::session::finish_session(&user, &bankconn, s.session_id).unwrap();
        let act = crate::session::activity(&user, 30).unwrap();
        assert!(!act.is_empty());
        let xlsx = tmp.join("out.xlsx");
        crate::excel::export_session_excel(&user, &bankconn, s.session_id, xlsx.to_str().unwrap()).unwrap();
        assert!(xlsx.exists());

        // 4) 备份/恢复校验
        let bk = tmp.join("bk.zip");
        crate::backup::backup_user(&user, bk.to_str().unwrap()).unwrap();
        let ri = crate::backup::restore_check(bk.to_str().unwrap()).unwrap();
        assert_eq!(ri.sessions, 1);

        // 5) 日志查看与诊断包并入日志
        let logdir = tmp.join("logs");
        std::fs::create_dir_all(&logdir).unwrap();
        std::fs::write(logdir.join("smart-quiz-app_旧.log"), "旧a\n旧b\n").unwrap();
        std::thread::sleep(std::time::Duration::from_millis(20));
        std::fs::write(logdir.join("smart-quiz-app.log"), "l1\nl2\nl3\n").unwrap();
        let lv = crate::telemetry::read_latest_log(Some(&logdir), 2);
        assert_eq!(lv.path.as_deref(), Some(logdir.join("smart-quiz-app.log").to_str().unwrap()));
        assert_eq!(lv.lines, vec!["l2".to_string(), "l3".to_string()]);
        let dz = tmp.join("diag.zip");
        crate::backup::diagnostics(&user, Some(&logdir), dz.to_str().unwrap()).unwrap();
        let mut zf = zip::ZipArchive::new(std::fs::File::open(&dz).unwrap()).unwrap();
        assert_eq!(zf.by_name("logs/smart-quiz-app.log").unwrap().size(), "l1\nl2\nl3\n".len() as u64);
        assert!(zf.by_name("logs/smart-quiz-app_旧.log").is_ok());
        assert!(zf.by_name("privacy.txt").is_ok());
        // 无日志目录时不报错
        let dz2 = tmp.join("diag2.zip");
        crate::backup::diagnostics(&user, None, dz2.to_str().unwrap()).unwrap();
        std::fs::remove_dir_all(&tmp).ok();
    }

    // Excel 导入 → 去重扫描 → 合并（原 m3 全链路用例）
    #[test]
    fn excel_import_dedup_flow() {
        use crate::bank;
        use crate::dedup::{dedup_merge, dedup_scan, DupGroup};
        use crate::excel::{excel_import_into, excel_preview, export_template, TEMPLATE_HEADERS};

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

        let tmp = std::env::temp_dir().join(format!("sqxlsx-{}", uuid::Uuid::new_v4()));
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
        let g: &DupGroup = &groups[0];
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
}
