# -*- coding: utf-8 -*-
"""共享路径解析：私有数据统一归档在数据仓，脚本不依赖本机固定路径。

数据根目录解析顺序：
  1. 环境变量 SQ_DATA_ROOT
  2. 代码仓同级的 s7-smart-quiz-data/（git clone Thneoly/s7-smart-quiz-data 后即满足）
  3. 代码仓内部（本机历史布局：题库资料/、考试模拟卷/ 直接在仓库根，已被 .gitignore 排除）

约定：题库资料/ 与 考试模拟卷/ 下的**读写都走数据根**（数据仓既存原料也存派生物，
重新生成后提交回数据仓归档）；应用资源类产物（smartbank/docpack/guide.json）写在代码仓。
"""
import os

BASE = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))  # 代码仓根


def _find_root():
    cand = os.environ.get('SQ_DATA_ROOT')
    if cand and os.path.isdir(os.path.join(cand, '题库资料')):
        return cand
    sibling = os.path.join(BASE, '..', 's7-smart-quiz-data')
    if os.path.isdir(os.path.join(sibling, '题库资料')):
        return os.path.abspath(sibling)
    if os.path.isdir(os.path.join(BASE, '题库资料')):
        return BASE
    raise SystemExit(
        '[数据缺失] 未找到题库资料/。解决：\n'
        '  git clone git@github.com:Thneoly/s7-smart-quiz-data.git  # 克隆到代码仓同级（私有，需权限）\n'
        '  或设置环境变量 SQ_DATA_ROOT 指向数据目录')


ROOT = _find_root()


def data(*parts):
    """数据仓内路径：data('题库资料', 'questions') """
    return os.path.join(ROOT, *parts)
