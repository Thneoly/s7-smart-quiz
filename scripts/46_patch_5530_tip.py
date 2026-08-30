# -*- coding: utf-8 -*-
"""讲义考点提示补强：5530 的来历（27648×20%）+ 活零点断线检测 + 换算例题
- ch1 考点：数值链路 tip 扩写来历与活零点
- ch5 考点：数值必背 tip 附换算例题（分母 22118 有效码数）
"""
import json, sys, io
sys.stdout = io.TextIOWrapper(sys.stdout.buffer, encoding='utf-8')

P = '../smart-quiz-app/src/study/lectures.json'
L = json.load(open(P, encoding='utf-8'))
orig_indent = open(P, encoding='utf-8').readline().startswith('{\n')  # 仅探测，不改判定
assert 'lectures' in L

T1_OLD_MARK = '模拟量数值链路要背熟'
T1_NEW = ('模拟量数值链路要背熟：满量程 -27648~27648，0~20mA 对应 0~27648，'
          '4~20mA 对应 5530~27648（工程量换算题的基础）；5530 的来历：27648×20%=5529.6≈5530'
          '——4mA 是 20mA 量程的 20%，故名"单极性 20% 偏移量"（PID 标定中固定不可改）；'
          '0~4mA 是死区，读到 0mA 或上溢 32767 表示断线/故障而非零测量（活零点设计）；'
          '温度模块读数=实际值×10（521→52.1℃）；'
          '"高精度必为高分辨率，高分辨率不代表高精度"是经典判断题。')

T2_OLD_MARK = '模拟量数值必背'
T2_NEW = ('模拟量数值必背：4~20mA对应5530~27648、0~20mA对应0~27648，超限判断32511/-32512；'
          '换算骨架记分母：27648−5530=22118 有效码数，例：0~100℃ 的 4~20mA 变送器读 13824，'
          '(13824−5530)÷22118×100≈37.5℃；转换指令中ROUND四舍五入、TRUNC舍去小数'
          '（123.789→124对123）同样是高频出题点。')

changed = []
for lec in L['lectures']:
    tips = lec.get('exam_tips', [])
    for i, t in enumerate(tips):
        if T1_OLD_MARK in t and lec['no'] == 1:
            tips[i] = T1_NEW; changed.append(f"ch1 tip#{i+1}")
        if T2_OLD_MARK in t and lec['no'] == 5:
            tips[i] = T2_NEW; changed.append(f"ch5 tip#{i+1}")

if sorted(changed) != ['ch1 tip#2', 'ch5 tip#4']:
    print('❌ 命中异常:', changed); sys.exit(1)

# 保持原有缩进风格（indent=1，与生成时一致）
json.dump(L, open(P, 'w', encoding='utf-8'), ensure_ascii=False, indent=1)
print('✅ 已更新:', ', '.join(changed))
print('ch1 新 tip 长度:', len(T1_NEW), '| ch5 新 tip 长度:', len(T2_NEW))
