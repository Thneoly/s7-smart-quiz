# -*- coding: utf-8 -*-
"""应用手册裁决：14题官方答案→手册口径（每题附手册页码引证+官方口径备注+official_answer存档）"""
import io, sys, os, json
sys.stdout = io.TextIOWrapper(sys.stdout.buffer, encoding='utf-8')
sys.path.insert(0, os.path.dirname(__file__))
from _data import data

# key → (新答案, 解析, 出处, 官方判分答案)
V = {
 'A14': ('D', '位存储器 M 共 256 个：M 区 MB0~MB31 共 32 字节 × 8 位 = 256 位（手册附录A.2 表A-9"位存储器(M) 256位"、附录E.5"M0.0 到 M31.7"）。', '系统手册V2.8 表A-9/E.5', 'B'),
 'B10': ('A', '1 WORD = 2 字节。手册9.18节：BYTE、WORD、DWORD 分别标识 1、2、4 字节——4 字节是 DWORD。', '系统手册V2.8 9.18节(页450)', 'C'),
 'B46': ('ABD', '手册9.1.1：常开/常闭触点操作数为 I、Q、V、M、SM、S、T、C、L——M21.2 在 M0.0~M31.7 范围内合法；I 仅 I0.0~I31.7，I33.5 越界非法。', '系统手册V2.8 9.1.1(页221-222)', 'AB'),
 'C38': ('D', '选型手册 SB CM01 信号板技术规范：RS232 收发器"电缆长度，屏蔽电缆 最大 10 m"（同表 RS485 无中继为 50m，官方答案疑混淆两协议）。系统手册10.7 泛指 RS232 最远 50 英尺≈15m。', '选型手册V2.8 SB CM01(页27)', 'C'),
 'C58': ('ABC', '手册6.3.6："线圈通常表示逻辑输出结果，如指示灯、电机启动器、干预继电器或内部输出条件"——指示灯属输出负载；急停按钮是输入（"触点表示逻辑输入条件，如开关、按钮"）。', '系统手册V2.8 6.3.6(页127)', 'AB'),
 'D15': ('C', '手册10.6：本体 RS485 端口"可使用 PPI 协议和自由端口"；PROFIBUS 须经 EM DP01 扩展模块接入（10.5.1.2），本体口不支持。两台 SMART 之间走本体 RS485 即 PPI。', '系统手册V2.8 10.6(页544)', 'A'),
 'D26': ('B', '手册9.3：字符中断接收时"接收到的字符存入 SMB2……SMB2 是自由端口接收字符缓冲区"；SMB3 仅含奇偶校验错误位（附录D.5 符号名 Parity_Err）。', '系统手册V2.8 9.3(页261)/附录D.4', 'A'),
 'D40': ('B', '西门子技术参考《S7-200 SMART Modbus 通信》："Modbus 是一种单主站的主/从通信模式。Modbus 网络上只能有一个主站存在"，从站地址 1-247（247 是从站地址上限非主站数）。', '技术参考 Modbus 通信（西门子官网）', 'C'),
 'D54': ('ABC', '手册表5-2"位寻址"：位在字节中"共 8 位，编号 7 到 0"——位号只能 0~7，M100.8 非法；且 M 区仅 MB0~MB31（页162），字节 100 不存在。I0.0/Q0.1/V10.0 均合法。', '系统手册V2.8 表5-2(页89)/页162', 'ABCD'),
 'D58': ('AC', '手册9.14.1：循环移位"操作为循环操作"——移出位从另一端进入（A）、不丢失位（C）；"补零"仅属普通移位指令（D 不成立）；全语料无循环移位"用于编码"的表述（B 无依据）。', '系统手册V2.8 9.14.1(页419)', 'BD'),
 'E4': ('A', '手册11.8.1.1："SINA_POS 指令仅支持 SIEMENS 报文 111"。"标准报文 1"是 SINA_SPEED 的要求（11.8.2.1）——官方判分卷疑混淆两条指令。', '系统手册V2.8 11.8.1.1(页683)', 'C'),
 'E9': ('B', '手册10.6：RS485"每个网段最多可有 32 个设备"（页543-544、556），主站占 1 节点故从站上限 31（技术参考 USS："一个网络最多 32 个节点，最多 31 个从站"）。"16"出自选型手册 SMART LINE 触摸屏条目，非 CPU 口径。', '系统手册V2.8 10.6 + 技术参考USS', 'D'),
 'E41': ('ABC', '手册11.2：Modbus RTU 是"通过 CPU 串行端口进行的通信"，IP 地址属 Modbus TCP（11.4.1）；MBUS_CTRL/MBUS_INIT 参数仅 Baud/Parity/Port 等，无 IP 项。', '系统手册V2.8 11.2(页569)/11.4.1(页594)', 'ABCD'),
 'E62': ('ABC', '手册7.1.1 系统块组态清单：通信、数字量输入（滤波与脉冲捕捉位，页157）、数字量输出、保持范围、安全、启动——含滤波不含子程序；子程序调用属程序指令（9.18节）。', '系统手册V2.8 7.1.1(页153-157)', 'ABD'),
}

NOTE = '（注：官方线上判分按{off}给分，考试时请自行取舍口径）'
applied = 0
for L in 'ABCDE':
    f = data('题库资料', 'answers', f'{L}卷.jsonl')
    rows = [json.loads(l) for l in open(f, encoding='utf-8') if l.strip()]
    for r in rows:
        key = f"{L}{r['n']}"
        if key not in V:
            continue
        newans, expl, src, off = V[key]
        r['official_answer'] = off
        r['explain'] = expl + NOTE.format(off=off)
        r['source'] = src
        if r['answer'] != newans:                    # A14 已提前修正为 D，仅刷新解析
            r['corrected_from'] = r['answer'] + '(官方卷)'
            r['answer'] = newans
        applied += 1
    with open(f, 'w', encoding='utf-8') as fp:
        for r in rows:
            fp.write(json.dumps(r, ensure_ascii=False) + '\n')
print(f'已应用 {applied}/{len(V)} 题手册裁决')
