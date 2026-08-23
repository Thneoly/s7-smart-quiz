<script setup lang="ts">
import { ref, onMounted } from 'vue'
import { api, type Dashboard, type SessionInfo, type DayCount } from '../api'
import { store } from '../store'
import { resumeSession, startWithQuestions } from '../session-start'

const dash = ref<Dashboard | null>(null)
const unfinished = ref<SessionInfo[]>([])
const activity = ref<DayCount[]>([])
const loading = ref(true)
const acc = (d: Dashboard) => d.answered ? Math.round((d.correct / d.answered) * 100) : 0

// 新手三步引导：从未做题时显示，可手动关闭
const onboardDismissed = ref(localStorage.getItem('sq_onboard_hide') === '1')
const showOnboard = ref(false)
function dismissOnboard() {
  onboardDismissed.value = true
  showOnboard.value = false
  localStorage.setItem('sq_onboard_hide', '1')
}
async function startFirst10() {
  // 摸底要跨章节随机：拉足量后洗牌取 10（直接 limit:10 会拿到同章节连续题）
  const pool = await api.questions({ status: 'active', limit: 500 })
  if (!pool.length) { alert('题库为空，请先在「管理」导入题库包'); return }
  for (let i = pool.length - 1; i > 0; i--) {
    const j = Math.floor(Math.random() * (i + 1))
    ;[pool[i], pool[j]] = [pool[j], pool[i]]
  }
  await startWithQuestions('practice', '新手摸底 · 10题', pool.slice(0, 10))
}

// 热力图：最近 15 周（列=周，行=周一~周日）
const heat = ref<{ date: string; count: number; level: number }[][]>([])
function buildHeat(days: DayCount[]) {
  const map = new Map(days.map(d => [d.date, d.count]))
  const today = new Date()
  const cells: { date: string; count: number; level: number }[] = []
  for (let i = 15 * 7 - 1; i >= 0; i--) {
    const d = new Date(today)
    d.setDate(today.getDate() - i)
    const ds = d.toISOString().slice(0, 10)
    const c = map.get(ds) ?? 0
    const level = c === 0 ? 0 : c < 10 ? 1 : c < 30 ? 2 : c < 60 ? 3 : 4
    cells.push({ date: ds, count: c, level })
  }
  const weeks: typeof cells[] = []
  for (let i = 0; i < cells.length; i += 7) weeks.push(cells.slice(i, i + 7))
  heat.value = weeks
}
const heatColor = (lv: number) => ['var(--chip)', '#9ec3f5', '#5b8fe8', '#2f66d0', '#1c47a8'][lv]

onMounted(async () => {
  try {
    const [d, u, a] = await Promise.all([api.dashboard(), api.unfinished(), api.activity(120)])
    dash.value = d; unfinished.value = u; activity.value = a
    buildHeat(a)
    // 无任何做题记录且未手动关闭时展示引导
    showOnboard.value = !onboardDismissed.value && d.answered === 0 && d.sessions_done === 0
  } finally { loading.value = false }
})
</script>

<template>
  <h2 class="pt">学习仪表盘</h2>
  <div v-if="loading" class="empty">加载中…</div>
  <template v-else>
    <div v-if="showOnboard" class="card onboard">
      <div style="display:flex;justify-content:space-between;align-items:center">
        <b>🚀 三步开始</b>
        <button class="btn ghost" style="padding:3px 10px;font-size:.78rem" @click="dismissOnboard()">不再显示</button>
      </div>
      <div class="steps3">
        <div class="step3" @click="store.go('study')">
          <b>① 读第 1 章</b>
          <span>硬件介绍 · 约 40 分钟，了解 CPU/模块家族</span>
        </div>
        <div class="step3" @click="startFirst10()">
          <b>② 10 题摸底</b>
          <span>测一下当前水平，错题自动进错题本</span>
        </div>
        <div class="step3" @click="store.go('wrong')">
          <b>③ 看错题本</b>
          <span>错题自动安排复习，答对两轮消灭</span>
        </div>
      </div>
      <p class="hint" style="margin:8px 0 0">备考路径：学习指南过章节 → 章节内随手练 → 真题模拟卷 → 错题清零。做任一练习后此卡自动收起。</p>
    </div>

    <div class="statrow">
      <div class="stat" @click="store.go('practice')"><b>{{ dash?.answered ?? 0 }}</b><span>累计做题</span></div>
      <div class="stat"><b>{{ dash ? acc(dash) : 0 }}%</b><span>正确率</span></div>
      <div class="stat" @click="store.go('practice')"><b>{{ dash?.due_count ?? 0 }}</b><span>复习到期</span></div>
      <div class="stat" @click="store.go('wrong')"><b>{{ dash?.wrong_active ?? 0 }}</b><span>活跃错题</span></div>
      <div class="stat"><b>{{ dash?.streak_days ?? 0 }}</b><span>学习天数</span></div>
      <div class="stat"><b>{{ dash?.sessions_done ?? 0 }}</b><span>完成场次</span></div>
    </div>

    <div v-if="unfinished.length" class="card">
      <h3>⏸ 进行中的会话（断点续考）</h3>
      <div v-for="s in unfinished" :key="s.session_id" class="rowitem" style="cursor:pointer" @click="resumeSession(s)">
        <div class="qq">{{ s.title }}</div>
        <div class="meta"><span class="tag">{{ s.mode }}</span>已答 {{ Object.keys((s.draft as any)?.picks ?? {}).length }}/{{ s.total_qty }} 题 · 点击继续</div>
      </div>
    </div>

    <div class="card">
      <h3>🔥 学习热力图 <span class="hint">最近 15 周</span></h3>
      <div class="heat">
        <div v-for="(w, wi) in heat" :key="wi" class="heatcol">
          <div v-for="c in w" :key="c.date" class="heatcell" :style="{ background: heatColor(c.level) }"
            :title="`${c.date}：${c.count} 题`"></div>
        </div>
      </div>
    </div>

    <div class="card">
      <h3>🚀 快速开始</h3>
      <div style="display:flex;gap:9px;flex-wrap:wrap">
        <button class="btn pri" @click="store.go('practice')">📚 开始练习</button>
        <button class="btn pri" @click="store.go('exam')">📝 真题模拟</button>
        <button class="btn" @click="store.go('wrong')">❌ 错题重练</button>
      </div>
    </div>

    <div class="card" v-if="dash?.by_topic?.length">
      <h3>📊 各主题正确率</h3>
      <div v-for="t in dash.by_topic" :key="t.topic" style="margin-bottom:9px">
        <div style="display:flex;justify-content:space-between;font-size:.82rem">
          <span>{{ t.topic }}</span><span style="color:var(--sub)">{{ t.c }}/{{ t.a }} · {{ Math.round(t.c / t.a * 100) }}%</span>
        </div>
        <div style="height:6px;background:var(--chip);border-radius:3px;overflow:hidden">
          <i :style="{ display:'block', height:'100%', width: (t.c / t.a * 100) + '%',
            background: t.c / t.a >= .8 ? 'var(--ok)' : t.c / t.a >= .6 ? 'var(--warn)' : 'var(--bad)' }"></i>
        </div>
      </div>
    </div>

    <div class="card" v-if="dash?.recent?.length">
      <h3>🕘 最近练习</h3>
      <div v-for="r in dash.recent.slice(0, 5)" :key="r.session_id" class="rowitem" style="cursor:pointer"
        @click="store.go('result', { id: r.session_id })">
        <div class="qq">{{ r.title }}</div>
        <div class="meta"><span class="tag">{{ r.mode }}</span>{{ r.correct_qty }}/{{ r.scored_qty }} 正确 · 得分 {{ r.score ?? '—' }}</div>
      </div>
    </div>
  </template>
</template>

<style scoped>
.heat { display: flex; gap: 3px; overflow-x: auto; padding: 2px; }
.heatcol { display: flex; flex-direction: column; gap: 3px; }
.heatcell { width: 13px; height: 13px; border-radius: 3px; }
.onboard { border-color: var(--brand); }
.steps3 { display: flex; gap: 10px; margin-top: 10px; flex-wrap: wrap; }
.step3 { flex: 1; min-width: 180px; border: 1.5px solid var(--line); border-radius: 10px; padding: 10px 12px;
  cursor: pointer; display: flex; flex-direction: column; gap: 4px; transition: border-color .15s; }
.step3:hover { border-color: var(--brand); }
.step3 b { font-size: .9rem; color: var(--ink); }
.step3 span { font-size: .78rem; color: var(--sub); line-height: 1.5; }
</style>
