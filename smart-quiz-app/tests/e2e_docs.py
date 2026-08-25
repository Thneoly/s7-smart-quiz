# -*- coding: utf-8 -*-
"""资料速查 E2E（mock）：五板块tab/搜索/分类/详情/全文检索"""
import sys, io, os, time, subprocess
sys.stdout = io.TextIOWrapper(sys.stdout.buffer, encoding='utf-8')
from playwright.sync_api import sync_playwright
proc = subprocess.Popen(['npm', 'run', 'dev'], cwd=os.environ.get('SQ_APP_DIR', 'D:/PLC/s7-200/smart-quiz-app'),
                        shell=True, stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL)
time.sleep(5)
errs, fails = [], []
def check(name, cond):
    print(('✓' if cond else '✗ FAIL'), name)
    if not cond: fails.append(name)
try:
    with sync_playwright() as p:
        b = p.chromium.launch(channel=None if __import__('os').environ.get('SQ_BROWSER') == 'chromium' else __import__('os').environ.get('SQ_BROWSER', 'msedge'), headless=True)
        pg = b.new_page(viewport={'width': 1280, 'height': 900})
        pg.on('pageerror', lambda e: errs.append(str(e)))
        pg.on('dialog', lambda d: d.accept())
        pg.goto('http://localhost:1420', timeout=30000)
        pg.wait_for_timeout(1000)
        if pg.locator('.eula').count():
            pg.click('button:has-text("同意并开始使用")'); pg.wait_for_timeout(400)
        pg.click('.side button:has-text("资料")')
        pg.wait_for_timeout(600)
        n_hw = pg.locator('.rowitem').count()
        check(f'硬件规格条目显示（{n_hw}）', n_hw > 0)
        # 搜索 ST60
        pg.fill('input[placeholder*="搜索"]', 'ST60')
        pg.wait_for_timeout(400)
        n_st = pg.locator('.rowitem').count()
        check('硬件搜索ST60过滤', 0 < n_st < n_hw)
        # 展开详情
        pg.locator('.rowitem').first.click()
        pg.wait_for_timeout(300)
        check('规格详情表展开', pg.locator('.ftab').count() == 1)
        # 切指令tab
        pg.click('.chip:has-text("指令速查")')
        pg.wait_for_timeout(400)
        check('指令条目显示', pg.locator('.rowitem').count() > 0)
        # 全文检索
        pg.click('.chip:has-text("全文检索")')
        pg.wait_for_timeout(400)
        pg.fill('input[placeholder*="手册"]', 'Modbus 地址')
        pg.click('button:has-text("搜索")')
        pg.wait_for_timeout(600)
        n_hit = pg.locator('.rowitem').count()
        check(f'全文检索mock结果（{n_hit}）', n_hit == 2)
        check('结果显示出处标签', '系统手册' in (pg.text_content('.main') or '') or '指令帮助' in (pg.text_content('.main') or ''))
        # 语料包导入按钮：仅应用内显示（mock 模式隐藏）
        check('语料包导入按钮按环境隐藏', pg.locator('button:has-text("导入语料包")').count() == 0)
        # 各tab切换不报错
        for t in ['通信速查', '故障诊断', '公式换算']:
            pg.click(f'.chip:has-text("{t}")')
            pg.wait_for_timeout(300)
        check('全部板块可切换', True)
        (os.path.isdir(os.environ.get('SQ_SHOTS', 'D:/PLC/s7-200/题库资料/shots')) and pg.screenshot(path=os.path.join(os.environ.get('SQ_SHOTS', 'D:/PLC/s7-200/题库资料/shots'), '12_资料速查.png')))
        b.close()
finally:
    # Windows: terminate() 只杀 npm 外壳，node(vite) 子进程会残留占住 1420 端口——按进程树击杀
    subprocess.run(['taskkill', '/F', '/T', '/PID', str(proc.pid)], capture_output=True) if os.name == 'nt' else proc.terminate()
    time.sleep(1)
print('JS错误:', errs if errs else '(无)')
print('失败项:', fails if fails else '无 —— 全部通过 🎉')
sys.exit(1 if fails or errs else 0)
