# -*- coding: utf-8 -*-
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
        pg.click('.side button:has-text("学习")')
        pg.wait_for_timeout(700)
        n = pg.locator('.chrow').count()
        check(f'22章学习指南加载（实得{n}）', n == 22)
        check('三篇齐全', pg.locator('h3:has-text("基础篇")').count() >= 1 and pg.locator('h3:has-text("中级篇")').count() >= 1)
        # 展开第1章
        pg.locator('.chhead').first.click()
        pg.wait_for_timeout(400)
        check('章节要点展开', pg.locator('.pts li').count() >= 6)
        check('配套练习chip', pg.locator('.lnks .chip').count() >= 1)
        # M4：章节纵深三块（mock 检索有返回 / refdata 静态过滤 / mock 题库含硬件与选型题）
        pg.wait_for_timeout(700)
        check('手册原文选段', pg.locator('.docHit').count() >= 1)
        check('关联速查命中', pg.locator('.refItem').count() >= 1)
        check('考点真题随手练', pg.locator('.miniQ').count() >= 1)
        pg.locator('.miniQ .mopt').first.click()
        pg.wait_for_timeout(300)
        check('随手练即时判定', pg.locator('.miniQ .judge').count() == 1)
        # 标记已读 → 进度变化
        before = pg.text_content('.card b') or ''
        pg.locator('.rd input').first.check()
        pg.wait_for_timeout(600)
        after = pg.text_content('.card b') or ''
        check('已读进度更新', before != after and '1/22' in after)
        # 配套练习直达
        pg.locator('.lnks .chip').first.click()
        pg.wait_for_timeout(900)
        check('配套练习进入做题页', pg.locator('.qcard').count() == 1)
        pg.screenshot(path='D:/PLC/s7-200/题库资料/shots/11_学习模式.png')
        b.close()
finally:
    # Windows: terminate() 只杀 npm 外壳，node(vite) 子进程会残留占住 1420 端口——按进程树击杀
    subprocess.run(['taskkill', '/F', '/T', '/PID', str(proc.pid)], capture_output=True)
    time.sleep(1)
print('JS错误:', errs if errs else '(无)')
print('失败项:', fails if fails else '无 —— 全部通过 🎉')
sys.exit(1 if fails or errs else 0)
