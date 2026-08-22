# -*- coding: utf-8 -*-
"""应用校验工作流的修正：wrong改答案、ambiguous删题，然后重建题库与平台数据
输入：题库资料/corrections.json  [{"topic":"07_PID控制.jsonl","results":[{"line":2,"verdict":"wrong","answer":"C","reason":"..."}]}]
"""
import sys, io, os, json
sys.stdout = io.TextIOWrapper(sys.stdout.buffer, encoding='utf-8')

BASE = os.path.dirname(os.path.abspath(__file__))
QDIR = os.path.join(BASE, '题库资料', 'questions')
CORR = os.path.join(BASE, '题库资料', 'corrections.json')

def main():
    if not os.path.exists(CORR):
        print('无 corrections.json，跳过'); return
    corrections = json.load(open(CORR, encoding='utf-8'))
    n_fix = n_drop = 0
    for c in corrections:
        f = os.path.join(QDIR, c['topic'])
        if not os.path.exists(f):
            print(f'! 找不到 {f}'); continue
        lines = open(f, encoding='utf-8').read().splitlines()
        verdicts = {r['line']: r for r in c.get('results', []) if r.get('verdict') != 'correct'}
        if not verdicts: continue
        out = []
        for i, line in enumerate(lines, 1):
            if i not in verdicts:
                out.append(line); continue
            v = verdicts[i]
            try: q = json.loads(line)
            except Exception: out.append(line); continue
            if v['verdict'] == 'wrong':
                old = q['answer']
                q['answer'] = v['answer']
                q['explain'] = f"[校验修正，原答案{old}] {v.get('reason','')[:200]}"
                print(f"修正 {c['topic']} 第{i}题: {old} -> {v['answer']} | {q['q'][:40]}")
                out.append(json.dumps(q, ensure_ascii=False)); n_fix += 1
            else:  # ambiguous：删除
                print(f"删除 {c['topic']} 第{i}题(歧义): {q['q'][:40]} | {v.get('reason','')[:100]}")
                n_drop += 1
        open(f, 'w', encoding='utf-8').write('\n'.join(out) + '\n')
    print(f'完成：修正 {n_fix} 题，删除 {n_drop} 题')

if __name__ == '__main__':
    main()
