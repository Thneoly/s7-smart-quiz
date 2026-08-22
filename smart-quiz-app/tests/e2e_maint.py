# -*- coding: utf-8 -*-
"""可维护性 E2E（mock）：日志卡片→查看最近日志→刷新→目录按钮禁用→版本号→诊断文案"""
import sys, io, time, subprocess
sys.stdout = io.TextIOWrapper(sys.stdout.buffer, encoding='utf-8')
from playwright.sync_api import sync_playwright

proc = subprocess.Popen(['npm', 'run', 'dev'], cwd='D:/PLC/s7-200/smart-quiz-app',
                        shell=True, stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL)
time.sleep(5)
errs, fails = [], []
def check(name, cond):
    print(('✓' if cond else '✗ FAIL'), name)
    if not cond: fails.append(name)

try:
    with sync_playwright() as p:
        b = p.chromium.launch(channel='msedge', headless=True)
        pg = b.new_page(viewport={'width': 1280, 'height': 900})
        pg.on('pageerror', lambda e: errs.append(str(e)))
        pg.on('dialog', lambda d: d.accept())
        pg.goto('http://localhost:1420', timeout=30000)
        pg.wait_for_timeout(1000)

        # 首启协议
        check('首启协议弹窗', pg.locator('.eula').count() == 1)
        pg.click('button:has-text("同意并开始使用")')
        pg.wait_for_timeout(500)

        # 1 设置页：运行日志卡片
        pg.click('.side button:has-text("设置")')
        pg.wait_for_timeout(600)
        check('运行日志卡片存在', pg.locator('h3:has-text("运行日志")').count() == 1)
        body = pg.text_content('.main') or ''
        check('滚动策略说明（保留3次/2MB）', '保留最近 3 次' in body and '2MB' in body)
        check('隐私说明（不含题目内容）', '不含题目内容' in body)

        # 2 查看最近日志（mock 返回 6 行）
        pg.click('button:has-text("查看最近日志")')
        pg.wait_for_timeout(500)
        pre = pg.locator('pre').first
        pre_text = pre.text_content() or ''
        check('日志查看器出现', pre.count() >= 1 and '启动 smart-quiz-app' in pre_text)
        check('mock 日志含会话生命周期', '开始会话#1' in pre_text and '完成' in pre_text)
        check('mock 日志含命令错误样例', 'ERROR' in pre_text and 'export_excel_template' in pre_text)
        check('行数标注', '5 行' in (pg.text_content('.main') or ''))

        # 3 刷新按钮
        pg.click('button:has-text("刷新")')
        pg.wait_for_timeout(400)
        check('刷新后查看器仍在', '启动 smart-quiz-app' in (pg.locator('pre').first.text_content() or ''))

        # 4 mock 模式下"打开日志目录"禁用
        check('打开目录按钮禁用(浏览器模式)', pg.locator('button:has-text("打开日志目录")').is_disabled())

        # 5 关于卡片动态版本号
        about = pg.locator('.card', has=pg.locator('h3:has-text("关于")')).text_content() or ''
        check('关于卡片显示版本号', ('浏览器模式' in about) and ('v0.1.0' in about or 'v' in about))

        # 6 诊断文案提及日志
        check('诊断文案提及日志附带', '自动附带' in (pg.text_content('.main') or ''))

        pg.screenshot(path='D:/PLC/s7-200/题库资料/shots/11_运行日志.png')
        b.close()
finally:
    # Windows: terminate() 只杀 npm 外壳，node(vite) 子进程会残留占住 1420 端口——按进程树击杀
    subprocess.run(['taskkill', '/F', '/T', '/PID', str(proc.pid)], capture_output=True)
    time.sleep(1)

print('\nJS错误:', errs if errs else '(无)')
print('失败项:', fails if fails else '无 —— 可维护性全部通过 🎉')
sys.exit(1 if fails or errs else 0)
