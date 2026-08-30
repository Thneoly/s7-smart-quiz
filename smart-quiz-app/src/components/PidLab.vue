<script setup lang="ts">
import { ref, computed } from 'vue'

// ---------- 对象模型：二阶惯性 + 纯滞后（加热炉→测温），近似真实温控 ----------
// 炉温一阶滞后 T1，测温再滞后 T2，控制到炉温有 DEAD 秒纯滞后——这正是"难控"的来源
const T1 = 20, T2 = 12, DEAD = 4, AMB = 25, RANGE = 75, DT = 0.1, T_END = 300, T_STEP = 20
const SP_HI = 50
// 确定性伪噪声（两条不同频率正弦叠加，避免随机数导致测试不可复现）
const noiseAt = (t: number) => (Math.sin(t * 7.3) + Math.sin(t * 13.1)) * 0.35

interface Param { kc: number; ti: number | null; td: number; noise: boolean }
const PRESETS: { name: string; p: Param; verdict: string }[] = [
  { name: '① 纯 P：看余差', p: { kc: 3, ti: null, td: 0, noise: false },
    verdict: '曲线爬到 ~44°C 就停住，永远差 6°C —— 余差：P 的输出与偏差成正比，偏差为零输出也为零，"没力气"再往上顶。' },
  { name: '② 加 I：余差消失', p: { kc: 3, ti: 30, td: 0, noise: false },
    verdict: '积分把历史偏差一点点累积顶掉了余差，最终精确 50°C —— 代价是动作慢半拍，冲过头约 6°C（超调）才回来。' },
  { name: '③ 加 D：压下超调', p: { kc: 3, ti: 30, td: 8, noise: false },
    verdict: '微分盯住变化趋势提前收油，超调从 6°C 压到 2°C 以内、稳定更快 —— D 给系统注入的是"阻尼"。' },
  { name: '④ 增益过大：振荡', p: { kc: 12, ti: null, td: 0, noise: false },
    verdict: 'Kc=12 反应过猛：升温冲过头 → 大幅回砍 → 再冲……来回振荡停不下来。余差是小了，代价却是失稳——P/I/D 的每个好处都有标价。' },
  { name: '⑤ 测量噪声 + D', p: { kc: 3, ti: 30, td: 8, noise: true },
    verdict: '测温叠加噪声后，D 把噪声当成"趋势"放大，输出高频抖动 —— 这就是 D 怕噪声、常被滤波或干脆关小的原因（向导默认微分 0.00）。' },
]

const kc = ref(3), ti = ref<number | null>(30), td = ref(8), noise = ref(false)
const presetIdx = ref(1)

// ---------- 仿真：位置式 PID + 微分先行（对测量值微分）+ 输出限幅抗积分饱和 ----------
const sim = computed(() => {
  let y1 = AMB, y2 = AMB, integ = 0, pvPrev = AMB
  const nD = Math.round(DEAD / DT), uHist: number[] = new Array(nD).fill(0)
  const pts: [number, number][] = [], uPts: [number, number][] = []
  let over = 0
  for (let k = 0; k * DT < T_END; k++) {
    const t = k * DT
    const sp = t >= T_STEP ? SP_HI : AMB
    const pv = y2 + (noise.value ? noiseAt(t) : 0)
    const e = (sp - pv) / RANGE * 100                    // 偏差，%满量程
    const dPv = (pv - pvPrev) / DT / RANGE * 100          // 测量值变化率
    const p = kc.value * e
    const tiE = ti.value ?? Infinity                       // I 关闭时速率恰为 0，不累积
    integ += kc.value / tiE * e * DT
    const d = -kc.value * td.value * dPv                  // 微分先行：对 PV 而非偏差
    let u = p + integ + d
    if (u > 100) { if (e > 0) integ -= kc.value / tiE * e * DT; u = 100 }  // 抗饱和
    if (u < 0) { if (e < 0) integ -= kc.value / tiE * e * DT; u = 0 }
    uHist.push(u); const uD = uHist.shift() ?? 0
    y1 += ((AMB + RANGE * uD / 100) - y1) * DT / T1       // 炉温
    y2 += (y1 - y2) * DT / T2                             // 测温
    pvPrev = pv
    if (t >= T_STEP && y2 - SP_HI > over) over = y2 - SP_HI
    if (k % 5 === 0) { pts.push([t, y2]); uPts.push([t, u]) }
  }
  const tail = pts.slice(-40)
  const ess = Math.abs(SP_HI - tail.reduce((s, r) => s + r[1], 0) / tail.length)
  let settle = 0
  for (const [t, y] of pts) if (t >= T_STEP && Math.abs(y - SP_HI) > 1.5) settle = t - T_STEP
  return { pts, uPts, ess, over, settle: Math.min(settle, T_END - T_STEP - 1) }
})

// ---------- 绘图 ----------
const W = 760, PV_TOP = 12, PV_H = 208, U_TOP = 240, U_H = 52, L = 34, R = 12
const x = (t: number) => L + t / T_END * (W - L - R)
const yT = (v: number) => PV_TOP + PV_H - (v - 20) / 80 * PV_H   // 20~100°C
const yU = (u: number) => U_TOP + U_H - u / 100 * U_H
const path = (arr: [number, number][], fy: (v: number) => number) =>
  arr.map(([t, v], i) => `${i ? 'L' : 'M'}${x(t).toFixed(1)} ${fy(v).toFixed(1)}`).join(' ')
const spPath = `M${x(0)} ${yT(AMB)} L${x(T_STEP)} ${yT(AMB)} L${x(T_STEP)} ${yT(SP_HI)} L${x(T_END)} ${yT(SP_HI)}`

const verdict = computed(() => {
  const hit = PRESETS[presetIdx.value]
  return hit.p.kc === kc.value && hit.p.ti === ti.value && hit.p.td === td.value && hit.p.noise === noise.value
    ? hit.verdict : '参数已手动修改——观察曲线形态，对照上方三个参数的脾气找感觉。'
})
function apply(i: number) {
  const p = PRESETS[i].p
  kc.value = p.kc; ti.value = p.ti; td.value = p.td; noise.value = p.noise
  presetIdx.value = i
}
</script>

<template>
  <div class="pidlab">
    <div class="presets">
      <button v-for="(ps, i) in PRESETS" :key="i" class="preset" :class="{ on: presetIdx === i }" @click="apply(i)">{{ ps.name }}</button>
    </div>

    <div class="ctrls">
      <label>增益 Kc <b>{{ kc.toFixed(1) }}</b><input type="range" min="0.5" max="15" step="0.5" v-model.number="kc" /></label>
      <label>积分时间 Ti <b>{{ ti ? ti + ' s' : '关' }}</b><input type="range" min="5" max="120" step="5" v-model.number="ti" :disabled="!ti" />
        <input type="checkbox" v-model="ti" :true-value="30" :false-value="null" title="积分作用开关" /></label>
      <label>微分时间 Td <b>{{ td }} s</b><input type="range" min="0" max="20" step="1" v-model.number="td" /></label>
      <label class="nk"><input type="checkbox" v-model="noise" /> 测温噪声 ±0.7°C</label>
    </div>

    <svg class="plot" :viewBox="`0 0 ${W} 300`" preserveAspectRatio="xMidYMid meet">
      <!-- 网格与坐标 -->
      <text v-for="v in [20, 40, 60, 80, 100]" :key="'g' + v" :x="L - 6" :y="yT(v) + 4" text-anchor="end" class="ax">{{ v }}°C</text>
      <line v-for="v in [40, 60, 80]" :key="'l' + v" :x1="L" :y1="yT(v)" :x2="W - R" :y2="yT(v)" class="grid" />
      <text v-for="u in [0, 50, 100]" :key="'u' + u" :x="L - 6" :y="yU(u) + 4" text-anchor="end" class="ax">{{ u }}%</text>
      <text :x="W / 2" y="298" text-anchor="middle" class="ax">时间 (s) —— 第 {{ T_STEP }}s 设定值阶跃 25 → {{ SP_HI }} °C（对象：二阶惯性 + {{ DEAD }}s 纯滞后，近似电加热炉）</text>
      <!-- 输出(下) / 设定值(虚线) / PV(上) -->
      <path :d="path(sim.uPts, yU)" class="uline" />
      <path :d="spPath" class="spline" />
      <path :d="path(sim.pts, yT)" class="pvline" />
    </svg>

    <div class="metrics">
      <span class="metric">余差 <b>{{ sim.ess.toFixed(1) }} °C</b></span>
      <span class="metric">超调 <b>{{ sim.over.toFixed(1) }} °C</b></span>
      <span class="metric">稳定时间 <b>{{ sim.settle >= T_END - T_STEP - 1 ? '未稳(振荡)' : sim.settle.toFixed(0) + ' s' }}</b></span>
    </div>
    <p class="verdict">💡 {{ verdict }}</p>
  </div>
</template>

<style scoped>
.pidlab { margin-top: 4px; }
.presets { display: flex; gap: 8px; flex-wrap: wrap; margin-bottom: 12px; }
.preset { border: 1px solid var(--line); background: var(--chip); border-radius: 999px; padding: 6px 14px;
  cursor: pointer; font-size: .84rem; color: var(--ink); }
.preset.on { background: var(--brand); color: var(--brand-ink); border-color: var(--brand); }
.ctrls { display: flex; gap: 22px; flex-wrap: wrap; align-items: center; margin-bottom: 10px; }
.ctrls label { display: flex; align-items: center; gap: 8px; font-size: .84rem; color: var(--sub); }
.ctrls b { color: var(--ink); min-width: 52px; }
.ctrls input[type=range] { width: 130px; }
.plot { width: 100%; height: auto; display: block; background: var(--card); }
.ax { font-size: 10px; fill: var(--sub); }
.grid { stroke: var(--line); stroke-width: .5; stroke-dasharray: 2 3; }
.spline { fill: none; stroke: var(--sub); stroke-width: 1.2; stroke-dasharray: 5 4; }
.pvline { fill: none; stroke: var(--brand); stroke-width: 2.2; }
.uline { fill: none; stroke: var(--ok); stroke-width: 1.4; opacity: .85; }
.metrics { display: flex; gap: 10px; margin: 10px 0 6px; flex-wrap: wrap; }
.metric { border: 1px solid var(--line); border-radius: 9px; padding: 5px 12px; font-size: .82rem; color: var(--sub); background: var(--chip); }
.metric b { color: var(--ink); margin-left: 4px; }
.verdict { font-size: .86rem; color: var(--ink); line-height: 1.65; background: var(--warn-bg);
  border-radius: 9px; padding: 9px 12px; }
</style>
