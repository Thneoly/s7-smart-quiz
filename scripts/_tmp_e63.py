# -*- coding: utf-8 -*-
"""E63 原始字形核验 + 手册 S/R 同时接通原文"""
import io, sys, re
sys.stdout = io.TextIOWrapper(sys.stdout.buffer, encoding='utf-8')

h = open(r'D:/PLC/s7-smart-quiz-data/考试模拟卷/官方答卷HTML/S7-200 SMART_练习题E.html', encoding='utf-8', errors='ignore').read()
for blk in re.split(r'(?=<div class="data__items")', h)[1:]:
    m = re.search(r'topic="(\d+)"', blk)
    if m and int(m.group(1)) == 63 and 'judge_ques_right' in blk:
        for g, txt in re.findall(r'class="[^"]*comIcon[^"]*"[^>]*>(.)</i>\s*<span>([^<]*)</span>', blk):
            print(f'  U+{ord(g):04X} {"选中" if ord(g) in (0xE6DF, 0xE6E1) else "未选"} → {txt[:30]}')
        break

man = open(r'D:/PLC/s7-smart-quiz-data/题库资料/系统手册v28.txt', encoding='utf-8', errors='ignore').read()
print('\n--- 手册 S/R 同时相关原文 ---')
for pat in ['同时置位和复位', '置位和复位.{0,12}同时', '同时.{0,6}复位', '最后.{0,8}(指令|扫描)']:
    for mm in list(re.finditer(pat, man))[:2]:
        print(' ·', man[max(0, mm.start()-40):mm.end()+60].replace('\n', '⏎')[:150])
