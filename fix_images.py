# -*- coding: utf-8 -*-
"""补下载 md 中的远程图片（加 Referer 绕过 403）并改写为本地链接"""
import sys, io, os, re, hashlib, urllib.request
sys.stdout = io.TextIOWrapper(sys.stdout.buffer, encoding='utf-8')

BASE = os.path.join(os.path.dirname(os.path.abspath(__file__)), '考试模拟卷')
IMG_DIR = os.path.join(BASE, 'images')
os.makedirs(IMG_DIR, exist_ok=True)
HDRS = {
    'User-Agent': 'Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/120.0 Safari/537.36',
    'Referer': 'https://ks.wjx.com/',
}

def main():
    ok = fail = 0
    for fn in sorted(os.listdir(BASE)):
        if not fn.endswith('.md'): continue
        path = os.path.join(BASE, fn)
        text = open(path, encoding='utf-8').read()
        urls = set(re.findall(r'!\[图\]\((https?://[^)]+)\)', text))
        if not urls: continue
        print(f'{fn}: {len(urls)} 张图片')
        def repl(m):
            nonlocal ok, fail
            url = m.group(1)
            ext = os.path.splitext(url.split('?')[0])[1].lower()
            if ext not in ('.png', '.jpg', '.jpeg', '.gif', '.bmp', '.webp'): ext = '.png'
            name = f"{fn[:-3]}-{hashlib.md5(url.encode()).hexdigest()[:8]}{ext}"
            local = os.path.join(IMG_DIR, name)
            if not os.path.exists(local):
                try:
                    req = urllib.request.Request(url, headers=HDRS)
                    with urllib.request.urlopen(req, timeout=30) as r, open(local, 'wb') as fw:
                        fw.write(r.read())
                    ok += 1
                except Exception as e:
                    print(f'  ! 失败 {url}: {e}'); fail += 1
                    return m.group(0)
            else:
                ok += 1
            return f'![图](images/{name})'
        text = re.sub(r'!\[图\]\((https?://[^)]+)\)', repl, text)
        open(path, 'w', encoding='utf-8').write(text)
    print(f'完成：成功 {ok}，失败 {fail}')

if __name__ == '__main__':
    main()
