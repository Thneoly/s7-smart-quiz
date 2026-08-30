// Tauri invoke 封装 + 浏览器 mock 回退
// mock 引擎在独立 mock.ts：仅开发/测试构建（import.meta.env.DEV）动态加载，
// 生产构建整条分支被摇除——正式产物不含 mock 与示例题数据
import { convertFileSrc, invoke as tauriInvoke } from '@tauri-apps/api/core'

export const hasTauri = typeof window !== 'undefined' && '__TAURI_INTERNALS__' in window

function invoke<T>(cmd: string, args?: Record<string, unknown>): Promise<T> {
  if (hasTauri) return tauriInvoke<T>(cmd, args)
  if (import.meta.env.DEV) return import('./mock').then(m => m.mock<T>(cmd, args))
  return Promise.reject(new Error('浏览器 Mock 仅开发模式可用'))
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
export interface DraftJson { picks: Record<string, DraftPick>; marks: Record<string, boolean>; remaining_sec?: number; idx?: number; saved_at?: number }
export interface AnswerRow { bank_id: string; qid: string; picked: string | null; is_correct: boolean | null; time_cost_ms: number | null; question: QuestionRow | null }
export interface SessionDetail { session: SessionInfo; records: AnswerRow[] }
export interface SessionBrief { session_id: number; mode: string; title: string; score: number | null; correct_qty: number; scored_qty: number; finished_at: string | null; duration_ms: number | null }
export interface TopicAcc { topic: string; a: number; c: number }
export interface Dashboard { answered: number; correct: number; sessions_done: number; streak_days: number; due_count: number; wrong_active: number; by_topic: TopicAcc[]; recent: SessionBrief[] }
export interface WrongRow { bank_id: string; qid: string; wrong_count: number; last_wrong_at: string; repetitions: number; due_date: string | null; question: QuestionRow }
export interface SpikeResult { rows: number; seg_build_ms: number; like_avg_ms: number; fts_avg_ms: number; fts_hits: number; like_hits: number }

/** 会话模式的中文显示名（首页/练习/记录页共用） */
export const MODE_NAME: Record<string, string> = { practice: '章节练习', random: '随机练习', recite: '背诵', review: '间隔复习', wrong: '错题重练', fav: '收藏练习', exam: '考试' }

// ---------- 命令 ----------
export const api = {
  overview: () => invoke<Overview>('bank_overview'),
  papers: () => invoke<PaperInfo[]>('list_papers'),
  paperQids: (paperId: number) => invoke<QID[]>('paper_questions', { paperId }),
  questions: (f: { topic_id?: number; qtype?: string; status?: string; search?: string; limit?: number; offset?: number }) =>
    // 注意：invoke 只认 camelCase 键（Tauri v2 约定）——曾因直传 snake_case 对象导致
    // topic_id 被静默丢弃，所有"按主题练习"实际下发全量题库（生产独有，mock 不做大小写转换）
    invoke<QuestionRow[]>('list_questions',
      { topicId: f.topic_id, qtype: f.qtype, status: f.status, search: f.search, limit: f.limit, offset: f.offset }),
  questionsByIds: (qids: QID[]) => invoke<QuestionRow[]>('get_questions_by_ids', { qids }),
  ftsSpike: (scaleTo = 100000) => invoke<SpikeResult>('fts_spike', { scaleTo }),

  startSession: (mode: string, title: string, bankId: string, paperId: number | null, qids: QID[], timeLimitSec: number | null) =>
    invoke<SessionInfo>('start_session', { mode, title, bankId, paperId, qids, timeLimitSec }),
  saveDraft: (sessionId: number, draft: DraftJson) => invoke<null>('save_draft', { sessionId, draft }),
  finishSession: (sessionId: number) => invoke<SessionInfo>('finish_session', { sessionId }),
  sessionDetail: (sessionId: number) => invoke<SessionDetail>('session_detail', { sessionId }),
  unfinished: () => invoke<SessionInfo[]>('unfinished_sessions'),
  discardSession: (sessionId: number) => invoke<null>('discard_session', { sessionId }),
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
  importPack: (path: string) => invoke<number>('import_docpack', { path }),
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

export interface BankPackReport { bank_id: string; bank_name: string; questions: number; papers: number; images: number; skipped: boolean }

export const m3Api = {
  excelPreview: (path: string) => invoke<ExcelPreview>('excel_preview', { path }),
  excelImport: (path: string, bankName: string) => invoke<ExcelImportReport>('excel_import', { path, bankName }),
  exportTemplate: (path: string) => invoke<string>('export_excel_template', { path }),
  dedupScan: (bankId: string) => invoke<DupGroup[]>('dedup_scan', { bankId }),
  dedupMerge: (bankId: string, keep: string, removes: string[]) => invoke<number>('dedup_merge', { bankId, keep, removes }),
  printData: (paperId: number) => invoke<PrintPaper>('paper_print_data', { paperId }),
  importBankPack: (path: string) => invoke<BankPackReport>('import_bank_file', { path }),
}
