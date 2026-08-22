// 会话启动辅助：各入口统一走这里
import { api, type QuestionRow, type QID, type SessionInfo } from './api'
import { store } from './store'

const EXAM_MIN = 90

export async function startWithQuestions(mode: string, title: string, questions: QuestionRow[], opts: { exam?: boolean; recite?: boolean; timeLimitSec?: number | null } = {}) {
  const qids: QID[] = questions.map(q => [q.bank_id, q.qid])
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
  const questions = await api.questionsByIds(s.qid_list)
  store.openSession({
    session: s, questions,
    draft: (s.draft as any) ?? { picks: {}, marks: {} },
    idx: (s.draft && (s.draft as any).idx) ?? 0,
    examMode: s.mode === 'exam',
    recite: s.mode === 'recite',
    remainingSec: s.time_limit_sec !== null ? ((s.draft as any)?.remaining_sec ?? s.time_limit_sec) : null,
    startTs: Date.now(),
  })
}

export async function startPaper(paperId: number, name: string, title: string) {
  const qids = await api.paperQids(paperId)
  const questions = await api.questionsByIds(qids)
  await startWithQuestions('exam', `${name} ${title}`, questions, { exam: true })
}
