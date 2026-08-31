# -*- coding: utf-8 -*-
"""用官方判分答卷校准 A~E 卷答案（2026-08-31，满分卷=标准答案）
- 一致题：confidence → high（官方确认）
- 不一致题：answer ← 官方，explain 重置为校准说明（原解析论证的是旧答案，不可留），
  corrected_from 记录原答案留审计痕迹
数据源：题库资料/answers/_official_graded.json（由官方答卷 HTML 提取，5卷×70题全对0错）
"""
import io, sys, os, json
sys.stdout = io.TextIOWrapper(sys.stdout.buffer, encoding='utf-8')
sys.path.insert(0, os.path.dirname(__file__))
from _data import data

OFFICIAL = json.load(open(data('题库资料', 'answers', '_official_graded.json'), encoding='utf-8'))
CAL_NOTE = '官方标准答案：问卷星 A~E 卷满分判分答卷（350/350 判对）校准，2026-08-31。'

total_same = total_fixed = 0
for name in sorted(OFFICIAL):
    path = data('题库资料', 'answers', f'{name}.jsonl')
    rows = [json.loads(l) for l in open(path, encoding='utf-8') if l.strip()]
    by_n = {r['n']: r for r in rows}
    off = OFFICIAL[name]
    same = fixed = miss = 0
    for nk, ans in sorted(off.items(), key=lambda kv: int(kv[0])):
        n = int(nk)                              # JSON 重载后整型键变字符串
        r = by_n.get(n)
        if not r:
            miss += 1; continue
        if r.get('answer') == ans:
            r['confidence'] = 'high'          # 官方确认，统一升为高置信
            same += 1
        else:
            r['corrected_from'] = r.get('answer', '')
            r['answer'] = ans
            r['confidence'] = 'high'
            r['explain'] = CAL_NOTE           # 旧解析论证旧答案，重置
            r['source'] = '官方答卷校准（A~E满分卷）'
            fixed += 1
    with open(path, 'w', encoding='utf-8') as f:
        for r in rows:
            f.write(json.dumps(r, ensure_ascii=False) + '\n')
    print(f'{name}: 一致升high {same} | 修正 {fixed} | 卷内无该题 {miss}')
    total_same += same; total_fixed += fixed

print(f'\n合计: 一致 {total_same} / 修正 {total_fixed} / 官方共 {sum(len(v) for v in OFFICIAL.values())} 题')
print('⚠ 下一步：重跑 30_pack_seed.py（bank.version 已升版）→ cargo test → 双仓提交')
