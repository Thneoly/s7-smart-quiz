<script setup lang="ts">
import { ref, onMounted } from 'vue'
import { api, type PaperInfo, type SessionInfo } from '../api'
import { store } from '../store'
import { startPaper, resumeSession } from '../session-start'

const papers = ref<PaperInfo[]>([])
const unfinished = ref<SessionInfo[]>([])
const busy = ref('')

onMounted(async () => {
  [papers.value, unfinished.value] = await Promise.all([api.papers(), api.unfinished()])
})
async function go(p: PaperInfo) {
  busy.value = p.name
  try { await startPaper(p.paper_id, p.name, p.title) } finally { busy.value = '' }
}
const fmtMin = (ms: number | null) => ms ? Math.round(ms / 60000) + ' 分钟' : '—'
</script>

<template>
  <h2 class="pt">考试中心</h2>
  <div v-if="unfinished.length" class="card">
    <h3>⏸ 未完成的考试</h3>
    <div v-for="s in unfinished.filter(s => s.mode === 'exam')" :key="s.session_id" class="rowitem" style="cursor:pointer" @click="resumeSession(s)">
      <div class="qq">{{ s.title }}</div>
      <div class="meta">已答 {{ Object.keys((s.draft as any)?.picks ?? {}).length }}/{{ s.total_qty }} · 剩余 {{ fmtMin(((s.draft as any)?.remaining_sec ?? s.time_limit_sec ?? 0) * 1000) }} · 点击继续</div>
    </div>
  </div>
  <div class="card">
    <h3>📝 真题模拟卷 <span class="hint">考试模式 · 计时 90 分钟 · 交卷判分（多选全对才得分）</span></h3>
    <div v-for="p in papers" :key="p.paper_id" class="rowitem">
      <div class="qq">{{ p.name }} <span class="hint">{{ p.title }}</span></div>
      <div class="meta" style="margin-bottom:8px">{{ p.count }} 题</div>
      <div style="display:flex;gap:8px">
        <button class="btn pri" :disabled="!!busy" @click="go(p)">{{ busy === p.name ? '准备中…' : '开始考试' }}</button>
        <button class="btn ghost" @click="store.go('print', { paperId: p.paper_id })">🖨 打印试卷</button>
      </div>
    </div>
    <div v-if="!papers.length" class="empty">无试卷</div>
    <div style="margin-top:12px">
      <button class="btn" @click="store.go('compose')">🧪 蓝图组卷（自定义题型/主题/难度）</button>
    </div>
  </div>
  <div class="card">
    <h3>ℹ️ 考试说明</h3>
    <p class="hint">· 中途退出自动保存草稿，可从「未完成的考试」续考（倒计时也保存）<br>
    · 低置信度答案（AI整理中）的题不计入分数<br>
    · 交卷后可逐题回顾解析与出处</p>
  </div>
</template>
