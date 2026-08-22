<script setup lang="ts">
import { ref, onMounted } from 'vue'
import { api, type SessionBrief } from '../api'
import { store } from '../store'

const rows = ref<SessionBrief[]>([])
onMounted(async () => { rows.value = await api.sessions() })
const modeName: Record<string, string> = { practice: '章节练习', random: '随机练习', recite: '背诵', review: '间隔复习', wrong: '错题重练', fav: '收藏练习', exam: '考试' }
</script>

<template>
  <h2 class="pt">练习记录</h2>
  <div v-if="!rows.length" class="card empty">还没有完成的练习</div>
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
