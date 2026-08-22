# -*- coding: utf-8 -*-
"""渲染图标设计稿：预览拼图 + 各方案独立1024png（含小尺寸效果）"""
import sys, io, os, asyncio
sys.stdout = io.TextIOWrapper(sys.stdout.buffer, encoding='utf-8')
from playwright.async_api import async_playwright

BASE = r'D:/PLC/s7-200/smart-quiz-app/icons-design'
HTML = f'file:///{BASE}/variants.html'

async def main():
    async with async_playwright() as p:
        b = await p.chromium.launch(channel='msedge', headless=True)
        pg = await b.new_page(viewport={'width': 540, 'height': 600}, device_scale_factor=2)
        await pg.goto(HTML)
        await pg.wait_for_timeout(400)
        # 预览拼图
        await pg.screenshot(path=f'{BASE}/preview.png', full_page=True)
        # 各方案独立渲染（含 32px 小图效果对比）
        for i, key in enumerate(['A', 'B', 'C', 'D']):
            svg = await pg.locator(f'.cell:nth-child({i+1}) svg').inner_html()
            page2 = await b.new_page(viewport={'width': 1024, 'height': 1024})
            await page2.set_content(f'<body style="margin:0"><svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 512 512" width="1024" height="1024">{svg}</svg></body>')
            await page2.wait_for_timeout(150)
            await page2.screenshot(path=f'{BASE}/icon_{key}.png', omit_background=True)
            # 小尺寸效果（模拟任务栏 32px）
            await page2.set_viewport_size({'width': 64, 'height': 64})
            await page2.wait_for_timeout(100)
            await page2.screenshot(path=f'{BASE}/small_{key}.png')
            await page2.close()
        await b.close()
    print('预览:', f'{BASE}/preview.png')
    for f in sorted(os.listdir(BASE)):
        print(' ', f, os.path.getsize(os.path.join(BASE, f)) // 1024, 'KB')

asyncio.run(main())
