<script setup lang="ts">
import { ref, computed, onMounted } from 'vue'
import { api, type Overview } from '../api'
import { startWithQuestions } from '../session-start'
import guide from '../study/guide.json'

interface Chapter {
  no: number; title: string; minutes: number; goal: string
  points: string[]; practice_topics: string[]; manual_ref: string; days: string
  priority: 'core' | 'key' | 'ext'
}
interface Stage { stage: string; stage_goal: string; chapters: Chapter[] }

const stages = (guide as any).stages as Stage[]
const ov = ref<Overview | null>(null)
const readSet = ref<Set<number>>(new Set())
const openCh = ref<number | null>(null)
const priLabel: Record<string, [string, string]> = {
  core: ['核心', 'var(--bad)'], key: ['重点', 'var(--warn)'], ext: ['拓展', 'var(--sub)'],
}

const totalCh = stages.reduce((n, s) => n + s.chapters.length, 0)
const totalMin = stages.reduce((n, s) => n + s.chapters.reduce((m, c) => m + c.minutes, 0), 0)
const readCount = computed(() => readSet.value.size)
const progressPct = computed(() => Math.round((readCount.value / totalCh) * 100))
const stageProgress = (s: Stage) => `${s.chapters.filter(c => readSet.value.has(c.no)).length}/${s.chapters.length}`
const fmtH = (min: number) => min >= 60 ? `${Math.round(min / 60)} 小时` : `${min} 分钟`

async function loadProgress() {
  const raw = await api.getSetting('guide_read')
  readSet.value = new Set(JSON.parse(raw ?? '[]'))
}
async function toggleRead(no: number) {
  const s = new Set(readSet.value)
  s.has(no) ? s.delete(no) : s.add(no)
  readSet.value = s
  await api.setSetting('guide_read', JSON.stringify([...s]))
}
async function practiceTopic(topic: string) {
  const tid = ov.value?.topics.find(t => t.name === topic)?.topic_id
  const qs = await api.questions({ topic_id: tid, status: 'active', limit: 500 })
  if (!qs.length) { alert(`题库中「${topic}」主题暂无题目`); return }
  await startWithQuestions('practice', `学习配套练习 · ${topic}`, qs)
}
onMounted(async () => {
  loadProgress()
  ov.value = await api.overview()
})
</script>

<template>
  <h2 class="pt">学习指南 <span class="hint">依据官方培训课程（22章）+ 认证考点整理</span></h2>

  <div class="card">
    <div style="display:flex;justify-content:space-between;align-items:center;margin-bottom:8px">
      <b>总进度 {{ readCount }}/{{ totalCh }} 章 · 全程约 {{ fmtH(totalMin) }} 课时</b>
      <span class="hint">{{ progressPct }}%</span>
    </div>
    <div style="height:8px;background:var(--chip);border-radius:4px;overflow:hidden">
      <i :style="{ display:'block', height:'100%', width: progressPct+'%', background:'var(--ok)', transition:'width .3s' }"></i>
    </div>
    <p class="hint" style="margin-top:10px">
      📌 <b>初级认证备考优先级</b>：<span class="tag" style="background:var(--bad-bg);color:var(--bad)">核心</span> 基础篇第1/2/4/5章 + 中级篇指令/HSC/PID/Modbus +
      <span class="tag" style="background:var(--warn-bg);color:var(--warn)">重点</span> 高级篇通信类。学完一章点「已读」，并做配套主题练习巩固。
    </p>
  </div>

  <div v-for="s in stages" :key="s.stage" class="card">
    <h3>{{ s.stage }} <span class="hint">已完成 {{ stageProgress(s) }}</span></h3>
    <p class="hint" style="margin-bottom:10px">{{ s.stage_goal }}</p>
    <div v-for="c in s.chapters" :key="c.no" class="chrow" :class="{ read: readSet.has(c.no) }">
      <div class="chhead" @click="openCh = openCh === c.no ? null : c.no">
        <label class="rd" @click.stop>
          <input type="checkbox" :checked="readSet.has(c.no)" @change="toggleRead(c.no)" />
        </label>
        <b>第{{ c.no }}章 {{ c.title }}</b>
        <span class="tag" :style="{ background: 'var(--chip)', color: priLabel[c.priority][1] }">{{ priLabel[c.priority][0] }}</span>
        <span class="hint">⏱ {{ fmtH(c.minutes) }}</span>
        <span v-if="c.days" class="hint">📅 {{ c.days }}</span>
        <span style="flex:1"></span>
        <span class="hint">{{ openCh === c.no ? '收起 ▲' : '展开 ▼' }}</span>
      </div>
      <div v-if="openCh === c.no" class="chbody">
        <p class="goal">🎯 {{ c.goal }}</p>
        <ul class="pts">
          <li v-for="(p, i) in c.points" :key="i">{{ p }}</li>
        </ul>
        <div class="lnks">
          <span class="hint">配套练习：</span>
          <button v-for="t in c.practice_topics" :key="t" class="chip" @click="practiceTopic(t)">{{ t }} →</button>
        </div>
        <p class="src">📖 {{ c.manual_ref }}</p>
      </div>
    </div>
  </div>
</template>

<style scoped>
.chrow { border: 1px solid var(--line); border-radius: 11px; margin-bottom: 9px; background: var(--card); }
.chrow.read { opacity: .62; }
.chhead { display: flex; align-items: center; gap: 9px; padding: 10px 13px; cursor: pointer; flex-wrap: wrap; font-size: .92rem; }
.chrow.read .chhead b { text-decoration: line-through; }
.rd input { width: 17px; height: 17px; cursor: pointer; }
.chbody { padding: 4px 16px 14px; border-top: 1px dashed var(--line); }
.goal { font-size: .88rem; margin: 10px 0 8px; color: var(--ink); }
.pts { margin: 0 0 10px 18px; font-size: .86rem; line-height: 1.9; color: var(--ink); }
.lnks { display: flex; align-items: center; gap: 7px; flex-wrap: wrap; margin-bottom: 6px; }
</style>
