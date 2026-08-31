// 浏览器 Mock 引擎 —— 仅供开发/测试构建使用（api.ts 的 invoke 在 !hasTauri 且 DEV 时动态加载本模块，
// 生产构建整条分支被摇除，本文件不进产物）。
// 规则镜像 Rust：全对才得分 / 未答=错 / 低置信不计分 / SM-2（权威实现在 src-tauri，此处只为 UI 流程测试服务）
import type { QuestionRow, SessionInfo, SessionBrief, AnswerRow, WrongRow, TopicStat, QID, Blueprint, SectionReport } from './api'

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
  records: Record<number, AnswerRow[]> = {}   // 按 session_id 键控（放弃中间会话不打乱索引）
  wrong: Record<string, { count: number; rep: number }> = {}
  favs: Record<string, true> = {}
  answered = 0; correct = 0
}
const mdb = new MockDB()
// mock 持久化：模拟真实 SQLite 落盘——页面刷新后未完成会话/成绩/错题保留（e2e 可测"下次打开续做"）
const MOCK_DB_KEY = 'sqmock_db'
function mockPersist() {
  try {
    localStorage.setItem(MOCK_DB_KEY, JSON.stringify({
      sessions: mdb.sessions, sid: mdb.sid, records: mdb.records,
      wrong: mdb.wrong, favs: mdb.favs, answered: mdb.answered, correct: mdb.correct,
    }))
  } catch { /* 配额满等异常忽略，内存仍可用 */ }
}
function mockRestore() {
  try {
    const raw = localStorage.getItem(MOCK_DB_KEY)
    if (raw) Object.assign(mdb, JSON.parse(raw))
  } catch { /* 损坏则从空库开始 */ }
}
mockRestore()
const MQUESTIONS: QuestionRow[] = []
const MTOPICS: TopicStat[] = []
export async function mock<T>(cmd: string, args?: Record<string, any>): Promise<T> {
  await new Promise(r => setTimeout(r, 60))
  switch (cmd) {
    case 'bank_overview':
      return { banks: [{ bank_id: 'smart-core', name: 'S7-200 SMART 认证题库（mock）', version: 1, total: MQUESTIONS.length, active: MQUESTIONS.length, pending: 0, papers: 1 }], topics: MTOPICS } as T
    case 'list_questions': {
      // 前端已统一 camelCase 传参（Tauri v2 约定），保留 snake_case 兼容读取
      const topicId = args?.topicId ?? args?.topic_id
      let qs = MQUESTIONS.filter(q => q.status !== 'retired')
      if (topicId) qs = qs.filter(q => q.topics.includes(MTOPICS.find(t => t.topic_id === topicId)!.name))
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
      mdb.sessions.push(s); mdb.records[s.session_id] = []
      mockPersist()
      return s as T
    }
    case 'save_draft': {
      const s = mdb.sessions.find(x => x.session_id === args!.sessionId)
      if (!s || s.finished_at) throw new Error('会话不存在或已完成')  // 与 Rust 0行→Err 语义对齐
      s.draft = args!.draft; mockPersist(); return null as T
    }
    case 'discard_session': {
      const i = mdb.sessions.findIndex(x => x.session_id === args!.sessionId && !x.finished_at)
      if (i < 0) throw new Error('会话不存在或已完成，无法放弃')
      mdb.sessions.splice(i, 1)
      mockPersist()
      return null as T
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
      mdb.records[s.session_id] = recs
      s.finished_at = new Date().toISOString()
      s.scored_qty = scored; s.correct_qty = correct
      s.score = scored ? Math.round((correct / scored) * 100) : null
      mockPersist()
      return s as T
    }
    case 'session_detail': {
      const s = mdb.sessions.find(x => x.session_id === args!.sessionId)
      if (!s) throw new Error('会话不存在')
      return { session: s, records: mdb.records[s.session_id] ?? [] } as T
    }
    case 'unfinished_sessions': return [...mdb.sessions].reverse().filter(s => !s.finished_at) as T
    case 'list_sessions': return [...mdb.sessions].reverse().filter(s => s.finished_at) as T
    case 'dashboard': {
      const due = Object.entries(mdb.wrong).filter(([, w]) => w.rep < 2 && w.count > 0).length
      // by_topic 按主题聚合作答数/正确数（示例数据供掌握度视图展示）
      const byTopic: Record<string, { a: number; c: number }> = {}
      for (const s of mdb.sessions) {
        if (!s.finished_at) continue
        const recs = mdb.records[s.session_id] ?? []
        for (const r of recs) {
          const q = MQUESTIONS.find(x => x.bank_id === r.bank_id && x.qid === r.qid)
          const t = q?.topics[0]
          if (!t || r.is_correct === null) continue
          const e = byTopic[t] ?? (byTopic[t] = { a: 0, c: 0 })
          e.a++; if (r.is_correct) e.c++
        }
      }
      return { answered: mdb.answered, correct: mdb.correct, sessions_done: mdb.sessions.filter(s => s.finished_at).length,
        streak_days: mdb.sessions.length, due_count: due, wrong_active: due, by_topic: Object.entries(byTopic).map(([topic, v]) => ({ topic, ...v })), recent: await mock<SessionBrief[]>('list_sessions') } as T
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
    case 'wrong_clear': { delete mdb.wrong[`${args!.bankId}::${args!.qid}`]; mockPersist(); return null as T }
    case 'fav_toggle': {
      const k = `${args!.bankId}::${args!.qid}`
      if (mdb.favs[k]) { delete mdb.favs[k]; mockPersist(); return false as T }
      mdb.favs[k] = true; mockPersist(); return true as T
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
      // 按会话遍历（records 已按 session_id 键控；放弃会话会 splice，按下标取会错位）
      const byDay: Record<string, number> = {}
      for (const s of mdb.sessions) {
        if (!s.finished_at) continue
        const d = s.finished_at.slice(0, 10)
        byDay[d] = (byDay[d] ?? 0) + (mdb.records[s.session_id]?.length ?? 0)
      }
      return Object.entries(byDay).map(([date, count]) => ({ date, count })).slice(0, args?.days ?? 120) as T
    }
    case 'export_session_excel': case 'backup_user': case 'export_diagnostics': return 'C:\\mock\\导出文件' as T
    case 'restore_check': return { sessions: 0, records: 0, created_at: '' } as T
    case 'logs_read': return { path: 'C:\\mock\\logs\\smart-quiz-app.log', lines: [
      '[19:30:01][INFO ][app] 启动 smart-quiz-app v0.1.0（mock）',
      '[19:30:02][INFO ][seed] 导入 smart-core v1：694题 6卷 11图',
      '[19:30:03][INFO ][session] 开始会话#1 exam 87题',
      '[19:35:00][INFO ][session] 会话#1 完成：得分85.7 对74/87计分题',
      '[19:35:02][ERROR][cmd] export_excel_template 失败(12ms): mock: 模板导出仅应用内可用',
    ] } as T
    case 'open_log_dir': return null as T
    case 'import_bank_file': {
      const name = String(args?.path ?? '').split(/[\\/]/).pop()?.replace(/\.smartbank$/i, '') || '题库包'
      return { bank_id: 'smart-core', bank_name: `${name}（mock）`, questions: 694, papers: 6, images: 11, skipped: false } as T
    }
    case 'import_docpack': return 1690000 as T
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

// 示例题注入（模块加载即执行；本模块只在非 Tauri 环境被加载）
{
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
