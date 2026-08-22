# -*- coding: utf-8 -*-
"""把 考试模拟卷/A~E卷.md 解析为结构化 JSON（答案留空，供平台与补答案用）"""
import sys, io, os, re, json
sys.stdout = io.TextIOWrapper(sys.stdout.buffer, encoding='utf-8')

BASE = os.path.dirname(os.path.abspath(__file__))
SRC = os.path.join(BASE, '考试模拟卷')
OUT = os.path.join(BASE, '题库资料', 'papers_raw.json')

def parse(md):
    lines = md.split('\n')
    title = ''
    m = re.match(r'# (.+)', lines[0] or '')
    if m: title = m.group(1).strip()
    url = ''
    for l in lines[:8]:
        m = re.match(r'> 来源：(\S+)', l.strip())
        if m: url = m.group(1)
    items, cur = [], None
    def flush():
        nonlocal cur
        if cur:
            cur['q'] = re.sub(r'【多选题?】|【填写】', '', cur['q']).strip()
            cur['q'] = re.sub(r'\s+', ' ', cur['q']).strip()
            items.append(cur); cur = None
    for l in lines:
        m = re.match(r'^(\d+)\.\s+(.*)$', l)
        if m:
            flush()
            cur = {'n': int(m.group(1)), 'q': m.group(2), 'img': None, 'options': [],
                   'type': 'fill' if '【填写】' in m.group(2) else ('multi' if ('【多选' in m.group(2) or '【多选题】' in m.group(2)) else 'single')}
            continue
        if cur is None: continue
        s = l.strip()
        if not s: continue
        m = re.match(r'^-\s+(.+)$', s)
        if m:
            cur['options'].append(m.group(1).strip()); continue
        m = re.match(r'!\[图\]\((.+)\)', s)
        if m: cur['img'] = m.group(1); continue
        # 题干续行（含内嵌图片）
        m = re.search(r'!\[图\]\(([^)]+)\)', s)
        if m and not cur['img']: cur['img'] = m.group(1)
        cur['q'] += ' ' + re.sub(r'!\[图\]\([^)]+\)', '', s).strip()
    flush()
    # 统一选项前缀：去掉原有 A、/A./A． 前缀后重新编号
    for it in items:
        it['options'] = [re.sub(r'^[A-H][、.．,，]\s*', '', o) for o in it['options']]
        it['options'] = [f'{chr(65+i)}、{o}' for i, o in enumerate(it['options'])]
    return title, url, items

papers = {}
for fn in sorted(os.listdir(SRC)):
    m = re.match(r'^([A-E])卷\.md$', fn)
    if not m: continue
    md = open(os.path.join(SRC, fn), encoding='utf-8').read()
    title, url, items = parse(md)
    papers[f'{m.group(1)}卷'] = {'file': fn, 'title': title, 'url': url, 'items': items}
    n_single = sum(1 for i in items if i['type'] == 'single')
    n_multi = sum(1 for i in items if i['type'] == 'multi')
    n_fill = sum(1 for i in items if i['type'] == 'fill')
    n_img = sum(1 for i in items if i['img'])
    no_opt = sum(1 for i in items if i['type'] != 'fill' and not i['options'])
    print(f"{m.group(1)}卷: {len(items)}题 单选{n_single} 多选{n_multi} 填写{n_fill} 带图{n_img} 选择题缺选项{no_opt}")
json.dump(papers, open(OUT, 'w', encoding='utf-8'), ensure_ascii=False, indent=1)
print('已写入', OUT)
