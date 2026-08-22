<script setup lang="ts">
import { ref, onMounted } from 'vue'
import { api, assetUrl, type Overview, type QuestionRow } from '../api'

const ov = ref<Overview | null>(null)
const questions = ref<QuestionRow[]>([])
const loading = ref(false)
const sel = ref<QuestionRow | null>(null)
const fTopic = ref<number | undefined>()
const fType = ref<string | undefined>()
const fStatus = ref<string | undefined>()
const fSearch = ref('')
const page = ref(0)
const PAGE = 20

async function load(reset = false) {
  if (reset) page.value = 0
  loading.value = true
  try {
    questions.value = await api.questions({
      topic_id: fTopic.value, qtype: fType.value || undefined, status: fStatus.value || undefined,
      search: fSearch.value.trim() || undefined, limit: PAGE, offset: page.value * PAGE,
    })
  } finally { loading.value = false }
}
const typeLabel: Record<string, string> = { single: '单选', multi: '多选', judge: '判断', fill: '填空' }
onMounted(async () => { ov.value = await api.overview(); await load() })
</script>

<template>
  <h2 class="pt">题库浏览 <span class="hint">{{ ov?.banks.map(b => `${b.name}（${b.total}题/启用${b.active}/待复审${b.pending}）`).join(' · ') }}</span></h2>
  <div class="card">
    <div style="display:flex;gap:8px;flex-wrap:wrap">
      <select v-model="fTopic" @change="load(true)">
        <option :value="undefined">全部主题</option>
        <option v-for="t in ov?.topics ?? []" :key="t.topic_id" :value="t.topic_id">{{ t.name }}（{{ t.active }}）</option>
      </select>
      <select v-model="fType" @change="load(true)">
        <option :value="undefined">全部题型</option><option value="single">单选</option><option value="multi">多选</option>
      </select>
      <select v-model="fStatus" @change="load(true)">
        <option :value="undefined">全部状态</option><option value="active">仅启用</option><option value="pending_review">仅待复审</option>
      </select>
      <input v-model="fSearch" placeholder="搜索题干/解析…" style="flex:1;min-width:160px;padding:7px 10px;border:1.5px solid var(--line);border-radius:9px;background:var(--card);color:var(--ink)"
        @keyup.enter="load(true)" />
      <button class="btn pri" @click="load(true)">搜索</button>
    </div>
  </div>
  <div v-if="loading" class="empty">加载中…</div>
  <div v-else-if="!questions.length" class="card empty">无匹配题目</div>
  <div v-for="q in questions" :key="q.bank_id + q.qid" class="rowitem" style="cursor:pointer" @click="sel = q">
    <div style="flex:1;min-width:0">
      <div class="qq" style="white-space:nowrap;overflow:hidden;text-overflow:ellipsis">{{ q.stem }}</div>
      <div class="meta">
        <span class="tag" :class="{ multi: q.qtype === 'multi' }">{{ typeLabel[q.qtype] }}</span>
        <span v-if="q.status === 'pending_review'" class="tag warn">待复审</span>
        <span v-else-if="q.answer_conf === 'medium'" class="tag warn">中置信</span>
        <span class="src">{{ q.topics.join('/') }} · {{ q.qid }}</span>
      </div>
    </div>
    <img v-if="q.img_path" :src="assetUrl(q.bank_id, q.img_path)" style="width:90px;height:56px;object-fit:cover;border-radius:8px;border:1px solid var(--line)" />
  </div>
  <div style="display:flex;justify-content:center;gap:14px;padding:10px">
    <button class="btn" :disabled="page === 0" @click="page--; load()">上一页</button>
    <span class="hint" style="align-self:center">第 {{ page + 1 }} 页</span>
    <button class="btn" :disabled="questions.length < PAGE" @click="page++; load()">下一页</button>
  </div>

  <div v-if="sel" class="maskbox" @click.self="sel = null">
    <div class="detailbox">
      <div class="meta">
        <span class="tag" :class="{ multi: sel.qtype === 'multi' }">{{ typeLabel[sel.qtype] }}</span>
        <span class="src">{{ sel.topics.join('/') }} · {{ sel.qid }} · {{ sel.status }}</span>
        <button style="margin-left:auto;border:none;background:none;font-size:1.1rem;cursor:pointer;color:var(--sub)" @click="sel = null">✕</button>
      </div>
      <div style="font-size:1.02rem;font-weight:700;margin-bottom:10px;white-space:pre-wrap">{{ sel.stem }}</div>
      <img v-if="sel.img_path" :src="assetUrl(sel.bank_id, sel.img_path!)" style="max-width:100%;border:1px solid var(--line);border-radius:10px;margin-bottom:12px" />
      <div style="display:flex;flex-direction:column;gap:7px;margin-bottom:14px">
        <div v-for="(o, i) in sel.options" :key="i"
          :style="{ display: 'flex', gap: '9px', border: '1.5px solid', borderRadius: '10px', padding: '9px 12px',
            borderColor: sel.answer.includes(String.fromCharCode(65 + i)) ? 'var(--ok)' : 'var(--line)',
            background: sel.answer.includes(String.fromCharCode(65 + i)) ? 'var(--ok-bg)' : 'transparent' }">
          <b style="color:var(--brand)">{{ String.fromCharCode(65 + i) }}</b>{{ o.replace(/^[A-H][、.．,，]\s*/, '') }}
        </div>
      </div>
      <div style="margin-bottom:8px">答案：<b style="color:var(--ok)">{{ sel.answer || '—' }}</b> <span class="tag" :class="{ warn: sel.answer_conf !== 'high' }">{{ sel.answer_conf }}</span></div>
      <div v-if="sel.explain" class="hint" style="background:var(--chip);border-radius:9px;padding:9px 12px;margin-bottom:8px">解析：{{ sel.explain }}</div>
      <div v-if="sel.source" class="src">出处：{{ sel.source }}</div>
    </div>
  </div>
</template>

<style scoped>
.maskbox { position: fixed; inset: 0; background: rgba(10,12,18,.5); display: flex; align-items: center; justify-content: center; z-index: 50; padding: 20px; }
.detailbox { background: var(--card); border-radius: 16px; max-width: 760px; width: 100%; max-height: 86vh; overflow: auto; padding: 22px; }
</style>
