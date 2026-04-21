export type StatusTone = 'healthy' | 'warning' | 'critical' | 'running' | 'ready' | 'draft' | 'completed' | 'open' | 'todo' | 'in_progress' | 'acknowledged' | 'attention' | 'configured'

export interface MetricCard {
  key: string
  label: string
  value: string
  unit: string | null
  trend: string | null
  trend_value: string | null
  status: StatusTone | string
}

export interface ProgressItem {
  key: string
  label: string
  status: StatusTone | string
  completed: number
  total: number
}

export interface QueueItem {
  key: string
  label: string
  count: number
  status: StatusTone | string
}

export interface DashboardOverviewResponse {
  generated_at: string
  metrics: MetricCard[]
  workflow: ProgressItem[]
  queues: QueueItem[]
}

export interface IngestionSourceSummary {
  source_key: string
  source_label: string
  batch_count: number
  record_count: number
  latest_import_at: string
  status: StatusTone | string
}

export interface IngestionSummaryResponse {
  generated_at: string
  totals: MetricCard[]
  sources: IngestionSourceSummary[]
}

export interface IngestionBatchItem {
  id: string
  source_key: string
  source_label: string
  file_name: string
  status: StatusTone | string
  record_count: number
  error_count: number
  imported_at: string
}

export interface IngestionBatchListResponse {
  generated_at: string
  items: IngestionBatchItem[]
}

export interface MappingFieldItem {
  source_field: string
  target_field: string
  confidence: number
  status: StatusTone | string
  sample_value: string
}

export interface MappingCurrentResponse {
  generated_at: string
  template_key: string
  template_label: string
  version: string
  status: StatusTone | string
  fields: MappingFieldItem[]
}

export interface ExtractionEntityItem {
  id: string
  entity_type: string
  name: string
  confidence: number
  extracted_at: string
}

export interface ExtractionSummaryResponse {
  generated_at: string
  metrics: MetricCard[]
  recent_entities: ExtractionEntityItem[]
}

export interface RelationTypeItem {
  key: string
  label: string
  count: number
}

export interface GraphOverviewResponse {
  generated_at: string
  metrics: MetricCard[]
  relation_types: RelationTypeItem[]
}

export interface RiskItem {
  id: string
  title: string
  level: string
  score: number
  area: string
  status: StatusTone | string
}

export interface RiskOverviewResponse {
  generated_at: string
  metrics: MetricCard[]
  top_risks: RiskItem[]
}

export interface AlertItem {
  id: string
  title: string
  level: string
  source: string
  triggered_at: string
  status: StatusTone | string
}

export interface AlertsSummaryResponse {
  generated_at: string
  metrics: MetricCard[]
  items: AlertItem[]
}

export interface DispatchTaskItem {
  id: string
  case_code: string
  title: string
  assignee: string
  priority: string
  status: StatusTone | string
  due_at: string
}

export interface DispatchTaskListResponse {
  generated_at: string
  items: DispatchTaskItem[]
}

export interface EvaluationDimensionItem {
  key: string
  label: string
  score: number
  status: StatusTone | string
}

export interface EvaluationSummaryResponse {
  generated_at: string
  metrics: MetricCard[]
  dimensions: EvaluationDimensionItem[]
}

export interface AgentStatusItem {
  key: string
  label: string
  status: StatusTone | string
  running_tasks: number
  updated_at: string
}

export interface SupervisionOverviewResponse {
  generated_at: string
  metrics: MetricCard[]
  agents: AgentStatusItem[]
}

export interface ReportItem {
  id: string
  title: string
  report_type: string
  period: string
  generated_at: string
  status: StatusTone | string
}

export interface ReportListResponse {
  generated_at: string
  items: ReportItem[]
}

export interface PlatformInfo {
  app_name: string
  environment: string
  api_base_path: string
  model_name: string
}

export interface IntegrationStatusItem {
  key: string
  label: string
  status: StatusTone | string
  endpoint: string
}

export interface PlatformSettingsResponse {
  generated_at: string
  platform: PlatformInfo
  integrations: IntegrationStatusItem[]
}
