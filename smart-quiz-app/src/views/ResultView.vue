<script setup lang="ts">
import { ref, computed, onMounted } from 'vue'
import { api, assetUrl, saveDialog, hasTauri, type SessionDetail } from '../api'
import { store } from '../store'
import { startWithQuestions } from '../session-start'

const detail = ref<SessionDetail | null>(null)
const expandAll = ref(false)
const exporting = ref(false)
const sid = computed(() => store.params.id ?? store.lastResultId)

onMounted(async () => { if (sid.value) detail.value = await api.sessionDetail(sid.value) })
const wrongs = computed(() => (detail.value?.records ?? []).filter(r => r.is_correct === false))
// 逐题回顾（答错在前）：携带原题号（会话题序），排序后仍显示做题时的编号
const review = computed(() => {
  const withIdx = (detail.value?.records ?? []).map((r, orig) => ({ r, orig }))
  return [...withIdx.filter(x => x.r.is_correct === false), ...withIdx.filter(x => x.r.is_correct !== false)]
})
const noscored = computed(() => (detail.value?.records ?? []).filter(r => r.is_correct === null))
async function redoWrong() {
  const qs = wrongs.value.map(r => r.question!).filter(Boolean)
  if (!qs.length) return
  await startWithQuestions('wrong', '错题重做', qs)
}
async function exportExcel() {
  if (!sid.value) return
  exporting.value = true
  try {
    const path = await saveDialog({
      defaultPath: `成绩单-${detail.value?.session.title ?? sid.value}.xlsx`,
      filters: [{ name: 'Excel', extensions: ['xlsx'] }],
    })
    if (!path) return
    const out = await api.exportExcel(sid.value, path)
    alert(hasTauri ? `已导出：${out}` : '已导出（mock）')
  } catch (e: any) { alert('导出失败：' + e) } finally { exporting.value = false }
}
function printPdf() { window.print() }
</script>

<template>
  <div v-if="detail">
    <div class="card" style="text-align:center">
      <div style="font-size:3rem;font-weight:800;color:var(--brand)">{{ detail.session.score ?? '—' }}<span style="font-size:1.2rem">分</span></div>
      <div class="hint">{{ detail.session.title }}</div>
      <div class="statrow" style="margin-top:14px;text-align:center">
        <div class="stat"><b style="color:var(--ok)">{{ detail.session.correct_qty }}</b><span>答对</span></div>
        <div class="stat"><b style="color:var(--bad)">{{ wrongs.length }}</b><span>答错</span></div>
        <div class="stat"><b>{{ detail.session.scored_qty }}</b><span>计分题数</span></div>
        <div class="stat"><b>{{ noscored.length }}</b><span>不计分题</span></div>
      </div>
      <div style="display:flex;gap:9px;justify-content:center;flex-wrap:wrap">
        <button v-if="wrongs.length" class="btn pri" @click="redoWrong">只重做错题（{{ wrongs.length }}）</button>
        <button class="btn" @click="store.go('home')">返回首页</button>
        <button class="btn ghost" @click="expandAll = !expandAll">{{ expandAll ? '收起' : '展开全部解析' }}</button>
        <button class="btn" :disabled="exporting" @click="exportExcel">{{ exporting ? '导出中…' : '📄 导出 Excel' }}</button>
        <button class="btn ghost" @click="printPdf">🖨 打印 / 存 PDF</button>
      </div>
    </div>
    <div v-if="noscored.length" class="card"><span class="tag warn"> {{ noscored.length }} 题答案整理中（低置信度），未计入分数</span></div>
    <div class="card">
      <h3>逐题回顾（答错在前）</h3>
      <div v-for="x in review" :key="x.orig" class="qreview" :class="{ bad: x.r.is_correct === false, ok: x.r.is_correct === true }">
        <div class="rvhead">
          <span>{{ x.orig + 1 }}.</span>
          <span class="tag" :class="{ multi: x.r.question?.qtype === 'multi' }">{{ x.r.question?.qtype === 'multi' ? '多选' : '单选' }}</span>
          <span v-if="x.r.is_correct === true" style="color:var(--ok)">✓</span>
          <span v-else-if="x.r.is_correct === false" style="color:var(--bad)">✗ 你答 {{ x.r.picked || '未答' }} · 正确 {{ x.r.question?.answer }}</span>
          <span v-else class="tag warn">不计分</span>
        </div>
        <div class="rvstem">{{ x.r.question?.stem }}</div>
        <img v-if="x.r.question?.img_path" :src="assetUrl(x.r.question.bank_id, x.r.question.img_path!)" style="max-width:100%;border-radius:9px;border:1px solid var(--line);margin:6px 0" />
        <div v-if="expandAll || x.r.is_correct === false" class="rvdetail">
          <div class="opts-mini">
            <div v-for="(o, j) in x.r.question?.options ?? []" :key="j"
              :class="{ r: (x.r.question?.answer ?? '').includes(String.fromCharCode(65 + j)), p: (x.r.picked ?? '').includes(String.fromCharCode(65 + j)) }">
              <b>{{ String.fromCharCode(65 + j) }}</b>{{ o.replace(/^[A-H][、.．,，]\s*/, '') }}
            </div>
          </div>
          <div v-if="x.r.question?.explain" class="hint">解析：{{ x.r.question.explain }}</div>
          <div v-if="x.r.question?.source" class="src">出处：{{ x.r.question.source }}</div>
        </div>
      </div>
    </div>
  </div>
  <div v-else class="empty">加载中…</div>
</template>

<style scoped>
.qreview { border: 1px solid var(--line); border-radius: 11px; padding: 12px; margin-bottom: 10px; }
.qreview.bad { border-color: var(--bad); }
.rvhead { display: flex; gap: 8px; align-items: center; font-size: .82rem; margin-bottom: 5px; }
.rvstem { font-size: .92rem; font-weight: 600; }
.rvdetail { margin-top: 8px; font-size: .86rem; }
.opts-mini { display: flex; flex-direction: column; gap: 4px; margin-bottom: 6px; }
.opts-mini > div { padding: 4px 9px; border-radius: 7px; border: 1px solid transparent; }
.opts-mini .r { border-color: var(--ok); background: var(--ok-bg); }
.opts-mini .p:not(.r) { border-color: var(--bad); background: var(--bad-bg); }
</style>
