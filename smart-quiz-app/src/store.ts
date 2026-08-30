// 极简全局状态 + 视图路由（M1 不引入 pinia/router，保持零额外依赖）
import { reactive } from 'vue'
import type { SessionInfo, QuestionRow, DraftJson } from './api'

export interface SessionCtx {
  session: SessionInfo
  questions: QuestionRow[]
  draft: DraftJson
  idx: number
  examMode: boolean
  recite: boolean
  remainingSec: number | null
  startTs: number
}

export const store = reactive({
  view: 'home' as string,
  topicId: null as string | null,
  params: {} as Record<string, any>,
  sessionCtx: null as SessionCtx | null,
  lastResultId: null as number | null,
  go(view: string, params: Record<string, any> = {}) {
    store.view = view
    store.params = params
    window.scrollTo(0, 0)
  },
  openSession(ctx: SessionCtx) {
    store.sessionCtx = ctx
    store.go('session')
  },
})
