# -*- coding: utf-8 -*-
"""抓取问卷星 S7-200 SMART 模拟卷（A~E），生成 Markdown + 本地图片"""
import sys, io, os, re, hashlib, urllib.request
sys.stdout = io.TextIOWrapper(sys.stdout.buffer, encoding='utf-8')
from playwright.sync_api import sync_playwright

PAPERS = [
    ('A卷', 'https://ks.wjx.com/vm/wEOMMqS.aspx'),
    ('B卷', 'https://ks.wjx.com/vm/m33CsRR.aspx'),
    ('C卷', 'https://ks.wjx.com/vm/wEagwsM.aspx'),
    ('D卷', 'https://ks.wjx.com/vm/wiVhMam.aspx'),
    ('E卷', 'https://ks.wjx.com/vm/ex0hKp3.aspx'),
]

OUT_DIR = os.path.join(os.path.dirname(os.path.abspath(__file__)), '考试模拟卷')
IMG_DIR = os.path.join(OUT_DIR, 'images')
os.makedirs(IMG_DIR, exist_ok=True)

EXTRACT_JS = '''() => {
  const md = (el) => {
    let out = '';
    el.childNodes.forEach(n => {
      if (n.nodeType === 3) out += n.textContent;
      else if (n.nodeName === 'IMG') out += `\\n![图](${n.getAttribute('src')||''})\\n`;
      else if (n.nodeName === 'BR') out += '\\n';
      else if (n.nodeName === 'SCRIPT' || n.nodeName === 'STYLE') {}
      else out += md(n);
    });
    return out;
  };
  const clean = (s) => s.replace(/[ \\t]+/g, ' ').replace(/\\n{3,}/g, '\\n\\n').trim();
  const title = document.querySelector('h1')?.innerText?.trim() || '';
  const fields = [];
  document.querySelectorAll('div.field').forEach(f => {
    const type = f.getAttribute('type') || '';
    const topic = f.getAttribute('topic') || '';
    const num = f.querySelector('.topicnumber')?.innerText?.replace(/\\s*/g,'') ||
                (f.querySelector('.field-label')?.innerText?.match(/^\\s*\\d+/)?.[0] || '');
    const stemEl = f.querySelector('.topichtml');
    const stem = stemEl ? clean(md(stemEl)) : '';
    const opts = [...f.querySelectorAll('.ui-radio > .label, .ui-checkbox > .label')].map(l => clean(md(l)));
    fields.push({type, topic, num, stem, opts});
  });
  return {title, fields};
}'''

UA = {'User-Agent': 'Mozilla/5.0 (Windows NT 10.0; Win64; x64) Chrome/120.0'}
_img_cache = {}

def localize(md_text, paper, topic):
    """把 markdown 里的远程图片下载到本地并改写链接"""
    def repl(m):
        url = m.group(1)
        if url.startswith('//'): url = 'https:' + url
        if not url.startswith('http'): return m.group(0)
        if url in _img_cache: local = _img_cache[url]
        else:
            ext = os.path.splitext(url.split('?')[0])[1].lower()
            if ext not in ('.png', '.jpg', '.jpeg', '.gif', '.bmp', '.webp'): ext = '.png'
            name = f'{paper}-{topic}-{hashlib.md5(url.encode()).hexdigest()[:6]}{ext}'
            local = os.path.join(IMG_DIR, name)
            try:
                req = urllib.request.Request(url, headers=UA)
                with urllib.request.urlopen(req, timeout=30) as r, open(local, 'wb') as fw:
                    fw.write(r.read())
            except Exception as e:
                print(f'  ! 图片下载失败 {url}: {e}')
                local = None
            _img_cache[url] = local
        return f'![图](images/{os.path.basename(local)})' if local else f'![图]({url})'
    return re.sub(r'!\[图\]\(([^)]+)\)', repl, md_text)

def run():
    with sync_playwright() as p:
        browser = p.chromium.launch(channel='msedge', headless=True)
        page = browser.new_page()
        for name, url in PAPERS:
            print(f'== {name} {url}')
            page.goto(url, wait_until='domcontentloaded', timeout=60000)
            page.wait_for_selector('div.field', timeout=20000)
            page.wait_for_timeout(2500)
            # 滚动到底部，确保懒加载图片全部出现
            page.evaluate('async () => { for(let i=0;i<40;i++){ window.scrollBy(0,1500); await new Promise(r=>setTimeout(r,150)); } }')
            data = page.evaluate(EXTRACT_JS)
            n_single = sum(1 for f in data['fields'] if f['type'] == '3')
            n_multi = sum(1 for f in data['fields'] if f['type'] == '4')
            n_fill = sum(1 for f in data['fields'] if f['type'] not in ('3', '4'))
            print(f"  标题: {data['title']}  共{len(data['fields'])}项: 单选{n_single} 多选{n_multi} 其他{n_fill}")

            lines = [f"# {data['title']}（{name}）", '',
                     f'> 来源：{url}', f'> 整理日期：2026-08-22',
                     f'> 题量：单选 {n_single} 题 / 多选 {n_multi} 题' + (f' / 填写项 {n_fill} 个' if n_fill else ''), '']
            qidx = 0
            for f in data['fields']:
                qidx += 1
                tag = '【多选】' if f['type'] == '4' else ('【填写】' if f['type'] != '3' else '')
                stem = localize(f['stem'], name, f['topic'] or f['num'])
                lines.append(f'{qidx}. {stem}{tag}')
                if f['opts']:
                    for o in f['opts']:
                        o = localize(o, name, f['topic'] or f['num'])
                        lines.append(f'   - {o}')
                lines.append('')
            out = os.path.join(OUT_DIR, f'{name}.md')
            with open(out, 'w', encoding='utf-8') as fw:
                fw.write('\n'.join(lines))
            print(f'  已写入 {out}')
        browser.close()

if __name__ == '__main__':
    run()
