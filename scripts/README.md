# scripts/ —— 数据流水线

编号即执行顺序：`1x` 抓取 → `2x` 题库生产 → `3x` 打包 → `4x` 前端资产生成。

## 私有数据仓（不再依赖本机）

题库语料/答案/真题存档统一归档在 **[s7-smart-quiz-data](https://github.com/Thneoly/s7-smart-quiz-data)**（私有仓，193MB，含 CHM 原件与反编译树）。
脚本经 `scripts/_data.py` 自动解析数据根目录，顺序：

1. 环境变量 `SQ_DATA_ROOT`
2. 代码仓同级的 `s7-smart-quiz-data/`（标准布局：两仓并排克隆）
3. 代码仓内部 `题库资料/ · 考试模拟卷/`（本机历史布局，已 .gitignore）

```bash
# 任意机器的标准用法
git clone https://github.com/Thneoly/s7-smart-quiz.git
git clone git@github.com:Thneoly/s7-smart-quiz-data.git   # 私有，需权限；克隆到同级即可
cd s7-smart-quiz && uv venv .venv && ...
.venv/Scripts/python.exe scripts/20_merge_bank.py && .venv/Scripts/python.exe scripts/30_pack_seed.py
```

**约定：派生产物（题库.md/json 等）生成在数据仓内，重新生成后提交回数据仓归档**——数据仓是数据唯一的家。

## 环境依赖

```bash
uv venv .venv
# Playwright 使用系统 Edge：channel='msedge'，无需下载 chromium
```

## 脚本与所需输入

| 脚本 | 作用 | 输入依赖 | 异地可跑 |
|---|---|---|---|
| `10_scrape_papers.py` | 抓取问卷星模拟卷（Playwright + Edge） | 试卷 URL（脚本内） | ✅ |
| `11_fix_images.py` | 补抓试卷配图（带 Referer 防盗链） | `10` 的 papers_raw.json | ✅ |
| `12_parse_papers.py` | 解析试卷为结构化题库 | `11` 的输出 | ✅ |
| `20_merge_bank.py` | 合并各主题题目 → 校验/去重 → 题库.json | 数据仓：questions+answers | ✅ 需数据仓 |
| `21_apply_corrections.py` | 应用校验工作流的修正 | 数据仓：corrections.json | ✅ 需数据仓 |
| `30_pack_seed.py` | 打包 `.smartbank` 种子题库 | 数据仓：题库.json + papers_raw + answers + images | ✅ 需数据仓 |
| `32_pack_docs.py` | 打包 `docs.docpack` 检索语料 | 数据仓：提取语料目录 | ✅ 需数据仓 |
| `40_gen_guide.py` | 生成学习指南 guide.json | 数据仓：guide_stages.json | ✅ 需数据仓 |
| `41_gen_refdata.py` | 生成资料速查 refdata.json | 数据仓：refdata_transcripts.json | ✅ 需数据仓 |
| `41b_render_icons.py` | 渲染应用图标设计稿 | `smart-quiz-app/icons-design/variants.html`（入库） | ✅ |
| `make_update_manifest.py`（app 目录） | 生成更新清单 latest.json | 构建产物 + 版本号 | ✅ 构建机 |

公开仓库（代码仓）不含任何题库/语料数据（版权原因）；数据仓私有，语料源头（CHM 原件）也已归档，不再依赖任何一台机器的本地文件。

- 改动数据后重跑顺序：`20 → 30`（题库）、`32`（语料）、`40/41`（指南/速查），产物提交回数据仓；应用侧 `npm run build` + 回归 `tests/e2e_*.py`。
