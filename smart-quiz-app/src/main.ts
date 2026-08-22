import { createApp } from "vue";
import App from "./App.vue";
import { hasTauri, logFrontendError } from "./api";

const app = createApp(App);

// 全局错误捕获：渲染层异常/未捕获 Promise 落滚动日志（命令层由 Rust 打点，此处只兜前端）
app.config.errorHandler = (err, _inst, info) => { void logFrontendError(`Vue:${info}`, err) }
window.addEventListener('error', e => { if (e.error) void logFrontendError('window', e.error) })
window.addEventListener('unhandledrejection', e => { void logFrontendError('unhandledrejection', e.reason) })
if (!hasTauri) console.info('[smartquiz] 浏览器 mock 模式：日志与部分功能仅应用内可用')

app.mount("#app");
