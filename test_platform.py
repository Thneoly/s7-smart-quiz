# -*- coding: utf-8 -*-
"""练习平台冒烟测试：加载、交互、截图"""
import sys, io
sys.stdout = io.TextIOWrapper(sys.stdout.buffer, encoding='utf-8')
from playwright.sync_api import sync_playwright

URL = 'file:///D:/PLC/s7-200/练习平台/index.html'
errors = []

def run():
    with sync_playwright() as p:
        b = p.chromium.launch(channel='msedge', headless=True)
        pg = b.new_page(viewport={'width': 900, 'height': 1000})
        pg.on('console', lambda m: errors.append(m.text) if m.type == 'error' else None)
        pg.on('pageerror', lambda e: errors.append(str(e)))
        pg.on('dialog', lambda d: d.accept())
        pg.goto(URL)
        pg.wait_for_timeout(800)
        # 1. 首页
        n_bank = pg.evaluate('BANK.length')
        n_papers = pg.evaluate('PAPERS.length')
        print(f'首页加载: 题库{n_bank}题, {n_papers}套卷')
        pg.screenshot(path='D:/PLC/s7-200/题库资料/shots/01_首页.png', full_page=True)
        # 2. 分类练习：点第一个主题chip（全部顺序）
        pg.click("button.chip:has-text('全部顺序')")
        pg.wait_for_timeout(400)
        pg.click('.opts .opt:nth-child(1)')  # 选A，单选立即判分
        pg.wait_for_timeout(400)
        fb = pg.text_content('#fb') or ''
        print('练习判分反馈出现:', ('正确' in fb) or ('错误' in fb) or ('暂未收录' in fb))
        pg.screenshot(path='D:/PLC/s7-200/题库资料/shots/02_练习判分.png')
        # 3. 多选题流程：跳到题库第一个多选
        idx = pg.evaluate("S.session.items.findIndex(x=>x.type==='multi')")
        pg.evaluate(f'S.session.idx={idx}')
        pg.evaluate("renderQuiz()")
        pg.wait_for_timeout(200)
        pg.click('.opts .opt:nth-child(1)')
        pg.click('.opts .opt:nth-child(2)')
        pg.click("button:has-text('提交答案')")
        pg.wait_for_timeout(300)
        print('多选提交:', ('正确答案' in (pg.text_content('#fb') or '')) or ('正确' in (pg.text_content('#fb') or '')))
        # 4. 真题模式 A卷 -> 答题卡 -> 交卷
        pg.evaluate("go('home')")
        pg.click("button:has-text('开始')")
        pg.wait_for_timeout(300)
        pg.click('.opts .opt:nth-child(2)')
        pg.evaluate('nav(1)')
        pg.click('.opts .opt:nth-child(1)')
        pg.click("button:has-text('答题卡')")
        pg.wait_for_timeout(200)
        pg.screenshot(path='D:/PLC/s7-200/题库资料/shots/03_答题卡.png')
        pg.evaluate("document.querySelector('.mask .cell:nth-child(5)').click()")
        pg.wait_for_timeout(200)
        pg.evaluate("finishExam()")
        pg.wait_for_timeout(400)
        body = pg.text_content('#app') or ''
        print('交卷结果页:', ('分' in body and ('答对' in body or '答错' in body)))
        pg.screenshot(path='D:/PLC/s7-200/题库资料/shots/04_交卷结果.png')
        # 5. 图片题：A卷里找带img的题
        has_img = pg.evaluate("PAPERS.some(p=>p.items.some(i=>i.img))")
        print('平台含图片题数据:', has_img)
        # 6. 错题本 & 搜索
        pg.evaluate("go('home')")
        pg.fill('#kw', 'Modbus')
        pg.click("button:has-text('搜索')")
        pg.wait_for_timeout(300)
        n_hit = pg.evaluate("($('#searchout')._hits||[]).length")
        print('搜索Modbus命中:', n_hit)
        pg.evaluate("go('wrong')")
        pg.wait_for_timeout(200)
        n_wrong = pg.evaluate("Object.keys(wrong).length")
        print('错题本记录:', n_wrong)
        pg.screenshot(path='D:/PLC/s7-200/题库资料/shots/05_错题本.png')
        # 7. localStorage 持久化：刷新后仍在
        pg.reload(); pg.wait_for_timeout(600)
        n_wrong2 = pg.evaluate("Object.keys(wrong).length")
        print('刷新后错题持久化:', n_wrong == n_wrong2)
        b.close()
    print('JS错误:', len(errors), errors[:5] if errors else '(无)')

if __name__ == '__main__':
    import os
    os.makedirs('D:/PLC/s7-200/题库资料/shots', exist_ok=True)
    run()
