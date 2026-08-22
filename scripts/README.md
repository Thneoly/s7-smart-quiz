# scripts/ —— 数据流水线

编号即执行顺序：`1x` 抓取 → `2x` 题库生产 → `3x` 打包 → `4x` 前端资产生成。

## 环境依赖（仅本机需要）

```bash
uv venv .venv && .venv/Scripts/python.exe -m pip install playwright beautifulsoup4 …
# Playwright 使用系统 Edge：channel='msedge'，无需下载 chromium
```

## 脚本与所需输入

| 脚本 | 作用 | 输入依赖 | 可否异地运行 |
|---|---|---|---|
| `10_scrape_papers.py` | 抓取问卷星模拟卷（Playwright + Edge） | 试卷 URL（脚本内） | ✅ 任意机器 |
| `11_fix_images.py` | 补抓试卷配图（带 Referer 防盗链） | `10` 的 papers_raw.json | ✅ |
| `12_parse_papers.py` | 解析试卷为结构化题库 | `11` 的输出 | ✅ |
| `20_merge_bank.py` | 合并各主题题目 → 校验/去重 → 题库.json | **`题库资料/questions/*.jsonl` + `answers/*.jsonl`（本地私有数据）** | ❌ 仅数据所在机 |
| `21_apply_corrections.py` | 应用校验工作流的修正 | 修正文件 + 题库 | ❌ 同上 |
| `30_pack_seed.py` | 打包 `.smartbank` 种子题库 | **`考试模拟卷/题库.json + images/`（本地私有数据）** | ❌ |
| `32_pack_docs.py` | 打包 `docs.docpack` 检索语料 | **官方文档提取语料（本地私有，来源见 docs/项目管理.md 再生表）** | ❌ |
| `40_gen_guide.py` | 生成学习指南 guide.json | **`题库资料/guide_stages.json`（本地私有）** | ❌ |
| `41_gen_refdata.py` | 生成资料速查 refdata.json | 一次性多智能体工作流 journal（本地私有） | ❌ |
| `41b_render_icons.py` | 渲染应用图标设计稿 | `smart-quiz-app/icons-design/variants.html`（入库） | ✅ |
| `make_update_manifest.py`（app 目录） | 生成更新清单 latest.json | 构建产物 + 版本号 | ✅ 构建机 |

## 说明

- **公开仓库不含任何题库/语料数据**（版权原因，见根 README"数据包"一节）。表中 ❌ 的脚本在本仓库只是**方法论展示**：拥有自己的源资料时，可按同款流水线生产自己的数据包。
- 私有数据文件在本机目录（`题库资料/`、`考试模拟卷/`）原地保留，已被 `.gitignore` 排除。
- 改动数据后重跑顺序：`20 → 30`（题库）、`32`（语料）、`40/41`（指南/速查），然后回到应用 `npm run build` + 回归 `tests/e2e_*.py`。
