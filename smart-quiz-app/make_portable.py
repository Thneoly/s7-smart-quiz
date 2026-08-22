# -*- coding: utf-8 -*-
"""打免安装便携包：release exe + resources（seed/docs 数据包）+ 使用说明 → zip

用法（release 构建之后）：
    python make_portable.py
产物：src-tauri/target/release/bundle/portable/smart-quiz-app_<版本>_x64-portable.zip

已实证验证的布局：exe 同级的 resources/seed、resources/docs 即被 resource_dir 命中，
首启自动导入种子题库（日志可见 [seed] 导入 ...694题）。
"""
import os, sys, io, zipfile

sys.stdout = io.TextIOWrapper(sys.stdout.buffer, encoding='utf-8')
BASE = os.path.dirname(os.path.abspath(__file__))
ST = os.path.join(BASE, 'src-tauri')
EXE = os.path.join(ST, 'target', 'release', 'smart-quiz-app.exe')
SEED_DIR = os.path.join(ST, 'resources', 'seed')
DOCS_DIR = os.path.join(ST, 'resources', 'docs')

README_TXT = """S7-200 SMART 题库平台（便携版）
================================

1. 解压到任意目录，双击 smart-quiz-app.exe 即可使用，无需安装
2. 运行环境：Windows 10/11（依赖系统自带的 WebView2 运行时；
   极少数精简系统若无，会提示下载安装）
3. 首次启动自动导入内置题库（694 题）与检索语料
4. 学习记录/错题本等用户数据保存在本机 AppData，删除解压目录不影响数据
5. 升级方式：下载新版 zip 解压覆盖（便携版不支持应用内自动更新）
"""


def version():
    for line in open(os.path.join(ST, 'Cargo.toml'), encoding='utf-8'):
        if line.startswith('version'):
            return line.split('"')[1]
    return '0.0.0'


def main():
    if not os.path.exists(EXE):
        sys.exit('未找到 release exe，先执行 npm run tauri build')
    out_dir = os.path.join(ST, 'target', 'release', 'bundle', 'portable')
    os.makedirs(out_dir, exist_ok=True)
    out = os.path.join(out_dir, f'smart-quiz-app_{version()}_x64-portable.zip')

    with zipfile.ZipFile(out, 'w', zipfile.ZIP_DEFLATED, compresslevel=9) as z:
        z.write(EXE, 'smart-quiz-app.exe')
        z.writestr('使用说明.txt', README_TXT)
        n_data = 0
        for d, arc in [(SEED_DIR, 'resources/seed'), (DOCS_DIR, 'resources/docs')]:
            if not os.path.isdir(d):
                print(f'⚠ 目录缺失（跳过）: {d}')
                continue
            for f in os.listdir(d):
                if f.endswith(('.smartbank', '.docpack')):
                    z.write(os.path.join(d, f), f'{arc}/{f}')
                    n_data += 1
                    print(f'+ {arc}/{f}')
        if n_data == 0:
            print('⚠ 未打包任何数据包（题库为空，需应用内导入）')
    print(f'✅ 便携包已生成：{out}（{os.path.getsize(out) // 1024} KB）')


if __name__ == '__main__':
    main()
