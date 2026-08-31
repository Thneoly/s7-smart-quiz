# -*- coding: utf-8 -*-
"""便携包隔离验证（v5）：解压→藏源码回退→运行→CDP断言（含逐题回顾原题号抽查）"""
import io, sys, os, json, zipfile, subprocess, time, socket, shutil
sys.stdout = io.TextIOWrapper(sys.stdout.buffer, encoding='utf-8')

ZIP = r'D:/PLC/s7-200/smart-quiz-app/src-tauri/target/release/bundle/portable/smart-quiz-app_0.1.0_x64-portable.zip'
TDIR = os.path.join(os.environ['LOCALAPPDATA'], 'Temp', 'portable_test2')
RES = r'D:/PLC/s7-200/smart-quiz-app/src-tauri/resources'
HIDE = RES + '.devhide'

shutil.rmtree(TDIR, ignore_errors=True)
os.makedirs(TDIR)
z = zipfile.ZipFile(ZIP)
z.extractall(TDIR)
seed = os.path.join(TDIR, 'resources', 'seed', 'smart-core.smartbank')
inner = json.loads(zipfile.ZipFile(seed).read('manifest.json'))
v = inner['bank']['version']
assert v == 5, f'zip 内 seed 版本 {v} != 5'
print('✓ zip 布局:', z.namelist(), '| seed v5 | 便携包内答案抽验 B010=',
      next(q['answer'] for q in inner['questions'] if q['qid'] == 'SC-B010'))

assert not os.path.exists(HIDE)
os.rename(RES, HIDE)
print('已藏源码 resources（杀编译回退）')
env = dict(os.environ, WEBVIEW2_ADDITIONAL_BROWSER_ARGUMENTS='--remote-debugging-port=9334')
try:
    proc = subprocess.Popen([os.path.join(TDIR, 'smart-quiz-app.exe')], env=env, cwd=TDIR)
    for _ in range(60):
        try:
            socket.create_connection(('127.0.0.1', 9334), 0.5).close(); break
        except OSError:
            time.sleep(1)
    from playwright.sync_api import sync_playwright
    with sync_playwright() as p:
        b = p.chromium.connect_over_cdp('http://127.0.0.1:9334')
        pg = b.contexts[0].pages[0]
        pg.wait_for_timeout(2500)
        if pg.locator('.eula').count():
            pg.click('button:has-text("同意并开始使用")'); pg.wait_for_timeout(500)
        # 快速做一套 3 题练习并交卷，验证结果页原题号
        pg.click('.side button:has-text("练习")'); pg.wait_for_timeout(800)
        pg.click('.chip:has-text("随机练习")'); pg.wait_for_timeout(1000)
        for _ in range(3):
            if pg.locator('.qcard').count():
                break
            pg.wait_for_timeout(500)
        # 答完 6 题交卷（错题必然存在 → 验证错题在前+原题号）
        for i in range(6):
            if pg.locator('button:has-text("完成练习")').count(): break
            pg.locator('.opt').first.click()   # 全选 A：mock 答案多为 B → 制造错题
            pg.wait_for_timeout(300)
            pg.click('button:has-text("下一题")'); pg.wait_for_timeout(250)
        pg.click('button:has-text("完成练习")'); pg.wait_for_timeout(1200)
        nums = pg.evaluate('''() => [...document.querySelectorAll('.qreview .rvhead span:first-child')].map(e => parseInt(e.textContent))''')
        ok_perm = sorted(nums) == list(range(1, len(nums) + 1))
        bad_first = pg.evaluate('''() => { const c = [...document.querySelectorAll('.qreview')]
            const n = c.filter(x => x.classList.contains('bad')).length
            return !n || c.slice(0, n).every(x => x.classList.contains('bad')) }''')
        print(f'✓ 便携版运行正常 | 回顾题号集合 1..{len(nums)} 排列: {ok_perm} | 错题在前: {bad_first}')
        assert ok_perm and bad_first
        b.close()
    print('✅ 隔离验证全部通过')
finally:
    subprocess.run(['taskkill', '/F', '/T', '/PID', str(proc.pid)], capture_output=True)
    time.sleep(1)
    os.rename(HIDE, RES)
    print('已恢复源码 resources')
