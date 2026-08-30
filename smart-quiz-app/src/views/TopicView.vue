<script setup lang="ts">
import { ref, computed, watch, onMounted } from 'vue'
import { api, type Overview } from '../api'
import { startWithQuestions } from '../session-start'
import { store } from '../store'
import topicsJson from '../study/topics.json'
import PidLab from '../components/PidLab.vue'
import ChooseLab from '../components/ChooseLab.vue'

interface Topic {
  id: string; icon: string; title: string; sub: string; why: string
  timeline: { year: string; title: string; text: string }[]
  problem: { sym: string; name: string; does: string; pain: string; hint: string }[]
  landing: { title: string; text: string; ref: string }[]
  exam: string[]
  chapter_no: number; practice_topic: string; lab: string
  timeline_title?: string; timeline_intro?: string; problem_title?: string; landing_title?: string
  lab_title?: string; lab_intro?: string
  lab_cases?: { q: string; ans: string; why: string }[]
}
const topics = (topicsJson as { topics: Topic[] }).topics
const cur = computed(() => topics.find(t => t.id === store.topicId) ?? topics[0])
const ov = ref<Overview | null>(null)
const picks = ref<Record<number, string>>({})
watch(() => cur.value.id, () => { picks.value = {} })

function pick(id: string) { store.go('topic'); store.topicId = id }

async function practice() {
  const t = cur.value
  const tid = ov.value?.topics.find(x => x.name === t.practice_topic)?.topic_id
  const list = await api.questions({ topic_id: tid, status: 'active', limit: 500 }).catch(() => [])
  if (!list.length) { alert('该专题暂无可练题目'); return }
  await startWithQuestions('practice', `专题突破 · ${t.title}`, list)
}
function goLecture() { store.go('study') }

onMounted(async () => { ov.value = await api.overview().catch(() => null) })
</script>

<template>
  <h2 class="pt">{{ cur.icon }} 重难点突破 <span class="hint">把最硬的骨头单独啃——来历、原理、动手、考点一条龙</span></h2>

  <div v-if="topics.length > 1" class="tabs">
    <button v-for="t in topics" :key="t.id" :class="{ on: t.id === cur.id }" @click="pick(t.id)">{{ t.icon }} {{ t.title }}</button>
  </div>

  <div class="card head">
    <h3>{{ cur.icon }} {{ cur.title }} <span class="hint" style="font-weight:400">—— {{ cur.sub }}</span></h3>
    <p class="why">{{ cur.why }}</p>
  </div>

  <!-- ① 时代与衍化：时间轴 -->
  <div class="card">
    <h3>{{ cur.timeline_title ?? '🕰️ 它从哪里来' }}</h3>
    <p class="hint" style="margin-bottom:12px">{{ cur.timeline_intro }}</p>
    <div class="tl">
      <div v-for="(e, i) in cur.timeline" :key="i" class="tli" :class="{ modern: e.year === '今天' }">
        <div class="yr">{{ e.year }}</div>
        <div class="tdot"></div>
        <div class="tbody">
          <b>{{ e.title }}</b>
          <p>{{ e.text }}</p>
        </div>
      </div>
    </div>
  </div>

  <!-- ② 解决的问题：三分量 -->
  <div class="card">
    <h3>{{ cur.problem_title ?? '🧩 它解决什么问题' }}</h3>
    <div class="pgrid">
      <div v-for="c in cur.problem" :key="c.sym" class="pcard" :class="{ union: c.sym.length > 1 }">
        <div class="psym">{{ c.sym }}</div>
        <b class="pname">{{ c.name }}</b>
        <p><span class="pk">干什么</span>{{ c.does }}</p>
        <p><span class="pk bad">代价</span>{{ c.pain }}</p>
        <p class="phint">{{ c.hint }}</p>
      </div>
    </div>
  </div>

  <!-- ③ 动手实验 -->
  <div class="card" v-if="cur.lab === 'pid'">
    <h3>🔬 动手实验：把三个参数的脾气看个明白</h3>
    <p class="hint" style="margin-bottom:6px">下面是浏览器里实时运行的一台"虚拟电加热炉"（二阶惯性 + 纯滞后）。按①→③顺序点预设，亲眼看余差怎么来、怎么消、超调怎么被压下去——这比背十遍定义都管用。改完滑块曲线立即重算。</p>
    <PidLab />
  </div>
  <div class="card" v-else-if="cur.lab === 'choose' && cur.lab_cases">
    <h3>{{ cur.lab_title }}</h3>
    <p class="hint" style="margin-bottom:6px">{{ cur.lab_intro }}</p>
    <ChooseLab :key="cur.id" :cases="cur.lab_cases as any" v-model:picks="picks" />
  </div>

  <!-- ④ 落到 S7-200 SMART -->
  <div class="card">
    <h3>{{ cur.landing_title ?? '⚙️ 落到 S7-200 SMART' }}</h3>
    <div v-for="(l, i) in cur.landing" :key="i" class="land">
      <b>{{ l.title }}</b>
      <p>{{ l.text }}</p>
      <span class="lref">📄 {{ l.ref }}</span>
    </div>
  </div>

  <!-- ⑤ 考点聚焦 -->
  <div class="card exam">
    <h3>⭐ 考点聚焦（认证备考视角）</h3>
    <ul>
      <li v-for="(e, i) in cur.exam" :key="i">{{ e }}</li>
    </ul>
  </div>

  <!-- ⑥ 行动 -->
  <div class="card" style="display:flex;gap:10px;flex-wrap:wrap;align-items:center">
    <span class="hint">看完就练，趁热打铁：</span>
    <button class="btn" @click="goLecture()">📖 第{{ cur.chapter_no }}章讲义与要点</button>
    <button class="btn pri" @click="practice()">🎯 练{{ cur.title }}真题</button>
  </div>
</template>

<style scoped>
.tabs { display: flex; gap: 8px; margin-bottom: 14px; flex-wrap: wrap; }
.tabs button { border: 1px solid var(--line); background: var(--card); border-radius: 999px; padding: 7px 16px; cursor: pointer; font-size: .88rem; }
.tabs button.on { background: var(--brand); color: var(--brand-ink); border-color: var(--brand); }
.head h3 { margin-bottom: 8px; }
.why { font-size: .9rem; color: var(--sub); line-height: 1.8; }
/* 三列文档流：年份 | 点线 | 内容——年份永不溢出卡片 */
.tli { display: grid; grid-template-columns: 58px 16px 1fr; column-gap: 8px; align-items: start; padding-bottom: 15px; }
.tli:last-child { padding-bottom: 2px; }
.yr { grid-column: 1; text-align: right; font-weight: 700; font-size: .84rem; color: var(--brand); padding-top: 1px; }
.tdot { grid-column: 2; position: relative; align-self: stretch; }
.tdot::before { content: ''; position: absolute; left: 4px; top: 5px; width: 8px; height: 8px; border-radius: 50%; background: var(--brand); opacity: .8; }
.tdot::after { content: ''; position: absolute; left: 7px; top: 16px; bottom: -2px; width: 2px; background: var(--line); }
.tli:last-child .tdot::after { display: none; }
.tbody { grid-column: 3; }
.tbody b { font-size: .9rem; }
.tbody p { font-size: .84rem; color: var(--sub); line-height: 1.7; margin-top: 3px; }
.tli.modern .yr { color: var(--warn); }
.tli.modern .tdot::before { background: var(--warn); }
.pgrid { display: grid; grid-template-columns: repeat(auto-fit, minmax(210px, 1fr)); gap: 10px; }
.pcard { border: 1px solid var(--line); border-radius: 12px; padding: 12px; background: var(--card); }
.pcard.union { border-color: var(--warn); background: var(--warn-bg); }
.psym { font-size: 1.5rem; font-weight: 800; color: var(--brand); line-height: 1; margin-bottom: 2px; }
.pcard.union .psym { color: var(--warn); font-size: 1.1rem; }
.pname { font-size: .9rem; }
.pcard p { font-size: .8rem; color: var(--sub); line-height: 1.65; margin-top: 7px; }
.pk { display: inline-block; font-size: .68rem; border-radius: 5px; padding: 1px 6px; margin-right: 6px; background: var(--chip); color: var(--ink); }
.pk.bad { background: var(--bad-bg); color: var(--bad); }
.phint { color: var(--warn) !important; font-size: .74rem !important; }
.land { border-bottom: 1px dashed var(--line); padding: 10px 0; position: relative; padding-right: 110px; }
.land:last-child { border-bottom: none; }
.land b { font-size: .88rem; }
.land p { font-size: .84rem; color: var(--sub); line-height: 1.7; margin-top: 3px; }
.lref { position: absolute; right: 0; top: 12px; font-size: .7rem; color: var(--sub); background: var(--chip); border-radius: 6px; padding: 2px 8px; }
.exam ul { padding-left: 18px; }
.exam li { font-size: .86rem; line-height: 1.9; color: var(--ink); }
</style>
