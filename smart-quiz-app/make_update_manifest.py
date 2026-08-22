# -*- coding: utf-8 -*-
"""发布工具：为 tauri updater 生成 latest.json 清单
用法：先 `npm run tauri build` 产出安装包与 .sig 签名，然后：
  python make_update_manifest.py <版本号> <更新说明> <安装包URL前缀>
产物 latest.json 上传到任何静态文件托管（OSS/GitHub Releases），并同步改 tauri.conf.json 的 endpoints
"""
import sys, io, os, json, glob
sys.stdout = io.TextIOWrapper(sys.stdout.buffer, encoding='utf-8')

def main():
    if len(sys.argv) < 4:
        print(__doc__)
        return
    version, notes, base = sys.argv[1], sys.argv[2], sys.argv[3].rstrip('/')
    bundle_dir = os.path.join(os.path.dirname(os.path.abspath(__file__)), 'src-tauri', 'target', 'release', 'bundle', 'nsis')
    exe = glob.glob(os.path.join(bundle_dir, '*-setup.exe'))
    sig = glob.glob(os.path.join(bundle_dir, '*.sig'))
    if not exe or not sig:
        print('未找到安装包/签名（先运行 npm run tauri build，需设置 TAURI_SIGNING_PRIVATE_KEY 环境变量）')
        return
    exe_name = os.path.basename(exe[0])
    sig_text = open(sig[0], encoding='utf-8').read().strip()
    manifest = {
        'version': version,
        'notes': notes,
        'pub_date': __import__('datetime').datetime.utcnow().isoformat() + 'Z',
        'platforms': {
            'windows-x86_64': {
                'signature': sig_text,
                'url': f'{base}/{exe_name}',
            }
        },
    }
    out = os.path.join(os.path.dirname(bundle_dir), 'latest.json')
    json.dump(manifest, open(out, 'w', encoding='utf-8'), ensure_ascii=False, indent=2)
    print(f'已生成 {out}')
    print(f'安装包：{exe_name}')
    print(f'请将 {exe_name} 与 latest.json 上传到 {base}')

if __name__ == '__main__':
    main()
