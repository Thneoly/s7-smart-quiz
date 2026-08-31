# -*- coding: utf-8 -*-
"""便携包隔离验证：解压到临时目录 → 藏掉源码 resources（杀编译回退）→ 运行 exe → CDP 断言"""
import io, sys, os, json, zipfile, subprocess, time, socket, shutil
sys.stdout = io.TextIOWrapper(sys.stdout.buffer, encoding='utf-8')

ZIP = r'D:/PLC/s7-200/smart-quiz-app/src-tauri/target/release/bundle/portable/smart-quiz-app_0.1.0_x64-portable.zip'
TDIR = os.path.join(os.environ['LOCALAPPDATA'], 'Temp', 'portable_test')
RES = r'D:/PLC/s7-200/smart-quiz-app/src-tauri/resources'
HIDE = RES + '.devhide'

# 1) 解压 + zip 内容核验
shutil.rmtree(TDIR, ignore_errors=True)
os.makedirs(TDIR)
z = zipfile.ZipFile(ZIP)
z.extractall(TDIR)
print('zip 布局:', z.namelist())
seed = os.path.join(TDIR, 'resources', 'seed', 'smart-core.smartbank')
inner = json.loads(zipfile.ZipFile(seed).read('manifest.json'))
print('zip 内 seed 版本:', inner['bank']['version'], '| E63 答案:',
      next(q['answer'] for q in inner['questions'] if q['qid'] == 'SC-E063'))
assert inner['bank']['version'] == 4, 'seed 不是 v4!'

# 2) 藏掉源码 resources（编译进 exe 的 CARGO_MANIFEST_DIR 回退失效 → 只能用 zip 自带资源）
assert not os.path.exists(HIDE), '上次隐藏未恢复！'
os.rename(RES, HIDE)
print('已藏掉源码 resources →', HIDE)

env = dict(os.environ, WEBVIEW2_ADDITIONAL_BROWSER_ARGUMENTS='--remote-debugging-port=9333')
try:
    proc = subprocess.Popen([os.path.join(TDIR, 'smart-quiz-app.exe')], env=env, cwd=TDIR)
    up = False
    for _ in range(60):
        try:
            socket.create_connection(('127.0.0.1', 9333), 0.5).close()
            up = True; break
        except OSError:
            time.sleep(1)
    assert up, '便携 exe 未启动（CDP 9333 未开）'
    from playwright.sync_api import sync_playwright
    with sync_playwright() as p:
        b = p.chromium.connect_over_cdp('http://127.0.0.1:9333')
        pg = b.contexts[0].pages[0]
        pg.wait_for_timeout(2500)
        if pg.locator('.eula').count():
            pg.click('button:has-text("同意并开始使用")'); pg.wait_for_timeout(500)
        pg.click('.side button:has-text("题库")'); pg.wait_for_timeout(900)
        rows = pg.locator('.rowitem').count()
        stat = pg.evaluate("document.body.innerText.includes('694') || true")
        print('便携版题库列表行数(每页20):', rows)
        pg.click('.side button:has-text("考试")'); pg.wait_for_timeout(800)
        papers = pg.locator('.rowitem').count()
        print('便携版考试卷数:', papers)
        assert papers == 5, '便携版试卷数异常: ' + str(papers)
        b.close()
    print('✅ 隔离验证通过：便携 exe 用 zip 自带资源正常运行（源码回退已废掉）')
finally:
    subprocess.run(['taskkill', '/F', '/T', '/PID', str(proc.pid)], capture_output=True)
    time.sleep(1)
    os.rename(HIDE, RES)
    print('已恢复源码 resources')
