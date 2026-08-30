// 会话启动辅助：各入口统一走这里
import { api, type QuestionRow, type QID, type SessionInfo, type DraftJson } from './api'
import { store } from './store'

const EXAM_MIN = 90

// 同一套题（qid 顺序一致）判定：用于"同一任务最多一个进行中会话"
function sameQids(a: QID[], b: QID[]) {
  return a.length === b.length && a.every((p, i) => p[0] === b[i][0] && p[1] === b[i][1])
}

export async function startWithQuestions(mode: string, title: string, questions: QuestionRow[], opts: { exam?: boolean; recite?: boolean; timeLimitSec?: number | null } = {}) {
  const qids: QID[] = questions.map(q => [q.bank_id, q.qid])
  // 防重复：已有未完成的同 mode+同题集会话 → 直接续做最新一份，更旧的重复自动清理
  //（未完成列表按 session_id 降序，dup[0] 即最新；这也自愈历史堆积的重复会话）
  const unfinished = await api.unfinished().catch(() => [] as SessionInfo[])
  const dup = unfinished.filter(s => s.mode === mode && sameQids(s.qid_list, qids))
  if (dup.length) {
    dup.slice(1).forEach(s => { api.discardSession(s.session_id).catch(() => {}) })
    if (store.sessionCtx?.session.session_id === dup[0].session_id) { store.go('session'); return }
    await resumeSession(dup[0])
    return
  }
  const s: SessionInfo = await api.startSession(mode, title, questions[0]?.bank_id ?? 'smart-core', null, qids,
    opts.exam ? (opts.timeLimitSec ?? EXAM_MIN * 60) : null)
  store.openSession({
    session: s, questions, draft: { picks: {}, marks: {} }, idx: 0,
    examMode: !!opts.exam, recite: !!opts.recite,
    remainingSec: opts.exam ? (opts.timeLimitSec ?? EXAM_MIN * 60) : null,
    startTs: Date.now(),
  })
}

export async function resumeSession(s: SessionInfo) {
  // 防御 1：按 qid_list 重排——draft.idx 是位置索引，questionsByIds 不保证顺序
  const fetched = await api.questionsByIds(s.qid_list)
  const byK = new Map(fetched.map(q => [`${q.bank_id}::${q.qid}`, q]))
  const questions = s.qid_list.map(([b, q]) => byK.get(`${b}::${q}`)).filter(Boolean) as QuestionRow[]
  const draft: DraftJson = s.draft ?? { picks: {}, marks: {} }
  // 防御 2：题库变动（去重/删除）致题数缩水时 idx 夹取，避免落空态
  const idx = Math.min(Math.max(draft.idx ?? 0, 0), Math.max(questions.length - 1, 0))
  // 防御 3：考试剩余时间按墙钟续算（以最近一次草稿保存为锚），关窗不再"暂停计时"
  // elapsed 钳非负 + remaining 封顶总限时——防时钟回拨把剩余时间"充"回超过总限时
  let remaining: number | null = null
  if (s.time_limit_sec !== null) {
    const saved = draft.remaining_sec ?? s.time_limit_sec
    const elapsed = draft.saved_at ? Math.max(0, Math.floor((Date.now() - draft.saved_at) / 1000)) : 0
    remaining = Math.min(Math.max(0, saved - elapsed), s.time_limit_sec)
  }
  // 防御 4：恢复时已到时 → 直接判分进结果页（避免恢复即弹未答确认的循环）
  if (s.mode === 'exam' && remaining === 0) {
    const done = await api.finishSession(s.session_id)
    store.lastResultId = done.session_id
    store.sessionCtx = null
    store.go('result', { id: done.session_id })
    return
  }
  store.openSession({
    session: s, questions, draft, idx,
    examMode: s.mode === 'exam',
    recite: s.mode === 'recite',
    remainingSec: remaining,
    startTs: Date.now(),
  })
}

export async function startPaper(paperId: number, name: string, title: string) {
  const qids = await api.paperQids(paperId)
  const questions = await api.questionsByIds(qids)
  await startWithQuestions('exam', `${name} ${title}`, questions, { exam: true })
}
