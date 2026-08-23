// 章节纵深链接：每章的手册检索词与速查关键词（前端静态映射，
// 与 guide.json 的再生成解耦）。匹配规则：refdata 条目的 name/category 含任一关键词（不分大小写）。
export interface ChapterLink {
  /** 手册全文检索词（docs_search 的 query） */
  docs: string
  /** 关联速查：板块 key（hw/ins/comm/fault/formula）→ 命中关键词 */
  ref: { sec: string; kw: string[] }[]
}

export const CHAPTER_LINKS: Record<number, ChapterLink> = {
  1: { docs: 'CPU 扩展模块 信号板 订货号', ref: [{ sec: 'hw', kw: ['cpu', '扩展', '信号板', '模块'] }] },
  2: { docs: '存储区 数据类型 寻址 V区', ref: [] },
  3: { docs: '接线 端子 输入 输出', ref: [{ sec: 'hw', kw: ['端子', '接线', 'io'] }] },
  4: { docs: '编程软件 项目 编译 下载', ref: [] },
  5: { docs: '触点 线圈 定时器 计数器', ref: [{ sec: 'ins', kw: ['位逻辑', '定时器', '计数器'] }] },
  6: { docs: '电机 启保停 互锁', ref: [] },
  7: { docs: '状态图表 交叉引用 调试 监控', ref: [{ sec: 'fault', kw: ['led', '诊断', '错误'] }] },
  8: { docs: '子程序 局部变量', ref: [{ sec: 'ins', kw: ['子程序'] }] },
  9: { docs: '读实时时钟 转换 顺控 中断', ref: [{ sec: 'ins', kw: ['时钟', '转换', '中断', '顺控'] }] },
  10: { docs: 'PWM 脉冲输出', ref: [{ sec: 'ins', kw: ['脉冲'] }] },
  11: { docs: '高速计数器 HSC', ref: [{ sec: 'ins', kw: ['高速计数'] }] },
  12: { docs: 'PID 回路 整定', ref: [{ sec: 'formula', kw: ['pid', '回路'] }] },
  13: { docs: 'Modbus 主站 从站', ref: [{ sec: 'comm', kw: ['modbus'] }] },
  14: { docs: 'Modbus 报文 CRC 功能码', ref: [{ sec: 'comm', kw: ['modbus', 'rs485'] }] },
  15: { docs: '运动控制 轴 位控', ref: [{ sec: 'ins', kw: ['运动', '位控', '轴'] }] },
  16: { docs: '特殊功能 存储卡 系统时间', ref: [{ sec: 'fault', kw: ['存储卡', '电池'] }] },
  17: { docs: '表 查找 填充 字符串 指令', ref: [{ sec: 'ins', kw: ['表', '字符串', '转换'] }] },
  18: { docs: 'GET PUT S7通信', ref: [{ sec: 'comm', kw: ['s7', 'get', 'put', '以太网'] }] },
  19: { docs: 'PROFINET IO 控制器 设备', ref: [{ sec: 'comm', kw: ['profinet', '以太网'] }] },
  20: { docs: '自由口 发送 接收', ref: [{ sec: 'comm', kw: ['自由口', 'rs485', '串口'] }] },
  21: { docs: 'UDP 通信', ref: [{ sec: 'comm', kw: ['udp'] }] },
  22: { docs: 'TCP 连接 通信', ref: [{ sec: 'comm', kw: ['tcp'] }] },
}
