# -*- coding: utf-8 -*-
"""生成练习平台数据：data.js（题库+模拟卷+答案）并复制图片"""
import sys, io, os, re, json, shutil
sys.stdout = io.TextIOWrapper(sys.stdout.buffer, encoding='utf-8')

BASE = os.path.dirname(os.path.abspath(__file__))
BANK_JSON = os.path.join(BASE, '考试模拟卷', '题库.json')
PAPERS_JSON = os.path.join(BASE, '题库资料', 'papers_raw.json')
ANS_DIR = os.path.join(BASE, '题库资料', 'answers')
PLAT = os.path.join(BASE, '练习平台')
IMG_SRC = os.path.join(BASE, '考试模拟卷', 'images')

def js_str(o):
    return json.dumps(o, ensure_ascii=False)

def main():
    bank = json.load(open(BANK_JSON, encoding='utf-8'))
    # 统一选项前缀 & 生成稳定 id
    for i, q in enumerate(bank):
        q['options'] = [re.sub(r'^[A-H][、.．,，]\s*', '', o) for o in q['options']]
        q['options'] = [f'{chr(65 + j)}、{o}' for j, o in enumerate(q['options'])]
        q['id'] = f'T{i + 1:03d}'
    papers = json.load(open(PAPERS_JSON, encoding='utf-8'))
    paper_out = []
    for name in sorted(papers):
        p = papers[name]
        ans = {}
        ans_file = os.path.join(ANS_DIR, f'{name}.jsonl')
        if os.path.exists(ans_file):
            for line in open(ans_file, encoding='utf-8'):
                line = line.strip()
                if not line: continue
                try:
                    a = json.loads(line)
                    ans[a['n']] = a
                except Exception as e:
                    print(f'  ! {name} 答案行解析失败: {e}')
        n_ans = len(ans)
        items = []
        for it in p['items']:
            if it['type'] == 'fill': continue
            a = ans.get(it['n'], {})
            items.append({
                'id': f'{name}-{it["n"]}', 'n': it['n'], 'type': it['type'],
                'q': it['q'], 'img': it['img'], 'options': it['options'],
                'ans': a.get('answer', ''), 'explain': a.get('explain', ''),
                'src': a.get('source', ''), 'conf': a.get('confidence', ''),
            })
        paper_out.append({'name': name, 'title': p['title'], 'url': p['url'],
                          'items': items, 'answered': n_ans})
        print(f'{name}: {len(items)}题, 已有答案{n_ans}')
    os.makedirs(PLAT, exist_ok=True)
    # 复制图片
    img_dst = os.path.join(PLAT, 'images')
    n_img = 0
    if os.path.isdir(IMG_SRC):
        os.makedirs(img_dst, exist_ok=True)
        for fn in os.listdir(IMG_SRC):
            src, dst = os.path.join(IMG_SRC, fn), os.path.join(img_dst, fn)
            if not os.path.exists(dst):
                shutil.copy2(src, dst); n_img += 1
    with open(os.path.join(PLAT, 'data.js'), 'w', encoding='utf-8') as f:
        f.write('// 自动生成，勿手改；运行 build_platform.py 重新生成\n')
        f.write(f'const GENERATED = "2026-08-22";\n')
        f.write(f'const BANK = {js_str(bank)};\n')
        f.write(f'const PAPERS = {js_str(paper_out)};\n')
    print(f'题库 {len(bank)} 题 + {len(paper_out)} 卷 -> data.js；新增图片 {n_img} 张')

if __name__ == '__main__':
    main()
