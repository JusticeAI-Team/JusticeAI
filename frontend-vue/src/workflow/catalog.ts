export type WorkflowStepKey =
  | 'setup'
  | 'data-ingestion'
  | 'data-mapping'
  | 'knowledge-extraction'
  | 'knowledge-graph'
  | 'risk-analysis'
  | 'alerts'
  | 'case-dispatch'
  | 'evaluation'
  | 'supervision'
  | 'reports'
  | 'settings'

export type WorkflowAgentKey =
  | 'clue-mining'
  | 'risk-analysis'
  | 'graded-alert'
  | 'alert-execution'
  | 'effect-evaluation'
  | 'supervision'

export interface WorkflowNavigationLink {
  label: string
  path: string
}

export interface OperatorGuideItem {
  index: string
  title: string
  description: string
}

export interface WorkflowStepDefinition {
  key: WorkflowStepKey
  path: string
  stageCode: string
  title: string
  shortLabel: string
  summary: string
  usageLead: string
  nextRequirement: string
  focus: string[]
  ticker: string[]
  operatorGuide: OperatorGuideItem[]
  agents: WorkflowAgentKey[]
}

export interface WorkflowAgentDefinition {
  key: WorkflowAgentKey
  label: string
  tagline: string
  duty: string
  path?: string
}

export const workflowAgents: WorkflowAgentDefinition[] = [
  {
    key: 'clue-mining',
    label: '线索挖掘 Agent',
    tagline: '归集 · 映射 · 抽取',
    duty: '负责接收月度数据、清洗字段、抽取实体关系并形成可继续分析的结构化线索。',
    path: '/data-ingestion',
  },
  {
    key: 'risk-analysis',
    label: '风险研判 Agent',
    tagline: '图谱 · 评分 · 排序',
    duty: '基于图谱关系和重点领域规则，对对象、案件与事项生成风险分值和优先级。',
    path: '/risk-analysis',
  },
  {
    key: 'graded-alert',
    label: '分级推送 Agent',
    tagline: '预警 · 分层 · 触达',
    duty: '把高风险对象转成不同等级的预警结果，明确来源、等级和后续推送目标。',
    path: '/alerts',
  },
  {
    key: 'alert-execution',
    label: '预警执行 Agent',
    tagline: '派发 · 督办 · 回流',
    duty: '把预警结果转成案件分派和处置任务，跟踪承办人、时限和反馈状态。',
    path: '/case-dispatch',
  },
  {
    key: 'effect-evaluation',
    label: '成效评估 Agent',
    tagline: '评估 · 复盘 · 报告',
    duty: '汇总办结率、化解率和报告产出，形成阶段性评估和汇报材料。',
    path: '/evaluation',
  },
  {
    key: 'supervision',
    label: '监督协调 Agent',
    tagline: '调度 · 监控 · 人工介入',
    duty: '贯穿全部阶段，负责调度其他 Agent、监控异常冲突并记录人工介入。',
    path: '/supervision',
  },
]

export const workflowSteps: WorkflowStepDefinition[] = [
  {
    key: 'setup',
    path: '/setup',
    stageCode: 'S01',
    title: '系统准备',
    shortLabel: '检查后端、图谱、模型与目录',
    summary: '先确认后端接口可读，再进入数据归集。',
    usageLead: '点击检查后，确认系统信息、健康状态和平台配置均已返回。满足后再继续。',
    nextRequirement: '后端基础接口至少成功返回一次系统信息与健康检查结果。',
    focus: ['后端接口', '模型服务', '图谱服务', '本地目录'],
    ticker: ['后端健康', 'vLLM 模型', 'HugeGraph 图谱', 'Milvus 检索', '平台配置'],
    operatorGuide: [
      { index: '01', title: '启动后端', description: '先启动本地后端服务，再检查接口状态。' },
      { index: '02', title: '检查依赖', description: '确认模型、图谱、数据库和目录都已联通。' },
      { index: '03', title: '进入归集', description: '看到系统状态正常后，再进入数据归集。' },
    ],
    agents: ['supervision'],
  },
  {
    key: 'data-ingestion',
    path: '/data-ingestion',
    stageCode: 'S02',
    title: '数据归集',
    shortLabel: '上传 12345 / 110 / 395 / 信访来源',
    summary: '上传文件、查看批次、选择兼容导入记录。',
    usageLead: '把 Excel 上传到本页，先看批次是否生成，再在下方查看旧导入记录和文件详情。',
    nextRequirement: '至少存在一批已上传或已汇总的数据批次，可供字段映射继续处理。',
    focus: ['上传文件', '批次列表', '来源汇总', '兼容导入'],
    ticker: ['上传 Excel', '批次追踪', '失败重试', '来源汇总', '导入详情'],
    operatorGuide: [
      { index: '01', title: '上传文件', description: '选择 csv、xls 或 xlsx 并发起上传。' },
      { index: '02', title: '看批次状态', description: '确认已生成批次、记录数和错误数。' },
      { index: '03', title: '选一条记录', description: '从兼容导入记录中查看详情，为映射做准备。' },
    ],
    agents: ['clue-mining', 'supervision'],
  },
  {
    key: 'data-mapping',
    path: '/data-mapping',
    stageCode: 'S03',
    title: '数据映射',
    shortLabel: '把源字段对齐到业务字段',
    summary: '确认模版、版本、字段数量和置信度。',
    usageLead: '先看当前模版版本，再核对字段清单和样例值，确认后进入知识抽取。',
    nextRequirement: '形成至少一版可复用字段映射模版，并确认关键字段映射。',
    focus: ['模版版本', '字段清单', '样例值', '置信度'],
    ticker: ['模版版本', '字段映射', '样例值', '目标字段', '置信度'],
    operatorGuide: [
      { index: '01', title: '确认模版', description: '先看当前生效模版和版本号。' },
      { index: '02', title: '检查字段', description: '逐项核对源字段、目标字段和样例值。' },
      { index: '03', title: '继续抽取', description: '确认关键字段已映射后进入知识抽取。' },
    ],
    agents: ['clue-mining', 'supervision'],
  },
  {
    key: 'knowledge-extraction',
    path: '/knowledge-extraction',
    stageCode: 'S04',
    title: '知识抽取',
    shortLabel: '抽取实体、关系和事件候选',
    summary: '查看最近抽取到的实体和置信度。',
    usageLead: '本页先确认最近抽取结果是否稳定，再决定是否进入知识图谱写入。',
    nextRequirement: '至少产生一批实体或关系候选结果，可供图谱写入。',
    focus: ['最近实体', '实体类型', '抽取时间', '置信度'],
    ticker: ['实体识别', '关系候选', '抽取时间', '抽取结果', '抽样复核'],
    operatorGuide: [
      { index: '01', title: '看结果数', description: '先确认最近已有抽取结果返回。' },
      { index: '02', title: '核对实体', description: '检查名称、类型和置信度是否合理。' },
      { index: '03', title: '写入图谱', description: '抽取稳定后再进入知识图谱。' },
    ],
    agents: ['clue-mining', 'supervision'],
  },
  {
    key: 'knowledge-graph',
    path: '/knowledge-graph',
    stageCode: 'S05',
    title: '知识图谱',
    shortLabel: '查看关系类型和图谱规模',
    summary: '确认图谱关系已落库，为风险研判提供基础。',
    usageLead: '先确认关系类型和数量已经出现，再进入风险评分阶段。',
    nextRequirement: '图谱写入与检索链路可用，至少能看到关系类型或图谱规模指标。',
    focus: ['关系类型', '图谱规模', '落库状态', '图查询入口'],
    ticker: ['关系类型', '图谱写入', '图谱规模', 'HugeGraph', '检索入口'],
    operatorGuide: [
      { index: '01', title: '看关系类型', description: '确认当前已有关系类型和数量统计。' },
      { index: '02', title: '确认落库', description: '图谱概览稳定后再进行风险研判。' },
      { index: '03', title: '进入研判', description: '从图谱结果继续转到风险评分。' },
    ],
    agents: ['risk-analysis', 'supervision'],
  },
  {
    key: 'risk-analysis',
    path: '/risk-analysis',
    stageCode: 'S06',
    title: '风险研判',
    shortLabel: '生成重点风险列表与评分',
    summary: '聚焦重点对象、重点事项和评分结果。',
    usageLead: '先看高风险列表和评分，再决定是否下发预警。',
    nextRequirement: '完成至少一轮风险评分或重点线索识别，能够进入预警阶段。',
    focus: ['重点风险', '风险评分', '所属领域', '优先级'],
    ticker: ['风险评分', '重点事项', '优先级排序', '重点领域', '高风险清单'],
    operatorGuide: [
      { index: '01', title: '看评分', description: '先确认已有可用的风险评分结果。' },
      { index: '02', title: '筛重点', description: '从列表中识别需要进入预警的事项。' },
      { index: '03', title: '转入预警', description: '确认后继续到预警中心。' },
    ],
    agents: ['risk-analysis', 'supervision'],
  },
  {
    key: 'alerts',
    path: '/alerts',
    stageCode: 'S07',
    title: '预警中心',
    shortLabel: '生成并确认预警结果',
    summary: '查看预警标题、等级、来源和触发时间。',
    usageLead: '本页用于确认风险是否已经转成预警，以及每条预警的等级和来源。',
    nextRequirement: '预警引擎与规则链路已接通，可生成或接收预警事件。',
    focus: ['预警事件', '等级', '来源', '状态确认'],
    ticker: ['预警事件', '规则触发', '来源归因', '等级分层', '状态确认'],
    operatorGuide: [
      { index: '01', title: '看预警列表', description: '先确认是否已生成预警事件。' },
      { index: '02', title: '核对等级', description: '查看等级、来源和触发时间。' },
      { index: '03', title: '继续派发', description: '确认预警后进入案件分派。' },
    ],
    agents: ['graded-alert', 'supervision'],
  },
  {
    key: 'case-dispatch',
    path: '/case-dispatch',
    stageCode: 'S08',
    title: '案件分派',
    shortLabel: '把预警转成任务并分派',
    summary: '查看承办人、优先级、时限和任务状态。',
    usageLead: '本页用于确认预警已转成任务，并查看当前承办人与到期时间。',
    nextRequirement: '至少能读取或创建可派发任务，并看到责任人与时限。',
    focus: ['任务队列', '责任人', '优先级', '截止时间'],
    ticker: ['任务队列', '责任人', '优先级', '截止时间', '处置流转'],
    operatorGuide: [
      { index: '01', title: '看任务', description: '确认预警已经生成任务。' },
      { index: '02', title: '看承办人', description: '核对责任人、优先级和截止时间。' },
      { index: '03', title: '进入评估', description: '任务流转后继续看成效评估。' },
    ],
    agents: ['alert-execution', 'supervision'],
  },
  {
    key: 'evaluation',
    path: '/evaluation',
    stageCode: 'S09',
    title: '成效评估',
    shortLabel: '查看维度得分与阶段性效果',
    summary: '展示当前评估指标和维度结果。',
    usageLead: '先看维度得分，再看是否满足进入监督和报告阶段的条件。',
    nextRequirement: '可读取评估指标或维度得分，支撑监督与报告输出。',
    focus: ['维度得分', '阶段效果', '准确率', '办结率'],
    ticker: ['评估维度', '得分结果', '办结率', '化解率', '效果评估'],
    operatorGuide: [
      { index: '01', title: '看维度', description: '确认当前已有评估维度和分数。' },
      { index: '02', title: '看指标', description: '结合指标判断阶段效果。' },
      { index: '03', title: '进入监督', description: '继续看监督与人工介入情况。' },
    ],
    agents: ['effect-evaluation', 'supervision'],
  },
  {
    key: 'supervision',
    path: '/supervision',
    stageCode: 'S10',
    title: '监督协调',
    shortLabel: '查看 Agent 运行和人工介入',
    summary: '明确每个 Agent 当前状态和运行任务数。',
    usageLead: '本页专门回答“哪一步用了 Agent、谁在运行、谁需要人工介入”。',
    nextRequirement: '监督面板可见，至少能跟踪一类 Agent 或任务运行状态。',
    focus: ['Agent 状态', '运行任务', '更新时间', '人工介入'],
    ticker: ['监督协调', 'Agent 状态', '任务数', '人工介入', '异常监控'],
    operatorGuide: [
      { index: '01', title: '看谁在跑', description: '确认各 Agent 的当前运行状态。' },
      { index: '02', title: '看任务量', description: '通过任务数判断当前处理压力。' },
      { index: '03', title: '处理异常', description: '必要时从这里触发人工介入。' },
    ],
    agents: ['clue-mining', 'risk-analysis', 'graded-alert', 'alert-execution', 'effect-evaluation', 'supervision'],
  },
  {
    key: 'reports',
    path: '/reports',
    stageCode: 'S11',
    title: '报告输出',
    shortLabel: '查看专题报告和周期报告',
    summary: '确认报告类型、周期、状态和生成时间。',
    usageLead: '本页用于查看已经形成的报告产物，支持阶段汇报和后续归档。',
    nextRequirement: '能够读取或生成至少一类报告列表，作为阶段成果输出。',
    focus: ['报告列表', '报告周期', '专题类型', '生成状态'],
    ticker: ['专题报告', '周期报告', '生成时间', '报告状态', '成果归档'],
    operatorGuide: [
      { index: '01', title: '看报告', description: '确认已有可读取的报告列表。' },
      { index: '02', title: '看周期', description: '核对报告类型、周期与状态。' },
      { index: '03', title: '完成归档', description: '报告可作为当前阶段输出成果。' },
    ],
    agents: ['effect-evaluation', 'supervision'],
  },
  {
    key: 'settings',
    path: '/settings',
    stageCode: 'S12',
    title: '平台设置',
    shortLabel: '查看模型、接口与集成配置',
    summary: '统一查看平台名称、模型和集成状态。',
    usageLead: '在流程尾部集中检查平台环境、模型和接口配置，便于后续持续联调。',
    nextRequirement: '维护平台配置与集成状态，作为流程收口与长期运维入口。',
    focus: ['平台名称', '默认模型', 'API 基路径', '集成状态'],
    ticker: ['平台名称', '默认模型', 'API 基路径', '集成状态', '离线运维'],
    operatorGuide: [
      { index: '01', title: '看环境', description: '先确认当前平台名称和环境。' },
      { index: '02', title: '看模型', description: '核对默认模型和接口基路径。' },
      { index: '03', title: '看集成', description: '确认全部集成项的连接状态。' },
    ],
    agents: ['supervision'],
  },
]

export const homeTickerItems = [
  '12345 数据',
  '110 接警',
  '395 平台',
  '信访线索',
  '线索挖掘 Agent',
  '风险研判 Agent',
  '分级推送 Agent',
  '预警执行 Agent',
  '成效评估 Agent',
  '监督协调 Agent',
]

export const homeFeatureHighlights = [
  {
    index: '01 / 归集',
    title: '多源数据 <em>统一进入</em>',
    description: '上传月度 Excel 后，先形成批次，再进入映射和抽取，不再停留在旧导入页。',
    metricValue: '4+',
    metricLabel: '来源入口',
  },
  {
    index: '02 / 抽取',
    title: '线索到 <em>实体关系</em>',
    description: '字段对齐后继续抽取人、事、项目、部门和历史纠纷关系，准备写入图谱。',
    metricValue: 'S03-S05',
    metricLabel: '中段处理',
  },
  {
    index: '03 / 预警',
    title: '风险到 <em>预警分派</em>',
    description: '高风险事项直接转成预警、任务和后续处置，不让研判停在看板层。',
    metricValue: 'S06-S08',
    metricLabel: '处置闭环',
  },
  {
    index: '04 / 监督',
    title: '结果到 <em>评估报告</em>',
    description: '最后查看 Agent 运行、成效评估和报告输出，形成完整闭环。',
    metricValue: 'S09-S12',
    metricLabel: '监督输出',
  },
]

export const homeOperatorGuide: OperatorGuideItem[] = [
  { index: '01', title: '先做系统准备', description: '确认后端、模型、图谱和目录状态已可读取。' },
  { index: '02', title: '上传月度数据', description: '从数据归集页上传 Excel，生成批次并查看导入详情。' },
  { index: '03', title: '按阶段推进', description: '依次看映射、抽取、图谱、风险、预警和分派。' },
  { index: '04', title: '最后看 Agent 与报告', description: '在监督协调和报告输出页看运行状态与最终成果。' },
]

export function getWorkflowStep(stepKey: WorkflowStepKey) {
  const step = workflowSteps.find((item) => item.key === stepKey)

  if (!step) {
    throw new Error(`Unknown workflow step: ${stepKey}`)
  }

  return step
}

export function getWorkflowStepIndex(stepKey: WorkflowStepKey) {
  return workflowSteps.findIndex((item) => item.key === stepKey)
}

export function getPreviousWorkflowStep(stepKey: WorkflowStepKey) {
  const currentIndex = getWorkflowStepIndex(stepKey)
  return currentIndex > 0 ? workflowSteps[currentIndex - 1] : null
}

export function getNextWorkflowStep(stepKey: WorkflowStepKey) {
  const currentIndex = getWorkflowStepIndex(stepKey)
  return currentIndex >= 0 && currentIndex < workflowSteps.length - 1 ? workflowSteps[currentIndex + 1] : null
}

export function toNavigationLink(step: WorkflowStepDefinition | null): WorkflowNavigationLink | null {
  if (!step) {
    return null
  }

  return {
    label: step.title,
    path: step.path,
  }
}

export function getAgent(agentKey: WorkflowAgentKey) {
  const agent = workflowAgents.find((item) => item.key === agentKey)

  if (!agent) {
    throw new Error(`Unknown workflow agent: ${agentKey}`)
  }

  return agent
}

export function getStepAgents(stepKey: WorkflowStepKey) {
  return getWorkflowStep(stepKey).agents.map(getAgent)
}
