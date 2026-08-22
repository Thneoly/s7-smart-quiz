# -*- coding: utf-8 -*-
"""合并各主题 jsonl 题目 -> 校验、去重 -> 输出 题库.md / 题库.json / 统计"""
import sys, io, os, json, re
from difflib import SequenceMatcher
sys.stdout = io.TextIOWrapper(sys.stdout.buffer, encoding='utf-8')

from _data import data

QDIR = data('题库资料', 'questions')
OUT_MD = data('考试模拟卷', '题库.md')
OUT_JSON = data('考试模拟卷', '题库.json')

def norm(s):
    # 仅保留字母数字和中文，其余（含标点、空格）全部去除
    return re.sub(r'[^\w一-鿿]+', '', s or '').lower()

def load():
    items, bad = [], []
    for fn in sorted(os.listdir(QDIR)):
        if not fn.endswith('.jsonl'): continue
        topic_hint = fn.replace('.jsonl', '').split('_', 1)[-1]
        for i, line in enumerate(open(os.path.join(QDIR, fn), encoding='utf-8'), 1):
            line = line.strip()
            if not line: continue
            try:
                q = json.loads(line)
                assert q.get('q') and q.get('options') and q.get('answer') and q.get('type') in ('single', 'multi')
                # 答案字母必须在选项范围内
                letters = set(re.findall(r'[A-E]', q['answer'].upper()))
                n_opt = len(q['options'])
                assert all(ord(l) - 65 < n_opt for l in letters), f'answer超出选项范围'
                if q['type'] == 'single': assert len(letters) == 1, '单选答案应为单字母'
                q.setdefault('topic', topic_hint)
                q['explain'] = q.get('explain', '')
                q['source'] = q.get('source', '')
                items.append(q)
            except Exception as e:
                bad.append(f'{fn}:{i} {e} :: {line[:80]}')
    return items, bad

def dedup(items):
    kept, seen = [], []
    for it in items:
        n = norm(it['q'])
        dup = None
        for k, (kn, km) in enumerate(seen):
            if n == kn or (len(n) > 12 and SequenceMatcher(None, n, kn).ratio() > 0.88):
                dup = km; break
        if dup is not None:
            kept[dup].setdefault('dups', []).append(it['q'][:40])
        else:
            seen.append((n, len(kept)))
            kept.append(it)
    return kept

def to_md(items):
    order = []
    for it in items:
        if it['topic'] not in order: order.append(it['topic'])
    lines = ['# S7-200 SMART 初级认证题库', '',
             '> 依据官方资料生成：S7-200 SMART 技术参考 PLUS 2.6、系统手册 V2.8、选型手册 V2.8、Micro/WIN SMART 帮助',
             '> 每题均附答案、解析与出处；生成日期 2026-08-22', '']
    # 总览表
    lines.append('| 主题 | 题数 | 单选 | 多选 |')
    lines.append('|---|---|---|---|')
    total = s_total = m_total = 0
    for t in order:
        ts = [i for i in items if i['topic'] == t]
        s = sum(1 for i in ts if i['type'] == 'single'); m = len(ts) - s
        total += len(ts); s_total += s; m_total += m
        lines.append(f'| {t} | {len(ts)} | {s} | {m} |')
    lines.append(f'| **合计** | **{total}** | **{s_total}** | **{m_total}** |')
    lines.append('')
    for sec, t in enumerate(order, 1):
        ts = [i for i in items if i['topic'] == t]
        singles = [i for i in ts if i['type'] == 'single']
        multis = [i for i in ts if i['type'] == 'multi']
        lines.append(f'## {sec}、{t}')
        lines.append('')
        n = 0
        if singles:
            lines.append('### 单选题'); lines.append('')
            for it in singles:
                n += 1
                lines += [f'{n}. {it["q"]}', '']
                lines += [f'   - {o}' for o in it['options']]
                lines += ['', f'   **答案：{it["answer"].upper()}**', f'   解析：{it["explain"]}（出处：{it["source"]}）', '']
        if multis:
            lines.append('### 多选题'); lines.append('')
            for it in multis:
                n += 1
                lines += [f'{n}. {it["q"]}【多选】', '']
                lines += [f'   - {o}' for o in it['options']]
                lines += ['', f'   **答案：{it["answer"].upper()}**', f'   解析：{it["explain"]}（出处：{it["source"]}）', '']
    return '\n'.join(lines)

def main():
    items, bad = load()
    print(f'载入 {len(items)} 题，格式问题 {len(bad)} 条')
    for b in bad[:20]: print('  BAD', b)
    kept = dedup(items)
    ndup = sum(len(k.get('dups', [])) for k in kept)
    print(f'去重移除 {ndup} 题，保留 {len(kept)} 题')
    os.makedirs(os.path.dirname(OUT_MD), exist_ok=True)
    open(OUT_MD, 'w', encoding='utf-8').write(to_md(kept))
    for k in kept: k.pop('dups', None)
    json.dump(kept, open(OUT_JSON, 'w', encoding='utf-8'), ensure_ascii=False, indent=1)
    print(f'已写入 {OUT_MD} 和 {OUT_JSON}')

if __name__ == '__main__':
    main()
