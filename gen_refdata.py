# -*- coding: utf-8 -*-
"""从 journal 提取资料速查数据 → 按内容特征识别板块 → refdata.json"""
import sys, io, json, re
sys.stdout = io.TextIOWrapper(sys.stdout.buffer, encoding='utf-8')

JOURNAL = r'C:/Users/45110/.claude/projects/D--PLC-s7-200/49770f43-c379-4f00-bb8e-cef6365239f4/subagents/workflows/wf_b73c861d-9b3/journal.jsonl'
OUT = 'D:/PLC/s7-200/smart-quiz-app/src/study/refdata.json'

results = []
for line in open(JOURNAL, encoding='utf-8'):
    e = json.loads(line)
    if e.get('type') == 'result' and isinstance(e.get('result'), dict) and e['result'].get('items'):
        results.append(e['result']['items'])
# 补充：从代理 transcript 提取的结果（journal 可能丢），条目名全局去重
import os
TR = 'D:/PLC/s7-200/题库资料/refdata_transcripts.json'
if os.path.exists(TR):
    extra = json.load(open(TR, encoding='utf-8'))
    for k, items in extra.items():
        results.append(items)
    seen = set()
    deduped = []
    for r in results:
        rr = [i for i in r if not (i['name'] in seen or seen.add(i['name']))]
        if rr:
            deduped.append(rr)
    results = deduped
print('结果组数:', len(results))

def sig(items):
    txt = ' '.join(i['name'] + ' ' + i['category'] for i in items).lower()
    return txt

def classify(items):
    t = sig(items)
    if 'cpu cr' in t or 'cpu sr' in t or 'cpu st' in t or 'em ae04' in t: return 'hw'
    if any(k in t for k in ['故障', '密码', '存储卡', 'led', '看门狗', '诊断']): return 'fault'
    if any(k in t for k in ['modbus', 'profinet', 'ppi', 'tcp', 'udp', 'rs485', 'iso']): return 'comm'
    # 指令组特征：助记符（公式组一般不含这些裸助记符组合）
    if any(k in t for k in ['ton', 'ctud', 'ctu', 'mov_', 'shl', 'rol', 'trunc', 'round', '触点', '线圈']):
        return 'ins'
    if any(k in t for k in ['for/next', 'jmp', 'call', 'xmt', 'rcv', 'plc', '字符串指令', '程序控制']):
        return 'ins'
    if any(k in t for k in ['换算', '公式', 'sm0', '进制', '5530', '分辨率', '容量']): return 'formula'
    return '?'

sections = {'hw': [], 'ins': [], 'comm': [], 'fault': [], 'formula': []}
unknown = []
for items in results:
    k = classify(items)
    if k == '?':
        unknown.append([i['name'] for i in items][:3])
    else:
        sections[k].extend(items)

print('分布:', {k: len(v) for k, v in sections.items()})
if unknown: print('未识别组:', unknown)

# 校验字段结构
for k, arr in sections.items():
    for it in arr:
        assert it.get('name') and it.get('category') and isinstance(it.get('fields'), list) and it.get('source'), it

# 指令排序：按名称
sections['ins'].sort(key=lambda x: x['name'])
out = {'sections': [{'key': k, 'items': v} for k, v in sections.items() if v]}
json.dump(out, open(OUT, 'w', encoding='utf-8'), ensure_ascii=False, indent=1)
total = sum(len(v) for v in sections.values())
print(f'共 {total} 条 -> {OUT}')
