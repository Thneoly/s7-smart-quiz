# -*- coding: utf-8 -*-
"""会话续做 E2E（mock 持久化）：做一半→刷新(模拟下次打开)→首页/练习页/记录页三处恢复入口
→恢复到断点→交卷→进行中清空；另覆盖"放弃会话"路径"""
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

def answer_cur(pg, opt_i=1):
    """练习模式答当前单选题：选 B(默认)并等防抖保存落库"""
    pg.locator('.opt').nth(opt_i).click()
    pg.wait_for_timeout(2000)

try:
    with sync_playwright() as p:
        b = p.chromium.launch(channel=None if os.environ.get('SQ_BROWSER') == 'chromium' else os.environ.get('SQ_BROWSER', 'msedge'), headless=True)
        pg = b.new_page(viewport={'width': 1280, 'height': 900})
        pg.on('pageerror', lambda e: errs.append(str(e)))
        pg.on('dialog', lambda d: d.accept())
        pg.goto('http://localhost:1420', timeout=30000)
        pg.wait_for_timeout(1000)
        if pg.locator('.eula').count():
            pg.click('button:has-text("同意并开始使用")')
            pg.wait_for_timeout(400)

        # 1 练习页开始章节练习，答 2 题后做到第 3 题，退出（flushAndExit 落盘）
        pg.click('.side button:has-text("练习")')
        pg.wait_for_timeout(700)
        check('练习页无续做横幅(首轮)', pg.locator('.ongoing').count() == 0)
        pg.click('.chip:has-text("全部主题")')
        pg.wait_for_timeout(900)
        check('进入做题页', pg.locator('.qcard').count() == 1)
        answer_cur(pg)                                  # 第1题 B
        pg.click('button:has-text("下一题")')
        pg.wait_for_timeout(400)
        answer_cur(pg)                                  # 第2题 B
        pg.click('button:has-text("下一题")')            # 停在第3题
        pg.wait_for_timeout(400)
        pg.click('button:has-text("退出")')
        pg.wait_for_timeout(700)

        # 2 练习页横幅出现（未刷新，应用内路径）
        check('练习页续做横幅', pg.locator('.ongoing .orow').count() == 1)
        check('横幅显示进度', '已答 2/6' in (pg.locator('.ongoing .orow').first.text_content() or ''))

        # 3 刷新 = 模拟"下次打开应用"（mock 经 localStorage 持久化）
        pg.reload(timeout=30000)
        pg.wait_for_timeout(1200)
        check('刷新后回首页', pg.locator('.stat').count() >= 4)

        # 4 首页"进行中的练习"卡：进度与断点位置
        check('首页进行中卡片', pg.locator('.ongoing-row').count() == 1)
        home_row = pg.locator('.ongoing-row').first.text_content() or ''
        check('首页卡片进度', '已答 2/6' in home_row and '第 3 题' in home_row)

        # 5 点卡片恢复 → 断点与作答状态还原
        pg.locator('.ongoing-row').first.click()
        pg.wait_for_timeout(1000)
        pos = pg.locator('.pos').text_content() or ''
        check('恢复到第3题', pos.strip().startswith('3 / 6') and '已答 2' in pos)
        pg.click('button:has-text("上一题")')
        pg.wait_for_timeout(500)
        pg.click('button:has-text("上一题")')
        pg.wait_for_timeout(400)
        cls1 = pg.locator('.opt').nth(1).get_attribute('class') or ''
        check('第1题作答还原(B选中)', 'sel' in cls1)

        # 6 翻到最后一题交卷
        for _ in range(5):
            if pg.locator('button:has-text("完成练习")').count():
                break
            pg.click('button:has-text("下一题")')
            pg.wait_for_timeout(250)
        pg.click('button:has-text("完成练习")')
        pg.wait_for_timeout(1200)

        # 7 记录页：已完成 1 条，进行中区块消失
        pg.click('.side button:has-text("记录")')
        pg.wait_for_timeout(700)
        check('记录页已完成1条', pg.locator('.rowitem').count() == 1)
        check('记录页进行中清空', pg.locator('.ongoingbox').count() == 0 and pg.locator('.oitem').count() == 0)

        # 7.5 热力图计数真实（activity 按会话聚合——防 records 键控回归导致的假绿）
        pg.click('.side button:has-text("首页")')
        pg.wait_for_timeout(900)
        check('热力图计入6题', pg.locator('.heatcell[title*="：6 题"]').count() >= 1)

        # 8 练习页横幅消失
        pg.click('.side button:has-text("练习")')
        pg.wait_for_timeout(700)
        check('交卷后横幅消失', pg.locator('.ongoing .orow').count() == 0)

        # 9 防重复：同一套题挂着未完成会话时，再点同一入口应续做而非新开
        pg.click('.chip:has-text("全部主题")')
        pg.wait_for_timeout(900)
        answer_cur(pg)
        pg.click('button:has-text("退出")')
        pg.wait_for_timeout(700)
        check('新会话横幅出现', pg.locator('.ongoing .orow').count() == 1)
        pg.click('.chip:has-text("全部主题")')          # 再点同一练习入口
        pg.wait_for_timeout(1100)
        pos = pg.locator('.pos').text_content() or ''
        check('重复入口进入续做(非新开)', pg.locator('.qcard').count() == 1 and pos.strip().startswith('1 / 6') and '已答 1' in pos)
        pg.click('button:has-text("退出")')
        pg.wait_for_timeout(700)
        check('仍只有一个进行中会话', pg.locator('.ongoing .orow').count() == 1)

        # 10 放弃路径：横幅上放弃→消失且刷新后不再出现
        pg.locator('.ongoing .orow button:has-text("放弃")').first.click()
        pg.wait_for_timeout(600)
        check('放弃后横幅消失', pg.locator('.ongoing .orow').count() == 0)
        pg.reload(timeout=30000)
        pg.wait_for_timeout(1000)
        check('放弃持久化(刷新后无进行中)', pg.locator('.ongoing-row').count() == 0)
        b.close()
finally:
    subprocess.run(['taskkill', '/F', '/T', '/PID', str(proc.pid)], capture_output=True) if os.name == 'nt' else proc.terminate()
    time.sleep(1)
print('JS错误:', errs if errs else '(无)')
print('失败项:', fails if fails else '无 —— 全部通过 🎉')
sys.exit(1 if fails or errs else 0)
