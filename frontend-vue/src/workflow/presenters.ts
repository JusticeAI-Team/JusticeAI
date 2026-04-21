import { fetchAlertsSummary } from '../api/alerts'
import { fetchReports } from '../api/reports'
import { fetchRiskOverview } from '../api/risk'
import { fetchSupervisionOverview } from '../api/supervision'
import { fetchPlatformSettings } from '../api/system'
import {
  fetchDispatchTasks,
  fetchEvaluationSummary,
  fetchExtractionSummary,
  fetchGraphOverview,
  fetchMappingCurrent,
} from '../api/workflow'
import type { MetricCard } from '../types/workspace'
import type { WorkflowStepKey } from './catalog'
import {
  formatConfidence,
  formatDateTime,
  formatMetricValue,
  formatNumber,
  formatStatusLabel,
  resolveTone,
} from '../utils/format'

export type DataDrivenStepKey = Exclude<WorkflowStepKey, 'setup' | 'data-ingestion'>

export interface StageMetricDisplay {
  label: string
  value: string
  hint?: string
  tone?: string
}

export interface StageListItemDisplay {
  title: string
  subtitle?: string
  meta?: string
  status?: string
}

export interface StageKeyValueDisplay {
  label: string
  value: string
  hint?: string
}

export interface StageListSection {
  kind: 'list'
  title: string
  description: string
  emptyLabel?: string
  items: StageListItemDisplay[]
}

export interface StageTableSection {
  kind: 'table'
  title: string
  description: string
  emptyLabel?: string
  columns: string[]
  rows: string[][]
}

export interface StageKeyValueSection {
  kind: 'key-value'
  title: string
  description: string
  emptyLabel?: string
  items: StageKeyValueDisplay[]
}

export type StageSectionDisplay = StageListSection | StageTableSection | StageKeyValueSection

export interface StageReadiness {
  ready: boolean
  label: string
  detail: string
}

export interface StagePresentation {
  state: 'ready' | 'empty'
  generatedAt?: string
  metrics: StageMetricDisplay[]
  sections: StageSectionDisplay[]
  readiness: StageReadiness
  emptyTitle: string
  emptyDescription: string
}

function mapMetrics(metrics: MetricCard[]) {
  return metrics.map((metric) => ({
    label: metric.label,
    value: formatMetricValue(metric.value, metric.unit),
    hint: [metric.trend, metric.trend_value].filter(Boolean).join(' · '),
    tone: resolveTone(metric.status),
  }))
}

function average(values: number[]) {
  if (values.length === 0) {
    return 0
  }

  return values.reduce((sum, value) => sum + value, 0) / values.length
}

async function loadMappingPresentation(): Promise<StagePresentation> {
  const response = await fetchMappingCurrent()

  return {
    state: response.fields.length === 0 ? 'empty' : 'ready',
    generatedAt: response.generated_at,
    metrics: [
      {
        label: '当前模版',
        value: response.template_label || '未命名',
        hint: `模板标识 ${response.template_key}`,
        tone: 'accent',
      },
      {
        label: '版本',
        value: response.version,
        hint: `状态 ${formatStatusLabel(response.status)}`,
        tone: resolveTone(response.status),
      },
      {
        label: '字段数量',
        value: formatNumber(response.fields.length),
        hint: '当前映射模版已收录字段数',
        tone: response.fields.length > 0 ? 'good' : 'warning',
      },
      {
        label: '平均置信度',
        value: formatConfidence(average(response.fields.map((field) => field.confidence))),
        hint: '基于现有字段映射计算',
        tone: response.fields.length > 0 ? 'good' : 'neutral',
      },
    ],
    sections: [
      {
        kind: 'key-value',
        title: '模版信息',
        description: '当前字段映射模版的标识、版本和运行状态。',
        items: [
          { label: '模版名称', value: response.template_label || '未命名模版' },
          { label: '模版标识', value: response.template_key || '-' },
          { label: '当前版本', value: response.version || '-' },
          { label: '状态', value: formatStatusLabel(response.status) },
        ],
      },
      {
        kind: 'table',
        title: '字段映射清单',
        description: '源字段、目标字段、样例值与置信度的当前快照。',
        emptyLabel: '字段映射服务已接通，但当前尚未返回字段映射清单。',
        columns: ['源字段', '目标字段', '置信度', '状态', '样例值'],
        rows: response.fields.map((field) => [
          field.source_field,
          field.target_field,
          formatConfidence(field.confidence),
          formatStatusLabel(field.status),
          field.sample_value || '-',
        ]),
      },
    ],
    readiness: {
      ready: response.fields.length > 0,
      label: response.fields.length > 0 ? '条件已满足' : '仍需补齐',
      detail:
        response.fields.length > 0
          ? '已返回字段映射清单，可以进入知识抽取阶段。'
          : '字段映射服务已接通，但还没有可用于抽取的字段映射结果。',
    },
    emptyTitle: '暂无字段映射结果',
    emptyDescription: '请先在数据归集阶段完成样本上传，再等待或触发映射模版生成。',
  }
}

async function loadExtractionPresentation(): Promise<StagePresentation> {
  const response = await fetchExtractionSummary()

  return {
    state: response.recent_entities.length === 0 ? 'empty' : 'ready',
    generatedAt: response.generated_at,
    metrics: mapMetrics(response.metrics),
    sections: [
      {
        kind: 'list',
        title: '最近抽取实体',
        description: '展示最近识别出的重点实体，便于快速确认抽取链路是否工作。',
        emptyLabel: '当前没有实体抽取结果返回。',
        items: response.recent_entities.map((entity) => ({
          title: entity.name,
          subtitle: `${entity.entity_type} · 置信度 ${formatConfidence(entity.confidence)}`,
          meta: `抽取时间 ${formatDateTime(entity.extracted_at)}`,
          status: '已抽取',
        })),
      },
    ],
    readiness: {
      ready: response.recent_entities.length > 0,
      label: response.recent_entities.length > 0 ? '条件已满足' : '等待实体产出',
      detail:
        response.recent_entities.length > 0
          ? '至少已有一批实体结果，可以继续写入知识图谱。'
          : '抽取接口可访问，但还未返回实体候选结果。',
    },
    emptyTitle: '暂无知识抽取结果',
    emptyDescription: '请确认字段映射已经准备完成，并等待抽取任务返回实体与关系候选。',
  }
}

async function loadGraphPresentation(): Promise<StagePresentation> {
  const response = await fetchGraphOverview()

  return {
    state: response.relation_types.length === 0 ? 'empty' : 'ready',
    generatedAt: response.generated_at,
    metrics: mapMetrics(response.metrics),
    sections: [
      {
        kind: 'list',
        title: '关系类型概览',
        description: '图谱中当前已统计到的关系类型和数量。',
        emptyLabel: '图谱服务已接通，但还没有关系类型概览。',
        items: response.relation_types.map((relation) => ({
          title: relation.label,
          subtitle: `关系标识 ${relation.key}`,
          meta: `数量 ${formatNumber(relation.count)}`,
          status: '已写入',
        })),
      },
    ],
    readiness: {
      ready: response.relation_types.length > 0,
      label: response.relation_types.length > 0 ? '条件已满足' : '等待图谱写入',
      detail:
        response.relation_types.length > 0
          ? '已返回图谱关系类型，可进入风险研判阶段。'
          : '图谱概览接口可访问，但尚未出现关系类型数据。',
    },
    emptyTitle: '暂无图谱关系概览',
    emptyDescription: '请先确认知识抽取结果已经写入图谱，再进入风险研判。',
  }
}

async function loadRiskPresentation(): Promise<StagePresentation> {
  const response = await fetchRiskOverview()

  return {
    state: response.top_risks.length === 0 ? 'empty' : 'ready',
    generatedAt: response.generated_at,
    metrics: mapMetrics(response.metrics),
    sections: [
      {
        kind: 'list',
        title: '重点风险列表',
        description: '当前识别出的高风险事项，可继续转入预警中心。',
        emptyLabel: '风险研判接口已接通，但还没有重点风险列表。',
        items: response.top_risks.map((risk) => ({
          title: risk.title,
          subtitle: `${risk.level} · ${risk.area}`,
          meta: `评分 ${risk.score}`,
          status: formatStatusLabel(risk.status),
        })),
      },
    ],
    readiness: {
      ready: response.top_risks.length > 0,
      label: response.top_risks.length > 0 ? '条件已满足' : '等待风险结果',
      detail:
        response.top_risks.length > 0
          ? '已有风险评分结果，可以进入预警中心。'
          : '风险分析接口可访问，但还没有生成高风险事项。',
    },
    emptyTitle: '暂无风险研判结果',
    emptyDescription: '请确认图谱侧已沉淀有效关系，再等待风险评分与线索聚合输出。',
  }
}

async function loadAlertsPresentation(): Promise<StagePresentation> {
  const response = await fetchAlertsSummary()

  return {
    state: response.items.length === 0 ? 'empty' : 'ready',
    generatedAt: response.generated_at,
    metrics: mapMetrics(response.metrics),
    sections: [
      {
        kind: 'list',
        title: '预警事件',
        description: '展示当前触发的预警事件与处理状态。',
        emptyLabel: '预警中心已接通，当前暂无新增预警事件。',
        items: response.items.map((alert) => ({
          title: alert.title,
          subtitle: `${alert.level} · ${alert.source}`,
          meta: `触发时间 ${formatDateTime(alert.triggered_at)}`,
          status: formatStatusLabel(alert.status),
        })),
      },
    ],
    readiness: {
      ready: true,
      label: response.items.length > 0 ? '条件已满足' : '引擎已接通',
      detail:
        response.items.length > 0
          ? '已生成预警事件，可以继续流转到案件分派。'
          : '预警规则链路已接通，即使当前暂无事件也可继续联调整体流程。',
    },
    emptyTitle: '当前暂无预警事件',
    emptyDescription: '预警引擎已可访问，但暂时没有新增事件返回。',
  }
}

async function loadDispatchPresentation(): Promise<StagePresentation> {
  const response = await fetchDispatchTasks()

  return {
    state: response.items.length === 0 ? 'empty' : 'ready',
    generatedAt: response.generated_at,
    metrics: [
      {
        label: '派发任务数',
        value: formatNumber(response.items.length),
        hint: '当前返回的任务队列规模',
        tone: response.items.length > 0 ? 'good' : 'warning',
      },
    ],
    sections: [
      {
        kind: 'list',
        title: '任务队列',
        description: '承办人、优先级、状态和截止时间的当前快照。',
        emptyLabel: '派发接口已接通，但当前队列中暂无任务。',
        items: response.items.map((task) => ({
          title: task.title,
          subtitle: `${task.case_code} · ${task.assignee}`,
          meta: `优先级 ${task.priority} · 截止 ${formatDateTime(task.due_at)}`,
          status: formatStatusLabel(task.status),
        })),
      },
    ],
    readiness: {
      ready: true,
      label: response.items.length > 0 ? '条件已满足' : '队列已接通',
      detail:
        response.items.length > 0
          ? '已有可派发任务，可继续进入成效评估。'
          : '案件分派接口已接通，但当前尚未分配到具体任务。',
    },
    emptyTitle: '当前暂无派发任务',
    emptyDescription: '案件分派接口已经可用，但暂时还没有任务队列数据。',
  }
}

async function loadEvaluationPresentation(): Promise<StagePresentation> {
  const response = await fetchEvaluationSummary()

  return {
    state: response.dimensions.length === 0 ? 'empty' : 'ready',
    generatedAt: response.generated_at,
    metrics: mapMetrics(response.metrics),
    sections: [
      {
        kind: 'list',
        title: '评估维度',
        description: '结合题图中的评审方向，展示当前成效评估维度与得分。',
        emptyLabel: '评估接口已接通，但当前没有维度得分返回。',
        items: response.dimensions.map((dimension) => ({
          title: dimension.label,
          subtitle: `维度标识 ${dimension.key}`,
          meta: `得分 ${dimension.score}`,
          status: formatStatusLabel(dimension.status),
        })),
      },
    ],
    readiness: {
      ready: response.dimensions.length > 0 || response.metrics.length > 0,
      label: response.dimensions.length > 0 ? '条件已满足' : '指标已接通',
      detail:
        response.dimensions.length > 0
          ? '评估维度已经形成，可以进入监督协调阶段。'
          : '评估接口可访问，但暂未返回维度得分。',
    },
    emptyTitle: '暂无成效评估维度',
    emptyDescription: '请等待评估服务返回功能实现度、技术合理性、用户体验和应用价值等结果。',
  }
}

async function loadSupervisionPresentation(): Promise<StagePresentation> {
  const response = await fetchSupervisionOverview()

  return {
    state: response.agents.length === 0 ? 'empty' : 'ready',
    generatedAt: response.generated_at,
    metrics: mapMetrics(response.metrics),
    sections: [
      {
        kind: 'list',
        title: '监督对象',
        description: '多 Agent 运行状态、任务数与最近更新时间。',
        emptyLabel: '监督接口已接通，但当前没有 Agent 状态列表。',
        items: response.agents.map((agent) => ({
          title: agent.label,
          subtitle: `${agent.running_tasks} 个运行任务`,
          meta: `更新时间 ${formatDateTime(agent.updated_at)}`,
          status: formatStatusLabel(agent.status),
        })),
      },
    ],
    readiness: {
      ready: response.agents.length > 0 || response.metrics.length > 0,
      label: response.agents.length > 0 ? '条件已满足' : '监督面板已接通',
      detail:
        response.agents.length > 0
          ? '已可查看监督对象状态，可继续进入报告输出阶段。'
          : '监督面板接口可访问，但暂无 Agent 状态列表。',
    },
    emptyTitle: '暂无监督对象列表',
    emptyDescription: '监督协调接口已经可用，但还没有返回 Agent 运行状态。',
  }
}

async function loadReportsPresentation(): Promise<StagePresentation> {
  const response = await fetchReports()

  return {
    state: response.items.length === 0 ? 'empty' : 'ready',
    generatedAt: response.generated_at,
    metrics: [
      {
        label: '报告总数',
        value: formatNumber(response.items.length),
        hint: '当前已生成或可读取的报告数量',
        tone: response.items.length > 0 ? 'good' : 'warning',
      },
    ],
    sections: [
      {
        kind: 'list',
        title: '报告列表',
        description: '专题报告、周期报告等输出成果的当前快照。',
        emptyLabel: '报告接口已接通，但当前没有报告列表。',
        items: response.items.map((report) => ({
          title: report.title,
          subtitle: `${report.report_type} · 周期 ${report.period}`,
          meta: `生成时间 ${formatDateTime(report.generated_at)}`,
          status: formatStatusLabel(report.status),
        })),
      },
    ],
    readiness: {
      ready: true,
      label: response.items.length > 0 ? '条件已满足' : '输出链路已接通',
      detail:
        response.items.length > 0
          ? '报告链路已经有产物，可继续进入平台设置或后续精修。'
          : '报告输出接口已经接通，当前只是没有生成的报告内容。',
    },
    emptyTitle: '暂无报告输出',
    emptyDescription: '报告输出接口已可访问，等待专题报告或周期性产物生成。',
  }
}

async function loadSettingsPresentation(): Promise<StagePresentation> {
  const response = await fetchPlatformSettings()

  return {
    state: response.integrations.length === 0 ? 'empty' : 'ready',
    generatedAt: response.generated_at,
    metrics: [
      {
        label: '平台名称',
        value: response.platform.app_name || 'JusticeAI',
        hint: `环境 ${response.platform.environment}`,
        tone: 'accent',
      },
      {
        label: '模型',
        value: response.platform.model_name || '未配置',
        hint: '当前平台配置中的主模型',
        tone: response.platform.model_name ? 'good' : 'warning',
      },
      {
        label: 'API 基路径',
        value: response.platform.api_base_path || '/api',
        hint: '前后端通信入口',
        tone: 'neutral',
      },
      {
        label: '集成数量',
        value: formatNumber(response.integrations.length),
        hint: '当前可见的集成项总数',
        tone: response.integrations.length > 0 ? 'good' : 'warning',
      },
    ],
    sections: [
      {
        kind: 'key-value',
        title: '平台基础配置',
        description: '前端当前读到的平台基础设置。',
        items: [
          { label: '平台名称', value: response.platform.app_name || '-' },
          { label: '部署环境', value: response.platform.environment || '-' },
          { label: 'API 基路径', value: response.platform.api_base_path || '-' },
          { label: '默认模型', value: response.platform.model_name || '-' },
        ],
      },
      {
        kind: 'list',
        title: '集成状态',
        description: '模型、接口与外部依赖的配置状态快照。',
        emptyLabel: '设置接口已接通，但当前没有集成配置清单。',
        items: response.integrations.map((integration) => ({
          title: integration.label,
          subtitle: integration.endpoint,
          meta: `集成键 ${integration.key}`,
          status: formatStatusLabel(integration.status),
        })),
      },
    ],
    readiness: {
      ready: true,
      label: response.integrations.length > 0 ? '配置可维护' : '设置已接通',
      detail:
        response.integrations.length > 0
          ? '平台设置与集成状态可见，可作为流程收口与运维入口。'
          : '平台设置接口已经接通，但尚未读取到集成列表。',
    },
    emptyTitle: '暂无集成配置列表',
    emptyDescription: '平台设置接口已接通，但尚未读取到可展示的集成项。',
  }
}

export const stageLoaderMap: Record<DataDrivenStepKey, () => Promise<StagePresentation>> = {
  'data-mapping': loadMappingPresentation,
  'knowledge-extraction': loadExtractionPresentation,
  'knowledge-graph': loadGraphPresentation,
  'risk-analysis': loadRiskPresentation,
  alerts: loadAlertsPresentation,
  'case-dispatch': loadDispatchPresentation,
  evaluation: loadEvaluationPresentation,
  supervision: loadSupervisionPresentation,
  reports: loadReportsPresentation,
  settings: loadSettingsPresentation,
}
