<script setup lang="ts">
import { computed } from 'vue'

// 通用二选一挑战组件：场景作答 → 判分讲理由（数据来自 topics.json 的 lab_cases）
interface Case { q: string; ans: 'SR' | 'ST'; why: string }
const props = defineProps<{ cases: Case[]; options?: string[] }>()

const opts = props.options ?? ['SR', 'ST']
const picks = defineModel<Record<number, string>>('picks', { default: () => ({}) })

const answered = computed(() => Object.values(picks.value).length)
const correct = computed(() =>
  props.cases.filter((c, i) => picks.value[i] === c.ans).length)
const done = computed(() => answered.value === props.cases.length)
</script>

<template>
  <div class="chlab">
    <div class="score hint">
      已答 <b>{{ answered }}</b>/{{ cases.length }} · 答对 <b style="color:var(--ok)">{{ correct }}</b>
      <span v-if="done" style="color:var(--warn);margin-left:8px">{{ correct === cases.length ? '🎉 全对！选型直觉已成肌肉记忆' : correct >= cases.length - 2 ? '不错！回看错题的理由，正是考点' : '建议回到上方规格表，重点记 1 Hz / 2 A / 寿命 10⁵ 三个数字' }}</span>
    </div>
    <div class="cgrid">
      <div v-for="(c, i) in cases" :key="i" class="ccase" :class="{ ok: picks[i] === c.ans, bad: picks[i] && picks[i] !== c.ans }">
        <div class="cq"><span class="cno">{{ i + 1 }}</span>{{ c.q }}</div>
        <div class="copts">
          <button v-for="o in opts" :key="o" class="copt" :class="{ sel: picks[i] === o, right: picks[i] && o === c.ans }"
            :disabled="!!picks[i]" @click="picks[i] = o">{{ o }}</button>
        </div>
        <p v-if="picks[i]" class="cwhy">
          {{ picks[i] === c.ans ? '✅ ' : '❌ 正确答案 ' + c.ans + ' —— ' }}{{ c.why }}
        </p>
      </div>
    </div>
  </div>
</template>

<style scoped>
.score { margin-bottom: 10px; font-size: .86rem; }
.score b { color: var(--brand); }
.cgrid { display: grid; grid-template-columns: repeat(auto-fill, minmax(300px, 1fr)); gap: 10px; }
.ccase { border: 1px solid var(--line); border-radius: 12px; padding: 12px; background: var(--card); }
.ccase.ok { border-color: var(--ok); background: var(--ok-bg); }
.ccase.bad { border-color: var(--bad); background: var(--bad-bg); }
.cq { font-size: .86rem; font-weight: 600; color: var(--ink); line-height: 1.6; }
.cno { display: inline-block; width: 20px; height: 20px; border-radius: 50%; background: var(--chip);
  color: var(--sub); font-size: .72rem; text-align: center; line-height: 20px; margin-right: 7px; }
.copts { display: flex; gap: 8px; margin-top: 9px; }
.copt { flex: 1; border: 1.5px solid var(--line); background: var(--chip); border-radius: 9px;
  padding: 7px 0; cursor: pointer; font-size: .9rem; font-weight: 700; color: var(--ink); }
.copt:hover:not(:disabled) { border-color: var(--brand); }
.copt.sel { background: var(--brand); color: var(--brand-ink); border-color: var(--brand); }
.copt.right { border-color: var(--ok); box-shadow: 0 0 0 1.5px var(--ok); }
.copt:disabled { cursor: default; opacity: .85; }
.cwhy { font-size: .78rem; color: var(--sub); line-height: 1.65; margin-top: 9px; }
.ccase.ok .cwhy { color: var(--ok); }
.ccase.bad .cwhy { color: var(--bad); }
</style>
