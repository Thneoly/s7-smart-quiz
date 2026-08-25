<script setup lang="ts">
import { ref, computed, onMounted } from 'vue'
import { api, type Overview, type WrongRow } from '../api'
import { startWithQuestions } from '../session-start'

const rows = ref<WrongRow[]>([])
const ov = ref<Overview | null>(null)
const loading = ref(true)

async function load() { loading.value = true; try { rows.value = await api.wrongList() } finally { loading.value = false } }
onMounted(async () => { load(); ov.value = await api.overview() })
async function practice() {
  if (!rows.value.length) return
  await startWithQuestions('wrong', `错题重练 ${rows.value.length} 题`, rows.value.map(w => w.question))
}
// 薄弱主题聚合：错题集中的主题，整主题再练（含未错过的同主题题）
const topicGroups = computed(() => {
  const g: Record<string, number> = {}
  for (const w of rows.value) { const t = w.question.topics[0]; if (t) g[t] = (g[t] ?? 0) + 1 }
  return Object.entries(g).sort((a, b) => b[1] - a[1])
})
async function practiceRelated(topic: string) {
  const tid = ov.value?.topics.find(t => t.name === topic)?.topic_id
  const qs = await api.questions({ topic_id: tid, status: 'active', limit: 500 })
  if (!qs.length) { alert(`「${topic}」主题暂无可练题目`); return }
  await startWithQuestions('practice', `相关巩固 · ${topic}（${qs.length}题）`, qs)
}
async function clear(bankId: string, qid: string) {
  if (!confirm('确定从错题本移除？')) return
  await api.wrongClear(bankId, qid)
  await load()
}
</script>

<template>
  <h2 class="pt">错题本 <span class="hint">连续答对 2 次自动消灭（SM-2）</span></h2>
  <div class="card" v-if="rows.length">
    <button class="btn pri" @click="practice()">开始错题重练（{{ rows.length }} 题）</button>
  </div>
  <div class="card" v-if="topicGroups.length">
    <h3>🎯 薄弱主题巩固 <span class="hint">错题集中区——整主题再练一遍，不止错题</span></h3>
    <div style="display:flex;gap:8px;flex-wrap:wrap">
      <button v-for="[t, n] in topicGroups" :key="t" class="chip" @click="practiceRelated(t)">{{ t }}（错{{ n }}）→ 练全部</button>
    </div>
  </div>
  <div v-if="loading" class="empty">加载中…</div>
  <div v-else-if="!rows.length" class="card empty">🎉 没有活跃错题！</div>
  <div v-for="w in rows" :key="w.bank_id + w.qid" class="rowitem">
    <div class="qq">{{ w.question.stem }}</div>
    <div class="meta">
      <span class="tag" :class="{ multi: w.question.qtype === 'multi' }">{{ w.question.qtype === 'multi' ? '多选' : '单选' }}</span>
      <span class="tag">{{ w.question.topics.join('/') }}</span>
      <span class="tag warn">错 {{ w.wrong_count }} 次</span>
      <span class="tag">消灭进度 {{ w.repetitions }}/2</span>
      <span v-if="w.due_date" class="src">下次复习：{{ new Date(w.due_date).toLocaleString() }}</span>
      <span style="flex:1"></span>
      <button class="btn danger" @click="clear(w.bank_id, w.qid)">移除</button>
    </div>
    <details style="margin-top:8px">
      <summary class="hint" style="cursor:pointer">查看答案解析</summary>
      <div style="margin-top:6px;font-size:.88rem">
        <div>答案：<b style="color:var(--ok)">{{ w.question.answer }}</b></div>
        <div v-if="w.question.explain" class="hint">解析：{{ w.question.explain }}</div>
        <div v-if="w.question.source" class="src">出处：{{ w.question.source }}</div>
      </div>
    </details>
  </div>
</template>
