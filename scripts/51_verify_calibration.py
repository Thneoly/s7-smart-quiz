# -*- coding: utf-8 -*-
"""终验：从归档官方答卷 HTML 重新提取答案，与用户当前运行库逐题比对"""
import io, sys, os, re, json, sqlite3
sys.stdout = io.TextIOWrapper(sys.stdout.buffer, encoding='utf-8')

HTML_DIR = r'D:/PLC/s7-smart-quiz-data/考试模拟卷/官方答卷HTML'
FILES = {
    'A卷': 'S7-200 SMART_练习题A.html', 'B卷': 'S7-200SMART_练习题B.html',
    'C卷': 'S7-200 SMART_练习题C.html', 'D卷': 'S7-200 SMART_练习题D.html',
    'E卷': 'S7-200 SMART_练习题E.html',
}

def extract(path):
    h = open(path, encoding='utf-8', errors='ignore').read()
    out = {}
    for blk in re.split(r'(?=<div class="data__items")', h)[1:]:
        tm = re.search(r'topic="(\d+)"', blk)
        if not tm or 'judge_ques_right' not in blk or 'judge_ques_wrong' in blk:
            continue
        opts = re.findall(r'class="[^"]*comIcon[^"]*"[^>]*>(.)</i>\s*<span>([^<]*)</span>', blk)
        sel = []
        for idx, (g, txt) in enumerate(opts):
            m = re.match(r'^([A-H])[、.．,，]', txt.strip())
            letter = m.group(1) if m else chr(65 + idx)
            if ord(g) in (0xE6DF, 0xE6E1):
                sel.append(letter)
        if sel:
            out[int(tm.group(1))] = ''.join(sorted(set(sel)))
    return out

db = sqlite3.connect(os.path.expandvars(r'%APPDATA%/com.smartquiz.app/bank.db'))
total = bad = 0
for name, fn in FILES.items():
    off = extract(os.path.join(HTML_DIR, fn))
    for n, a in sorted(off.items()):
        qid = 'SC-%s%03d' % (name[0], n)
        row = db.execute("SELECT answer FROM questions WHERE qid=?", (qid,)).fetchone()
        total += 1
        if not row:
            print(f'❌ {qid} 不在库中!'); bad += 1
        elif row[0] != a:
            print(f'❌ {qid} 库={row[0]} 官方HTML={a}'); bad += 1
print(f'\n比对 {total} 题 | 不一致 {bad} 题')
print('✅ 当前运行库与官方答卷 HTML 完全一致' if bad == 0 else '❌ 存在未校准项，见上')
db.close()
