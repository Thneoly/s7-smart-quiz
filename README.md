# S7-200 SMART 认证备考项目

> 本地优先的题库与学习平台：官方资料 → 多智能体题库 → Tauri 桌面应用。
> Git monorepo · 当前版本 v0.1.0 · 详细管理规范见 [docs/项目管理.md](docs/项目管理.md)

## 目录结构

```
D:\PLC\s7-200\
├── smart-quiz-app/            # ⭐ Tauri2+Vue3 桌面应用（正式产品）
│   ├── src-tauri/             #   Rust 后端（题库/会话/SM-2/组卷/导入/去重/检索/更新）
│   ├── src/                   #   前端（学习/练习/考试/错题本/资料速查/管理/设置）
│   ├── tests/                 #   Playwright E2E（M1/M2/M3/学习/资料/打印）
│   └── make_update_manifest.py#   发布：更新清单生成
├── 练习平台/                    # 离线网页版（双击 index.html，轻量分发用）
├── 考试模拟卷/                  # 数据：5套真题350题(含答案) + 344题题库(md/json) + 配图
├── 题库资料/                    # 语料与中间数据（提取文本/出题jsonl/答案/指南/速查）
├── scripts/                    # 数据流水线脚本（编号=执行顺序）
│   ├── 1x 抓取与解析           #   10抓取问卷星 11补图 12卷→JSON
│   ├── 2x 题库构建             #   20合并去重 21应用校验修正
│   ├── 3x 打包分发             #   30种子.smartbank 31网页平台数据 32文档docpack
│   ├── 4x 内容生成             #   40学习指南 41资料速查（数据源自多智能体工作流）
│   └── 9x 测试                 #   90网页平台E2E
├── docs/                       # 文档：设计方案/学习指南/项目管理
├── blogs/                      # 技术博客4篇（发布清单见 blogs/README.md）
└── .venv/                      # uv Python环境（不入库）
```

## 快速开始

```bash
# 桌面应用（开发）
cd smart-quiz-app && npm install && npm run tauri dev

# 测试
cd smart-quiz-app/src-tauri && cargo test          # 后端 11 项
cd .. && .venv/Scripts/python.exe tests/e2e_m1.py  # 前端 E2E（vite 自动拉起）

# 数据流水线（改题库/资料后重跑，详见 docs/项目管理.md）
.venv/Scripts/python.exe scripts/20_merge_bank.py
```

## 数据与规模

| 项 | 值 |
|---|---|
| 题库 | 694 题（10 主题 344 题 + A~E 真题 350 题，全部带答案/解析/出处/置信度） |
| 学习指南 | 官方 22 章课程结构化（目标/要点/时长/优先级/配套练习） |
| 资料速查 | 211 条结构化数据（CPU/指令/通信/故障/公式） |
| 全文检索 | 962 个官方文档，jieba×FTS5，0.42ms |
| 安装包 | 9.5MB（NSIS + minisign 更新签名） |

## 质量红线

- main 分支保持 cargo 11 项 + E2E 47 项全绿
- 题库答案改动必须过校验工作流（低置信度不参与判分）
- 签名私钥 `~/.tauri/smartquiz.key` 永不入库
