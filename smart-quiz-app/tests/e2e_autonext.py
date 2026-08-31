# -*- coding: utf-8 -*-
"""⚡自动下一题 E2E：默认关闭 → 答对自动前进(1s) → 手动导航不受扰 → 偏好记忆 → 恢复默认"""
import sys, io, os, time, socket, subprocess
sys.stdout = io.TextIOWrapper(sys.stdout.buffer, encoding='utf-8')
from playwright.sync_api import sync_playwright

proc = subprocess.Popen(['npm', 'run', 'dev'], cwd=os.environ.get('SQ_APP_DIR', 'D:/PLC/s7-200/smart-quiz-app'),
                        shell=(os.name == 'nt'), stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL)
for _ in range(90):
    try:
        socket.create_connection(('127.0.0.1', 1420), 0.5).close()
        break
    except OSError:
        time.sleep(1)
errs, fails = [], []
def check(name, cond):
    print(('✓' if cond else '✗ FAIL'), name)
    if not cond: fails.append(name)
def is_on(pg):
    cls = (pg.locator('.autonext').get_attribute('class') or '').split()
    return 'on' in cls

try:
    with sync_playwright() as p:
        b = p.chromium.launch(channel=None if os.environ.get('SQ_BROWSER') == 'chromium' else os.environ.get('SQ_BROWSER', 'msedge'), headless=True)
        pg = b.new_page(viewport={'width': 1280, 'height': 900})
        pg.on('pageerror', lambda e: errs.append(str(e)))
        pg.on('dialog', lambda d: d.accept())
        pg.goto('http://localhost:1420', timeout=30000)
        pg.wait_for_timeout(1000)
        if pg.locator('.eula').count():
            pg.click('button:has-text("同意并开始使用")'); pg.wait_for_timeout(400)

        pg.click('.side button:has-text("练习")')
        pg.wait_for_timeout(700)
        pg.click('.chip:has-text("全部主题")')
        pg.wait_for_timeout(900)

        # 1 默认关闭；且关闭时答题不前进（第1题答B后停留）
        check('开关存在且默认关闭', pg.locator('.autonext').count() == 1 and not is_on(pg))
        pg.locator('.opt').nth(1).click()
        pg.wait_for_timeout(1600)
        check('关闭时不自动前进', (pg.locator('.pos').text_content() or '').strip().startswith('1 /'))

        # 2 开启后：翻到第2题作答（B为mock第2题答案）→ ~1s 自动到第3题
        pg.click('.autonext')
        check('开关可开启', is_on(pg))
        pg.click('button:has-text("下一题")')
        pg.wait_for_timeout(400)
        pg.locator('.opt').nth(1).click()
        pg.wait_for_timeout(600)
        check('等待期仍在第2题', (pg.locator('.pos').text_content() or '').strip().startswith('2 /'))
        pg.wait_for_timeout(1400)
        check('答对自动前进到第3题', (pg.locator('.pos').text_content() or '').strip().startswith('3 /'))

        # 3 手动导航取消挂起：翻回第2题，1.5s 内无额外跳动
        pg.click('button:has-text("上一题")')
        pg.wait_for_timeout(1600)
        check('手动导航不受影响', (pg.locator('.pos').text_content() or '').strip().startswith('2 /'))

        # 4 偏好记忆：刷新 → 续做进入做题页，开关仍开启
        pg.click('button:has-text("退出")')
        pg.wait_for_timeout(700)
        pg.reload(timeout=30000)
        pg.wait_for_timeout(1200)
        pg.click('.side button:has-text("练习")')
        pg.wait_for_timeout(700)
        pg.locator('.ongoing .orow button:has-text("继续")').first.click()
        pg.wait_for_timeout(1100)
        check('偏好记忆(刷新后仍开)', is_on(pg))

        # 5 恢复默认关闭（不留状态影响其他场景）并放弃该练习
        pg.click('.autonext')
        check('可恢复关闭', not is_on(pg))
        pg.click('button:has-text("退出")')
        pg.wait_for_timeout(600)
        pg.locator('.ongoing .orow button:has-text("放弃")').first.click()
        pg.wait_for_timeout(500)
        check('已清理进行中会话', pg.locator('.ongoing .orow').count() == 0)
        b.close()
finally:
    subprocess.run(['taskkill', '/F', '/T', '/PID', str(proc.pid)], capture_output=True) if os.name == 'nt' else proc.terminate()
    time.sleep(1)
print('JS错误:', errs if errs else '(无)')
print('失败项:', fails if fails else '无 —— 全部通过 🎉')
sys.exit(1 if fails or errs else 0)
