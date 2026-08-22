<script setup lang="ts">
import { computed, ref, onMounted } from 'vue'
import { store } from './store'
import { hasTauri } from './api'
import HomeView from './views/HomeView.vue'
import StudyView from './views/StudyView.vue'
import PracticeView from './views/PracticeView.vue'
import ExamView from './views/ExamView.vue'
import ComposeView from './views/ComposeView.vue'
import SessionView from './views/SessionView.vue'
import ResultView from './views/ResultView.vue'
import WrongView from './views/WrongView.vue'
import HistoryView from './views/HistoryView.vue'
import BrowseView from './views/BrowseView.vue'
import SettingsView from './views/SettingsView.vue'
import DocsView from './views/DocsView.vue'
import BankAdminView from './views/BankAdminView.vue'
import PrintPaperView from './views/PrintPaperView.vue'

const views: Record<string, any> = {
  home: HomeView, study: StudyView, practice: PracticeView, exam: ExamView, compose: ComposeView, session: SessionView,
  result: ResultView, wrong: WrongView, history: HistoryView, browse: BrowseView, docs: DocsView,
  bankadmin: BankAdminView, print: PrintPaperView, settings: SettingsView,
}
const cur = computed(() => views[store.view] ?? HomeView)
const navs = [
  { key: 'home', icon: '🏠', label: '首页' },
  { key: 'study', icon: '📖', label: '学习' },
  { key: 'practice', icon: '📚', label: '练习' },
  { key: 'docs', icon: '📇', label: '资料' },
  { key: 'exam', icon: '📝', label: '考试' },
  { key: 'wrong', icon: '❌', label: '错题本' },
  { key: 'history', icon: '🕘', label: '记录' },
  { key: 'browse', icon: '🗂️', label: '题库' },
  { key: 'bankadmin', icon: '🛠️', label: '管理' },
  { key: 'settings', icon: '⚙️', label: '设置' },
]

// 首启用户协议与隐私声明（V1.1 §6 合规：零遥测声明 + 明确告知）
const eulaOk = ref(localStorage.getItem('sq_eula') === '1')
function acceptEula() { localStorage.setItem('sq_eula', '1'); eulaOk.value = true }

onMounted(() => {
  const t = localStorage.getItem('sq_theme') ?? 'light'
  document.documentElement.dataset.theme = t
  const f = localStorage.getItem('sq_fontsize')
  if (f) document.documentElement.style.fontSize = f + 'px'
})
</script>

<template>
  <div class="shell">
    <aside class="side">
      <div class="logo">🎓<span>SMART 题库</span></div>
      <button v-for="n in navs" :key="n.key" :class="{ on: store.view === n.key || (n.key === 'practice' && store.view === 'session') }"
        @click="store.go(n.key)">
        <i>{{ n.icon }}</i>{{ n.label }}
      </button>
      <div class="env">{{ hasTauri ? '🟢 Tauri 运行中' : '🟡 浏览器 Mock' }}</div>
    </aside>
    <main class="main"><component :is="cur" /></main>
  </div>

  <!-- 首启协议 -->
  <div v-if="!eulaOk" class="eula-mask">
    <div class="eula">
      <h2>欢迎使用 S7-200 SMART 题库平台</h2>
      <p>在使用前，请了解：</p>
      <p><b>🔒 零遥测承诺</b>：本应用完全离线运行，不上传、不收集任何个人信息，无崩溃上报。全部学习数据仅保存在您的电脑。</p>
      <p><b>📚 内容说明</b>：题库依据西门子公开技术资料（系统手册 V2.8、技术参考 PLUS 2.6、选型手册 V2.8）整理，部分答案由 AI 辅助标注并附置信度，仅供学习备考参考。</p>
      <p><b>⏳ 更新检查</b>：默认关闭。开启后仅访问版本信息文件，不发送任何设备或使用数据。</p>
      <div class="eula-btns">
        <button class="btn pri" @click="acceptEula">同意并开始使用</button>
      </div>
    </div>
  </div>
</template>

<style>
:root { --bg:#f3f5f9; --card:#fff; --ink:#1c2333; --sub:#5b6478; --line:#e3e7ef;
  --brand:#2456d6; --brand-ink:#fff; --ok:#0e9f6e; --ok-bg:#e6f7f0; --bad:#d64545; --bad-bg:#fdeeee;
  --warn:#b7791f; --warn-bg:#fdf3e3; --chip:#eef2fb; }
[data-theme=dark] { --bg:#12151d; --card:#1b2029; --ink:#e8ecf4; --sub:#98a2b8; --line:#2a3140;
  --brand:#5b84f0; --brand-ink:#0d1220; --ok:#34c48e; --ok-bg:#12271e; --bad:#f07171; --bad-bg:#2e1a1a;
  --warn:#e2b25a; --warn-bg:#2b2110; --chip:#232b3b; }
* { box-sizing: border-box; margin: 0; padding: 0; }
body { background: var(--bg); color: var(--ink); font-family: system-ui, "Segoe UI", "Microsoft YaHei", sans-serif; }
.shell { display: flex; min-height: 100vh; }
.side { width: 170px; flex-shrink: 0; border-right: 1px solid var(--line); background: var(--card);
  display: flex; flex-direction: column; gap: 3px; padding: 14px 10px; position: sticky; top: 0; height: 100vh; }
.logo { font-weight: 800; font-size: 1.05rem; padding: 4px 10px 14px; display: flex; align-items: center; gap: 7px; }
.side button { display: flex; align-items: center; gap: 9px; border: none; background: none; color: var(--sub);
  padding: 9px 12px; border-radius: 10px; cursor: pointer; font-size: .92rem; text-align: left; }
.side button:hover { background: var(--chip); color: var(--ink); }
.side button.on { background: var(--brand); color: var(--brand-ink); }
.side .env { margin-top: auto; font-size: .72rem; color: var(--sub); padding: 8px 10px; }
.main { flex: 1; padding: 22px 26px; max-width: 980px; min-width: 0; }
h2.pt { font-size: 1.1rem; margin-bottom: 14px; }
.card { background: var(--card); border: 1px solid var(--line); border-radius: 14px; padding: 16px; margin-bottom: 14px; }
.card h3 { font-size: .95rem; margin-bottom: 10px; }
.hint { font-size: .8rem; color: var(--sub); }
.chip { border: 1px solid var(--line); background: var(--chip); border-radius: 999px; padding: 5px 13px; cursor: pointer; font-size: .86rem; }
.chip:hover { border-color: var(--brand); }
.chip small { color: var(--sub); margin-left: 4px; }
.btn { border: none; border-radius: 10px; padding: 9px 16px; cursor: pointer; font-size: .9rem; background: var(--chip); color: var(--ink); }
.btn.pri { background: var(--brand); color: var(--brand-ink); }
.btn.ghost { background: transparent; border: 1px solid var(--line); }
.btn:disabled { opacity: .45; cursor: not-allowed; }
.btn.danger { color: var(--bad); }
.tag { font-size: .7rem; padding: 1px 8px; border-radius: 999px; background: var(--chip); color: var(--sub); }
.tag.multi { background: #e9e2fb; color: #5b3df0; }
[data-theme=dark] .tag.multi { background: #2a2140; color: #b9a5f5; }
.tag.warn { background: var(--warn-bg); color: var(--warn); }
.empty { text-align: center; color: var(--sub); padding: 36px 0; }
.rowitem { border: 1px solid var(--line); border-radius: 11px; padding: 12px 14px; margin-bottom: 10px; background: var(--card); }
.rowitem .qq { font-weight: 600; font-size: .92rem; margin-bottom: 4px; }
.rowitem .meta { display: flex; gap: 6px; align-items: center; flex-wrap: wrap; font-size: .76rem; color: var(--sub); }
.src { font-size: .74rem; color: var(--sub); }
.statrow { display: grid; grid-template-columns: repeat(auto-fill, minmax(150px, 1fr)); gap: 10px; margin-bottom: 14px; }
.stat { background: var(--card); border: 1px solid var(--line); border-radius: 12px; padding: 12px 8px; text-align: center; cursor: pointer; }
.stat b { display: block; font-size: 1.3rem; color: var(--brand); }
.stat span { font-size: .74rem; color: var(--sub); }
.eula-mask { position: fixed; inset: 0; background: rgba(8,10,16,.72); z-index: 100; display: flex; align-items: center; justify-content: center; padding: 20px; }
.eula { background: var(--card); border-radius: 18px; padding: 28px; max-width: 560px; font-size: .92rem; line-height: 1.75; }
.eula h2 { margin-bottom: 10px; }
.eula-btns { margin-top: 18px; text-align: center; }
/* 打印（成绩单导出PDF）：只打印主内容 */
@media print {
  .side, .eula-mask { display: none !important; }
  .main { max-width: 100%; padding: 0; }
  .btn { display: none !important; }
  body { background: #fff; }
}
@media (max-width: 760px) {
  .shell { flex-direction: column; }
  .side { width: 100%; height: auto; flex-direction: row; overflow-x: auto; position: fixed; bottom: 0; z-index: 40; border-right: none; border-top: 1px solid var(--line); padding: 6px; }
  .side .logo, .side .env { display: none; }
  .side button { flex-direction: column; gap: 2px; font-size: .68rem; padding: 6px 10px; }
  .main { padding: 14px; padding-bottom: 90px; }
}
</style>
