<script setup lang="ts">
import { ref, computed, onMounted } from 'vue'
import { api, docsApi, type Overview, type DocHit, type QuestionRow, type TopicAcc } from '../api'
import { startWithQuestions } from '../session-start'
import guide from '../study/guide.json'
import refdata from '../study/refdata.json'
import lecturesData from '../study/lectures.json'
import { CHAPTER_LINKS } from '../study/chapter_links'

interface Chapter {
  no: number; title: string; minutes: number; goal: string
  points: string[]; practice_topics: string[]; manual_ref: string; days: string
  priority: 'core' | 'key' | 'ext'
}
interface Stage { stage: string; stage_goal: string; chapters: Chapter[] }
interface RefItem { name: string; category: string; fields: [string, string][]; note: string; source: string }
interface LectureSec { h: string; paras: string[]; ref: string }
interface Lecture { no: number; title: string; intro: string; sections: LectureSec[]; exam_tips: string[] }

const stages = (guide as any).stages as Stage[]
const LECTURES: Record<number, Lecture> = Object.fromEntries(
  (((lecturesData as any).lectures ?? []) as Lecture[]).map(l => [l.no, l]))
const lecture = (c: Chapter) => LECTURES[c.no]
const ov = ref<Overview | null>(null)
const readSet = ref<Set<number>>(new Set())
const openCh = ref<number | null>(null)
const mastery = ref<TopicAcc[]>([])
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

// ---------- 章节纵深：手册原文 / 关联速查 / 考点真题（展开时懒加载） ----------
const chDocs = ref<Record<number, DocHit[]>>({})
const chQs = ref<Record<number, QuestionRow[]>>({})
const chPick = ref<Record<string, string>>({})   // 随手练已选：`章号:qid` -> 选项字母
const chSent = ref<Record<string, boolean>>({})  // 已判定

async function openChapter(c: Chapter) {
  openCh.value = openCh.value === c.no ? null : c.no
  if (openCh.value !== c.no) return
  const link = CHAPTER_LINKS[c.no]
  if (link?.docs && !chDocs.value[c.no]) {
    chDocs.value[c.no] = await docsApi.search(link.docs, 5).catch(() => [])
  }
  if (!chQs.value[c.no]) {
    const qs: QuestionRow[] = []
    for (const t of c.practice_topics) {
      const tid = ov.value?.topics.find(x => x.name === t)?.topic_id
      if (!tid) continue
      const list = await api.questions({ topic_id: tid, status: 'active', limit: 3 }).catch(() => [])
      qs.push(...list)
    }
    // 随手练只支持可选字母的单选/多选（判断/填空去练习模式作答）
    chQs.value[c.no] = qs.filter(q => q.qtype === 'single' || q.qtype === 'multi').slice(0, 5)
  }
}
const linkDocs = (c: Chapter) => chDocs.value[c.no] ?? []
const linkQs = (c: Chapter) => chQs.value[c.no] ?? []

/** 关联速查：板块内 name/category 命中任一关键词（静态数据本地过滤） */
const refItems = (c: Chapter): RefItem[] => {
  const link = CHAPTER_LINKS[c.no]
  if (!link?.ref.length) return []
  const out: RefItem[] = []
  for (const { sec, kw } of link.ref) {
    const items = ((refdata as any).sections ?? []).find((s: any) => s.key === sec)?.items ?? []
    for (const it of items) {
      const hay = `${it.name} ${it.category}`.toLowerCase()
      if (kw.some(k => hay.includes(k.toLowerCase())) && !out.includes(it)) out.push(it)
    }
  }
  return out.slice(0, 6)
}

// 随手练判定（镜像 Rust：单选即判；多选全对才得分；低置信度/无答案不判定）
const normL = (s: string) => [...new Set((s || '').split('').filter(ch => 'ABCDE'.includes(ch)))].sort().join('')
const qkey = (c: Chapter, q: QuestionRow) => `${c.no}:${q.qid}`
const sent = (c: Chapter, q: QuestionRow) => !!chSent.value[qkey(c, q)]
const judgeCls = (c: Chapter, q: QuestionRow): string | null => {
  if (!sent(c, q)) return null
  if (!q.answer || q.answer_conf !== 'high') return 'na'
  return normL(chPick.value[qkey(c, q)] ?? '') === normL(q.answer) ? 'ok' : 'bad'
}
function pick(c: Chapter, q: QuestionRow, letter: string) {
  const key = qkey(c, q)
  if (sent(c, q)) return
  if (q.qtype === 'single') { chPick.value[key] = letter; chSent.value[key] = true }
  else {
    const set = new Set((chPick.value[key] ?? '').split('').filter(Boolean))
    set.has(letter) ? set.delete(letter) : set.add(letter)
    chPick.value[key] = [...set].sort().join('')
  }
}
const optLetter = (opt: string) => opt.trim()[0] ?? ''
function optCls(c: Chapter, q: QuestionRow, opt: string): string {
  const letter = optLetter(opt)
  const picked = (chPick.value[qkey(c, q)] ?? '').includes(letter)
  const judged = judgeCls(c, q)
  if (!judged) return picked ? 'picked' : ''
  if (q.answer_conf === 'high' && normL(q.answer).includes(letter)) return 'correct'
  return picked ? 'wrong' : ''
}

onMounted(async () => {
  loadProgress()
  ov.value = await api.overview()
  api.dashboard().then(d => { mastery.value = d.by_topic.filter(t => t.a > 0) }).catch(() => {})
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

  <!-- 知识点掌握度（作答后出现，薄弱优先） -->
  <div v-if="mastery.length" class="card">
    <h3>📊 知识点掌握度 <span class="hint">按正确率升序 · 点条目开练</span></h3>
    <div v-for="m in [...mastery].sort((x, y) => (x.c / x.a) - (y.c / y.a))" :key="m.topic"
      class="mrow" @click="practiceTopic(m.topic)">
      <span class="mname">{{ m.topic }}</span>
      <div class="mbar"><i :style="{ width: Math.round(m.c / m.a * 100) + '%' }"></i></div>
      <span class="hint" style="width:86px;text-align:right">{{ m.c }}/{{ m.a }} · {{ Math.round(m.c / m.a * 100) }}%</span>
    </div>
  </div>

  <div v-for="s in stages" :key="s.stage" class="card">
    <h3>{{ s.stage }} <span class="hint">已完成 {{ stageProgress(s) }}</span></h3>
    <p class="hint" style="margin-bottom:10px">{{ s.stage_goal }}</p>
    <div v-for="c in s.chapters" :key="c.no" class="chrow" :class="{ read: readSet.has(c.no) }">
      <div class="chhead" @click="openChapter(c)">
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
        <!-- 📘 本章讲义（工作流从官方语料生成，段落转述+出处标注） -->
        <div v-if="lecture(c)" class="deep lecture">
          <div class="dhead">📘 本章讲义 <span class="hint">依据官方资料整理 · 段落附出处</span></div>
          <p class="lintro">{{ lecture(c)!.intro }}</p>
          <div v-for="(s, i) in lecture(c)!.sections" :key="i" class="lsec">
            <b>{{ s.h }}</b>
            <p v-for="(p, j) in s.paras" :key="j">{{ p }}</p>
            <div class="src">出处：{{ s.ref }}</div>
          </div>
          <div v-if="lecture(c)!.exam_tips.length" class="ltips">
            <b>🎯 考点提示</b>
            <ul><li v-for="(t, i) in lecture(c)!.exam_tips" :key="i">{{ t }}</li></ul>
          </div>
        </div>

        <p class="goal">🎯 {{ c.goal }}</p>
        <ul class="pts">
          <li v-for="(p, i) in c.points" :key="i">{{ p }}</li>
        </ul>
        <div class="lnks">
          <span class="hint">配套练习：</span>
          <button v-for="t in c.practice_topics" :key="t" class="chip" @click="practiceTopic(t)">{{ t }} →</button>
        </div>

        <!-- 📖 手册原文选段 -->
        <div v-if="linkDocs(c).length" class="deep">
          <div class="dhead">📖 手册原文选段</div>
          <div v-for="(h, i) in linkDocs(c)" :key="i" class="docHit">
            <div class="meta"><span class="src">{{ h.path }}</span> · {{ h.title }}</div>
            <div class="snip">{{ h.snippet }}</div>
          </div>
        </div>

        <!-- 📇 关联速查 -->
        <div v-if="refItems(c).length" class="deep">
          <div class="dhead">📇 关联速查</div>
          <div v-for="(r, i) in refItems(c)" :key="i" class="refItem">
            <b>{{ r.name }}</b> <span class="tag">{{ r.category }}</span>
            <div class="fmini">
              <span v-for="(f, j) in r.fields.slice(0, 3)" :key="j"><i>{{ f[0] }}</i>{{ f[1] }}</span>
            </div>
            <p v-if="r.note" class="hint" style="margin:4px 0 0">{{ r.note }}</p>
          </div>
        </div>

        <!-- 🎯 考点真题随手练 -->
        <div v-if="linkQs(c).length" class="deep">
          <div class="dhead">🎯 考点真题 <span class="hint">随手练，不计入统计</span></div>
          <div v-for="q in linkQs(c)" :key="q.qid" class="miniQ">
            <div class="qq">{{ q.stem }}</div>
            <div class="opts">
              <button v-for="opt in q.options" :key="opt" class="mopt" :class="optCls(c, q, opt)"
                @click="pick(c, q, optLetter(opt))">{{ opt }}</button>
            </div>
            <div v-if="q.qtype === 'multi' && !sent(c, q) && (chPick[qkey(c, q)] ?? '').length >= 1" style="margin-top:6px">
              <button class="btn" @click="chSent[qkey(c, q)] = true">提交多选</button>
            </div>
            <div v-if="judgeCls(c, q)" class="judge" :class="judgeCls(c, q)">
              {{ judgeCls(c, q) === 'ok' ? '✓ 答对' : judgeCls(c, q) === 'bad' ? '✗ 答错' : '— 不判定（低置信/无答案）' }}
              <span class="hint">答案 {{ q.answer }} · 出处 {{ q.source }}</span>
              <p v-if="q.explain" class="exp">{{ q.explain }}</p>
            </div>
          </div>
        </div>

        <p class="src">📖 {{ c.manual_ref }}</p>
      </div>
    </div>
  </div>
</template>

<style scoped>
.chrow { border: 1px solid var(--line); border-radius: 11px; margin-bottom: 9px; background: var(--card); }
.mrow { display: flex; align-items: center; gap: 10px; padding: 6px 4px; cursor: pointer; border-radius: 7px; font-size: .86rem; }
.mrow:hover { background: var(--chip); }
.mname { width: 150px; color: var(--ink); overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
.mbar { flex: 1; height: 8px; background: var(--chip); border-radius: 4px; overflow: hidden; }
.mbar i { display: block; height: 100%; background: var(--ok); }
.chrow.read { opacity: .62; }
.chhead { display: flex; align-items: center; gap: 9px; padding: 10px 13px; cursor: pointer; flex-wrap: wrap; font-size: .92rem; }
.chrow.read .chhead b { text-decoration: line-through; }
.rd input { width: 17px; height: 17px; cursor: pointer; }
.chbody { padding: 4px 16px 14px; border-top: 1px dashed var(--line); }
.goal { font-size: .88rem; margin: 10px 0 8px; color: var(--ink); }
.pts { margin: 0 0 10px 18px; font-size: .86rem; line-height: 1.9; color: var(--ink); }
.lnks { display: flex; align-items: center; gap: 7px; flex-wrap: wrap; margin-bottom: 6px; }
.deep { border: 1px solid var(--line); border-radius: 10px; padding: 9px 12px; margin: 10px 0; background: var(--bg, transparent); }
.dhead { font-size: .85rem; font-weight: 700; margin-bottom: 8px; color: var(--ink); }
.lintro { font-size: .86rem; color: var(--ink); line-height: 1.8; margin: 0 0 10px; }
.lsec { margin-bottom: 12px; }
.lsec > b { font-size: .88rem; color: var(--ink); display: block; margin-bottom: 4px; }
.lsec p { font-size: .85rem; color: var(--ink); line-height: 1.8; margin: 0 0 6px; }
.lsec .src { font-size: .76rem; }
.ltips { border-top: 1px dashed var(--line); padding-top: 8px; font-size: .84rem; }
.ltips ul { margin: 6px 0 0 18px; line-height: 1.8; color: var(--ink); }
.docHit { margin-bottom: 8px; }
.docHit .snip { font-size: .82rem; color: var(--sub); line-height: 1.7; margin-top: 2px; }
.refItem { margin-bottom: 9px; font-size: .85rem; color: var(--ink); }
.fmini { display: flex; gap: 12px; flex-wrap: wrap; margin-top: 4px; font-size: .8rem; }
.fmini span { color: var(--sub); }
.fmini i { font-style: normal; color: var(--ink); font-weight: 600; margin-right: 4px; }
.miniQ { margin-bottom: 12px; }
.miniQ .qq { font-size: .86rem; color: var(--ink); line-height: 1.6; }
.opts { display: flex; flex-direction: column; gap: 5px; margin-top: 6px; }
.mopt { text-align: left; padding: 7px 11px; border: 1.5px solid var(--line); border-radius: 8px;
  background: var(--card); color: var(--ink); cursor: pointer; font-size: .84rem; }
.mopt.picked { border-color: var(--brand); background: var(--chip); }
.mopt.correct { border-color: var(--ok); background: var(--ok-bg); }
.mopt.wrong { border-color: var(--bad); background: var(--bad-bg); }
.judge { font-size: .84rem; margin-top: 7px; }
.judge.ok { color: var(--ok); }
.judge.bad { color: var(--bad); }
.judge.na { color: var(--sub); }
.judge .exp { color: var(--sub); margin: 4px 0 0; line-height: 1.6; }
</style>
