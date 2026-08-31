<script setup lang="ts">
import { ref, computed, onMounted } from 'vue'
import { api, MODE_NAME, type Overview, type QuestionRow, type SessionInfo } from '../api'
import { startWithQuestions, resumeSession } from '../session-start'

const ov = ref<Overview | null>(null)
const busy = ref('')
const randN = ref(30)
// 只展示学科主题（真题卷 A~E 属考试 tab，不进章节练习/背诵；无题目的分组父主题也隐藏）
const subjects = computed(() => (ov.value?.topics ?? []).filter(t => !t.parent_id && t.total > 0))
const subjectNames = computed(() => new Set(subjects.value.map(t => t.name)))
const onlySubjects = (qs: QuestionRow[]) => qs.filter(q => q.topics.some(t => subjectNames.value.has(t)))
// 续做：进入练习页即提示未完成会话（非阻塞——新练习照常开新会话，从第 1 题开始）
const ongoing = ref<SessionInfo[]>([])
const ansCount = (s: SessionInfo) => Object.keys(s.draft?.picks ?? {}).length
async function discardOngoing(s: SessionInfo) {
  if (!confirm(`放弃「${s.title}」？已答的 ${ansCount(s)} 题进度将被删除`)) return
  try { await api.discardSession(s.session_id) } catch { alert('删除失败，请重试'); return }
  ongoing.value = ongoing.value.filter(x => x.session_id !== s.session_id)
}

async function topicPractice(name?: string) {
  busy.value = name ?? '全部'
  try {
    const qs = await api.questions({ topic_id: ov.value?.topics.find(t => t.name === name)?.topic_id, status: 'active', limit: 500 })
    await startWithQuestions('practice', name ? `章节练习 · ${name}` : '全部顺序练习', shuffleStable(name ? qs : onlySubjects(qs)))
  } finally { busy.value = '' }
}
async function randomPractice() {
  busy.value = 'random'
  try {
    const qs = await api.questions({ status: 'active', limit: 500 })
    const picked = [...qs].sort(() => Math.random() - .5).slice(0, Math.min(randN.value, qs.length))
    await startWithQuestions('random', `随机练习 ${picked.length} 题`, picked)
  } finally { busy.value = '' }
}
async function recite(name?: string) {
  busy.value = 'recite'
  try {
    const qs = await api.questions({ topic_id: ov.value?.topics.find(t => t.name === name)?.topic_id, status: 'active', limit: 500 })
    await startWithQuestions('recite', `背诵模式${name ? ' · ' + name : ''}`, qs, { recite: true })
  } finally { busy.value = '' }
}
async function reviewDue() {
  busy.value = 'review'
  try {
    const qs = await api.dueReview(30)
    if (!qs.length) { alert('当前没有到期复习的题目（SM-2 间隔未到）'); return }
    await startWithQuestions('review', `间隔复习 ${qs.length} 题`, qs)
  } finally { busy.value = '' }
}
async function wrongPractice() {
  busy.value = 'wrong'
  try {
    const ws = await api.wrongList()
    if (!ws.length) { alert('错题本是空的 🎉'); return }
    await startWithQuestions('wrong', `错题重练 ${ws.length} 题`, ws.map(w => w.question))
  } finally { busy.value = '' }
}
async function favPractice() {
  busy.value = 'fav'
  try {
    const qs = await api.favList()
    if (!qs.length) { alert('收藏夹是空的，做题时点 ☆ 收藏'); return }
    await startWithQuestions('fav', `收藏练习 ${qs.length} 题`, qs)
  } finally { busy.value = '' }
}
function shuffleStable(qs: QuestionRow[]) { return qs } // 章节练习保持题序

onMounted(async () => {
  const [o, u] = await Promise.all([api.overview(), api.unfinished().catch(() => [])])
  ov.value = o
  ongoing.value = u
})
</script>

<template>
  <h2 class="pt">练习中心</h2>

  <!-- 续做横幅：上次没做完？接着做，不用从头开始 -->
  <div v-if="ongoing.length" class="card ongoing">
    <h3>⏸ 继续上次的练习 <span class="hint">进度已自动保存 · 也可忽略，直接开始新练习</span></h3>
    <div v-for="s in ongoing" :key="s.session_id" class="orow">
      <span class="tag">{{ MODE_NAME[s.mode] ?? s.mode }}</span>
      <b class="otitle">{{ s.title }}</b>
      <span class="hint">已答 {{ ansCount(s) }}/{{ s.total_qty }}</span>
      <span style="flex:1"></span>
      <button class="btn pri" @click="resumeSession(s)">▶ 继续</button>
      <button class="btn ghost" @click="discardOngoing(s)">放弃</button>
    </div>
  </div>

  <div class="card">
    <h3>📖 章节练习 <span class="hint">即时判分 · 答错自动进错题本与复习计划</span></h3>
    <div class="chips" style="display:flex;flex-wrap:wrap;gap:8px">
      <button class="chip" :disabled="!!busy" @click="topicPractice()">全部主题</button>
      <button v-for="t in subjects" :key="t.topic_id" class="chip" :disabled="!!busy"
        @click="topicPractice(t.name)">{{ t.name }}<small>{{ t.active }}</small></button>
    </div>
  </div>
  <div class="card">
    <h3>🎯 更多练习方式</h3>
    <div style="display:flex;gap:9px;flex-wrap:wrap;align-items:center">
      <select v-model.number="randN"><option :value="10">10 题</option><option :value="30">30 题</option><option :value="50">50 题</option></select>
      <button class="btn pri" :disabled="!!busy" @click="randomPractice()">🎲 随机练习</button>
      <button class="btn" :disabled="!!busy" @click="reviewDue()">⏰ 间隔复习（SM-2 到期题）</button>
      <button class="btn" :disabled="!!busy" @click="wrongPractice()">❌ 错题重练（连续答对2次消灭）</button>
      <button class="btn" :disabled="!!busy" @click="favPractice()">⭐ 收藏练习</button>
      <button class="btn ghost" :disabled="!!busy" @click="recite()">👀 背诵模式</button>
    </div>
    <p class="hint" style="margin-top:10px">做题页快捷键：数字键 1~5 选择答案，← → 翻页；多选题选完点「提交答案」（全对才得分）。</p>
  </div>
  <div class="card">
    <h3>👀 背诵模式（按主题）</h3>
    <div style="display:flex;flex-wrap:wrap;gap:8px">
      <button class="chip" @click="recite()">全部</button>
      <button v-for="t in subjects" :key="t.topic_id" class="chip" @click="recite(t.name)">{{ t.name }}</button>
    </div>
  </div>
  <div v-if="busy" class="empty">正在准备题目…</div>
</template>

<style scoped>
.ongoing { border-color: var(--warn); }
.orow { display: flex; align-items: center; gap: 10px; padding: 8px 0; border-bottom: 1px dashed var(--line); flex-wrap: wrap; }
.orow:last-child { border-bottom: none; padding-bottom: 2px; }
.otitle { font-size: .9rem; min-width: 0; overflow: hidden; text-overflow: ellipsis; white-space: nowrap; max-width: 340px; }
</style>
