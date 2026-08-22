# -*- coding: utf-8 -*-
"""从 resume 工作流输出（带key）组装 refdata.json；缺失板块回退 journal/transcripts"""
import sys, io, json, os
sys.stdout = io.TextIOWrapper(sys.stdout.buffer, encoding='utf-8')

OUTFILE = r'C:/Users/45110/AppData/Local/Temp/claude/D--PLC-s7-200/49770f43-c379-4f00-bb8e-cef6365239f4/tasks/ws0c033w8.output'
JOURNAL = r'C:/Users/45110/.claude/projects/D--PLC-s7-200/49770f43-c379-4f00-bb8e-cef6365239f4/subagents/workflows/wf_b73c861d-9b3/journal.jsonl'
from _data import data
TR = data('题库资料', 'refdata_transcripts.json')
DEST = 'D:/PLC/s7-200/smart-quiz-app/src/study/refdata.json'

KEY2SEC = {'硬件': 'hw', '指令': 'ins', '通信': 'comm', '故障': 'fault', '公式': 'formula'}
sections = {v: [] for v in KEY2SEC.values()}

def sig(items):
    return ' '.join(i['name'] + ' ' + i['category'] for i in items).lower()

def classify(items):
    t = sig(items)
    if 'cpu cr' in t or 'cpu sr' in t or 'cpu st' in t: return 'hw'
    for k in ['故障', '密码', '存储卡', 'led', '看门狗']:
        if k in t: return 'fault'
    if any(k in t for k in ['modbus', 'profinet', 'ppi', 'tcp', 'udp', 'rs485', 'iso']): return 'comm'
    if any(k in t for k in ['ton', 'ctud', 'mov_', 'shl', 'trunc', '触点', '线圈', 'for/next', 'jmp', 'xmt']): return 'ins'
    return 'formula'

# 1) 首选：输出文件（带 key）
n_key = 0
if os.path.exists(OUTFILE):
    raw = open(OUTFILE, encoding='utf-8').read()
    try:
        data = json.loads(raw)['result']['data']
        for d in data:
            for prefix, sec in KEY2SEC.items():
                if d['key'].startswith(prefix):
                    sections[sec].extend(d['items'])
                    n_key += 1
                    break
    except Exception as e:
        print('输出文件解析失败:', e)

# 2) 回退：journal + transcripts（按内容分类，条目名去重）
seen = {i['name'] for v in sections.values() for i in v}
n_fb = 0
for src_items in [json.loads(l)['result']['items'] for l in open(JOURNAL, encoding='utf-8')
                  if json.loads(l).get('type') == 'result' and isinstance(json.loads(l).get('result'), dict) and json.loads(l)['result'].get('items')] + \
                 (list(json.load(open(TR, encoding='utf-8')).values()) if os.path.exists(TR) else []):
    sec = classify(src_items)
    add = [i for i in src_items if i['name'] not in seen]
    for i in add: seen.add(i['name'])
    if add:
        sections[sec].extend(add)
        n_fb += len(add)

for k, arr in sections.items():
    for it in arr:
        assert it.get('name') and isinstance(it.get('fields'), list) and it.get('source'), (k, it.get('name'))
sections['ins'].sort(key=lambda x: x['name'])

out = {'sections': [{'key': k, 'items': v} for k, v in sections.items() if v]}
json.dump(out, open(DEST, 'w', encoding='utf-8'), ensure_ascii=False, indent=1)
print(f'输出文件来源 {n_key} 组，回退补充 {n_fb} 条')
print('最终分布:', {k: len(v) for k, v in sections.items()}, '总计', sum(len(v) for v in sections.values()))
