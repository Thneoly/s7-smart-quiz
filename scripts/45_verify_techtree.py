# -*- coding: utf-8 -*-
"""技能树数据机械校验：22章不重不漏 / 依赖边指向更低层(DAG) / 快速路径前置闭合 / 估时自洽"""
import json, sys, io
sys.stdout = io.TextIOWrapper(sys.stdout.buffer, encoding='utf-8')

T = json.load(open('../smart-quiz-app/src/study/techtree.json', encoding='utf-8'))
layers = T['layers']
nodes = {n['no']: n for l in layers for n in l['nodes']}
depth = {n['no']: i for i, l in enumerate(layers) for n in l['nodes']}
errs = []

# 1) 22 章不重不漏
nos = sorted(nodes)
if nos != list(range(1, 23)):
    errs.append(f'章节覆盖异常: {nos}')
# 2) 每层 2~5 节点
for i, l in enumerate(layers):
    if not 2 <= len(l['nodes']) <= 5:
        errs.append(f'第{i}层节点数 {len(l["nodes"])} 超出 2~5')
# 3) 依赖边指向更低层（DAG 无环由分层保证）且指向存在的章节
for n in nodes.values():
    for p in n['prereqs']:
        if p not in nodes:
            errs.append(f'第{n["no"]}章 前置 {p} 不存在')
        elif depth[p] >= depth[n['no']]:
            errs.append(f'第{n["no"]}章(层{depth[n["no"]]}) 前置 {p}(层{depth[p]}) 未指向更低层')
# 4) 快速路径前置闭合：路径内任一节点的前置要么在路径中且更早，要么已在路径里
qs = T['quickstart_path']
if len(qs) != len(set(qs)):
    errs.append('快速路径有重复')
for i, no in enumerate(qs):
    if no not in nodes:
        errs.append(f'快速路径含未知章节 {no}')
        continue
    for p in nodes[no]['prereqs']:
        if p not in qs:
            errs.append(f'快速路径第{no}章 前置 {p} 不在路径中（断链）')
        elif qs.index(p) > i:
            errs.append(f'快速路径顺序问题：第{no}章 在前置 {p} 之前出现')
# 5) 估时自洽
qs_h = sum(nodes[no]['est_h'] for no in qs if no in nodes)
total_h = sum(n['est_h'] for n in nodes.values())
if qs_h > 20:
    errs.append(f'快速路径估时 {qs_h}h 超 20h')
if not 50 <= total_h <= 70:
    errs.append(f'总估时 {total_h}h 偏离课程量级(50~70)')
core_n = sum(1 for n in nodes.values() if n['core'])
if core_n > 12:
    errs.append(f'核心节点 {core_n} 个超上限 12')

print(f'节点 {len(nodes)}/22 · 层 {len(layers)} · 核心主干 {core_n} · 快速路径 {len(qs)} 章 {qs_h}h · 总 {total_h}h')
if errs:
    print('❌ 校验失败:')
    for e in errs:
        print('  -', e)
    sys.exit(1)
print('✅ 技能树数据校验全部通过')
