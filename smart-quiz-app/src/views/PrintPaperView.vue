<script setup lang="ts">
import { ref, onMounted } from 'vue'
import { m3Api, assetUrl, type PrintPaper } from '../api'
import { store } from '../store'

const paper = ref<PrintPaper | null>(null)
const showAnswers = ref(false)
const secName: Record<string, string> = { single: '单项选择题', multi: '多项选择题', judge: '判断题', fill: '填空题' }
const secNote: Record<string, string> = { single: '（每题备选项中只有一个正确答案）', multi: '（每题备选项中有两个或以上正确答案，错选不得分，少选不得分）' }

onMounted(async () => {
  const id = store.params.paperId
  if (!id) { store.go('exam'); return }
  paper.value = await m3Api.printData(id)
})
function print() { window.print() }
const letters = (n: number) => Array.from({ length: n }, (_, i) => String.fromCharCode(65 + i))
</script>

<template>
  <div v-if="paper">
    <div class="no-print toolbar">
      <button class="btn ghost" @click="store.go('exam')">← 返回</button>
      <button class="btn" @click="showAnswers = !showAnswers">{{ showAnswers ? '隐藏答案' : '显示答案（教师版）' }}</button>
      <button class="btn pri" @click="print">🖨 打印 / 另存 PDF</button>
    </div>

    <div class="a4">
      <!-- 卷头 -->
      <div class="paper-head">
        <div class="seal-line">密 封 线</div>
        <h1>S7-200 SMART 认证考试 · {{ paper.name }}</h1>
        <div class="head-info">
          姓名：__________　学号：__________　班级：__________
        </div>
        <div class="head-meta">
          {{ paper.sections.map(s => `${secName[s.qtype]} ${s.questions.length} 题×${s.score_each} 分`).join('　｜　') }}
          　合计 {{ paper.total_count }} 题 / {{ paper.total_score }} 分　考试时间 90 分钟
        </div>
      </div>

      <!-- 试题（双栏） -->
      <div class="columns">
        <section v-for="s in paper.sections" :key="s.qtype" class="sec">
          <h2 class="sec-title">{{ secName[s.qtype] }}<span class="sec-note">{{ secNote[s.qtype] ?? '' }}（每题 {{ s.score_each }} 分，共 {{ s.questions.length * s.score_each }} 分）</span></h2>
          <div v-for="(q, i) in s.questions" :key="q.qid" class="pq">
            <div class="pq-stem">{{ i + 1 }}. {{ q.stem }}
              <span v-if="showAnswers" class="ans-key">【答案：{{ q.answer }}】</span></div>
            <img v-if="q.img_path" :src="assetUrl(q.bank_id, q.img_path!)" class="pq-img" />
            <div class="pq-opts">
              <span v-for="(o, j) in q.options" :key="j" class="pq-opt">
                {{ String.fromCharCode(65 + j) }}．{{ o.replace(/^[A-H][、.．,，]\s*/, '') }}
              </span>
            </div>
          </div>
        </section>
      </div>

      <!-- 答题卡（独立页） -->
      <div class="answersheet">
        <h1 class="as-title">答题卡 · {{ paper.name }}</h1>
        <div class="head-info">姓名：__________　学号：__________</div>
        <section v-for="s in paper.sections" :key="s.qtype" class="as-sec">
          <h3>{{ secName[s.qtype] }}（{{ s.questions.length }} 题，每题 {{ s.score_each }} 分）</h3>
          <div class="as-grid">
            <div v-for="(q, i) in s.questions" :key="q.qid" class="as-row">
              <span class="as-no">{{ i + 1 }}</span>
              <span v-for="l in letters(q.options.length)" :key="l" class="as-bubble" :class="{ filled: showAnswers && q.answer.includes(l) }">{{ l }}</span>
            </div>
          </div>
        </section>
        <div class="as-note">注意事项：用 2B 铅笔将对应题目的答案字母涂黑；多选题涂满所有正确字母。</div>
      </div>
    </div>
  </div>
  <div v-else class="empty">加载试卷中…</div>
</template>

<style scoped>
.toolbar { display: flex; gap: 9px; margin-bottom: 12px; }
.a4 { background: #fff; color: #000; padding: 34px 40px; border-radius: 4px; box-shadow: 0 2px 12px rgba(0,0,0,.18); }
[data-theme=dark] .a4 { background: #fff; color: #000; }
.paper-head { text-align: center; border-bottom: 2px solid #000; padding-bottom: 14px; margin-bottom: 8px; position: relative; }
.paper-head h1 { font-size: 1.35rem; letter-spacing: 2px; }
.seal-line { position: absolute; left: -34px; top: 40%; writing-mode: vertical-lr; font-size: .7rem; letter-spacing: 8px; color: #666; border-left: 1px dashed #999; padding-left: 4px; }
.head-info { font-size: .9rem; margin-top: 8px; }
.head-meta { font-size: .82rem; margin-top: 6px; color: #333; }
.columns { column-count: 2; column-gap: 28px; column-rule: 1px solid #ccc; }
.sec-title { font-size: 1rem; margin: 14px 0 10px; break-after: avoid; }
.sec-note { font-size: .78rem; font-weight: 400; color: #444; }
.pq { margin-bottom: 10px; break-inside: avoid; }
.pq-stem { font-size: .88rem; line-height: 1.65; }
.ans-key { color: #b00; font-weight: 600; }
.pq-img { max-width: 92%; border: 1px solid #ddd; margin: 4px 0; }
.pq-opts { display: flex; flex-direction: column; font-size: .85rem; line-height: 1.55; padding-left: 14px; }
.answersheet { page-break-before: always; margin-top: 24px; }
.as-title { text-align: center; font-size: 1.2rem; margin-bottom: 6px; }
.as-sec h3 { font-size: .9rem; margin: 14px 0 8px; }
.as-grid { display: grid; grid-template-columns: repeat(auto-fill, minmax(130px, 1fr)); gap: 6px 14px; }
.as-row { display: flex; align-items: center; gap: 6px; font-size: .82rem; }
.as-no { width: 22px; text-align: right; border-bottom: 1px solid #999; }
.as-bubble { width: 18px; height: 18px; border: 1.4px solid #000; border-radius: 50%; display: inline-flex; align-items: center; justify-content: center; font-size: .68rem; }
.as-bubble.filled { background: #000; color: #fff; }
.as-note { margin-top: 18px; font-size: .78rem; color: #333; border-top: 1px dashed #999; padding-top: 8px; }
@media print {
  .no-print { display: none !important; }
  .a4 { box-shadow: none; padding: 0; border-radius: 0; }
  @page { size: A4; margin: 14mm 12mm; }
}
</style>
