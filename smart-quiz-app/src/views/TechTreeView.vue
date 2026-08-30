<script setup lang="ts">
import { ref, computed, onMounted } from 'vue'
import { api, type Overview, type TopicAcc } from '../api'
import { startWithQuestions } from '../session-start'
import { store } from '../store'
import guide from '../study/guide.json'
import tree from '../study/techtree.json'

interface Ch { no: number; title: string; minutes: number; practice_topics: string[]; priority: string }
interface Stage { stage: string; chapters: Ch[] }
interface TNode { no: number; prereqs: number[]; core: boolean; quickstart: boolean; beginner_note: string; min_master: string; est_h: number }
interface Layer { name: string; nodes: TNode[] }

const stages = (guide as any).stages as Stage[]
const chapters: Record<number, Ch> = Object.fromEntries(stages.flatMap(s => s.chapters).map(c => [c.no, c]))
const layers = (tree as any).layers as Layer[]
const quick = new Set<number>((tree as any).quickstart_path as number[])
const nextStep = ((tree as any).meta?.next_step ?? []) as number[]
const nodes: TNode[] = layers.flatMap(l => l.nodes)
const byNo = Object.fromEntries(nodes.map(n => [n.no, n]))

const ov = ref<Overview | null>(null)
const mastery = ref<Record<string, TopicAcc>>({})
const readSet = ref<Set<number>>(new Set())
const sel = ref<number | null>(null)

// ---------- 状态判定：主题作答聚合（≥70% 且 ≥10 题 = 掌握） ----------
const nodeStat = (n: TNode): { state: 'todo' | 'doing' | 'done'; acc: number; a: number } => {
  let a = 0, c = 0
  for (const t of chapters[n.no]?.practice_topics ?? []) {
    const m = mastery.value[t]
    if (m && m.a > 0) { a += m.a; c += m.c }
  }
  if (a === 0) return { state: 'todo', acc: 0, a: 0 }
  const acc = c / a
  return { state: a >= 10 && acc >= 0.7 ? 'done' : 'doing', acc, a }
}
const stateColor: Record<string, string> = { todo: 'var(--chip)', doing: 'var(--warn)', done: 'var(--ok)' }

// ---------- 布局：层=列，节点=行卡；SVG 贝塞尔画前置边 ----------
const COL_W = 210, ROW_H = 96, GAP_X = 62, NODE_W = 190, NODE_H = 74
const pos = (n: TNode) => {
  const li = layers.findIndex(l => l.nodes.some(x => x.no === n.no))
  const ni = layers[li].nodes.findIndex(x => x.no === n.no)
  return { x: li * (COL_W + GAP_X), y: ni * ROW_H, li }
}
const width = computed(() => layers.length * (COL_W + GAP_X))
const height = computed(() => Math.max(...layers.map(l => l.nodes.length)) * ROW_H + 10)
const edges = computed(() => {
  const out: { d: string; qs: boolean; weak: boolean }[] = []
  for (const l of layers) for (const n of l.nodes) {
    for (const p of n.prereqs) {
      const from = byNo[p], to = n
      if (!from) continue
      const f = pos(from), t = pos(to)
      const x1 = f.x + NODE_W, y1 = f.y + NODE_H / 2, x2 = t.x, y2 = t.y + NODE_H / 2
      const mx = (x1 + x2) / 2
      out.push({
        d: `M ${x1} ${y1} C ${mx} ${y1}, ${mx} ${y2}, ${x2} ${y2}`,
        qs: quick.has(p) && quick.has(to.no),
        weak: nodeStat(to).state !== 'done' && nodeStat(from).state !== 'done',
      })
    }
  }
  return out
})
const qsEdges = computed(() => edges.value.filter(e => e.qs))

// ---------- 汇总 ----------
const doneCount = computed(() => nodes.filter(n => nodeStat(n).state === 'done').length)
const qsNodes = nodes.filter(n => quick.has(n.no))
const qsDone = computed(() => qsNodes.filter(n => nodeStat(n).state === 'done').length)
const remainH = computed(() => nodes.filter(n => nodeStat(n).state !== 'done').reduce((s, n) => s + n.est_h, 0))
const fmtAcc = (n: TNode) => { const s = nodeStat(n); return s.a ? `${Math.round(s.acc * 100)}% · ${s.a}题` : '未作答' }

// ---------- 交互 ----------
const selNode = computed(() => sel.value != null ? byNo[sel.value] : null)
const prereqNodes = computed(() => (selNode.value?.prereqs ?? []).map(p => byNo[p]).filter(Boolean))
async function practiceChapter(n: TNode) {
  const qs = []
  for (const t of chapters[n.no]?.practice_topics ?? []) {
    const tid = ov.value?.topics.find(x => x.name === t)?.topic_id
    const list = await api.questions({ topic_id: tid, status: 'active', limit: 500 }).catch(() => [])
    qs.push(...list)
  }
  if (!qs.length) { alert('该章节暂无可练题目'); return }
  await startWithQuestions('practice', `技能树 · 第${n.no}章 ${chapters[n.no].title}`, qs)
}
function goLecture() { store.go('study') }

onMounted(async () => {
  const [o, d, r] = await Promise.all([api.overview(), api.dashboard(), api.getSetting('guide_read')])
  ov.value = o
  mastery.value = Object.fromEntries(d.by_topic.map(t => [t.topic, t]))
  readSet.value = new Set(JSON.parse(r ?? '[]'))
})
</script>

<template>
  <h2 class="pt">技能树 <span class="hint">知识依赖全景 · 高亮路径为零基础快速入门</span></h2>

  <div class="card" style="display:flex;gap:18px;flex-wrap:wrap;align-items:center">
    <b>已点亮 {{ doneCount }}/{{ nodes.length }}</b>
    <span class="hint">快速入门路径 <b style="color:var(--warn)">{{ qsDone }}/{{ qsNodes.length }}</b></span>
    <span class="hint">剩余约 {{ remainH }} 小时</span>
    <span v-if="qsDone >= qsNodes.length && nextStep.length" class="hint" style="color:var(--warn)">
      🎉 入门路径已走完！下一步：{{ nextStep.map(n => `第${n}章 ${chapters[n]?.title}`).join(' / ') }}</span>
    <span style="flex:1"></span>
    <span class="hint">状态：<i class="dot" :style="{ background: 'var(--ok)' }"></i>已掌握(≥70%且≥10题)
      <i class="dot" :style="{ background: 'var(--warn)' }"></i>学习中
      <i class="dot" :style="{ background: 'var(--chip)' }"></i>未开始</span>
  </div>

  <div class="card scrollx">
    <div class="treebox" :style="{ width: width + 'px', height: height + 'px' }">
      <svg class="edges" :width="width" :height="height">
        <path v-for="(e, i) in edges" :key="i" :d="e.d" fill="none" stroke="var(--line)" stroke-width="1.4" :opacity="e.weak ? 0.9 : 0.45" />
        <path v-for="(e, i) in qsEdges" :key="'q' + i" :d="e.d" fill="none" stroke="var(--warn)" stroke-width="3" opacity="0.85" />
      </svg>
      <div v-for="l in layers" :key="l.name" class="layerlbl" :style="{ left: pos(l.nodes[0]).x + 'px' }">{{ l.name }}</div>
      <div v-for="n in nodes" :key="n.no" class="tnode" :class="{ sel: sel === n.no, qs: quick.has(n.no) }"
        :style="{ left: pos(n).x + 'px', top: pos(n).y + 'px', width: NODE_W + 'px', height: NODE_H + 'px' }"
        @click="sel = sel === n.no ? null : n.no">
        <div class="ttop">
          <i class="dot" :style="{ background: stateColor[nodeStat(n).state] }"></i>
          <b>第{{ n.no }}章</b>
          <span v-if="readSet.has(n.no)" title="已读">✓</span>
          <span v-if="n.core" class="tag" style="margin-left:auto">核心</span>
        </div>
        <div class="ttitle">{{ chapters[n.no]?.title ?? '?' }}</div>
        <div class="tmeta">{{ fmtAcc(n) }} · {{ n.est_h }}h</div>
      </div>
    </div>
  </div>

  <!-- 节点详情 -->
  <div v-if="selNode" class="card detail">
    <h3>第{{ selNode.no }}章 {{ chapters[selNode.no]?.title }}
      <span class="tag" :style="{ background: 'var(--chip)', color: stateColor[nodeStat(selNode).state] }">
        {{ nodeStat(selNode).state === 'done' ? '已掌握' : nodeStat(selNode).state === 'doing' ? '学习中' : '未开始' }}</span>
    </h3>
    <p style="font-size:.88rem;color:var(--ink);margin:6px 0">{{ selNode.beginner_note }}</p>
    <p class="hint">🎯 入门达标：{{ selNode.min_master }} · 预计 {{ selNode.est_h }} 小时 · 作答 {{ fmtAcc(selNode) }}</p>
    <div v-if="prereqNodes.length" class="hint" style="margin-top:6px">
      前置：<span v-for="p in prereqNodes" :key="p.no" class="tag"
        :style="{ background: 'var(--chip)', color: stateColor[nodeStat(p).state] }">第{{ p.no }}章 {{ chapters[p.no]?.title.slice(0, 8) }}</span>
      <span v-if="prereqNodes.some(p => nodeStat(p).state !== 'done')" style="color:var(--warn)">
        ⚠ 有前置未掌握，建议先补——不锁定，按需安排</span>
    </div>
    <div style="display:flex;gap:9px;margin-top:10px;flex-wrap:wrap">
      <button class="btn" @click="goLecture()">📖 读讲义与要点</button>
      <button class="btn pri" @click="practiceChapter(selNode)">🎯 练本章题</button>
    </div>
  </div>
</template>

<style scoped>
.scrollx { overflow-x: auto; padding: 14px; }
.treebox { position: relative; }
.edges { position: absolute; left: 0; top: 0; pointer-events: none; }
.layerlbl { position: absolute; top: -12px; font-size: .74rem; color: var(--sub); width: 190px; }
.tnode { position: absolute; border: 1.5px solid var(--line); border-radius: 11px; background: var(--card);
  padding: 7px 10px; cursor: pointer; display: flex; flex-direction: column; justify-content: space-between;
  transition: border-color .15s, transform .1s; }
.tnode:hover { border-color: var(--brand); transform: translateY(-1px); }
.tnode.sel { border-color: var(--brand); box-shadow: 0 0 0 2px var(--chip); }
.tnode.qs { border-color: var(--warn); border-style: solid; }
.ttop { display: flex; align-items: center; gap: 6px; font-size: .72rem; color: var(--sub); }
.ttop b { color: var(--ink); }
.ttitle { font-size: .84rem; font-weight: 600; color: var(--ink); overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
.tmeta { font-size: .7rem; color: var(--sub); }
.dot { display: inline-block; width: 9px; height: 9px; border-radius: 50%; margin: 0 2px; }
.detail { border-color: var(--brand); }
</style>
