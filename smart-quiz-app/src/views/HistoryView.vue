<script setup lang="ts">
import { ref, computed, onMounted } from 'vue'
import { api, MODE_NAME, type SessionBrief, type SessionInfo } from '../api'
import { store } from '../store'
import { resumeSession } from '../session-start'

const rows = ref<SessionBrief[]>([])
const ongoing = ref<SessionInfo[]>([])
const modeName = MODE_NAME
const ansCount = (s: SessionInfo) => Object.keys(s.draft?.picks ?? {}).length
onMounted(async () => {
  const [r, u] = await Promise.all([api.sessions(), api.unfinished().catch(() => [])])
  rows.value = r
  ongoing.value = u
})
const noData = computed(() => !rows.value.length && !ongoing.value.length)
async function discardOngoing(s: SessionInfo) {
  if (!confirm(`放弃「${s.title}」？已答的 ${ansCount(s)} 题进度将被删除`)) return
  try { await api.discardSession(s.session_id) } catch { alert('删除失败，请重试'); return }
  ongoing.value = ongoing.value.filter(x => x.session_id !== s.session_id)
}
</script>

<template>
  <h2 class="pt">练习记录</h2>

  <!-- 进行中：未完成、进度已保存的会话（与下方已完成记录区分） -->
  <div v-if="ongoing.length" class="card ongoingbox">
    <h3>⏸ 进行中 <span class="hint">未完成 · 进度已自动保存</span></h3>
    <div v-for="s in ongoing" :key="s.session_id" class="oitem">
      <div class="qq">{{ s.title }}</div>
      <div class="meta">
        <span class="tag">{{ modeName[s.mode] ?? s.mode }}</span>
        <span>已答 {{ ansCount(s) }}/{{ s.total_qty }} 题</span>
        <span class="src">{{ new Date(s.started_at).toLocaleString() }}</span>
        <span style="flex:1"></span>
        <button class="btn pri" style="padding:4px 14px;font-size:.8rem" @click="resumeSession(s)">▶ 继续作答</button>
        <button class="btn ghost" style="padding:4px 12px;font-size:.8rem" @click="discardOngoing(s)">放弃</button>
      </div>
    </div>
  </div>

  <div v-if="noData" class="card empty">还没有完成的练习</div>
  <div v-for="r in rows" :key="r.session_id" class="rowitem" style="cursor:pointer" @click="store.go('result', { id: r.session_id })">
    <div class="qq">{{ r.title }}</div>
    <div class="meta">
      <span class="tag">{{ modeName[r.mode] ?? r.mode }}</span>
      <span>{{ r.correct_qty }}/{{ r.scored_qty }} 正确</span>
      <span>得分 <b style="color:var(--brand)">{{ r.score ?? '—' }}</b></span>
      <span v-if="r.duration_ms">用时 {{ Math.round(r.duration_ms / 60000) }} 分钟</span>
      <span class="src">{{ r.finished_at ? new Date(r.finished_at).toLocaleString() : '' }}</span>
    </div>
  </div>
</template>

<style scoped>
.ongoingbox { border-color: var(--warn); }
.oitem { border: 1px dashed var(--line); border-radius: 11px; padding: 10px 14px; margin-bottom: 10px; background: var(--card); }
</style>
