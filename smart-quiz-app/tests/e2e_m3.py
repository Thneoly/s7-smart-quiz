# -*- coding: utf-8 -*-
"""M3 E2E（mock）：导入向导3步 → 打印试卷 → 更新检查提示"""
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
        if pg.locator('.eula').count():
            pg.click('button:has-text("同意并开始使用")'); pg.wait_for_timeout(400)

        # 1 管理页导入向导（mock 演示数据）
        pg.click('.side button:has-text("管理")')
        pg.wait_for_timeout(500)
        check('向导第1步', '选择文件' in (pg.text_content('.main') or ''))
        pg.click('button:has-text("使用演示数据")')
        pg.wait_for_timeout(600)
        check('向导第2步预览', pg.locator('.stat').count() >= 3 and '预览' in (pg.text_content('.main') or ''))
        pg.locator('button').filter(has_text='导入 2').click()
        pg.wait_for_timeout(700)
        body = pg.text_content('.main') or ''
        check('向导第3步报告', '成功导入' in body and '题库' in body)

        # 2 题库列表出现新库
        pg.click('.chip:has-text("题库列表")')
        pg.wait_for_timeout(400)
        check('题库列表含导入库', '我的Excel题库' in (pg.text_content('.main') or '') or 'mock' in (pg.text_content('.main') or ''))

        # 3 打印试卷
        pg.click('.side button:has-text("考试")')
        pg.wait_for_timeout(600)
        pg.click('button:has-text("打印试卷")')
        pg.wait_for_timeout(800)
        pbody = pg.text_content('.main') or ''
        check('打印视图卷头', '认证考试' in pbody and '密封线' in pbody.replace(' ', ''))
        check('答题卡页存在', '答题卡' in pbody and pg.locator('.as-bubble').count() > 10)
        check('双栏排版类', pg.evaluate("getComputedStyle(document.querySelector('.columns')).columnCount") == '2')
        pg.click('button:has-text("显示答案")')
        pg.wait_for_timeout(300)
        check('教师版答案标注', '【答案' in (pg.text_content('.main') or ''))
        pg.screenshot(path='D:/PLC/s7-200/题库资料/shots/13_试卷打印.png', full_page=False)

        # 4 设置页更新检查（mock 提示不可用）
        pg.click('.side button:has-text("设置")')
        pg.wait_for_timeout(500)
        pg.click('button:has-text("检查更新")')
        pg.wait_for_timeout(600)
        check('更新检查mock提示', '浏览器模式' in (pg.text_content('.main') or ''))
        b.close()
finally:
    # Windows: terminate() 只杀 npm 外壳，node(vite) 子进程会残留占住 1420 端口——按进程树击杀
    subprocess.run(['taskkill', '/F', '/T', '/PID', str(proc.pid)], capture_output=True)
    time.sleep(1)
print('JS错误:', errs if errs else '(无)')
print('失败项:', fails if fails else '无 —— M3 全部通过 🎉')
sys.exit(1 if fails or errs else 0)
