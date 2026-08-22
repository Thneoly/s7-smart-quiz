# -*- coding: utf-8 -*-
"""校验指南数据 -> 写入 app guide.json + 生成可读 学习指南.md"""
import sys, io, json
sys.stdout = io.TextIOWrapper(sys.stdout.buffer, encoding='utf-8')

stages = json.load(open('D:/PLC/s7-200/题库资料/guide_stages.json', encoding='utf-8'))['stages']
print('篇数:', len(stages))
for s in stages:
    chs = s.get('chapters', [])
    mins = sum(c.get('minutes', 0) for c in chs)
    for c in chs:
        for k in ['no', 'title', 'minutes', 'goal', 'points', 'practice_topics', 'manual_ref', 'days', 'priority']:
            assert k in c, f"{s['stage']} 第{c.get('no')}章 缺 {k}"
        assert c['priority'] in ('core', 'key', 'ext')
    print(f"  {s['stage']}: {len(chs)}章 {mins}分钟 章节{[c['no'] for c in chs]}")

json.dump({'stages': stages}, open('D:/PLC/s7-200/smart-quiz-app/src/study/guide.json', 'w', encoding='utf-8'), ensure_ascii=False, indent=1)
print('guide.json 已写入 app')

PRI = {'core': '核心', 'key': '重点', 'ext': '拓展'}
lines = ['# S7-200 SMART 初级认证学习指南', '',
         '> 依据官方培训课程（基础篇 7 章 + 中级篇 6 章 + 高级篇 9 章，共 22 章）整理',
         '> 每章含学习目标、核心要点、配套题库练习（在应用「学习模式」中可点击直达）、手册出处', '']
for s in stages:
    lines += [f"## {s['stage']}", '', f"> {s['stage_goal']}", '']
    for c in s['chapters']:
        days = f" · {c['days']}" if c['days'] else ''
        lines += [f"### 第{c['no']}章 {c['title']}（{c['minutes']} 分钟 · {PRI[c['priority']]}{days}）", '',
                  f"**学习目标**：{c['goal']}", '']
        lines += ['**核心要点**'] + [f"- {p}" for p in c['points']] + ['']
        if c['practice_topics']:
            lines += [f"**配套练习**：{'、'.join(c['practice_topics'])}", '']
        lines += [f"**手册出处**：{c['manual_ref']}", '']
open('D:/PLC/s7-200/学习指南.md', 'w', encoding='utf-8').write('\n'.join(lines))
print('学习指南.md 已生成')
