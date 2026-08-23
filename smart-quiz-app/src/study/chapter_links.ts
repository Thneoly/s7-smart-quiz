// 章节纵深链接：每章的手册检索词与速查关键词（前端静态映射，
// 与 guide.json 的再生成解耦）。
// 注意：docs_search 为 jieba 分词后 AND 语义——检索词必须精简（1~2 个高信号词），
// 词表与 docs.rs 的 study_doc_queries_hit 测试保持同步（真实语料非空命中）；
// refdata 匹配规则：条目 name/category 含任一关键词（不分大小写）。
export interface ChapterLink {
  /** 手册全文检索词（AND 语义，宁少勿多） */
  docs: string
  /** 关联速查：板块 key（hw/ins/comm/fault/formula）→ 命中关键词 */
  ref: { sec: string; kw: string[] }[]
}

export const CHAPTER_LINKS: Record<number, ChapterLink> = {
  1: { docs: '扩展模块', ref: [{ sec: 'hw', kw: ['cpu', '扩展', '信号板', '模块'] }] },
  2: { docs: '存储区', ref: [{ sec: 'formula', kw: ['存储区'] }] },
  3: { docs: '接线', ref: [] },
  4: { docs: '编程软件', ref: [] },
  5: { docs: '定时器', ref: [{ sec: 'ins', kw: ['位逻辑', '定时器', '计数器'] }] },
  6: { docs: '电机', ref: [] },
  7: { docs: '状态图表', ref: [{ sec: 'fault', kw: ['led', '诊断', '错误'] }] },
  8: { docs: '子程序', ref: [{ sec: 'ins', kw: ['call'] }] },
  9: { docs: '中断', ref: [{ sec: 'ins', kw: ['时钟', '转换', '中断', '顺控'] }] },
  10: { docs: 'PWM', ref: [{ sec: 'ins', kw: ['pls'] }] },
  11: { docs: '高速计数器', ref: [{ sec: 'ins', kw: ['hsc', 'hdef'] }, { sec: 'formula', kw: ['高速计数'] }] },
  12: { docs: 'PID', ref: [{ sec: 'ins', kw: ['pid'] }] },
  13: { docs: 'Modbus', ref: [{ sec: 'comm', kw: ['modbus'] }] },
  14: { docs: 'CRC', ref: [{ sec: 'comm', kw: ['modbus', 'rs485'] }] },
  15: { docs: '运动控制', ref: [{ sec: 'ins', kw: ['pls'] }] },
  16: { docs: '存储卡', ref: [{ sec: 'fault', kw: ['存储卡', '电池'] }] },
  17: { docs: '字符串', ref: [{ sec: 'ins', kw: ['字符串', '表', '移位'] }] },
  18: { docs: 'PUT', ref: [{ sec: 'comm', kw: ['s7', 'get', 'put', '以太网'] }] },
  19: { docs: 'PROFINET', ref: [{ sec: 'comm', kw: ['profinet', '以太网'] }] },
  20: { docs: '自由口', ref: [{ sec: 'comm', kw: ['自由口', 'rs485', '串口'] }] },
  21: { docs: 'UDP', ref: [{ sec: 'comm', kw: ['udp'] }] },
  22: { docs: 'TCP', ref: [{ sec: 'comm', kw: ['tcp'] }] },
}
