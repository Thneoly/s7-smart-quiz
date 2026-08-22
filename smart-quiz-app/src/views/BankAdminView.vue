<script setup lang="ts">
import { ref, onMounted } from 'vue'
import { api, m3Api, saveDialog, openDialog, hasTauri, type Overview, type ExcelPreview, type ExcelImportReport, type DupGroup } from '../api'

const tab = ref<'import' | 'dedup' | 'banks'>('import')
const ov = ref<Overview | null>(null)

// ---- 导入向导（3步） ----
const step = ref(1)
const filePath = ref('')
const bankName = ref('我的Excel题库')
const preview = ref<ExcelPreview | null>(null)
const importing = ref(false)
const report = ref<ExcelImportReport | null>(null)
const msg = ref('')

async function downloadTemplate() {
  const path = await saveDialog({ defaultPath: '题库导入模板.xlsx', filters: [{ name: 'Excel', extensions: ['xlsx'] }] })
  if (!path) return
  try {
    const out = await m3Api.exportTemplate(path)
    msg.value = hasTauri ? `✅ 模板已保存：${out}（含示例行与填写说明）` : '✅ 模板已保存（mock）'
  } catch (e: any) { msg.value = '❌ ' + e }
}
async function pickFile() {
  const p = await openDialog({ filters: [{ name: 'Excel', extensions: ['xlsx'] }] })
  if (!p) return
  filePath.value = p
  try {
    preview.value = await m3Api.excelPreview(p)
    step.value = 2
  } catch (e: any) { msg.value = '❌ ' + e }
}
async function demoPick() {
  filePath.value = 'demo.xlsx'
  preview.value = await m3Api.excelPreview(filePath.value)
  step.value = 2
}
async function doImport() {
  importing.value = true
  try {
    report.value = await m3Api.excelImport(filePath.value, bankName.value || 'Excel题库')
    step.value = 3
    ov.value = await api.overview()
  } catch (e: any) { msg.value = '❌ ' + e } finally { importing.value = false }
}
function restart() { step.value = 1; filePath.value = ''; preview.value = null; report.value = null; msg.value = '' }

// ---- 去重 ----
const groups = ref<DupGroup[]>([])
const scanning = ref('')
const keepSel = ref<Record<number, string>>({}) // 组内保留选择
async function scan(bankId: string, bankName2: string) {
  scanning.value = bankId
  try {
    groups.value = await m3Api.dedupScan(bankId)
    groups.value.forEach((g, i) => { keepSel.value[i] = g.items[0].qid })
    if (!groups.value.length) msg.value = `✅ 「${bankName2}」未发现重复题目`
  } catch (e: any) { msg.value = '❌ ' + e } finally { scanning.value = '' }
}
async function mergeGroup(g: DupGroup, i: number) {
  const keep = keepSel.value[i]
  const removes = g.items.filter(x => x.qid !== keep).map(x => x.qid)
  if (!removes.length) return
  if (!confirm(`合并将删除 ${removes.length} 道重复题（作答记录/错题/收藏自动迁移到保留题），确定？`)) return
  try {
    const bankId = ov.value?.banks.find(b => !b.bank_id.startsWith('smart-core'))?.bank_id ?? ov.value?.banks[0].bank_id ?? 'smart-core'
    await m3Api.dedupMerge(bankId, keep, removes)
    await scan(bankId, '当前题库')
  } catch (e: any) { msg.value = '❌ ' + e }
}

onMounted(async () => { ov.value = await api.overview() })
</script>

<template>
  <h2 class="pt">题库管理 <span class="hint">Excel 导入 · 去重合并</span></h2>

  <div style="display:flex;gap:7px;margin-bottom:14px;flex-wrap:wrap">
    <button class="chip" :style="tab==='import'?'background:var(--brand);color:var(--brand-ink)':''" @click="tab='import'">📥 Excel 导入</button>
    <button class="chip" :style="tab==='dedup'?'background:var(--brand);color:var(--brand-ink)':''" @click="tab='dedup'">🔍 去重合并</button>
    <button class="chip" :style="tab==='banks'?'background:var(--brand);color:var(--brand-ink)':''" @click="tab='banks'">📚 题库列表</button>
  </div>

  <!-- 导入向导 -->
  <template v-if="tab==='import'">
    <div class="card">
      <div class="steps">
        <span :class="{ on: step>=1 }">① 选择文件</span> →
        <span :class="{ on: step>=2 }">② 校验预览</span> →
        <span :class="{ on: step>=3 }">③ 导入完成</span>
      </div>
      <div v-if="step===1">
        <p class="hint" style="margin-bottom:12px">支持 .xlsx（单工作表）。列：题干/题型/选项A~F/答案/解析/出处/一级章节/二级章节/难度/置信度。<br>
        题型：单选/多选/判断/填空；多选答案支持 <b>A,B,D</b> 或 <b>ABD</b>；判断支持 <b>对/错/√/×/T/F</b>；低置信度题导入后进入待复审、不参与判分。</p>
        <div style="display:flex;gap:9px;flex-wrap:wrap">
          <button class="btn" @click="downloadTemplate()">📄 下载导入模板</button>
          <button class="btn pri" @click="pickFile()">选择 Excel 文件…</button>
          <button v-if="!hasTauri" class="btn ghost" @click="demoPick">使用演示数据（mock）</button>
        </div>
      </div>

      <div v-else-if="step===2 && preview">
        <div class="statrow">
          <div class="stat"><b>{{ preview.total }}</b><span>总行数</span></div>
          <div class="stat"><b style="color:var(--ok)">{{ preview.valid }}</b><span>可导入</span></div>
          <div class="stat"><b style="color:var(--bad)">{{ preview.errors.length }}</b><span>错误行</span></div>
        </div>
        <div v-if="preview.errors.length" class="card" style="background:var(--bad-bg)">
          <h3>⚠ 错误行（将跳过）</h3>
          <div v-for="e in preview.errors.slice(0, 20)" :key="e.row" class="hint">第 {{ e.row }} 行：{{ e.msg }}</div>
          <div v-if="preview.errors.length > 20" class="hint">… 共 {{ preview.errors.length }} 条</div>
        </div>
        <div v-if="preview.sample.length" class="card">
          <h3>预览（前 {{ Math.min(5, preview.valid) }} 题）</h3>
          <div v-for="(q, i) in preview.sample" :key="i" class="rowitem">
            <div class="qq">{{ q.stem }}</div>
            <div class="meta"><span class="tag">{{ q.qtype }}</span><span>答案 {{ q.answer }}</span><span class="src">{{ q.topic1 }}{{ q.topic2 ? '/' + q.topic2 : '' }}</span></div>
          </div>
        </div>
        <label>导入为题库：<input v-model="bankName" style="padding:7px 10px;border:1.5px solid var(--line);border-radius:8px;background:var(--card);color:var(--ink)" /></label>
        <div style="display:flex;gap:9px;margin-top:12px">
          <button class="btn pri" :disabled="importing || !preview.valid" @click="doImport()">{{ importing ? '导入中…' : `导入 ${preview.valid} 题` }}</button>
          <button class="btn ghost" @click="restart">重新选择</button>
        </div>
      </div>

      <div v-else-if="step===3 && report">
        <div class="statrow">
          <div class="stat"><b style="color:var(--ok)">{{ report.imported }}</b><span>成功导入</span></div>
          <div class="stat"><b style="color:var(--warn)">{{ report.skipped }}</b><span>跳过错误行</span></div>
          <div class="stat"><b>{{ report.topics }}</b><span>章节</span></div>
        </div>
        <p class="hint">题库「{{ report.bank_name }}」（{{ report.bank_id }}）已创建，可在练习/组卷中使用。</p>
        <button class="btn pri" @click="restart">继续导入</button>
      </div>
    </div>
  </template>

  <!-- 去重 -->
  <template v-else-if="tab==='dedup'">
    <div class="card">
      <p class="hint" style="margin-bottom:10px">扫描规则：题干归一化（去标点/空格/大小写）完全一致 = 精确重复；SimHash 海明距离 ≤3 = 高度相似。合并时保留一题，其余删除——试卷引用、作答记录、错题本、收藏、复习计划、笔记自动迁移到保留题（qid_mapping 留痕可追溯）。</p>
      <div style="display:flex;gap:9px;flex-wrap:wrap">
        <button v-for="b in ov?.banks ?? []" :key="b.bank_id" class="btn" :disabled="!!scanning" @click="scan(b.bank_id, b.name)">
          {{ scanning === b.bank_id ? '扫描中…' : `🔍 扫描「${b.name}」` }}
        </button>
      </div>
    </div>
    <div v-for="(g, i) in groups" :key="i" class="card">
      <h3><span class="tag" :class="{ warn: g.kind === 'similar' }">{{ g.kind === 'exact' ? '精确重复' : '高度相似' }}</span> {{ g.items.length }} 题</h3>
      <div v-for="it in g.items" :key="it.qid" class="rowitem" style="display:flex;gap:10px;align-items:flex-start">
        <label style="display:flex;gap:6px;align-items:center;font-size:.85rem;padding-top:2px">
          <input type="radio" :name="'g'+i" :checked="keepSel[i]===it.qid" @change="keepSel[i]=it.qid" /> 保留
        </label>
        <div style="flex:1">
          <div class="qq">{{ it.stem }}</div>
          <div class="meta"><span class="src">{{ it.qid }}</span><span v-if="it.status!=='active'" class="tag warn">{{ it.status }}</span></div>
        </div>
      </div>
      <button class="btn pri" style="margin-top:8px" @click="mergeGroup(g, i)">合并（保留选中题，迁移数据后删除其余）</button>
    </div>
  </template>

  <!-- 题库列表 -->
  <template v-else>
    <div v-for="b in ov?.banks ?? []" :key="b.bank_id" class="card">
      <h3>{{ b.name }} <span class="hint">v{{ b.version }} · {{ b.bank_id.startsWith('smart-core') ? '内置' : '导入' }}</span></h3>
      <div class="meta" style="display:flex;gap:16px;flex-wrap:wrap;font-size:.85rem;color:var(--sub)">
        <span>共 {{ b.total }} 题（启用 {{ b.active }} / 待复审 {{ b.pending }}）</span>
        <span>{{ b.papers }} 套试卷</span>
      </div>
    </div>
  </template>
  <div v-if="msg" class="card" style="border-color:var(--brand)">{{ msg }}</div>
</template>

<style scoped>
.steps { display: flex; gap: 8px; font-size: .88rem; color: var(--sub); margin-bottom: 14px; }
.steps .on { color: var(--brand); font-weight: 700; }
</style>
