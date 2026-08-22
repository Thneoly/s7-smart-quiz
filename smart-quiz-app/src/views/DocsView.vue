<script setup lang="ts">
import { ref, computed, onMounted } from 'vue'
import { docsApi, hasTauri, openDialog, type DocHit, type DocsStatus } from '../api'
import refdata from '../study/refdata.json'

interface RefItem { name: string; category: string; fields: [string, string][]; note: string; source: string }
const sections = (refdata as any).sections ?? []
const SEC_META: Record<string, { label: string; icon: string }> = {
  hw: { label: '硬件规格', icon: '📇' }, ins: { label: '指令速查', icon: '⚡' },
  comm: { label: '通信速查', icon: '📡' }, fault: { label: '故障诊断', icon: '🔧' },
  formula: { label: '公式换算', icon: '🧮' }, search: { label: '全文检索', icon: '🔍' },
}
const tab = ref('hw')
const kw = ref('')
const selCat = ref<string>('')
const openIdx = ref(-1)

const curItems = computed<RefItem[]>(() => sections.find((s: any) => s.key === tab.value)?.items ?? [])
const cats = computed(() => {
  const m = new Map<string, number>()
  curItems.value.forEach(i => m.set(i.category, (m.get(i.category) ?? 0) + 1))
  return [...m.entries()]
})
const filtered = computed(() => {
  let arr = curItems.value
  if (selCat.value) arr = arr.filter(i => i.category === selCat.value)
  if (kw.value.trim()) {
    const k = kw.value.trim().toLowerCase()
    arr = arr.filter(i => (i.name + i.category + i.fields.flat().join('') + i.note).toLowerCase().includes(k))
  }
  return arr
})

// 全文检索
const q = ref('')
const hits = ref<DocHit[]>([])
const st = ref<DocsStatus | null>(null)
const searching = ref(false)
const building = ref(false)

async function ensureIndex() {
  st.value = await docsApi.status()
  if (st.value.chunks === 0 && hasTauri) {
    building.value = true
    try {
      const n = await docsApi.build(false)
      st.value = await docsApi.status()
      alert(`资料索引构建完成：${n} 个文档块`)
    } catch (e: any) { alert('索引构建失败：' + e) }
    finally { building.value = false }
  }
}
// 导入语料包后强制重建索引（公开版自备数据包；导入的优先于安装包内置）
async function importPack() {
  const p = await openDialog({ filters: [{ name: '语料包', extensions: ['docpack'] }] })
  if (!p) return
  building.value = true
  try {
    await docsApi.importPack(p)
    const n = await docsApi.build(true)
    st.value = await docsApi.status()
    alert(`语料包导入完成，索引已重建：${n} 个文档块`)
  } catch (e: any) { alert('语料包导入失败：' + e) }
  finally { building.value = false }
}
async function doSearch() {
  if (!q.value.trim()) return
  searching.value = true
  try { hits.value = await docsApi.search(q.value.trim(), 20) }
  finally { searching.value = false }
}
onMounted(ensureIndex)
function switchTab(t: string) {
  tab.value = t; kw.value = ''; selCat.value = ''; openIdx.value = -1
  if (t === 'search') ensureIndex()
}
</script>

<template>
  <h2 class="pt">资料速查 <span class="hint">硬件规格 / 指令 / 通信 / 故障 / 公式 + 手册全文检索</span></h2>

  <div style="display:flex;gap:7px;flex-wrap:wrap;margin-bottom:14px">
    <button v-for="(m, k) in SEC_META" :key="k" class="chip" :style="tab === k ? 'background:var(--brand);color:var(--brand-ink);border-color:var(--brand)' : ''"
      @click="switchTab(k as string)">{{ m.icon }} {{ m.label }}</button>
  </div>

  <!-- 结构化速查板块 -->
  <template v-if="tab !== 'search'">
    <div class="card">
      <div style="display:flex;gap:8px;flex-wrap:wrap;align-items:center">
        <input v-model="kw" :placeholder="`搜索${SEC_META[tab]?.label}…（型号/指令名/参数）`"
          style="flex:1;min-width:200px;padding:8px 12px;border:1.5px solid var(--line);border-radius:9px;background:var(--card);color:var(--ink)" />
        <span class="hint">{{ filtered.length }} 条</span>
      </div>
      <div style="display:flex;gap:7px;flex-wrap:wrap;margin-top:10px" v-if="cats.length > 1">
        <button class="chip" :style="!selCat ? 'background:var(--brand);color:var(--brand-ink)' : ''" @click="selCat = ''">全部</button>
        <button v-for="[c, n] in cats" :key="c" class="chip" :style="selCat === c ? 'background:var(--brand);color:var(--brand-ink)' : ''"
          @click="selCat = c">{{ c }}<small>{{ n }}</small></button>
      </div>
    </div>
    <div v-if="!filtered.length" class="card empty">无匹配条目</div>
    <div v-for="(it, i) in filtered" :key="i" class="rowitem" style="cursor:pointer" @click="openIdx = openIdx === i ? -1 : i">
      <div class="qq">{{ it.name }} <span class="tag" style="margin-left:6px">{{ it.category }}</span></div>
      <div class="meta">
        <span v-for="(f, j) in it.fields.slice(0, 3)" :key="j" class="src" style="margin-right:10px">{{ f[0] }}: {{ String(f[1]).slice(0, 40) }}</span>
      </div>
      <div v-if="openIdx === i" style="margin-top:10px;border-top:1px dashed var(--line);padding-top:10px">
        <table class="ftab">
          <tr v-for="(f, j) in it.fields" :key="j">
            <td class="k">{{ f[0] }}</td>
            <td>{{ f[1] }}</td>
          </tr>
        </table>
        <div class="hint" style="margin-top:8px;color:var(--warn)">⚠ {{ it.note }}</div>
        <div class="src" style="margin-top:4px">出处：{{ it.source }}</div>
      </div>
    </div>
  </template>

  <!-- 全文检索 -->
  <template v-else>
    <div class="card">
      <div style="display:flex;gap:8px">
        <input v-model="q" placeholder="搜手册/FAQ/指令帮助/课程全文，如：Modbus 地址 / 模拟量换算 / 清除密码"
          style="flex:1;padding:9px 13px;border:1.5px solid var(--line);border-radius:9px;background:var(--card);color:var(--ink)"
          @keyup.enter="doSearch" />
        <button class="btn pri" :disabled="searching || building" @click="doSearch()">{{ building ? '索引构建中…' : searching ? '搜索中…' : '搜索' }}</button>
      </div>
      <p class="hint" style="margin-top:8px">
        <template v-if="hasTauri">索引：{{ st?.chunks ?? 0 }} 个文档块（jieba 分词 × FTS5）<span v-if="st?.built_at"> · 构建于 {{ new Date(st.built_at).toLocaleString() }}</span></template>
        <template v-else>浏览器 Mock 模式：返回示例结果；Tauri 环境下搜索真实文档</template>
        <button v-if="hasTauri" class="btn" style="margin-left:10px" :disabled="building" @click="importPack()">📦 导入语料包（.docpack）…</button>
      </p>
    </div>
    <div v-for="(h, i) in hits" :key="i" class="rowitem">
      <div class="qq">{{ h.title }}</div>
      <div class="meta" style="margin-bottom:5px"><span class="tag">{{ h.path.split('/')[0] === 'manual' ? '系统手册' : h.path.split('/')[0] === 'techref' ? '技术参考' : h.path.split('/')[0] === 'microwin' ? '指令帮助' : '课程' }}</span>
        <span class="src">{{ h.path }}</span></div>
      <div style="font-size:.86rem;color:var(--ink);background:var(--chip);border-radius:8px;padding:8px 11px">{{ h.snippet }}</div>
    </div>
    <div v-if="q && !hits.length && !searching" class="card empty">未找到相关内容</div>
  </template>
</template>

<style scoped>
.ftab { border-collapse: collapse; width: 100%; }
.ftab td { border: 1px solid var(--line); padding: 6px 11px; font-size: .87rem; }
.ftab td.k { background: var(--chip); color: var(--sub); width: 150px; white-space: nowrap; }
</style>
