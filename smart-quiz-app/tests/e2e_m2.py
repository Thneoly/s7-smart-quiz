# -*- coding: utf-8 -*-
"""M2 E2E（mock）：EULA→蓝图组卷→考试→结果导出→设置/主题→热力图"""
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

        # 0 首启协议
        check('首启协议弹窗', pg.locator('.eula').count() == 1)
        pg.click('button:has-text("同意并开始使用")')
        pg.wait_for_timeout(500)
        check('同意后进入首页', pg.locator('.stat').count() >= 4)

        # 1 蓝图组卷（mock：3单选3多选 → 组 2单+1多）
        pg.click('.side button:has-text("考试")')
        pg.wait_for_timeout(600)
        pg.click('button:has-text("蓝图组卷")')
        pg.wait_for_timeout(600)
        # 默认 40单+10多，候选不足：不允许降级 → 报错
        pg.click('button:has-text("组卷预览")')
        pg.wait_for_timeout(500)
        check('候选不足报错提示', '候选不足' in (pg.text_content('.main') or ''))
        # 开启降级 → 成功
        pg.check('input[type=checkbox]')  # 第一个checkbox=允许降级
        pg.click('button:has-text("组卷预览")')
        pg.wait_for_timeout(700)
        body = pg.text_content('.main') or ''
        check('降级组卷成功+报告', '组卷结果' in body and '降级' in body)
        pg.click('button:has-text("开始考试")')
        pg.wait_for_timeout(1000)
        check('进入组卷考试', pg.locator('.qcard').count() == 1)
        # 快速答完交卷
        for i in range(6):
            if pg.locator('.opt').count():
                pg.locator('.opt').first.click()
                pg.wait_for_timeout(150)
            nxt = pg.locator('button:has-text("下一题")')
            if nxt.count(): nxt.first.click(); pg.wait_for_timeout(150)
        fin = pg.locator('button:has-text("交卷并查看成绩")')
        if fin.count(): fin.click()
        else:
            pg.click('button:has-text("交卷")')
        pg.wait_for_timeout(1500)
        rbody = pg.text_content('.main') or ''
        check('交卷出结果', '分' in rbody and '答对' in rbody)

        # 2 结果页导出按钮
        check('导出Excel按钮', pg.locator('button:has-text("导出 Excel")').count() == 1)
        check('打印PDF按钮', pg.locator('button:has-text("打印")').count() == 1)
        pg.click('button:has-text("导出 Excel")')
        pg.wait_for_timeout(600)

        # 3 首页热力图
        pg.click('.side button:has-text("首页")')
        pg.wait_for_timeout(900)
        check('热力图渲染', pg.locator('.heatcell').count() >= 90)

        # 4 设置页 + 深色主题
        pg.click('.side button:has-text("设置")')
        pg.wait_for_timeout(500)
        check('设置页加载', '备份' in (pg.text_content('.main') or ''))
        pg.click('button:has-text("深色")')
        pg.wait_for_timeout(400)
        bg = pg.evaluate("getComputedStyle(document.body).backgroundColor")
        check('深色主题生效', bg == 'rgb(18, 21, 29)')
        check('诊断包按钮', pg.locator('button:has-text("导出诊断包")').count() == 1)
        pg.click('button:has-text("导出诊断包")')
        pg.wait_for_timeout(600)
        check('协议与隐私说明', '零遥测' in (pg.text_content('.main') or ''))

        pg.screenshot(path='D:/PLC/s7-200/题库资料/shots/10_M2设置深色.png')
        b.close()
finally:
    # Windows: terminate() 只杀 npm 外壳，node(vite) 子进程会残留占住 1420 端口——按进程树击杀
    subprocess.run(['taskkill', '/F', '/T', '/PID', str(proc.pid)], capture_output=True)
    time.sleep(1)

print('\nJS错误:', errs if errs else '(无)')
print('失败项:', fails if fails else '无 —— M2 全部通过 🎉')
sys.exit(1 if fails or errs else 0)
