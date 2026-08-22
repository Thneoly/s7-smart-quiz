<script setup lang="ts">
import { ref, onMounted } from 'vue'
import { api, saveDialog, openDialog, hasTauri, type RestoreInfo } from '../api'

const theme = ref(localStorage.getItem('sq_theme') ?? 'light')
const fontSize = ref(parseInt(localStorage.getItem('sq_fontsize') ?? '16'))
const restoreInfo = ref<RestoreInfo | null>(null)
const restorePath = ref('')
const msg = ref('')
const updating = ref(false)

async function checkUpdate() {
  if (!hasTauri) { msg.value = '更新检查仅应用内可用（当前为浏览器模式）'; return }
  updating.value = true
  try {
    const { check } = await import('@tauri-apps/plugin-updater')
    const update = await check()
    if (update?.available) {
      if (!confirm(`发现新版本 ${update.version}：\n${update.body ?? ''}\n\n下载并安装？`)) return
      msg.value = '下载更新中…'
      await update.downloadAndInstall()
      const { relaunch } = await import('@tauri-apps/plugin-process')
      await relaunch()
    } else {
      msg.value = '✅ 当前已是最新版本'
    }
  } catch (e: any) {
    msg.value = '检查更新失败：' + e + '（若未配置更新服务器属正常，发布后生效）'
  } finally { updating.value = false }
}

function applyTheme(t: string) {
  theme.value = t
  localStorage.setItem('sq_theme', t)
  document.documentElement.dataset.theme = t
  api.setSetting('theme', t)
}
function applyFont(d: number) {
  fontSize.value = Math.max(13, Math.min(21, fontSize.value + d))
  localStorage.setItem('sq_fontsize', String(fontSize.value))
  document.documentElement.style.fontSize = fontSize.value + 'px'
  api.setSetting('fontsize', String(fontSize.value))
}
async function doBackup() {
  msg.value = ''
  try {
    const path = await saveDialog({
      defaultPath: `smartquiz-备份-${new Date().toISOString().slice(0, 10)}.zip`,
      filters: [{ name: '备份包', extensions: ['zip'] }],
    })
    if (!path) return
    await api.backup(path)
    msg.value = hasTauri ? `✅ 备份完成：${path}` : '✅ 备份完成（mock）'
  } catch (e: any) { msg.value = '❌ ' + e }
}
async function pickRestore() {
  restoreInfo.value = null; msg.value = ''
  try {
    const p = await openDialog({ filters: [{ name: '备份包', extensions: ['zip'] }] })
    if (!p) return
    restorePath.value = p
    restoreInfo.value = await api.restoreCheck(p)
  } catch (e: any) { msg.value = '❌ ' + e }
}
async function doRestore() {
  if (!restorePath.value) return
  if (!confirm(`恢复将覆盖当前全部学习记录（备份内：${restoreInfo.value?.sessions} 场练习 / ${restoreInfo.value?.records} 条作答）。确定继续？`)) return
  msg.value = '恢复功能需要重启应用完成替换（M3 提供一键完成），当前可先记录备份路径。'
}
async function doDiagnostics() {
  try {
    const path = await saveDialog({
      defaultPath: `smartquiz-诊断包-${new Date().toISOString().slice(0, 10)}.zip`,
      filters: [{ name: '诊断包', extensions: ['zip'] }],
    })
    if (!path) return
    await api.diagnostics(path)
    msg.value = hasTauri ? `✅ 诊断包已导出：${path}（无任何个人数据，可放心发给开发者）` : '✅ 诊断包已导出（mock）'
  } catch (e: any) { msg.value = '❌ ' + e }
}
onMounted(() => { document.documentElement.dataset.theme = theme.value })
</script>

<template>
  <h2 class="pt">设置</h2>
  <div class="card">
    <h3>🎨 外观</h3>
    <div style="display:flex;gap:9px;align-items:center;flex-wrap:wrap">
      <button class="btn" :class="{ pri: theme === 'light' }" @click="applyTheme('light')">☀️ 浅色</button>
      <button class="btn" :class="{ pri: theme === 'dark' }" @click="applyTheme('dark')">🌙 深色</button>
      <span style="margin-left:12px" class="hint">字号</span>
      <button class="btn" @click="applyFont(-1)">A⁻</button>
      <span>{{ fontSize }}px</span>
      <button class="btn" @click="applyFont(1)">A⁺</button>
    </div>
  </div>

  <div class="card">
    <h3>💾 数据备份与恢复</h3>
    <p class="hint" style="margin-bottom:10px">备份为一致性快照（VACUUM INTO），包含全部练习记录/错题本/收藏/笔记。建议每周备份一次。</p>
    <div style="display:flex;gap:9px;flex-wrap:wrap">
      <button class="btn pri" @click="doBackup()">📤 备份数据</button>
      <button class="btn" @click="pickRestore()">📥 选择备份文件校验</button>
    </div>
    <div v-if="restoreInfo" class="rowitem" style="margin-top:10px">
      <div class="meta">
        备份时间 {{ restoreInfo.created_at || '—' }} · {{ restoreInfo.sessions }} 场练习 · {{ restoreInfo.records }} 条作答
        <button class="btn danger" style="margin-left:10px" @click="doRestore()">恢复此备份</button>
      </div>
    </div>
  </div>

  <div class="card">
    <h3>🩺 诊断与反馈</h3>
    <p class="hint" style="margin-bottom:10px">遇到问题？导出诊断包发给我们（仅含版本/系统/本地统计数量，零个人数据、零遥测）。</p>
    <div style="display:flex;gap:9px;flex-wrap:wrap">
      <button class="btn" @click="doDiagnostics()">📦 导出诊断包</button>
      <button class="btn ghost" :disabled="updating" @click="checkUpdate()">{{ updating ? '检查中…' : '🔄 检查更新' }}</button>
    </div>
    <p class="hint" style="margin-top:8px">应用不会自动检查更新（隐私默认）；点击上方按钮手动检查。</p>
  </div>

  <div class="card">
    <h3>📜 用户协议与隐私</h3>
    <p class="hint">
      本应用为本地离线应用：<b>不上传、不收集、不联网遥测</b>任何用户数据。<br>
      全部学习记录保存在本机（应用数据目录）。应用更新检查默认关闭，开启时仅访问版本信息文件。<br>
      题库内容依据西门子公开技术资料整理，仅供学习交流。
    </p>
  </div>

  <div class="card">
    <h3>ℹ️ 关于</h3>
    <p class="hint">S7-200 SMART 题库平台 · M2 (0.2.0)<br>种子题库：S7-200 SMART 认证题库 v1（344 题 + A~E 卷 350 题）</p>
  </div>
  <div v-if="msg" class="card" style="border-color: var(--brand)">{{ msg }}</div>
</template>
