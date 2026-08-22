# S7-200 SMART 初级认证备考资料库

> 整理日期：2026-08-22 · 全部内容依据西门子官方资料生成

## 🚀 快速开始

**双击 `练习平台/index.html` 开始刷题**（离线可用，Edge/Chrome 浏览器）

## 目录结构

```
D:\PLC\s7-200\
├── 练习平台/            ⭐ 离线刷题 Web 应用（双击 index.html）
│   ├── index.html       单文件应用（练习/考试/错题本/统计）
│   ├── data.js          题库+试卷数据（自动生成）
│   └── images/          试卷配图
├── 考试模拟卷/           5 套官方模拟卷 + AI 题库
│   ├── A卷.md ~ E卷.md  350 道真题（含图）
│   ├── 题库.md          344 道题·带答案解析出处（10 大主题）
│   ├── 题库.json        机器可读版
│   └── images/          题目配图
├── 题库资料/             中间产物（可重建）
│   ├── techref26/       技术参考 PLUS 2.6 反编译（261篇）
│   ├── microwin/        Micro/WIN SMART 帮助反编译（661篇）
│   ├── 手册章节/         系统手册 v2.8 分章文本（12章/1130页）
│   ├── questions/       各主题 AI 出题原始文件（10个 jsonl）
│   ├── answers/         A-E 卷答案（AI 依据资料整理+置信度）
│   └── 选型手册v28.txt   选型手册全文
└── *.py                 流水线脚本（见下）
```

## 数据流水线（可重跑）

```
wjx_scrape.py      抓取问卷星 5 套模拟卷 → 考试模拟卷/*.md
html2txt.py        (临时) CHM→文本        merge_bank.py     合并10主题题目→题库.md/json
parse_papers.py    模拟卷md→papers_raw.json
build_platform.py  生成练习平台 data.js + 复制图片
apply_corrections.py 应用校验修正 → 之后重跑 merge + build
test_platform.py   Playwright 自动化测试平台
```

## 质量说明

- **题库 344 题**：10 个主题，每题含答案/解析/出处；经 10 个独立校验代理逐题核对资料原文（发现并修正版本冲突，如 PID 回路数 8→16）
- **模拟卷 350 题**：答案由 AI 依据官方资料整理，附置信度标记；低置信度题不计入判分
- 官方资料：S7-200 SMART 技术参考 PLUS 2.6、系统手册 V2.8、选型手册 V2.8、STEP 7-Micro/WIN SMART 帮助
- 注意：AI 整理的答案仍可能存在疏漏，重要考点请对照原始文档（见 `题库资料/`）
