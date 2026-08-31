<script setup lang="ts">
import { ref, computed, onMounted, onUnmounted } from 'vue'
import { store } from '../store'
import { api, assetUrl, type QuestionRow } from '../api'

const ctx = computed(() => store.sessionCtx)
const q = computed(() => ctx.value ? ctx.value.questions[ctx.value.idx] : null)
const isMulti = computed(() => q.value?.qtype === 'multi')
const locked = ref(false)                // 练习模式：本题已判分
const showAns = ref(false)               // 背诵模式
const sheet = ref(false)
const noteOpen = ref(false)
const noteText = ref('')
const favOn = ref(false)
const finishing = ref(false)
const localGrade = ref<boolean | null>(null)
let saveTimer: number | null = null
let tickTimer: number | null = null
const now = ref(Date.now())

// ⚡自动下一题（仅练习模式）：答对 1s / 答错 2.5s（留看解析）后自动前进，默认关闭、记忆偏好
const autoNext = ref(localStorage.getItem('sq_autonext') === '1')
let autoT: number | null = null
function toggleAutoNext() {
  autoNext.value = !autoNext.value
  localStorage.setItem('sq_autonext', autoNext.value ? '1' : '0')
  if (!autoNext.value && autoT) { clearTimeout(autoT); autoT = null }
}
function scheduleAutoNext(good: boolean | null) {
  if (!autoNext.value || ctx.value?.examMode || ctx.value?.recite) return
  if (autoT) clearTimeout(autoT)
  const delay = good === false ? 2500 : good === true ? 1000 : 1500
  autoT = window.setTimeout(() => {
    autoT = null
    const c = ctx.value
    if (c && autoNext.value && c.idx < c.questions.length - 1) nav(1)
  }, delay)
}
function clearAutoNext() { if (autoT) { clearTimeout(autoT); autoT = null } }

const picked = computed<string>(() => {
  if (!q.value) return ''
  const p = ctx.value!.draft.picks[key(q.value)]
  return p?.picked ?? ''
})
const pickedArr = computed(() => picked.value.split('').filter(Boolean))
const norm = (s: string) => [...new Set((s || '').split('').filter(c => 'ABCDE'.includes(c)))].sort().join('')
const key = (qq: QuestionRow) => `${qq.bank_id}::${qq.qid}`
const graded = computed(() => { // 本地判分（仅展示用；权威判分在 Rust finish_session）
  if (!q.value || !q.value.answer || q.value.answer_conf !== 'high') return null
  return norm(picked.value) === norm(q.value.answer)
})
const noScore = computed(() => !q.value || !q.value.answer || q.value.answer_conf !== 'high')

function pick(L: string) {
  const c = ctx.value!; const qq = q.value!
  if (c.examMode) {
    const cur = c.draft.picks[key(qq)]?.picked ?? ''
    c.draft.picks[key(qq)] = { picked: isMulti.value ? toggle(cur, L) : L, t: c.draft.picks[key(qq)]?.t }
  } else if (c.recite) { showAns.value = !showAns.value; return }
  else {
    if (locked.value) return
    const cur = c.draft.picks[key(qq)]?.picked ?? ''
    const np = isMulti.value ? toggle(cur, L) : L
    c.draft.picks[key(qq)] = { picked: np }
    if (!isMulti.value) { locked.value = true; localGrade.value = graded.value; scheduleAutoNext(localGrade.value) }
  }
  scheduleSave()
}
function toggle(cur: string, L: string) { return cur.includes(L) ? cur.replace(L, '') : cur + L }
function submitMulti() {
  if (!pickedArr.value.length) { alert('请先选择选项'); return }
  locked.value = true; localGrade.value = graded.value
  scheduleAutoNext(localGrade.value)
  scheduleSave(true)
}
function nav(d: number) {
  const c = ctx.value!
  const ni = c.idx + d
  if (ni < 0 || ni >= c.questions.length) return
  clearAutoNext()
  c.idx = ni
  locked.value = false; localGrade.value = null; showAns.value = false
  c.draft.idx = ni
  loadNote(); scheduleSave(true)
}
function markQ() {
  const c = ctx.value!; const qq = q.value!
  c.draft.marks[key(qq)] = !c.draft.marks[key(qq)]
  scheduleSave(true)
}
async function togFav() {
  const qq = q.value!
  favOn.value = await api.favToggle(qq.bank_id, qq.qid)
}
async function loadNote() {
  if (!q.value) return
  favOn.value = (await api.favList()).some(x => x.bank_id === q.value!.bank_id && x.qid === q.value!.qid)
  noteText.value = (await api.noteGet(q.value.bank_id, q.value.qid)) ?? ''
}
async function saveNote() {
  if (!q.value) return
  await api.noteSet(q.value.bank_id, q.value.qid, noteText.value)
  noteOpen.value = false
}
let saveT: number | null = null
function doSave() {
  const c = ctx.value!
  if (!c) return
  c.draft.idx = c.idx
  if (c.examMode && c.remainingSec !== null) c.draft.remaining_sec = c.remainingSec
  c.draft.saved_at = Date.now()   // 墙钟锚点：恢复会话时据此续算考试剩余时间
  return api.saveDraft(c.session.session_id, c.draft).catch(() => {})
}
function scheduleSave(immediate = false) {
  if (saveT) clearTimeout(saveT)
  // 500ms 防抖：兼顾写放大与"答完立即关窗"的丢失窗口（此前 1.5s 内刷新必丢，实测复现）
  if (immediate) doSave()
  else saveT = window.setTimeout(() => { saveT = null; doSave() }, 500)
}
// 卸载/关窗前冲刷防抖窗口内的未决保存（否则丢失最后 1.5s 作答）
function flushPending() {
  if (!saveT) return
  clearTimeout(saveT); saveT = null
  scheduleSave(true)
}
function fmt(s: number) { return `${String(Math.floor(s / 60)).padStart(2, '0')}:${String(s % 60).padStart(2, '0')}` }
async function flushAndExit() {
  await doSaveNow()
  store.go(ctx.value!.examMode ? 'exam' : 'practice')
}
async function finish(auto = false) {
  const c = ctx.value!
  const unanswered = c.questions.filter(qq => !c.draft.picks[key(qq)]?.picked).length
  if (c.examMode && unanswered && !auto && !confirm(`还有 ${unanswered} 题未作答，确定交卷？`)) return
  await doSaveNow()
  finishing.value = true
  try {
    await api.finishSession(c.session.session_id)
    store.lastResultId = c.session.session_id
    store.sessionCtx = null
    store.go('result', { id: c.session.session_id })
  } catch {
    // 会话可能已被放弃（放弃/续做竞态）等：明确告知并退出做题页，避免卡死
    alert('交卷失败：该练习可能已被删除，请重新开始')
    store.sessionCtx = null
    store.go(c.examMode ? 'exam' : 'practice')
  } finally { finishing.value = false }
}
async function doSaveNow() {
  if (saveT) { clearTimeout(saveT); saveT = null }
  await doSave()
}
onMounted(() => {
  const c = ctx.value!
  if (!c) { store.go('home'); return }
  locked.value = false
  // 恢复练习已答题状态
  if (!c.examMode && !c.recite && c.draft.picks[key(c.questions[c.idx])] && !isMulti.value) locked.value = true
  loadNote()
  if (c.examMode && c.remainingSec !== null) {
    tickTimer = window.setInterval(() => {
      c.remainingSec!--
      now.value = Date.now()
      if (c.remainingSec! % 30 === 0) scheduleSave(true)
      if (c.remainingSec! <= 0) { clearInterval(tickTimer!); finish(true) }
    }, 1000)
  }
  saveTimer = window.setInterval(() => scheduleSave(true), 30000)
  window.addEventListener('pagehide', flushPending)
})
onUnmounted(() => {
  if (saveTimer) clearInterval(saveTimer)
  if (tickTimer) clearInterval(tickTimer)
  clearAutoNext()
  flushPending()
  window.removeEventListener('pagehide', flushPending)
})
function onKey(e: KeyboardEvent) {
  const c = ctx.value; if (!c) return
  if ('12345'.includes(e.key)) { const i = +e.key - 1; if (q.value?.options[i]) pick(String.fromCharCode(65 + i)) }
  if (e.key === 'ArrowRight') nav(1)
  if (e.key === 'ArrowLeft') nav(-1)
}
window.addEventListener('keydown', onKey)
onUnmounted(() => window.removeEventListener('keydown', onKey))
const progressPct = computed(() => {
  const c = ctx.value!
  return Math.round((answeredCount.value / c.questions.length) * 100)
})
// 已答计数统一口径：只数当前题单中已作答的题（题库去重删题后旧 picks 键不计，避免 7/6）
const answeredCount = computed(() => {
  const c = ctx.value
  if (!c) return 0
  return c.questions.filter(qq => c.draft.picks[key(qq)]?.picked).length
})
</script>

<template>
  <div v-if="ctx && q">
    <!-- 顶栏 -->
    <div class="qtop">
      <button class="btn ghost" @click="flushAndExit">← 退出</button>
      <div class="tt">
        <b>{{ ctx.session.title }}</b>
        <span class="pos">{{ ctx.idx + 1 }} / {{ ctx.questions.length }} · 已答 {{ answeredCount }}</span>
        <div class="pbar"><i :style="{ width: progressPct + '%' }"></i></div>
      </div>
      <div v-if="ctx.examMode" class="timer" :class="{ urgent: (ctx.remainingSec ?? 1) < 300 }">⏱ {{ fmt(ctx.remainingSec ?? 0) }}</div>
      <button v-if="!ctx.examMode && !ctx.recite" class="btn ghost autonext" :class="{ on: autoNext }"
        :title="autoNext ? '自动下一题已开：答对1秒/答错2.5秒后前进' : '开启自动下一题（答完自动前进）'" @click="toggleAutoNext">⚡自动</button>
      <button v-if="ctx.examMode" class="btn" @click="sheet = true">答题卡</button>
    </div>

    <!-- 题目卡 -->
    <div class="card qcard">
      <div class="qmeta">
        <span class="tag" :class="{ multi: isMulti }">{{ isMulti ? '多选题' : '单选题' }}</span>
        <span class="tag">{{ q.topics.join('/') }}</span>
        <span v-if="noScore" class="tag warn">答案整理中 · 不计分</span>
        <span v-if="ctx.draft.marks[key(q)]" class="tag warn">🚩 已标记</span>
        <span style="flex:1"></span>
        <button class="fav" :class="{ on: favOn }" @click="togFav" title="收藏">☆</button>
        <button class="fav" @click="noteOpen = true" title="笔记">📝</button>
      </div>
      <div class="stem">{{ q.stem }}</div>
      <img v-if="q.img_path" class="qimg" :src="assetUrl(q.bank_id, q.img_path)" alt="题目图" />

      <div class="opts">
        <div v-for="(o, i) in q.options" :key="i" class="opt"
          :class="{ sel: pickedArr.includes(String.fromCharCode(65 + i)),
                    right: (locked || showAns) && !noScore && norm(q.answer).includes(String.fromCharCode(65 + i)),
                    wrong: (locked || showAns) && !noScore && pickedArr.includes(String.fromCharCode(65 + i)) && !norm(q.answer).includes(String.fromCharCode(65 + i)),
                    lock: locked }"
          @click="pick(String.fromCharCode(65 + i))">
          <b>{{ String.fromCharCode(65 + i) }}</b>
          <span>{{ o.replace(/^[A-H][、.．,，]\s*/, '') }}</span>
        </div>
      </div>

      <!-- 反馈 -->
      <div v-if="locked && !noScore" class="fb" :class="localGrade ? 'ok' : 'bad'">
        <b>{{ localGrade ? '✓ 正确' : '✗ 错误' }}{{ isMulti ? ' · 正确答案 ' + norm(q.answer) : '' }}</b>
        <div v-if="q.explain" class="ex">{{ q.explain }}</div>
        <div v-if="q.source" class="fsrc">出处：{{ q.source }}</div>
      </div>
      <div v-else-if="locked && noScore" class="fb bad">该题参考答案暂未收录，不计分</div>
      <div v-if="ctx.recite && showAns && !noScore" class="fb ok">
        <b>答案：{{ q.answer }}</b>
        <div v-if="q.explain" class="ex">{{ q.explain }}</div>
      </div>
      <button v-if="ctx.recite && !showAns" class="btn pri" style="margin-top:12px" @click="showAns = true">显示答案</button>

      <div class="navrow">
        <button class="btn" :disabled="ctx.idx === 0" @click="nav(-1)">← 上一题</button>
        <button class="btn ghost" @click="markQ">{{ ctx.draft.marks[key(q)] ? '取消标记' : '🚩 标记' }}</button>
        <button v-if="isMulti && !locked && !ctx.examMode && !ctx.recite" class="btn pri" @click="submitMulti">提交答案</button>
        <button v-if="ctx.examMode || ctx.idx < ctx.questions.length - 1 || ctx.recite" class="btn pri" @click="nav(1)">下一题 →</button>
        <button v-else class="btn pri" :disabled="finishing" @click="finish()">{{ finishing ? '判分中…' : (ctx.examMode ? '交卷' : '完成练习') }}</button>
      </div>
      <div v-if="ctx.examMode && ctx.idx === ctx.questions.length - 1" class="navrow" style="justify-content:center">
        <button class="btn pri" :disabled="finishing" @click="finish()">交卷并查看成绩</button>
      </div>
    </div>

    <!-- 答题卡 -->
    <div v-if="sheet" class="mask" @click.self="sheet = false">
      <div class="sheetbox">
        <h3>答题卡 <span class="hint">已答 {{ answeredCount }}/{{ ctx.questions.length }}</span></h3>
        <div class="sgrid">
          <button v-for="(qq, i) in ctx.questions" :key="i" class="cell"
            :class="{ did: ctx.draft.picks[key(qq)]?.picked, mark: ctx.draft.marks[key(qq)], cur: i === ctx.idx }"
            @click="ctx.idx = i; locked = false; sheet = false; clearAutoNext(); loadNote()">{{ i + 1 }}</button>
        </div>
        <div class="btnrow2">
          <button class="btn pri" @click="finish()">交卷</button>
          <button class="btn" @click="sheet = false">继续答题</button>
        </div>
      </div>
    </div>

    <!-- 笔记 -->
    <div v-if="noteOpen" class="mask" @click.self="noteOpen = false">
      <div class="sheetbox" style="max-width:520px">
        <h3>📝 题目笔记</h3>
        <textarea v-model="noteText" rows="6" placeholder="记口诀、疑问、易混点…" style="width:100%;padding:10px;border-radius:9px;border:1.5px solid var(--line);background:var(--card);color:var(--ink);font-size:.92rem"></textarea>
        <div class="btnrow2"><button class="btn pri" @click="saveNote">保存</button><button class="btn" @click="noteOpen = false">取消</button></div>
      </div>
    </div>
  </div>
  <div v-else class="empty">没有进行中的会话 <button class="btn pri" @click="store.go('home')">回首页</button></div>
</template>

<style scoped>
.qtop { display: flex; align-items: center; gap: 12px; margin-bottom: 14px; }
.qtop .tt { flex: 1; min-width: 0; }
.qtop b { font-size: .95rem; }
.pos { font-size: .78rem; color: var(--sub); margin-left: 8px; }
.pbar { height: 5px; background: var(--chip); border-radius: 3px; overflow: hidden; margin-top: 5px; }
.pbar i { display: block; height: 100%; background: var(--brand); transition: width .2s; }
.timer { font-size: 1.05rem; font-weight: 700; font-variant-numeric: tabular-nums; }
.timer.urgent { color: var(--bad); animation: blink 1s infinite; }
@keyframes blink { 50% { opacity: .5; } }
.qmeta { display: flex; align-items: center; gap: 7px; flex-wrap: wrap; margin-bottom: 9px; }
.fav { background: none; border: none; font-size: 1.1rem; cursor: pointer; color: var(--sub); }
.fav.on { color: #f0a020; }
.stem { font-size: 1.02rem; font-weight: 600; margin-bottom: 12px; white-space: pre-wrap; }
.qimg { max-width: 100%; border: 1px solid var(--line); border-radius: 10px; margin-bottom: 12px; }
.autonext { padding: 6px 12px; font-size: .82rem; white-space: nowrap; }
.autonext.on { background: var(--warn-bg); color: var(--warn); border-color: var(--warn); }
.opts { display: flex; flex-direction: column; gap: 9px; }
.opt { display: flex; gap: 10px; border: 1.5px solid var(--line); border-radius: 11px; padding: 11px 13px; cursor: pointer; font-size: .95rem; }
.opt:hover { border-color: var(--brand); }
.opt b { color: var(--brand); }
.opt.sel { border-color: var(--brand); background: var(--chip); }
.opt.right { border-color: var(--ok); background: var(--ok-bg); }
.opt.wrong { border-color: var(--bad); background: var(--bad-bg); }
.opt.lock { cursor: default; }
.fb { border-radius: 11px; padding: 12px 14px; margin-top: 12px; font-size: .9rem; }
.fb.ok { background: var(--ok-bg); color: var(--ok); }
.fb.bad { background: var(--bad-bg); color: var(--bad); }
.fb .ex { color: var(--ink); margin-top: 5px; }
.fb .fsrc { font-size: .76rem; color: var(--sub); margin-top: 5px; }
.navrow { display: flex; gap: 9px; margin-top: 16px; }
.navrow .btn { flex: 1; text-align: center; }
.mask { position: fixed; inset: 0; background: rgba(10,12,18,.55); display: flex; align-items: center; justify-content: center; z-index: 50; padding: 16px; }
.sheetbox { background: var(--card); border-radius: 16px; padding: 20px; max-width: 560px; width: 100%; max-height: 82vh; overflow: auto; }
.sgrid { display: grid; grid-template-columns: repeat(auto-fill, minmax(44px, 1fr)); gap: 8px; margin: 12px 0; }
.cell { border: 1.5px solid var(--line); border-radius: 9px; text-align: center; padding: 7px 0; cursor: pointer; background: var(--card); color: var(--ink); font-size: .9rem; }
.cell.did { background: var(--brand); color: var(--brand-ink); border-color: var(--brand); }
.cell.mark::after { content: '🚩'; font-size: .6rem; }
.cell.cur { outline: 2px solid #f0a020; }
.btnrow2 { display: flex; gap: 9px; margin-top: 10px; }
.btnrow2 .btn { flex: 1; }
</style>
