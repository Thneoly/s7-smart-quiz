// Tauri invoke 封装 + 浏览器 mock 回退（mock 内含与 Rust 相同规则的迷你引擎，供纯浏览器 E2E）
import { convertFileSrc, invoke as tauriInvoke } from '@tauri-apps/api/core'

export const hasTauri = typeof window !== 'undefined' && '__TAURI_INTERNALS__' in window

function invoke<T>(cmd: string, args?: Record<string, unknown>): Promise<T> {
  if (hasTauri) return tauriInvoke<T>(cmd, args)
  return mock<T>(cmd, args)
}

export function assetUrl(bankId: string, relPath: string): string {
  if (!hasTauri) return relPath
  return convertFileSrc(`${bankId}/${relPath}`, 'bankasset')
}

// ---------- 类型 ----------
export interface BankStat { bank_id: string; name: string; version: number; total: number; active: number; pending: number; papers: number }
export interface TopicStat { topic_id: number; name: string; total: number; active: number }
export interface Overview { banks: BankStat[]; topics: TopicStat[] }
export interface QuestionRow {
  bank_id: string; qid: string; qtype: string; stem: string; img_path: string | null
  options: string[]; answer: string; answer_conf: string
  explain: string; source: string; status: string; topics: string[]
}
export interface PaperInfo { paper_id: number; bank_id: string; name: string; title: string; count: number }
export type QID = [string, string]
export interface SessionInfo {
  session_id: number; mode: string; title: string; time_limit_sec: number | null
  total_qty: number; started_at: string; finished_at: string | null
  score: number | null; correct_qty: number; scored_qty: number
  qid_list: QID[]; draft: DraftJson | null
}
export interface DraftPick { picked: string; t?: number }
export interface DraftJson { picks: Record<string, DraftPick>; marks: Record<string, boolean>; remaining_sec?: number; idx?: number }
export interface AnswerRow { bank_id: string; qid: string; picked: string | null; is_correct: boolean | null; time_cost_ms: number | null; question: QuestionRow | null }
export interface SessionDetail { session: SessionInfo; records: AnswerRow[] }
export interface SessionBrief { session_id: number; mode: string; title: string; score: number | null; correct_qty: number; scored_qty: number; finished_at: string | null; duration_ms: number | null }
export interface TopicAcc { topic: string; a: number; c: number }
export interface Dashboard { answered: number; correct: number; sessions_done: number; streak_days: number; due_count: number; wrong_active: number; by_topic: TopicAcc[]; recent: SessionBrief[] }
export interface WrongRow { bank_id: string; qid: string; wrong_count: number; last_wrong_at: string; repetitions: number; due_date: string | null; question: QuestionRow }
export interface SpikeResult { rows: number; seg_build_ms: number; like_avg_ms: number; fts_avg_ms: number; fts_hits: number; like_hits: number }

// ---------- 命令 ----------
export const api = {
  overview: () => invoke<Overview>('bank_overview'),
  papers: () => invoke<PaperInfo[]>('list_papers'),
  paperQids: (paperId: number) => invoke<QID[]>('paper_questions', { paperId }),
  questions: (f: { topic_id?: number; qtype?: string; status?: string; search?: string; limit?: number; offset?: number }) =>
    invoke<QuestionRow[]>('list_questions', f),
  questionsByIds: (qids: QID[]) => invoke<QuestionRow[]>('get_questions_by_ids', { qids }),
  ftsSpike: (scaleTo = 100000) => invoke<SpikeResult>('fts_spike', { scaleTo }),

  startSession: (mode: string, title: string, bankId: string, paperId: number | null, qids: QID[], timeLimitSec: number | null) =>
    invoke<SessionInfo>('start_session', { mode, title, bankId, paperId, qids, timeLimitSec }),
  saveDraft: (sessionId: number, draft: DraftJson) => invoke<null>('save_draft', { sessionId, draft }),
  finishSession: (sessionId: number) => invoke<SessionInfo>('finish_session', { sessionId }),
  sessionDetail: (sessionId: number) => invoke<SessionDetail>('session_detail', { sessionId }),
  unfinished: () => invoke<SessionInfo[]>('unfinished_sessions'),
  sessions: () => invoke<SessionBrief[]>('list_sessions'),

  dashboard: () => invoke<Dashboard>('dashboard'),
  dueReview: (limit = 20) => invoke<QuestionRow[]>('due_review', { limit }),
  wrongList: () => invoke<WrongRow[]>('wrong_list'),
  wrongClear: (bankId: string, qid: string) => invoke<null>('wrong_clear', { bankId, qid }),
  favToggle: (bankId: string, qid: string) => invoke<boolean>('fav_toggle', { bankId, qid }),
  favList: () => invoke<QuestionRow[]>('fav_list'),
  noteGet: (bankId: string, qid: string) => invoke<string | null>('note_get', { bankId, qid }),
  noteSet: (bankId: string, qid: string, content: string) => invoke<null>('note_set', { bankId, qid, content }),

  // M2
  compose: (blueprint: Blueprint) => invoke<ComposeReport>('compose_blueprint', { blueprint }),
  activity: (days = 120) => invoke<DayCount[]>('activity_calendar', { days }),
  exportExcel: (sessionId: number, path: string) => invoke<string>('export_session_excel', { sessionId, path }),
  backup: (dest: string) => invoke<string>('backup_user', { dest }),
  restoreCheck: (zipPath: string) => invoke<RestoreInfo>('restore_check', { zipPath }),
  diagnostics: (dest: string) => invoke<string>('export_diagnostics', { dest }),
  getSetting: (key: string) => invoke<string | null>('setting_get', { key }),
  setSetting: (key: string, value: string) => invoke<null>('setting_set', { key, value }),

  // 可维护性：日志
  logsRead: (tail = 200) => invoke<LogView>('logs_read', { tail }),
  openLogDir: () => invoke<null>('open_log_dir'),
}

export interface LogView { path: string | null; lines: string[] }

// 前端异常落滚动日志（命令级打点由 Rust 侧完成，这里只上报渲染层/未捕获异常）
export async function logFrontendError(where: string, err: unknown): Promise<void> {
  const detail = err instanceof Error ? `${err.message}\n${err.stack ?? ''}` : String(err)
  console.error(`[${where}]`, err)
  if (!hasTauri) return
  try {
    const { error } = await import('@tauri-apps/plugin-log')
    await error(`[${where}] ${detail}`)
  } catch { /* 日志链路自身故障时放弃，避免递归上报 */ }
}

// ---------- M2 类型 ----------
export interface SectionSpec { type: string; qty: number; score_each?: number; from_topics?: number[]; difficulty?: [number, number] | null }
export interface Blueprint { name: string; time_limit_min: number; sections: SectionSpec[]; allow_fallback: boolean }
export interface SectionReport { qtype: string; requested: number; actual: number; fallback: string | null }
export interface ComposeReport { sections: SectionReport[]; total: number; qids: QID[] }
export interface DayCount { date: string; count: number }
export interface RestoreInfo { sessions: number; records: number; created_at: string }

// 文件对话框（仅 Tauri）
export async function saveDialog(opts: { defaultPath?: string; filters?: { name: string; extensions: string[] }[] }): Promise<string | null> {
  if (!hasTauri) return null
  const { save } = await import('@tauri-apps/plugin-dialog')
  return save(opts)
}
export async function openDialog(opts: { filters?: { name: string; extensions: string[] }[] }): Promise<string | null> {
  if (!hasTauri) return null
  const { open } = await import('@tauri-apps/plugin-dialog')
  const r = await open(opts)
  return typeof r === 'string' ? r : null
}

// ---------- M3：资料检索 ----------
export interface DocHit { title: string; path: string; snippet: string }
export interface DocsStatus { chunks: number; built_at: string }
export const docsApi = {
  status: () => invoke<DocsStatus>('docs_status'),
  build: (force = false) => invoke<number>('docs_build', { force }),
  search: (query: string, limit = 20) => invoke<DocHit[]>('docs_search', { query, limit }),
}

// ---------- M3：导入/去重/打印 ----------
export interface ParsedQ { stem: string; qtype: string; options: string[]; answer: string; explain: string; source: string; topic1: string; topic2: string; difficulty: number; conf: string }
export interface RowError { row: number; msg: string }
export interface ExcelPreview { total: number; valid: number; errors: RowError[]; sample: ParsedQ[] }
export interface ExcelImportReport { bank_id: string; bank_name: string; imported: number; skipped: number; topics: number; errors: RowError[] }
export interface DupItem { qid: string; stem: string; status: string }
export interface DupGroup { kind: string; items: DupItem[] }
export interface PrintSection { qtype: string; score_each: number; questions: QuestionRow[] }
export interface PrintPaper { name: string; title: string; total_score: number; total_count: number; sections: PrintSection[] }

export const m3Api = {
  excelPreview: (path: string) => invoke<ExcelPreview>('excel_preview', { path }),
  excelImport: (path: string, bankName: string) => invoke<ExcelImportReport>('excel_import', { path, bankName }),
  exportTemplate: (path: string) => invoke<string>('export_excel_template', { path }),
  dedupScan: (bankId: string) => invoke<DupGroup[]>('dedup_scan', { bankId }),
  dedupMerge: (bankId: string, keep: string, removes: string[]) => invoke<number>('dedup_merge', { bankId, keep, removes }),
  printData: (paperId: number) => invoke<PrintPaper>('paper_print_data', { paperId }),
}

// ---------- Mock（浏览器 E2E 用；规则镜像 Rust：全对才得分/未答=错/低置信不计分/SM-2） ----------
const norm = (s: string) => [...new Set((s || '').split('').filter(c => 'ABCDE'.includes(c)))].sort().join('')
const mGrade = (q: QuestionRow, picked: string): boolean | null => {
  if (!q.answer || q.answer_conf !== 'high') return null
  const a = norm(q.answer), p = norm(picked)
  if (!p) return false
  return p === a
}
class MockDB {
  sessions: SessionInfo[] = []
  sid = 0
  records: AnswerRow[][] = []
  wrong: Record<string, { count: number; rep: number }> = {}
  favs: Record<string, true> = {}
  answered = 0; correct = 0
}
const mdb = new MockDB()
const MQUESTIONS: QuestionRow[] = []
const MTOPICS: TopicStat[] = []
async function mock<T>(cmd: string, args?: Record<string, any>): Promise<T> {
  await new Promise(r => setTimeout(r, 60))
  switch (cmd) {
    case 'bank_overview':
      return { banks: [{ bank_id: 'smart-core', name: 'S7-200 SMART 认证题库（mock）', version: 1, total: MQUESTIONS.length, active: MQUESTIONS.length, pending: 0, papers: 1 }], topics: MTOPICS } as T
    case 'list_questions': {
      let qs = MQUESTIONS.filter(q => q.status !== 'retired')
      if (args?.topic_id) qs = qs.filter(q => q.topics.includes(MTOPICS.find(t => t.topic_id === args.topic_id)!.name))
      if (args?.qtype) qs = qs.filter(q => q.qtype === args.qtype)
      if (args?.status) qs = qs.filter(q => q.status === args.status)
      if (args?.search) qs = qs.filter(q => (q.stem + q.explain).includes(args.search))
      return qs.slice(args?.offset ?? 0, (args?.offset ?? 0) + (args?.limit ?? 50)) as T
    }
    case 'get_questions_by_ids':
      return (args!.qids as QID[]).map(([b, q]) => MQUESTIONS.find(x => x.bank_id === b && x.qid === q)!).filter(Boolean) as T
    case 'list_papers':
      return [{ paper_id: 1, bank_id: 'smart-core', name: 'A卷', title: '模拟卷A（mock）', count: MQUESTIONS.length }] as T
    case 'paper_questions':
      return MQUESTIONS.map(q => [q.bank_id, q.qid] as QID) as T
    case 'start_session': {
      const s: SessionInfo = { session_id: ++mdb.sid, mode: args!.mode, title: args!.title, time_limit_sec: args!.timeLimitSec ?? null,
        total_qty: args!.qids.length, started_at: new Date().toISOString(), finished_at: null, score: null, correct_qty: 0, scored_qty: 0,
        qid_list: args!.qids, draft: { picks: {}, marks: {} } }
      mdb.sessions.push(s); mdb.records.push([])
      return s as T
    }
    case 'save_draft': {
      const s = mdb.sessions.find(x => x.session_id === args!.sessionId)!
      s.draft = args!.draft; return null as T
    }
    case 'finish_session': {
      const s = mdb.sessions.find(x => x.session_id === args!.sessionId)!
      if (s.finished_at) return s as T
      const qs = await mock<QuestionRow[]>('get_questions_by_ids', { qids: s.qid_list })
      const picks = s.draft?.picks ?? {}
      let scored = 0, correct = 0
      const recs: AnswerRow[] = []
      for (const q of qs) {
        const picked = picks[`${q.bank_id}::${q.qid}`]?.picked ?? ''
        const g = mGrade(q, picked)
        if (g !== null) { scored++; if (g) correct++ }
        recs.push({ bank_id: q.bank_id, qid: q.qid, picked, is_correct: g, time_cost_ms: null, question: q })
        mdb.answered++
        if (g === true) mdb.correct++
        const k = `${q.bank_id}::${q.qid}`
        if (g === false) mdb.wrong[k] = { count: (mdb.wrong[k]?.count ?? 0) + 1, rep: 0 }
        if (g !== null && !['exam', 'recite'].includes(s.mode) && mdb.wrong[k]) mdb.wrong[k].rep = g ? mdb.wrong[k].rep + 1 : 0
      }
      mdb.records[s.session_id - 1] = recs
      s.finished_at = new Date().toISOString()
      s.scored_qty = scored; s.correct_qty = correct
      s.score = scored ? Math.round((correct / scored) * 10000) / 100 : null
      return s as T
    }
    case 'session_detail': {
      const s = mdb.sessions.find(x => x.session_id === args!.sessionId)!
      return { session: s, records: mdb.records[s.session_id - 1] } as T
    }
    case 'unfinished_sessions': return mdb.sessions.filter(s => !s.finished_at) as T
    case 'list_sessions': return [...mdb.sessions].reverse().filter(s => s.finished_at) as T
    case 'dashboard': {
      const due = Object.entries(mdb.wrong).filter(([, w]) => w.rep < 2 && w.count > 0).length
      return { answered: mdb.answered, correct: mdb.correct, sessions_done: mdb.sessions.filter(s => s.finished_at).length,
        streak_days: mdb.sessions.length, due_count: due, wrong_active: due, by_topic: [], recent: await mock<SessionBrief[]>('list_sessions') } as T
    }
    case 'wrong_list': {
      const out: WrongRow[] = []
      for (const [k, w] of Object.entries(mdb.wrong)) {
        if (w.rep >= 2) continue
        const [b, q] = k.split('::')
        const question = MQUESTIONS.find(x => x.bank_id === b && x.qid === q)
        if (question) out.push({ bank_id: b, qid: q, wrong_count: w.count, last_wrong_at: '', repetitions: w.rep, due_date: null, question })
      }
      return out as T
    }
    case 'due_review': return (await mock<WrongRow[]>('wrong_list')).map(w => w.question) as T
    case 'wrong_clear': { delete mdb.wrong[`${args!.bankId}::${args!.qid}`]; return null as T }
    case 'fav_toggle': {
      const k = `${args!.bankId}::${args!.qid}`
      if (mdb.favs[k]) { delete mdb.favs[k]; return false as T }
      mdb.favs[k] = true; return true as T
    }
    case 'fav_list': return Object.keys(mdb.favs).map(k => { const [b, q] = k.split('::'); return MQUESTIONS.find(x => x.bank_id === b && x.qid === q)! }).filter(Boolean) as T
    case 'note_get': return null as T
    case 'note_set': return null as T
    case 'setting_get': return (localStorage.getItem('sqset_' + args!.key) ?? null) as T
    case 'setting_set': { localStorage.setItem('sqset_' + args!.key, args!.value); return null as T }
    case 'compose_blueprint': {
      const bp = args!.blueprint as Blueprint
      const qs = MQUESTIONS.filter(q => q.status === 'active' && q.answer_conf === 'high')
      const secs: SectionReport[] = []
      const used = new Set<string>()
      const qids: QID[] = []
      for (const s of bp.sections) {
        let pool = qs.filter(q => q.qtype === s.type && !used.has(q.qid))
        let fb: string | null = null
        let actual = s.qty
        if (pool.length < s.qty) {
          if (!bp.allow_fallback) throw new Error(`候选不足：题型${s.type} 需要${s.qty}题，可用${pool.length}题`)
          actual = pool.length; fb = `题量 ${s.qty}→${pool.length}`
        }
        pool = pool.sort(() => Math.random() - .5).slice(0, actual)
        pool.forEach(q => { used.add(q.qid); qids.push([q.bank_id, q.qid]) })
        secs.push({ qtype: s.type, requested: s.qty, actual: pool.length, fallback: fb })
      }
      return { sections: secs, total: qids.length, qids } as T
    }
    case 'activity_calendar': {
      const byDay: Record<string, number> = {}
      for (let i = 1; i <= mdb.sid; i++) {
        const s = mdb.sessions[i - 1]
        if (!s?.finished_at) continue
        const d = s.finished_at.slice(0, 10)
        byDay[d] = (byDay[d] ?? 0) + (mdb.records[i - 1]?.length ?? 0)
      }
      return Object.entries(byDay).map(([date, count]) => ({ date, count })).slice(0, args?.days ?? 120) as T
    }
    case 'export_session_excel': case 'backup_user': case 'export_diagnostics': return 'C:\\mock\\导出文件' as T
    case 'restore_check': return { sessions: 0, records: 0, created_at: '' } as T
    case 'logs_read': return { path: 'C:\\mock\\logs\\smart-quiz-app.log', lines: [
      '[19:30:01][INFO ][app] 启动 smart-quiz-app v0.1.0（mock）',
      '[19:30:02][INFO ][seed] 导入 smart-core v1：694题 6卷 11图',
      '[19:30:03][INFO ][session] 开始会话#1 exam「模拟卷A（mock）」87题',
      '[19:35:00][INFO ][session] 会话#1 完成：得分85.7 对74/87计分题',
      '[19:35:02][ERROR][cmd] export_excel_template 失败(12ms): mock: 模板导出仅应用内可用',
    ] } as T
    case 'open_log_dir': return null as T
    case 'docs_status': return { chunks: 0, built_at: '' } as T
    case 'docs_build': return 0 as T
    case 'excel_preview': return { total: 2, valid: 2, errors: [], sample: MQUESTIONS.slice(0, 2).map(q => ({ stem: q.stem, qtype: q.qtype, options: q.options, answer: q.answer, explain: q.explain, source: q.source, topic1: q.topics[0] ?? '未分类', topic2: '', difficulty: 3, conf: 'high' })) } as T
    case 'excel_import': return { bank_id: 'xlsx-mock', bank_name: String(args?.bankName ?? 'Excel题库'), imported: 2, skipped: 0, topics: 2, errors: [] } as T
    case 'export_excel_template': return 'C:\\mock\\题库模板.xlsx' as T
    case 'dedup_scan': return [] as T
    case 'dedup_merge': return ((args?.removes as string[])?.length ?? 0) as unknown as T
    case 'paper_print_data': {
      const p = (await mock<any>('list_papers'))[0]
      const qs = MQUESTIONS
      return { name: p.name, title: p.title, total_score: qs.length, total_count: qs.length,
        sections: [{ qtype: 'single', score_each: 1, questions: qs.filter(q => q.qtype === 'single') },
                   { qtype: 'multi', score_each: 1, questions: qs.filter(q => q.qtype === 'multi') }] } as T
    }
    case 'docs_search': {
      const q = String(args?.query ?? '')
      if (!q) return [] as T
      return [
        { title: `模拟量输入换算（mock）`, path: 'manual/07_设备组态.txt', snippet: `…${q}…普通模拟量通道值范围 0~27648，4~20mA 对应 5530~27648，按线性比例换算工程量…` },
        { title: 'MBUS_MSG 指令（mock）', path: 'microwin/116160224907.txt', snippet: `…包含 ${q} 的 Modbus 主站读写参数：Slave 从站地址、Addr 起始地址（40001 保持寄存器）、Count 最多 120 字…` },
      ] as T
    }
    case 'fts_spike': throw new Error('FTS 基准仅 Tauri 环境可用')
    default: throw new Error(`mock 未实现: ${cmd}`)
  }
}

// mock 数据初始化（首次调用自动注入）
export async function initMock() {
  if (hasTauri || MQUESTIONS.length) return
  const seed: Array<[string, string, string, string[], string, string]> = [
    ['single', '硬件与选型', '标准型 CPU 最多可扩展多少个扩展模块？', ['3 个', '6 个', '8 个', '12 个'], 'B', 'SR/ST 标准型最多 6 个扩展模块 + 1 个信号板。'],
    ['single', '硬件与选型', 'CPU SR40 的输出类型是？', ['晶体管', '继电器', '晶闸管', '模拟量'], 'B', 'SR=继电器输出，ST=晶体管输出。'],
    ['multi', '串口通信', 'Modbus RTU 通信需要设置哪些参数？', ['波特率', '数据位', '校验位', 'IP 地址'], 'ABC', 'IP 地址属于以太网通信。'],
    ['multi', '以太网通信', '以下属于开放式通信协议的有？', ['TCP', 'UDP', 'ISO-on-TCP', 'PPI'], 'ABC', 'PPI 是西门子专用协议（RS485/以太网上的专有协议）。'],
    ['single', '基本指令', 'TON 是什么定时器？', ['延时断开', '延时接通', '保持性延时接通', '自复位'], 'B', 'TON=通电延时接通。'],
    ['multi', '基本指令', '下列哪些指令属于程序控制类？', ['FOR/NEXT', 'JMP/LBL', 'CALL', 'MOV_B'], 'ABC', 'MOV_B 是传送指令。'],
  ]
  seed.forEach((s, i) => {
    MQUESTIONS.push({
      bank_id: 'smart-core', qid: `SC-T${String(i + 1).padStart(3, '0')}`, qtype: s[0],
      stem: s[2], img_path: null, options: s[3].map((o, j) => `${String.fromCharCode(65 + j)}、${o}`),
      answer: s[4], answer_conf: 'high', explain: s[5], source: 'mock 数据', status: 'active',
      topics: [s[1]],
    })
    if (!MTOPICS.find(t => t.name === s[1])) MTOPICS.push({ topic_id: MTOPICS.length + 1, name: s[1], total: 0, active: 0 })
  })
  MTOPICS.forEach(t => { t.total = MQUESTIONS.filter(q => q.topics.includes(t.name)).length; t.active = t.total })
}
initMock()
