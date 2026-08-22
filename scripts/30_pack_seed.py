# -*- coding: utf-8 -*-
"""打包种子题库 .smartbank（zip: manifest.json + assets/）
数据源：题库.json(344题) + papers_raw.json(5卷350题) + answers/*.jsonl + 考试模拟卷/images/
"""
import sys, io, os, json, zipfile, re
sys.stdout = io.TextIOWrapper(sys.stdout.buffer, encoding='utf-8')

from _data import data, BASE
OUT = os.path.join(BASE, 'smart-quiz-app', 'src-tauri', 'resources', 'seed', 'smart-core.smartbank')

def clean_opts(opts):
    opts = [re.sub(r'^[A-H][、.．,，]\s*', '', o) for o in opts]
    return [f'{chr(65+i)}、{o}' for i, o in enumerate(opts)]

def main():
    bank = json.load(open(data('考试模拟卷', '题库.json'), encoding='utf-8'))
    papers = json.load(open(data('题库资料', 'papers_raw.json'), encoding='utf-8'))
    img_src = data('考试模拟卷', 'images')

    questions, topics = [], {}
    def add_topic(name, parent=None):
        if name not in topics:
            topics[name] = {'topic_key': name, 'name': name.split('/')[-1], 'parent': parent}
        return name

    # 344 题库题
    for i, q in enumerate(bank, 1):
        tname = add_topic(q['topic'])
        questions.append({
            'qid': f'SC-T{i:03d}', 'type': q['type'], 'stem': q['q'],
            'options': clean_opts(q['options']), 'answer': q['answer'].upper(),
            'answer_conf': 'high', 'explain': q.get('explain', ''), 'source': q.get('source', ''),
            'difficulty': 3, 'topics': [q['topic']], 'img_path': None, 'status': 'active',
        })
    # 350 试卷题
    papers_meta = []
    for name in sorted(papers):
        p = papers[name]
        ans_file = data('题库资料', 'answers', f'{name}.jsonl')
        ans = {}
        if os.path.exists(ans_file):
            for line in open(ans_file, encoding='utf-8'):
                line = line.strip()
                if line:
                    a = json.loads(line); ans[a['n']] = a
        paper_t = add_topic(f'真题卷/{name}')
        items = []
        for it in p['items']:
            if it['type'] == 'fill': continue
            qid = f'SC-{name[0]}{it["n"]:03d}'
            a = ans.get(it['n'], {})
            img = None
            if it.get('img'):
                fn = os.path.basename(it['img'])
                img = f'assets/{fn}'
            questions.append({
                'qid': qid, 'type': it['type'], 'stem': it['q'],
                'options': clean_opts(it['options']),
                'answer': (a.get('answer') or '').upper(),
                'answer_conf': a.get('confidence', '') or ('high' if (a.get('answer') or '') else 'none'),
                'explain': a.get('explain', ''), 'source': a.get('source', ''),
                'difficulty': 3, 'topics': [paper_t], 'img_path': img,
                'status': 'active' if a.get('confidence') == 'high' else 'pending_review',
            })
            items.append({'qid': qid, 'sort_no': it['n'], 'score': 1})
        papers_meta.append({'name': name, 'title': p['title'], 'source_url': p.get('url', ''), 'items': items})

    manifest = {
        'format': 'smartbank', 'schema_ver': 1,
        'bank': {'bank_id': 'smart-core', 'name': 'S7-200 SMART 认证题库',
                 'version': 1, 'description': '西门子S7-200 SMART初级认证：10主题344题+A~E五套模拟卷350题（含答案解析出处）'},
        'topics': list(topics.values()),
        'questions': questions,
        'papers': papers_meta,
        'license': None,   # 预留字段（决策：方案Ⅰ不防盗版）
    }
    os.makedirs(os.path.dirname(OUT), exist_ok=True)
    with zipfile.ZipFile(OUT, 'w', zipfile.ZIP_DEFLATED) as z:
        z.writestr('manifest.json', json.dumps(manifest, ensure_ascii=False))
        n_img = 0
        for fn in os.listdir(img_src):
            z.write(os.path.join(img_src, fn), f'assets/{fn}')
            n_img += 1
    n_pending = sum(1 for q in questions if q['status'] == 'pending_review')
    print(f'打包完成: {OUT}')
    print(f'题目 {len(questions)}（active {len(questions)-n_pending} / pending_review {n_pending}），主题 {len(topics)}，试卷 {len(papers_meta)}，图片 {n_img}')
    print(f'文件大小: {os.path.getsize(OUT)//1024} KB')

if __name__ == '__main__':
    main()
