<script setup lang="ts">
import { ref, onMounted } from 'vue'
import { api, type Overview, type ComposeReport } from '../api'
import { store } from '../store'
import { startWithQuestions } from '../session-start'

const ov = ref<Overview | null>(null)
const name = ref('智能组卷')
const timeLimit = ref(90)
const allowFallback = ref(true)
const sections = ref([
  { type: 'single', qty: 40, topics: [] as number[], dmin: 1, dmax: 5, useDiff: false },
  { type: 'multi', qty: 10, topics: [] as number[], dmin: 1, dmax: 5, useDiff: false },
])
const report = ref<ComposeReport | null>(null)
const busy = ref('')
const err = ref('')

onMounted(async () => { ov.value = await api.overview() })

function addSection() { sections.value.push({ type: 'single', qty: 10, topics: [], dmin: 1, dmax: 5, useDiff: false }) }
function delSection(i: number) { sections.value.splice(i, 1) }

// 预设卷型：一键填充蓝图（新手摸底=跨章节快速定位薄弱点；全真冲刺=按认证卷面配比）
function applyPreset(kind: 'starter' | 'full') {
  report.value = null
  if (kind === 'starter') {
    name.value = '新手摸底卷'
    timeLimit.value = 25
    allowFallback.value = true
    sections.value = [
      { type: 'single', qty: 25, topics: [], dmin: 1, dmax: 5, useDiff: false },
      { type: 'multi', qty: 5, topics: [], dmin: 1, dmax: 5, useDiff: false },
    ]
  } else {
    name.value = '全真冲刺卷'
    timeLimit.value = 90
    allowFallback.value = false
    sections.value = [
      { type: 'single', qty: 40, topics: [], dmin: 1, dmax: 5, useDiff: false },
      { type: 'multi', qty: 10, topics: [], dmin: 1, dmax: 5, useDiff: false },
    ]
  }
}

async function compose() {
  busy.value = 'compose'; err.value = ''
  try {
    const bp = {
      name: name.value, time_limit_min: timeLimit.value, allow_fallback: allowFallback.value,
      sections: sections.value.map(s => ({
        type: s.type, qty: s.qty, from_topics: s.topics,
        difficulty: s.useDiff ? ([s.dmin, s.dmax] as [number, number]) : null,
      })),
    }
    report.value = await api.compose(bp)
  } catch (e: any) { err.value = String(e) } finally { busy.value = '' }
}
async function startExam() {
  if (!report.value) return
  busy.value = 'start'
  try {
    const qs = await api.questionsByIds(report.value.qids)
    await startWithQuestions('exam', `${name.value}（${report.value.total}题）`, qs, { exam: true, timeLimitSec: timeLimit.value * 60 })
  } finally { busy.value = '' }
}
const typeLabel: Record<string, string> = { single: '单选', multi: '多选' }
</script>

<template>
  <h2 class="pt">蓝图组卷 <span class="hint">按题型/主题/难度抽题 · 候选不足自动降级</span></h2>
  <div class="card">
    <div style="display:flex;gap:8px;flex-wrap:wrap;margin-bottom:12px">
      <span class="hint" style="align-self:center">快速开始：</span>
      <button class="chip" @click="applyPreset('starter')">🧭 新手摸底卷（25单+5多 · 25分钟）</button>
      <button class="chip" @click="applyPreset('full')">🏁 全真冲刺卷（40单+10多 · 90分钟）</button>
    </div>
    <div style="display:flex;gap:10px;flex-wrap:wrap;align-items:center;margin-bottom:12px">
      <label>卷名 <input v-model="name" style="padding:6px 10px;border:1.5px solid var(--line);border-radius:8px;background:var(--card);color:var(--ink)" /></label>
      <label>限时 <input v-model.number="timeLimit" type="number" min="10" style="width:70px;padding:6px 10px;border:1.5px solid var(--line);border-radius:8px;background:var(--card);color:var(--ink)" /> 分钟</label>
      <label style="display:flex;align-items:center;gap:5px;font-size:.86rem">
        <input v-model="allowFallback" type="checkbox" /> 候选不足时自动降级（否则报错）
      </label>
    </div>
    <div v-for="(s, i) in sections" :key="i" class="secbox">
      <b>Section {{ i + 1 }}</b>
      <select v-model="s.type"><option value="single">单选</option><option value="multi">多选</option></select>
      <label>数量 <input v-model.number="s.qty" type="number" min="1" style="width:70px" /></label>
      <label style="display:flex;align-items:center;gap:4px"><input v-model="s.useDiff" type="checkbox" />难度</label>
      <template v-if="s.useDiff">
        <input v-model.number="s.dmin" type="number" min="1" max="5" style="width:55px" /> ~
        <input v-model.number="s.dmax" type="number" min="1" max="5" style="width:55px" />
      </template>
      <select v-model="s.topics" multiple size="1" style="min-width:220px;max-height:60px">
        <option v-for="t in ov?.topics ?? []" :key="t.topic_id" :value="t.topic_id">{{ t.name }}</option>
      </select>
      <button v-if="sections.length > 1" class="btn danger" @click="delSection(i)">删</button>
    </div>
    <div style="display:flex;gap:9px;margin-top:12px">
      <button class="btn" @click="addSection">+ 添加 Section</button>
      <button class="btn pri" :disabled="busy === 'compose'" @click="compose()">{{ busy === 'compose' ? '组卷中…' : '组卷预览' }}</button>
      <button class="btn ghost" @click="store.go('exam')">返回考试中心</button>
    </div>
    <div v-if="err" class="tag warn" style="display:inline-block;margin-top:10px;padding:6px 12px">{{ err }}</div>
  </div>

  <div v-if="report" class="card">
    <h3>组卷结果 <span class="hint">共 {{ report.total }} 题</span></h3>
    <div v-for="(r, i) in report.sections" :key="i" class="rowitem">
      <div class="meta">
        <span class="tag">{{ typeLabel[r.qtype] }}</span>
        <span>请求 {{ r.requested }} 题 → 实得 <b>{{ r.actual }}</b> 题</span>
        <span v-if="r.fallback" class="tag warn">降级：{{ r.fallback }}</span>
        <span v-else class="tag" style="background:var(--ok-bg);color:var(--ok)">满足</span>
      </div>
    </div>
    <button class="btn pri" :disabled="busy === 'start'" @click="startExam()">{{ busy === 'start' ? '准备中…' : '开始考试' }}</button>
  </div>
</template>

<style scoped>
.secbox { display: flex; gap: 10px; align-items: center; flex-wrap: wrap; border: 1px dashed var(--line); border-radius: 10px; padding: 9px 12px; margin-bottom: 8px; font-size: .88rem; }
.secbox input, .secbox select { padding: 5px 8px; border: 1.5px solid var(--line); border-radius: 7px; background: var(--card); color: var(--ink); }
</style>
