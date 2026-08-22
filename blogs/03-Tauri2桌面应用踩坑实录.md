# Tauri 2 桌面应用踩坑实录：WebView2、中文全文检索、自定义协议与备份一致性

> **TL;DR**：用 Tauri 2 做了一个 Windows 本地优先的题库应用，最终安装包 9.5MB（含题库与 962 个文档的检索语料）。真正花时间的不是"能不能跑起来"，而是五件事：WebView2 分发矛盾、FTS5 中文分词默认不可用、本地图片的自定义协议、WAL 模式备份一致性、SM-2 状态机的零间隔死锁。每件事都有可直接抄的解法。

## 〇、技术栈与形态

Rust（rusqlite/tauri 2）+ Vue3/TS + Playwright 测试。定位是**本地优先、零遥测**的备考工具：数据全在本地、无任何上报。这个定位影响了后面几乎所有技术选择。

## 一、先想清楚：桌面端选 Tauri 的真实账本

对比 Electron 的体积优势（最终 9.5MB vs 通常 150MB+）是真实的，但账要算全：

**WebView2 依赖。** Tauri 在 Windows 上跑 WebView2，Win10 老机器/工控内网机可能没有。解法是分发矩阵：

| 渠道 | 形态 | 适用 |
|---|---|---|
| 主渠道 | NSIS `downloadBootstrapper`（~10MB） | 联网用户，首装引导下载 WebView2 |
| 离线渠道 | NSIS `offlineInstaller`（~140MB） | 内网/工控现场，内嵌运行时 |
| 免安装 zip | 附属产物 | 文档明示"需已装 WebView2" |

配套一个必须做的细节：**启动时显式探测 WebView2，缺失时弹系统级错误对话框**（附下载指引），绝不能白屏——白屏用户会直接卸载。

**Rust 学习曲线。** 对策是后端保持薄：CRUD、导出、统计。判分和 SM-2 放 Rust 不是炫技，是"交卷重算=信任边界"——成绩单是可能给培训讲师看的东西，不能由前端说了算。

**更新没有差分包。** Tauri 2 的 updater 是"全量安装包 + minisign 签名"。私钥丢失 = 所有历史用户无法升级，密钥管理要当正经事（我们单独生成了密钥对并写进发布 checklist）。

## 二、FTS5 中文检索：默认配置等于没有

方案最初写的是"用 SQLite FTS5 做全文检索"。这句话**默认不成立**：

> FTS5 默认 unicode61 分词器把连续汉字串当作一个 token——"如何配置以太网通信"整个是一个词，用"以太网"根本搜不到它。对纯中文语料，等于没有全文检索。

最终方案是 **jieba-rs 写入时预分词**：

```rust
// 建索引：原文 → jieba 分词 → 空格连接存入 seg 列，FTS5 建在该列上
let seg: String = jb.cut(raw, true).iter().map(|t| t.word)
    .collect::<Vec<&str>>().join(" ");
// 查询：查询词同样过 jieba，多词 AND
let expr = toks.iter().map(|t| format!("\"{t}\"")).collect::<Vec<_>>().join(" AND ");
```

一个报错很隐蔽的坑：**content-linked FTS5（`content='docs_chunks'`）要求内容表里存在与 FTS 列同名的列**。我起初只在 FTS 表里放 seg，MATCH 时报 `no such column: T.seg`——FTS5 短语校验要从内容表回读原文。修正：seg 列在内容表和 FTS 表各存一份（空间约 2 倍，1.6MB 语料完全可接受）。

实测数据（10 万行）：

| 指标 | 数值 |
|---|---|
| jieba 分词 + 建索引 | 1.7s（一次性） |
| LIKE '%kw%' 平均 | 22.7ms |
| **FTS5 MATCH 平均** | **0.42ms（快 54 倍）** |
| 命中数 FTS/LIKE | 10522 / 4320（分词匹配召回更高） |

语料是 962 个文档（系统手册 12 章、技术参考 FAQ 261 篇、指令帮助 661 篇、培训课程 22 章）打成一个 1.6MB zip 进安装包，首次使用时解包建索引（放 `spawn_blocking`，数秒）。

## 三、本地图片：自定义协议比 asset 协议省心

题目配图（梯形图截图）要从应用数据目录加载。Tauri 内置 asset 协议的问题：要逐目录配 scope、前端要 `convertFileSrc()` 换算、未来付费题库是单文件分发也不适合落盘直读。

最后注册了自定义协议，一套逻辑通吃内置/导入/打包题库：

```rust
.register_uri_scheme_protocol("bankasset", |ctx, request| {
    protocol::handle(ctx.app_handle(), request)
    // 解析 bankasset://<bank_id>/<相对路径> → AppData/banks/<bank_id>/assets/...
    // 路径穿越防护（拒绝 ..）在协议层统一做
})
```

前端 URL 用 `convertFileSrc(\`${bankId}/${path}\`, 'bankasset')` 生成。**CSP 的 `img-src` 必须加 `http://bankasset.localhost`**——Windows 下自定义协议的实际请求形态是 `http://<scheme>.localhost/`，不是 `bankasset://`。

## 四、备份：WAL 模式下直接拷文件会丢数据

这是设计评审里最"救命"的一条（详见系列下一篇）：

> SQLite WAL 模式下，应用运行中直接 copy `.db` 主文件，会**丢掉 -wal 里已提交的事务**——丢的恰恰是最近一次考试的记录；连 -wal 一起拷而不持锁，可能拷出撕裂帧，恢复时报"数据库损坏"。

正解一行 SQL，运行态安全：

```sql
VACUUM INTO '/path/backup.db'   -- 一致性快照
```

再包一层 zip（manifest 记 schema 版本 + 题库指纹），恢复前先只读校验（能否打开、有没有核心表）再替换。

## 五、Excel 导入：校验做在预览里

calamine 读 xlsx 很顺，真正的工作量在**校验规则产品化**。来自真实题库数据的规则清单：

- 题干空 / 题型不可识别（单选/多选/判断/填空之外）
- 答案字母越界（只有 4 个选项答案填了 D 之后）、单选题多个答案
- 选项中间空档（A、C 有值 B 为空——拼版时常见）
- **答案归一化**：判断题"对/错/√/×/T/F"、多选"A,B,D"或"ABD"、大小写与乱序

全部在"预览"步骤逐行展示错误，用户确认后导入，错误行跳过并进导入报告。**校验前置到预览**，比导入后报错再回 Excel 改，体验差着量级。

## 六、杂项坑清单（每条都真实踩过）

1. **SM-2 零间隔死锁**：简化版写"答对间隔×ease"，interval 初值 0——0×2.5=0，题永不到期。正解：rep1→1天、rep2→6天、之后×ease，答错重置 1 天。写成第一个单元测试
2. **复合主键第一天就用**：`qid` 单列主键在"第二个题库导入"那天必撞，晚改等于已发布用户双库迁移。全链路 `(bank_id, qid)`
3. **Tauri 2 capabilities 目录**：dialog/updater/process 权限都要在 `capabilities/default.json` 声明，v1 转 v2 最容易漏的一步
4. **后台打包签名静默失败**：`TAURI_SIGNING_PRIVATE_KEY` 经 detached shell 传递会丢，构建"成功"但没有 .sig。签名构建要么前台要么进 CI，并检查产物里有 .sig
5. **rusqlite command 全部标 `async`**，长任务（建索引/导入）`spawn_blocking`——否则 10 万行导入冻结 UI 事件循环
6. **前端判分只做展示**：交卷时 Rust 按答案与计分规则重算入库。同一逻辑写两份必然漂移，权威只有一份

## 七、最终数字与收尾

- 安装包 **9.5MB**（种子题库 203KB + 文档语料 1.6MB + 应用本体）
- cargo 集成测试 11 项 + Playwright E2E 47 项，全绿
- 题库 694 题、检索 0.42ms、组卷含降级策略、打印 A4 双栏带答题卡

一句话总结：**Tauri 很适合本地优先的小工具，但决定成败的是把 WebView2 分发、中文分词、备份一致性这些"文档不会主动告诉你"的问题提前掐掉**——这靠的其实是一次认真的方案评审，那是下一件事。
