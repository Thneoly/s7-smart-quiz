# -*- coding: utf-8 -*-
"""文本对齐重校准：官方选中项按【选项文本】映射回 papers_raw 字母（免疫选项乱序）
输出：当前库答案 vs 文本对齐后的官方答案 的差异清单"""
import io, sys, os, re, json
sys.stdout = io.TextIOWrapper(sys.stdout.buffer, encoding='utf-8')

FILES = {'A': r'D:/PLC/s7-smart-quiz-data/考试模拟卷/官方答卷HTML/S7-200 SMART_练习题A.html',
         'B': r'D:/PLC/s7-smart-quiz-data/考试模拟卷/官方答卷HTML/S7-200SMART_练习题B.html',
         'C': r'D:/PLC/s7-smart-quiz-data/考试模拟卷/官方答卷HTML/S7-200 SMART_练习题C.html',
         'D': r'D:/PLC/s7-smart-quiz-data/考试模拟卷/官方答卷HTML/S7-200 SMART_练习题D.html',
         'E': r'D:/PLC/s7-smart-quiz-data/考试模拟卷/官方答卷HTML/S7-200 SMART_练习题E.html'}

def norm(t):
    t = re.sub(r'^[A-H][、.．,，]\s*', '', t.strip())
    return re.sub(r'[\s，。；;：:（）()\-]', '', t)

papers = json.load(open(r'D:/PLC/s7-smart-quiz-data/题库资料/papers_raw.json', encoding='utf-8'))
official = {}
unmatched = []
for L, path in FILES.items():
    h = open(path, encoding='utf-8', errors='ignore').read()
    raw = {it['n']: it for it in papers[f'{L}卷']['items']}
    for blk in re.split(r'(?=<div class="data__items")', h)[1:]:
        m = re.search(r'topic="(\d+)"', blk)
        if not m or 'judge_ques_right' not in blk or 'judge_ques_wrong' in blk:
            continue
        n = int(m.group(1))
        page_opts = [(g, txt) for g, txt in re.findall(
            r'class="[^"]*comIcon[^"]*"[^>]*>(.)</i>\s*<span>([^<]*)</span>', blk)]
        sel_txt = [norm(txt) for g, txt in page_opts if ord(g) in (0xE6DF, 0xE6E1)]
        it = raw.get(n)
        if not it or not sel_txt:
            continue
        raw_map = {norm(o): chr(65 + i) for i, o in enumerate(it['options'])}
        letters, miss = [], []
        for st in sel_txt:
            if st in raw_map:
                letters.append(raw_map[st])
            else:
                miss.append(st)
        ans = ''.join(sorted(set(letters)))
        if miss:
            unmatched.append((f'{L}{n}', miss[:2], [norm(o)[:12] for o in it['options']]))
        if ans:
            official[f'{L}{n:02d}'] = ans
        # 顺序差异检测：页面上带字母前缀的选项字母是否与 raw 一致
        pref = [(re.match(r'^([A-H])[、.．]', t.strip()), norm(t)) for _, t in page_opts]
        shuffled = any(pm and norm(it['options'][ord(pm.group(1))-65]) != nt
                       for pm, nt in pref if pm and ord(pm.group(1))-65 < len(it['options']))
        if shuffled:
            print(f'  ⚠ {L}{n} 选项顺序与papers_raw不同（字母映射不可信）')

json.dump(official, open(r'D:/PLC/s7-smart-quiz-data/题库资料/answers/_official_by_text.json', 'w', encoding='utf-8'), ensure_ascii=False, indent=1)
print(f'\n文本对齐提取 {len(official)} 题；文本对不上的 {len(unmatched)} 题')
for k, miss, raws in unmatched:
    print(f'  ✗ {k} 页面选中未见于raw: {miss} | raw选项: {raws}')

# 与当前库答案对比
print('\n=== 当前库 vs 文本对齐官方 ===')
cur = {}
for L in 'ABCDE':
    for line in open(rf'D:/PLC/s7-smart-quiz-data/题库资料/answers/{L}卷.jsonl', encoding='utf-8'):
        if line.strip():
            d = json.loads(line)
            cur[f'{L}{d["n"]:02d}'] = d.get('answer') or ''
diff = [(k, cur.get(k), a) for k, a in sorted(official.items()) if cur.get(k) != a]
print(f'不一致 {len(diff)} 题:')
for k, old, new in diff:
    print(f'  {k}: 库={old} → 文本对齐官方={new}')
