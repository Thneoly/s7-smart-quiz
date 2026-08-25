# -*- coding: utf-8 -*-
"""讲义内容质量扫描：句子截断、异常字符、空段落、长度异常"""
import json, re, sys, io
sys.stdout = io.TextIOWrapper(sys.stdout.buffer, encoding='utf-8')

L = json.load(open('../smart-quiz-app/src/study/lectures.json', encoding='utf-8'))['lectures']
issues = {'截断': [], '乱码': [], '过短': [], '空': []}
BAD_TAIL = '，、；：的是和与在为（'  # 句中不该结尾的字符（"的是和与在为"多为截断征兆）
GARBAGE = re.compile(r'[\[\]{}"]{2,}|\\u[0-9a-f]{4}|\.\.\.\]\]')
for l in L:
    for si, s in enumerate(l['sections']):
        for pi, p in enumerate(s['paras']):
            where = f"第{l['no']}章[{s['h'][:12]}]段{pi + 1}"
            if not p.strip():
                issues['空'].append(where)
                continue
            if len(p.strip()) < 25:
                issues['过短'].append(f"{where}({len(p)}字): {p[:30]}")
            if p.rstrip().endswith(tuple(BAD_TAIL)):
                issues['截断'].append(f"{where}: …{p[-35:]}")
            if GARBAGE.search(p):
                issues['乱码'].append(f"{where}: {GARBAGE.search(p).group()[:20]} in …{p[GARBAGE.search(p).start():GARBAGE.search(p).start() + 40]}")
for k, v in issues.items():
    print(f"== {k}: {len(v)} 处")
    for x in v[:12]:
        print('  ', x)
