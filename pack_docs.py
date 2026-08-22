# -*- coding: utf-8 -*-
"""打包全文检索语料 docs.docpack（zip: manual/ techref/ microwin/ course/）"""
import sys, io, os, zipfile
sys.stdout = io.TextIOWrapper(sys.stdout.buffer, encoding='utf-8')
BASE = os.path.dirname(os.path.abspath(__file__))
SRC = os.path.join(BASE, '题库资料')
OUT = os.path.join(BASE, 'smart-quiz-app', 'src-tauri', 'resources', 'docs', 'docs.docpack')
os.makedirs(os.path.dirname(OUT), exist_ok=True)

RULES = [
    ('手册章节', 'manual'),
    ('techref_txt/S7-200SMART/SMART', 'techref'),
    ('microwin_txt/29039463947', 'microwin'),
    ('课程资料', 'course'),
]
n = 0
with zipfile.ZipFile(OUT, 'w', zipfile.ZIP_DEFLATED) as z:
    for src, prefix in RULES:
        root = os.path.join(SRC, src)
        if not os.path.isdir(root):
            continue
        for r, _, files in os.walk(root):
            for fn in files:
                if not fn.endswith('.txt'):
                    continue
                full = os.path.join(r, fn)
                rel = os.path.relpath(full, root)
                arc = (prefix + '/' + rel).replace(os.sep, '/')
                z.write(full, arc)
                n += 1
print(f'打包 {n} 个文件 -> {OUT} ({os.path.getsize(OUT)//1024} KB)')
