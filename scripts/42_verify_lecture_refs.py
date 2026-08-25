# -*- coding: utf-8 -*-
"""校验讲义出处真实性：lectures.json 每节 ref 中的 .txt 文件名须存在于数据仓语料"""
import json, os, re, sys, io
sys.stdout = io.TextIOWrapper(sys.stdout.buffer, encoding='utf-8')
from _data import data

L = json.load(open('../smart-quiz-app/src/study/lectures.json', encoding='utf-8'))['lectures']
r1 = L[0]['sections'][0]['ref']
print('第1章第1节 ref repr:', repr(r1[:60]))
CTRL = chr(1)
print('含\\x01控制字符的ref数:', sum(1 for l in L for s in l['sections'] if CTRL in s['ref']))

names = set()
for root, _, fs in os.walk(data('题库资料')):
    for f in fs:
        names.add(f)

hit = miss = 0
missed = []
for l in L:
    for s in l['sections']:
        segs = re.split(r'[\\/]', s['ref'])
        cand = [x for x in segs if '.txt' in x]
        ok = any(c.split('·')[0].strip() in names for c in cand)
        if ok:
            hit += 1
        else:
            miss += 1
            missed.append(f"第{l['no']}章: {s['ref'][:70]}")
print(f'出处核对（{hit + miss}条）：真实文件命中 {hit}，未命中 {miss}')
for x in missed[:8]:
    print('  x', x)
