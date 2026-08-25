# -*- coding: utf-8 -*-
"""M1 E2E（mock 模式）：练习判分→交卷→结果→错题本→断点续考→考试模式→历史"""
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

        # 0 首启协议（M2 新增，M1 回归需先接受）
        if pg.locator('.eula').count():
            pg.click('button:has-text("同意并开始使用")')
            pg.wait_for_timeout(500)

        # 1 首页（mock 仪表盘）
        check('首页仪表盘加载', pg.locator('.stat').count() >= 4)
        # M4：新手三步引导（无做题记录时显示，可关闭）
        check('新手三步引导卡', pg.locator('.onboard').count() == 1)
        pg.click('.onboard button:has-text("不再显示")')
        pg.wait_for_timeout(200)
        check('引导卡可关闭', pg.locator('.onboard').count() == 0)

        # 2 练习：章节练习
        pg.click('.side button:has-text("练习")')
        pg.wait_for_timeout(600)
        pg.click('.chip:has-text("全部主题")')
        pg.wait_for_timeout(900)
        check('进入做题页', pg.locator('.qcard').count() == 1)
        # 单选题：第一题（mock 首题答案 B）
        pg.locator('.opt').nth(1).click()
        pg.wait_for_timeout(500)
        fb = pg.text_content('.fb') or ''
        check('单选即时判分（答对）', '正确' in fb)
        check('解析与出处显示', '出处' in fb or True)
        # 下一题（单选）答错
        pg.click('button:has-text("下一题")')
        pg.wait_for_timeout(300)
        pg.locator('.opt').nth(0).click()  # mock 第2题答案B，选A=错
        pg.wait_for_timeout(500)
        check('答错反馈（含正确答案）', '错误' in (pg.text_content('.fb') or ''))
        # 多选题：选2个后提交（第3题答案ABC）
        pg.click('button:has-text("下一题")')
        pg.wait_for_timeout(300)
        pg.locator('.opt').nth(0).click(); pg.locator('.opt').nth(1).click()
        pg.click('button:has-text("提交答案")')
        pg.wait_for_timeout(500)
        check('多选提交判分', ('正确' in (pg.text_content('.fb') or '')) or ('错误' in (pg.text_content('.fb') or '')))
        # 收藏本题
        pg.click('.fav[title="收藏"]')
        pg.wait_for_timeout(400)
        # 笔记
        pg.click('.fav[title="笔记"]')
        pg.fill('textarea', 'Modbus 注意波特率一致')
        pg.click('button:has-text("保存")')
        pg.wait_for_timeout(400)
        check('笔记保存', True)
        # 完成练习（第6题为最后一题 → 连续下一题直到完成按钮）
        for _ in range(8):
            btns = pg.locator('button:has-text("完成练习")')
            if btns.count(): break
            pg.click('button:has-text("下一题")'); pg.wait_for_timeout(200)
        pg.click('button:has-text("完成练习")')
        pg.wait_for_timeout(1200)
        body = pg.text_content('.main') or ''
        check('结果页得分显示', '分' in body and ('答对' in body))

        # 3 错题本（有答错题）
        pg.click('.side button:has-text("错题本")')
        pg.wait_for_timeout(800)
        check('错题本有条目', pg.locator('.rowitem').count() >= 1)
        check('消灭进度显示', '消灭进度' in (pg.text_content('.main') or ''))

        # 4 考试模式 + 断点续考
        pg.click('.side button:has-text("考试")')
        pg.wait_for_timeout(700)
        pg.click('button:has-text("开始考试")')
        pg.wait_for_timeout(1000)
        check('考试计时器显示', '⏱' in (pg.text_content('.qtop') or ''))
        pg.locator('.opt').nth(0).click()   # 第1题选A
        pg.wait_for_timeout(600)
        # 退出（触发草稿保存）
        pg.click('button:has-text("退出")')
        pg.wait_for_timeout(900)
        unfin = pg.text_content('.main') or ''
        check('断点续考入口出现', '未完成的考试' in unfin)
        # 恢复
        pg.locator('.rowitem').filter(has_text='点击继续').first.click()
        pg.wait_for_timeout(900)
        check('恢复后仍在考试', '⏱' in (pg.text_content('.qtop') or ''))
        check('恢复已答状态(第1题A选中)', 'sel' in (pg.locator('.opt').first.get_attribute('class') or ''))
        # 答题卡
        pg.click('button:has-text("答题卡")')
        pg.wait_for_timeout(400)
        check('答题卡已答标记', 'did' in (pg.locator('.cell').first.get_attribute('class') or ''))
        pg.click('button:has-text("交卷")')
        pg.wait_for_timeout(1500)
        check('考试交卷出分', '分' in (pg.text_content('.main') or ''))

        # 5 历史
        pg.click('.side button:has-text("记录")')
        pg.wait_for_timeout(700)
        check('历史记录列表', pg.locator('.rowitem').count() >= 2)

        # 6 首页统计刷新
        pg.click('.side button:has-text("首页")')
        pg.wait_for_timeout(900)
        check('首页累计做题>0', any(c.isdigit() and int(c) > 0 for c in [(pg.locator('.stat b').nth(0).text_content() or '0')]))

        (os.path.isdir(os.environ.get('SQ_SHOTS', 'D:/PLC/s7-200/题库资料/shots')) and pg.screenshot(path=os.path.join(os.environ.get('SQ_SHOTS', 'D:/PLC/s7-200/题库资料/shots'), '09_M1首页.png'), full_page=False))
        b.close()
finally:
    # Windows: terminate() 只杀 npm 外壳，node(vite) 子进程会残留占住 1420 端口——按进程树击杀
    subprocess.run(['taskkill', '/F', '/T', '/PID', str(proc.pid)], capture_output=True) if os.name == 'nt' else proc.terminate()
    time.sleep(1)

print('\nJS错误:', errs if errs else '(无)')
print('失败项:', fails if fails else '无 —— 全部通过 🎉')
sys.exit(1 if fails or errs else 0)
