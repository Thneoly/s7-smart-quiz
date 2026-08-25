---
name: s7-data-pipeline
description: S7-200 SMART 题库项目的数据流水线操作、题库答案修正、讲义生成与质检、版本冲突裁决。当需要：改题库/指南数据并重跑流水线、修正题目答案、再生成章节讲义、质检生成内容、排查数据版本冲突时使用。
---

# S7 题库数据流水线与质检

## 双仓结构（改数据前必读）

- **代码仓**（公开）：`D:\PLC\s7-200` —— 应用、脚本、guide/lectures/refdata 前端资产
- **数据仓**（私有）：`D:\PLC\s7-smart-quiz-data` —— 题库源 jsonl、语料、真题存档；**派生产物生成在数据仓内，改完提交回去**
- 数据根由 `scripts/_data.py` 三级解析：`SQ_DATA_ROOT` 环境变量 → 代码仓同级数据仓 → 仓库内（历史布局）。所有流水线脚本走 `data()` 取数，不硬编码路径

## 流水线重跑顺序

```
改 questions/*.jsonl（源题）
  → 21_apply_corrections.py   # 仅当有 corrections.json 新条目
  → 20_merge_bank.py          # → 考试模拟卷/题库.json + 题库.md（数据仓）
  → 30_pack_seed.py           # → 代码仓 resources/seed/*.smartbank
改 guide_stages.json（指南源）
  → 40_gen_guide.py           # → 代码仓 src/study/guide.json + docs/学习指南.md
改语料 → 32_pack_docs.py → resources/docs/docs.docpack
之后统一：代码仓 cargo test + tests/e2e_*.py 回归，双仓分别提交推送
```

## 版本冲突裁决规则（改"事实类"数据前必查）

**系统手册 V2.8 > 技术参考 PLUS 2.6 > 课程资料**。同一事实不同资料版本不一致时，以手册为准，但解析中注明版本差异（考试若明确按旧版出题以题目为准）。先例：
- PID 回路数：手册 V2.8 = 16 条（技术参考 2.6 写 8）
- V 区范围：**V2.8 固件 SR/ST 全系 +4KB**——SR20=VB12287 / SR30=VB16383 / SR40=VB20479 / SR60=VB24575（CR 系列仍 VB8191）；技术参考 2.6 与课程资料是扩容前旧值（8191/12287/16383/20479）

## 题库答案修正规程

1. 在数据仓 `题库资料/corrections.json` **追加**条目（勿覆盖已有）：
   ```json
   {"topic": "03_存储区与寻址.jsonl", "results": [
     {"line": 18, "verdict": "wrong", "answer": "D", "reason": "≤200字，注明裁决依据与版本差异"}]}
   ```
   （`line` 是该 jsonl 内 1 起始行号；`verdict: "ambiguous"` 删题）
2. 跑 21 → 20 → 30，验证 题库.json 新答案与解析
3. ⚠️ **21 不可重跑**：它按行覆盖重写解析中的"[校验修正，原答案X]"——重复套用会把 X 记成已修正后的答案（历史失真，踩过两次）。重跑流水线时跳过 21，或先从 corrections.json 摘除已应用条目
4. 事实类修正的 reason 必须给出手册原文出处；解析若含失真的历史标注，去除而非猜测原值

## 章节讲义生成与质检（M5① 流程）

**生成**（多智能体工作流，模板要点）：
- 每章一 agent：读 `guide.json` 对应章 + 数据仓语料（grep 关键词定位→读透再写）
- **防截断硬约束（必写进 prompt）**："每段必须是完整句子，以句号收尾，段内不得在逗号/顿号处中断——输出前逐段自检"。第2章教训：schema 输出在长章会截断，产生句中断裂与 `],` 垃圾段
- 其余约束：转述不抄原文（事实数据可原样）、每节 ref 必须真实读过、正文 800~1500 字、3 并发限流（防 429）
- 产物组装进 `smart-quiz-app/src/study/lectures.json`（`{"lectures":[…22项]}`，前端静态加载）

**三重质检（生成后必跑）**：
```bash
cd scripts
../.venv/Scripts/python.exe 43_audit_lectures.py       # 完整性：截断/乱码/过短/空段，全零才算过
../.venv/Scripts/python.exe 42_verify_lecture_refs.py  # 出处真实性：ref 的 .txt 须存在于数据仓
```
第三重是**事实双源互证**：抽查数字类论断（参数/范围/单位）对照 `src/study/refdata.json`（已校验数据）——单一来源的自我一致没有意义

## 真实数据检视三原则

1. **mock 只测接线，真实数据测可用性**：E2E 全绿 ≠ 内容正确；生成内容必须过完整性扫描+出处核对+事实互证
2. **分发物验证先废掉开发机回退**：exe 内编译了 CARGO_MANIFEST_DIR 回退路径，开发机上跑分发物永远"看着正常"（发布清单 5c 条的教训）
3. **生成内容的事实错误要追到底**：一处可疑数字 → 查多源 → 可能牵出指南/题库/讲义连锁错误（V 区冲突就是这样从讲义追到题库答案 C→D 的）

## 环境

- Python：`.venv/Scripts/python.exe`（仓库根）；e2e 浏览器默认 msedge，CI 设 `SQ_BROWSER=chromium`
- 测试基线：cargo 14 项 + E2E 6 套件全绿才可提交；数据类测试无数据包时自动跳过
