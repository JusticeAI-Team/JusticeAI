use std::{collections::HashMap, fs};

use axum::{
    extract::{Path, Query, State},
    http::{header, HeaderMap, HeaderValue},
    response::IntoResponse,
    routing::{get, post},
    Json, Router,
};
use chrono::{DateTime, Duration, Utc};
use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use uuid::Uuid;

use crate::{
    app::AppState,
    services::{
        ai::{
            ModelContract as AiModelContract, OpenAiCompatibleAiService, RecommendationInput,
            RecommendationOutput, ReportInput,
        },
        embedding::{EmbeddingContract, OpenAiCompatibleEmbeddingService},
        graph::{GraphCaseSyncInput, GraphEntitySync, GraphRelationSync, HugeGraphSyncService},
        pipeline::{execute_extraction_run, process_import_batch},
        vector::{MilvusVectorStore, SimilarCaseHit, VectorCaseDocument, VectorSearchQuery},
    },
    shared::{
        error::AppError,
        response::{ok, ApiResponse},
    },
};

pub fn routes() -> Router<AppState> {
    Router::new()
        .route("/dashboard/overview", get(dashboard_overview))
        .route("/dashboard/summary", get(dashboard_summary))
        .route("/dashboard/stages", get(stage_summary))
        .route("/demo/full-flow", post(run_demo_full_flow))
        .route("/ingestion/summary", get(ingestion_summary_view))
        .route("/ingestion/batches", get(ingestion_batches))
        .route("/ingestion/list", get(ingestion_list))
        .route("/ingestion/:id", get(ingestion_detail))
        .route(
            "/ingestion/:id/process",
            post(process_ingestion_action_live),
        )
        .route(
            "/ingestion/:id/process/async",
            post(start_ingestion_processing_job),
        )
        .route("/imports/:id/process", post(process_ingestion_action_live))
        .route("/mapping/summary", get(mapping_summary))
        .route("/mapping/current", get(mapping_current))
        .route(
            "/mapping/templates",
            get(mapping_templates).post(save_mapping_template),
        )
        .route("/mapping/templates/:id", get(mapping_template_detail))
        .route("/mapping/validate", post(validate_mapping))
        .route("/processing/summary", get(processing_summary))
        .route("/processing/runs", get(processing_runs))
        .route("/processing/runs/:id", get(processing_run_detail))
        .route("/processing/runs/:id/retry", post(retry_processing_run))
        .route("/extraction/summary", get(extraction_summary_view))
        .route("/extraction/runs", get(extraction_runs))
        .route("/extraction/runs/:id", get(extraction_run_detail))
        .route("/extraction/run", post(create_extraction_run_live))
        .route("/extraction/run/async", post(start_extraction_job))
        .route("/graph/summary", get(graph_summary))
        .route("/graph/overview", get(graph_overview))
        .route("/graph/cases/:id", get(graph_case_view))
        .route("/graph/cases/:id/rebuild", post(rebuild_graph_case))
        .route("/vectors/cases/:id/rebuild", post(rebuild_case_vector))
        .route("/vectors/cases/:id/similar", get(case_vector_similar))
        .route("/risk/summary", get(risk_summary))
        .route("/risk/overview", get(risk_overview))
        .route("/risk/cases", get(risk_case_list))
        .route("/risk/cases/:id", get(risk_case_detail_view))
        .route("/risk/cases/:id/status", post(update_risk_case_status))
        .route("/agent/analyze", post(agent_analyze))
        .route("/alerts/summary", get(alerts_summary_view))
        .route("/alerts", get(alert_list))
        .route("/alerts/:id", get(alert_detail))
        .route("/alerts/:id/status", post(update_alert_status))
        .route("/dispatch/summary", get(dispatch_summary))
        .route(
            "/dispatch/tasks",
            get(dispatch_list).post(create_dispatch_task),
        )
        .route("/dispatch/tasks/:id", get(dispatch_detail))
        .route("/dispatch/tasks/:id/status", post(update_dispatch_status))
        .route("/evaluation/summary", get(evaluation_summary_view))
        .route("/evaluation/trends", get(evaluation_trends))
        .route("/supervision/overview", get(supervision_overview))
        .route("/supervision/summary", get(supervision_summary))
        .route("/supervision/failures", get(supervision_failures))
        .route("/jobs", get(job_list))
        .route("/jobs/:id", get(job_detail))
        .route("/jobs/:id/retry", post(retry_platform_job))
        .route("/reports/summary", get(report_summary))
        .route("/reports", get(report_list).post(create_report_live))
        .route("/reports/generate", post(create_report_live))
        .route("/reports/generate/async", post(start_report_job))
        .route("/reports/:id", get(report_detail))
        .route("/reports/:id/download", get(download_report))
        .route("/reports/:id/regenerate", post(regenerate_report))
        .route(
            "/settings/platform",
            get(get_platform_settings_view).post(save_platform_settings_view),
        )
        .route(
            "/settings/integrations",
            get(get_integrations_settings_view).post(save_integrations_settings),
        )
        .route("/settings/integrations/test", post(test_integrations_view))
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "snake_case")]
struct SummaryMetric {
    key: String,
    label: String,
    value: serde_json::Value,
    unit: Option<String>,
    status: String,
    is_placeholder: bool,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "snake_case")]
struct MetricCard {
    key: String,
    label: String,
    value: String,
    unit: Option<String>,
    trend: Option<String>,
    trend_value: Option<String>,
    status: String,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "snake_case")]
struct ProgressItem {
    key: String,
    label: String,
    status: String,
    completed: u32,
    total: u32,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "snake_case")]
struct QueueItem {
    key: String,
    label: String,
    count: u32,
    status: String,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "snake_case")]
struct DashboardOverviewResponse {
    generated_at: String,
    metrics: Vec<MetricCard>,
    workflow: Vec<ProgressItem>,
    queues: Vec<QueueItem>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "snake_case")]
struct SummaryResponse {
    generated_at: String,
    metrics: Vec<SummaryMetric>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "snake_case")]
struct StageSummaryResponse {
    generated_at: String,
    items: Vec<StageItem>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "snake_case")]
struct StageItem {
    stage_key: String,
    stage_label: String,
    status: String,
    item_count: i32,
    success_count: i32,
    failure_count: i32,
    started_at: String,
    finished_at: Option<String>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "snake_case")]
struct PageResponse<T> {
    generated_at: String,
    items: Vec<T>,
    page: i64,
    page_size: i64,
    total: i64,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "snake_case")]
struct ActionResponse {
    id: String,
    status: String,
    message: String,
    updated_at: String,
    is_placeholder: bool,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "snake_case")]
struct JobDto {
    id: String,
    job_type: String,
    target_type: String,
    target_id: Option<String>,
    status: String,
    progress_percent: i32,
    message: String,
    request: serde_json::Value,
    result: serde_json::Value,
    error_message: Option<String>,
    started_at: Option<String>,
    finished_at: Option<String>,
    created_at: String,
    updated_at: String,
}

#[derive(Debug, FromRow)]
struct JobRow {
    id: Uuid,
    job_type: String,
    target_type: String,
    target_id: Option<Uuid>,
    status: String,
    progress_percent: i32,
    message: String,
    request_json: String,
    result_json: String,
    error_message: Option<String>,
    started_at: Option<DateTime<Utc>>,
    finished_at: Option<DateTime<Utc>>,
    created_at: DateTime<Utc>,
    updated_at: DateTime<Utc>,
}

#[derive(Debug, Deserialize)]
struct PaginationQuery {
    page: Option<i64>,
    page_size: Option<i64>,
    status: Option<String>,
    source_type: Option<String>,
    area_name: Option<String>,
    risk_level: Option<String>,
    import_id: Option<Uuid>,
}

#[derive(Debug, Deserialize)]
struct JobQuery {
    page: Option<i64>,
    page_size: Option<i64>,
    status: Option<String>,
    job_type: Option<String>,
}

#[derive(Debug, Deserialize)]
struct SimilarQuery {
    limit: Option<usize>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "snake_case")]
struct MappingFieldInput {
    source_field: String,
    target_field: String,
    confidence: f32,
    status: String,
    sample_value: String,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "snake_case")]
struct SaveMappingTemplateRequest {
    template_key: String,
    template_label: String,
    version: String,
    status: String,
    source_type: String,
    fields: Vec<MappingFieldInput>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "snake_case")]
struct ValidateMappingRequest {
    source_type: String,
    required_fields: Vec<String>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "snake_case")]
struct UpdateStatusRequest {
    status: String,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "snake_case")]
struct CreateExtractionRunRequest {
    case_ids: Option<Vec<Uuid>>,
    mode: Option<String>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "snake_case")]
struct CreateDispatchTaskRequest {
    case_id: Uuid,
    title: Option<String>,
    assignee: String,
    priority: Option<String>,
    due_at: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "snake_case")]
struct CreateReportRequest {
    report_type: String,
    period: String,
    title: Option<String>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "snake_case")]
struct AgentAnalyzeRequest {
    query: Option<String>,
    case_id: Option<Uuid>,
    intent: Option<String>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "snake_case")]
struct SavePlatformSettingsRequest {
    platform_name: Option<String>,
    environment: Option<String>,
    upload_dir: Option<String>,
    report_dir: Option<String>,
    training_dir: Option<String>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "snake_case")]
struct SaveIntegrationsSettingsRequest {
    hugegraph_base_url: Option<String>,
    hugegraph_gremlin_url: Option<String>,
    milvus_address: Option<String>,
    milvus_token: Option<String>,
    milvus_collection: Option<String>,
    model_base_url: Option<String>,
    model_name: Option<String>,
    model_request_style: Option<String>,
    model_chat_endpoint: Option<String>,
    model_json_mode_supported: Option<bool>,
    openai_api_key: Option<String>,
    embedding_base_url: Option<String>,
    embedding_model: Option<String>,
    embedding_api_key: Option<String>,
    embedding_endpoint: Option<String>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "snake_case")]
struct TestIntegrationsRequest {
    hugegraph_base_url: Option<String>,
    milvus_address: Option<String>,
    milvus_token: Option<String>,
    model_base_url: Option<String>,
    model_name: Option<String>,
    model_chat_endpoint: Option<String>,
    openai_api_key: Option<String>,
    embedding_base_url: Option<String>,
    embedding_model: Option<String>,
    embedding_endpoint: Option<String>,
}

#[derive(Debug, Serialize, FromRow)]
#[serde(rename_all = "snake_case")]
struct IngestionItem {
    id: Uuid,
    source_type: String,
    status: String,
    created_at: DateTime<Utc>,
    updated_at: DateTime<Utc>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "snake_case")]
struct IngestionDetailResponse {
    id: String,
    source_type: String,
    status: String,
    created_at: String,
    updated_at: String,
    files: Vec<ImportFileDto>,
    process_summary: ProcessSummaryDto,
}

#[derive(Debug, Serialize, FromRow)]
#[serde(rename_all = "snake_case")]
struct ImportFileDto {
    id: Uuid,
    original_filename: String,
    stored_filename: String,
    stored_path: String,
    file_size: i64,
    mime_type: Option<String>,
    created_at: DateTime<Utc>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "snake_case")]
struct ProcessSummaryDto {
    created_case_count: i64,
    processing_run_count: i64,
    latest_status: String,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "snake_case")]
struct IngestionSourceSummary {
    source_key: String,
    source_label: String,
    batch_count: u32,
    record_count: u32,
    latest_import_at: String,
    status: String,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "snake_case")]
struct IngestionSummaryResponse {
    generated_at: String,
    totals: Vec<MetricCard>,
    sources: Vec<IngestionSourceSummary>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "snake_case")]
struct IngestionBatchItem {
    id: String,
    source_key: String,
    source_label: String,
    file_name: String,
    status: String,
    record_count: i32,
    error_count: i32,
    imported_at: String,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "snake_case")]
struct IngestionBatchListResponse {
    generated_at: String,
    items: Vec<IngestionBatchItem>,
}

#[derive(Debug, FromRow)]
struct IngestionSourceRow {
    source_type: String,
    batch_count: i64,
    record_count: i64,
    latest_import_at: DateTime<Utc>,
}

#[derive(Debug, FromRow)]
struct IngestionBatchRow {
    id: Uuid,
    source_type: String,
    status: String,
    created_at: DateTime<Utc>,
    record_count: i64,
    error_count: i64,
    file_name: Option<String>,
}

#[derive(Debug, Serialize, FromRow)]
#[serde(rename_all = "snake_case")]
struct MappingTemplateListItem {
    id: Uuid,
    template_key: String,
    template_label: String,
    version: String,
    status: String,
    source_type: String,
    is_active: bool,
    created_at: DateTime<Utc>,
    updated_at: DateTime<Utc>,
}

#[derive(Debug, Serialize, FromRow)]
#[serde(rename_all = "snake_case")]
struct MappingFieldDto {
    source_field: String,
    target_field: String,
    confidence: f64,
    status: String,
    sample_value: String,
    sort_order: i32,
    required: bool,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "snake_case")]
struct MappingFieldItem {
    source_field: String,
    target_field: String,
    confidence: f32,
    status: String,
    sample_value: String,
    required: bool,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "snake_case")]
struct MappingTemplateDetailResponse {
    id: String,
    template_key: String,
    template_label: String,
    version: String,
    status: String,
    source_type: String,
    is_active: bool,
    fields: Vec<MappingFieldDto>,
    completion_rate: f32,
    missing_required_fields: Vec<String>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "snake_case")]
struct MappingCurrentResponse {
    generated_at: String,
    template_id: String,
    template_key: String,
    template_label: String,
    version: String,
    status: String,
    source_type: String,
    completion_rate: f32,
    missing_required_fields: Vec<String>,
    fields: Vec<MappingFieldItem>,
}

#[derive(Debug, Serialize, FromRow)]
#[serde(rename_all = "snake_case")]
struct WorkflowRunDto {
    id: Uuid,
    stage_key: String,
    stage_label: String,
    status: String,
    started_at: DateTime<Utc>,
    finished_at: Option<DateTime<Utc>>,
    item_count: i32,
    success_count: i32,
    failure_count: i32,
    created_at: DateTime<Utc>,
    updated_at: DateTime<Utc>,
}

#[derive(Debug, Serialize, FromRow)]
#[serde(rename_all = "snake_case")]
struct ExtractionRunDto {
    id: Uuid,
    scope_type: String,
    mode: String,
    status: String,
    item_count: i32,
    success_count: i32,
    failure_count: i32,
    summary: Option<String>,
    provider_style: String,
    model_name: Option<String>,
    graph_sync_status: String,
    graph_sync_message: String,
    vector_sync_status: String,
    vector_sync_message: String,
    started_at: DateTime<Utc>,
    finished_at: Option<DateTime<Utc>>,
    created_at: DateTime<Utc>,
    updated_at: DateTime<Utc>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "snake_case")]
struct ExtractionEntityItem {
    id: String,
    entity_type: String,
    name: String,
    confidence: f32,
    extracted_at: String,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "snake_case")]
struct ExtractionSummaryResponse {
    generated_at: String,
    metrics: Vec<MetricCard>,
    recent_entities: Vec<ExtractionEntityItem>,
}

#[derive(Debug, FromRow)]
struct ExtractionEntityRow {
    id: Uuid,
    entity_type: String,
    entity_name: String,
    confidence: f64,
    extracted_at: DateTime<Utc>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "snake_case")]
struct GraphCaseResponse {
    case_id: String,
    nodes: Vec<GraphNodeDto>,
    edges: Vec<GraphEdgeDto>,
    sync_target: GraphSyncTarget,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "snake_case")]
struct GraphNodeDto {
    id: String,
    node_type: String,
    label: String,
    confidence: f64,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "snake_case")]
struct GraphEdgeDto {
    id: String,
    relation_type: String,
    source: String,
    target: String,
    confidence: f64,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "snake_case")]
struct GraphSyncTarget {
    provider: String,
    status: String,
    is_placeholder: bool,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "snake_case")]
struct RelationTypeItem {
    key: String,
    label: String,
    count: u32,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "snake_case")]
struct GraphOverviewResponse {
    generated_at: String,
    metrics: Vec<MetricCard>,
    relation_types: Vec<RelationTypeItem>,
}

#[derive(Debug, FromRow)]
struct RelationTypeRow {
    relation_type: String,
    relation_count: i64,
}

#[derive(Debug, Serialize, FromRow)]
#[serde(rename_all = "snake_case")]
struct RiskCaseListItem {
    id: Uuid,
    case_code: String,
    title: String,
    source_type: String,
    area_name: String,
    risk_level: String,
    risk_score: f64,
    status: String,
    alert_status: String,
    assignee: Option<String>,
    occurred_at: Option<DateTime<Utc>>,
    due_at: Option<DateTime<Utc>>,
    closed_at: Option<DateTime<Utc>>,
    report_period: Option<String>,
    review_status: String,
    risk_tags: String,
    risk_reason_summary: String,
    disposal_advice: String,
    graph_sync_status: String,
    graph_sync_message: String,
    graph_synced_at: Option<DateTime<Utc>>,
    vector_sync_status: String,
    vector_sync_message: String,
    vector_synced_at: Option<DateTime<Utc>>,
    created_at: DateTime<Utc>,
    updated_at: DateTime<Utc>,
}

#[derive(Debug, FromRow)]
struct RiskCaseRow {
    id: Uuid,
    title: String,
    risk_level: String,
    risk_score: f64,
    area_name: String,
    status: String,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "snake_case")]
struct RiskItem {
    id: String,
    title: String,
    level: String,
    score: f32,
    area: String,
    status: String,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "snake_case")]
struct RiskOverviewResponse {
    generated_at: String,
    metrics: Vec<MetricCard>,
    top_risks: Vec<RiskItem>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "snake_case")]
struct RiskCaseView {
    id: String,
    case_code: String,
    title: String,
    source_type: String,
    area_name: String,
    risk_level: String,
    risk_score: f64,
    status: String,
    alert_status: String,
    assignee: Option<String>,
    occurred_at: Option<String>,
    due_at: Option<String>,
    closed_at: Option<String>,
    report_period: Option<String>,
    review_status: String,
    risk_tags: Vec<String>,
    risk_reason_summary: String,
    disposal_advice: Vec<String>,
    graph_sync_status: String,
    graph_sync_message: String,
    graph_synced_at: Option<String>,
    vector_sync_status: String,
    vector_sync_message: String,
    vector_synced_at: Option<String>,
    created_at: String,
    updated_at: String,
}

#[derive(Debug, Serialize, FromRow)]
#[serde(rename_all = "snake_case")]
struct KnowledgeEntityView {
    id: Uuid,
    entity_type: String,
    entity_name: String,
    confidence: f64,
    extracted_at: DateTime<Utc>,
}

#[derive(Debug, Serialize, FromRow)]
#[serde(rename_all = "snake_case")]
struct GraphRelationView {
    id: Uuid,
    relation_type: String,
    source_entity_id: Uuid,
    target_entity_id: Uuid,
    confidence: f64,
    created_at: DateTime<Utc>,
}

#[derive(Debug, FromRow)]
struct GraphCaseSyncRow {
    id: Uuid,
    case_code: String,
    title: String,
    area_name: String,
    risk_level: String,
    source_type: String,
}

#[derive(Debug, FromRow)]
struct GraphEntitySyncRow {
    entity_type: String,
    entity_name: String,
    confidence: f64,
}

#[derive(Debug, FromRow)]
struct GraphRelationSyncRow {
    relation_type: String,
    source_entity_name: String,
    target_entity_name: String,
    confidence: f64,
}

#[derive(Debug, Serialize, FromRow)]
#[serde(rename_all = "snake_case")]
struct AlertView {
    id: Uuid,
    case_id: Uuid,
    title: String,
    severity: String,
    status: String,
    summary: String,
    created_at: DateTime<Utc>,
    updated_at: DateTime<Utc>,
    handled_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Serialize, FromRow)]
#[serde(rename_all = "snake_case")]
struct DispatchTaskView {
    id: Uuid,
    case_id: Uuid,
    case_code: String,
    title: String,
    assignee: String,
    priority: String,
    status: String,
    progress_note: Option<String>,
    due_at: Option<DateTime<Utc>>,
    completed_at: Option<DateTime<Utc>>,
    feedback_result: Option<String>,
    created_at: DateTime<Utc>,
    updated_at: DateTime<Utc>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "snake_case")]
struct RiskCaseDetailResponse {
    case_info: RiskCaseView,
    entities: Vec<KnowledgeEntityView>,
    relations: Vec<GraphRelationView>,
    alerts: Vec<AlertView>,
    dispatch_tasks: Vec<DispatchTaskView>,
    recommendations: RecommendationBundle,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "snake_case")]
struct SimilarCaseReference {
    id: String,
    case_id: String,
    case_code: String,
    title: String,
    risk_level: String,
    score: f64,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "snake_case")]
struct RecommendationBundle {
    reason_summary: String,
    disposal_advice: Vec<String>,
    reference_cases: Vec<SimilarCaseReference>,
    is_placeholder: bool,
    model_contract: AiModelContract,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "snake_case")]
struct AgentGraphNode {
    id: String,
    label: String,
    node_type: String,
    size: u32,
    color: String,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "snake_case")]
struct AgentGraphEdge {
    source: String,
    target: String,
    label: String,
    confidence: f64,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "snake_case")]
struct AgentAnalyzeResponse {
    generated_at: String,
    intent: String,
    matched_case_id: String,
    matched_by: String,
    query: String,
    case_detail: RiskCaseDetailResponse,
    answer_markdown: String,
    graph: AgentGraphPayload,
    sync_status: AgentSyncStatus,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "snake_case")]
struct AgentGraphPayload {
    nodes: Vec<AgentGraphNode>,
    edges: Vec<AgentGraphEdge>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "snake_case")]
struct AgentSyncStatus {
    graph_sync_status: String,
    graph_sync_message: String,
    vector_sync_status: String,
    vector_sync_message: String,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "snake_case")]
struct VectorActionResponse {
    case_id: String,
    case_code: String,
    status: String,
    message: String,
    embedding_dimension: usize,
    model_contract: EmbeddingContract,
    updated_at: String,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "snake_case")]
struct SimilarCaseResponse {
    case_id: String,
    case_code: String,
    query_text: String,
    embedding_dimension: usize,
    model_contract: EmbeddingContract,
    items: Vec<SimilarCaseReference>,
    generated_at: String,
}

#[derive(Debug, Serialize, FromRow)]
#[serde(rename_all = "snake_case")]
struct AlertListItem {
    id: Uuid,
    case_id: Uuid,
    title: String,
    severity: String,
    status: String,
    summary: String,
    created_at: DateTime<Utc>,
    updated_at: DateTime<Utc>,
    handled_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "snake_case")]
struct AlertItem {
    id: String,
    title: String,
    level: String,
    source: String,
    triggered_at: String,
    status: String,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "snake_case")]
struct AlertsSummaryResponse {
    generated_at: String,
    metrics: Vec<MetricCard>,
    items: Vec<AlertItem>,
}

#[derive(Debug, Serialize, FromRow)]
#[serde(rename_all = "snake_case")]
struct DispatchTaskListItem {
    id: Uuid,
    case_id: Uuid,
    case_code: String,
    title: String,
    assignee: String,
    priority: String,
    status: String,
    progress_note: Option<String>,
    due_at: Option<DateTime<Utc>>,
    completed_at: Option<DateTime<Utc>>,
    feedback_result: Option<String>,
    created_at: DateTime<Utc>,
    updated_at: DateTime<Utc>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "snake_case")]
struct EvaluationDimensionItem {
    key: String,
    label: String,
    score: f64,
    status: String,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "snake_case")]
struct EvaluationSummaryResponse {
    generated_at: String,
    metrics: Vec<MetricCard>,
    dimensions: Vec<EvaluationDimensionItem>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "snake_case")]
struct TrendPoint {
    period: String,
    value: f64,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "snake_case")]
struct EvaluationTrendResponse {
    generated_at: String,
    closure_rate: Vec<TrendPoint>,
    alert_accuracy_placeholder: Vec<TrendPoint>,
    review_pass_rate_placeholder: Vec<TrendPoint>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "snake_case")]
struct AgentStatusItem {
    key: String,
    label: String,
    status: String,
    running_tasks: u32,
    updated_at: String,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "snake_case")]
struct SupervisionOverviewResponse {
    generated_at: String,
    metrics: Vec<MetricCard>,
    agents: Vec<AgentStatusItem>,
}

#[derive(Debug, Serialize, FromRow)]
#[serde(rename_all = "snake_case")]
struct ReportListItem {
    id: Uuid,
    title: String,
    report_type: String,
    period: String,
    status: String,
    file_path: Option<String>,
    summary: Option<String>,
    provider_style: String,
    model_name: Option<String>,
    generated_at: DateTime<Utc>,
    created_at: DateTime<Utc>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "snake_case")]
struct ReportDetailItem {
    #[serde(flatten)]
    report: ReportListItem,
    content_markdown: Option<String>,
}

#[derive(Debug, Serialize, FromRow)]
#[serde(rename_all = "snake_case")]
struct PlatformSettingRow {
    setting_key: String,
    setting_value: String,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "snake_case")]
struct PlatformInfo {
    app_name: String,
    environment: String,
    api_base_path: String,
    model_name: String,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "snake_case")]
struct IntegrationStatusItem {
    key: String,
    label: String,
    status: String,
    endpoint: String,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "snake_case")]
struct PlatformSettingsResponse {
    generated_at: String,
    platform: PlatformInfo,
    integrations: Vec<IntegrationStatusItem>,
    storage: HashMap<String, String>,
    model_contract: AiModelContract,
    embedding_contract: EmbeddingContract,
    is_placeholder: bool,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "snake_case")]
struct IntegrationStatus {
    key: String,
    endpoint: String,
    status: String,
    configured: bool,
    message: String,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "snake_case")]
struct ModelIntegrationStatus {
    key: String,
    endpoint: String,
    status: String,
    configured: bool,
    message: String,
    request_style: String,
    model: String,
    api_key_configured: bool,
    chat_endpoint: String,
    json_mode_supported: bool,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "snake_case")]
struct IntegrationSettingsResponse {
    generated_at: String,
    database: IntegrationStatus,
    hugegraph: IntegrationStatus,
    milvus: IntegrationStatus,
    model_service: ModelIntegrationStatus,
    embedding_service: ModelIntegrationStatus,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "snake_case")]
struct IntegrationTestResponse {
    generated_at: String,
    hugegraph: IntegrationStatus,
    milvus: IntegrationStatus,
    model_service: ModelIntegrationStatus,
    embedding_service: ModelIntegrationStatus,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "snake_case")]
struct DemoFullFlowResponse {
    generated_at: String,
    demo_mode: bool,
    import_id: String,
    extraction_run_id: String,
    report_id: String,
    case_ids: Vec<String>,
    stages: Vec<DemoStage>,
    metrics: Vec<MetricCard>,
    notes: Vec<String>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "snake_case")]
struct DemoStage {
    key: String,
    label: String,
    status: String,
    detail: String,
    count: i32,
}

struct DemoCaseSeed {
    case_code: &'static str,
    title: &'static str,
    source_type: &'static str,
    area_name: &'static str,
    risk_level: &'static str,
    risk_score: f64,
    status: &'static str,
    alert_status: &'static str,
    assignee: &'static str,
    risk_tags: &'static str,
    reason: &'static str,
    advice: &'static str,
    entity_names: [&'static str; 3],
}

async fn run_demo_full_flow(
    State(state): State<AppState>,
) -> Result<Json<ApiResponse<DemoFullFlowResponse>>, AppError> {
    let now_at = Utc::now();
    let import_id = Uuid::new_v4();
    let extraction_run_id = Uuid::new_v4();
    let report_id = Uuid::new_v4();
    let mapping_template_id = Uuid::new_v4();
    let demo_cases = demo_case_seeds();

    reset_demo_full_flow_data(state.db()).await?;
    upsert_demo_mapping_template(state.db(), mapping_template_id, now_at).await?;
    insert_demo_import(
        state.db(),
        import_id,
        mapping_template_id,
        demo_cases.len() as i32,
        now_at,
    )
    .await?;
    insert_demo_workflow_runs(state.db(), now_at).await?;

    let mut case_ids = Vec::with_capacity(demo_cases.len());
    let mut entity_count = 0_i32;
    let mut relation_count = 0_i32;
    for (index, seed) in demo_cases.iter().enumerate() {
        let case_id = insert_demo_case(state.db(), import_id, seed, index as i64, now_at).await?;
        let entity_ids = insert_demo_entities(state.db(), case_id, seed, now_at).await?;
        insert_demo_relations(state.db(), &entity_ids, now_at).await?;
        insert_demo_alert(state.db(), case_id, seed, index as i64, now_at).await?;
        if matches!(seed.risk_level, "high" | "medium") {
            insert_demo_dispatch_task(state.db(), case_id, seed, index as i64, now_at).await?;
        }
        entity_count += entity_ids.len() as i32;
        relation_count += 2;
        case_ids.push(case_id);
    }

    insert_demo_extraction_run(
        state.db(),
        extraction_run_id,
        demo_cases.len() as i32,
        entity_count,
        relation_count,
        now_at,
    )
    .await?;
    insert_demo_report(&state, report_id, demo_cases.len() as i32, now_at).await?;

    Ok(ok(DemoFullFlowResponse {
        generated_at: now(),
        demo_mode: true,
        import_id: import_id.to_string(),
        extraction_run_id: extraction_run_id.to_string(),
        report_id: report_id.to_string(),
        case_ids: case_ids.into_iter().map(|id| id.to_string()).collect(),
        stages: vec![
            demo_stage("readiness", "系统准备", "completed", "PostgreSQL、vLLM、HugeGraph、Embedding 状态可展示；Milvus 使用演示向量状态兜底。", 5),
            demo_stage("ingestion", "数据导入", "completed", "已生成一批通州多源样例数据导入批次。", demo_cases.len() as i32),
            demo_stage("mapping", "字段映射", "completed", "已生成 12345/110/信访/395 统一字段映射模板。", 8),
            demo_stage("processing", "数据处理入库", "completed", "已转换为标准风险案件并写入风险原因、建议、标签。", demo_cases.len() as i32),
            demo_stage("extraction", "知识抽取", "completed", "已生成演示实体和关系，供图谱页面展示。", entity_count),
            demo_stage("graph_vector", "图谱/向量同步", "completed", "HugeGraph 标记 synced；Milvus 未就绪时使用 indexed_demo 明确演示兜底。", relation_count),
            demo_stage("risk", "风险研判", "completed", "已覆盖高/中风险案件、处置建议和人工复核状态。", demo_cases.len() as i32),
            demo_stage("alerts", "预警管理", "completed", "已生成预警记录，支持确认/关闭演示。", demo_cases.len() as i32),
            demo_stage("dispatch", "任务分派", "completed", "已生成责任人、优先级、截止时间和进展记录。", 3),
            demo_stage("report", "报告生成", "completed", "已生成一份基层治理风险研判演示报告。", 1),
        ],
        metrics: vec![
            metric_card("demo_cases", "Demo Cases", demo_cases.len().to_string(), None, None, None, "healthy"),
            metric_card("demo_entities", "Demo Entities", entity_count.to_string(), None, None, None, "healthy"),
            metric_card("demo_relations", "Demo Relations", relation_count.to_string(), None, None, None, "healthy"),
            metric_card("demo_report", "Demo Report", "1".to_string(), None, None, None, "healthy"),
        ],
        notes: vec![
            "这是演示闭环数据，会清理并重建 DEMO-TZ-* 案件，不影响真实导入批次。".to_string(),
            "Milvus 未运行时向量状态使用 indexed_demo，页面可展示完整闭环，但运维页仍会真实显示 Milvus down。".to_string(),
            "后续 OCR/Milvus 修复后，可把 demo fallback 替换成真实外部同步结果。".to_string(),
        ],
    }))
}

fn demo_stage(key: &str, label: &str, status: &str, detail: &str, count: i32) -> DemoStage {
    DemoStage {
        key: key.to_string(),
        label: label.to_string(),
        status: status.to_string(),
        detail: detail.to_string(),
        count,
    }
}

fn demo_case_seeds() -> Vec<DemoCaseSeed> {
    vec![
        DemoCaseSeed {
            case_code: "DEMO-TZ-2026-0001",
            title: "12345 食品安全投诉触发商户合规风险",
            source_type: "hotline_12345",
            area_name: "九棵树街道",
            risk_level: "medium",
            risk_score: 78.6,
            status: "disposed",
            alert_status: "acknowledged",
            assignee: "市场监管联络员",
            risk_tags: "食品安全,12345热线,消费投诉,商户监管",
            reason: "多次投诉指向同一餐饮门店，涉及食品异物、沟通态度和行政调解诉求，存在监管复核和舆情扩散风险。",
            advice: "联动市场监管所复核商户进货票据、消杀记录和现场整改情况 | 对同主体历史诉求做相似案件回溯 | 对答复不满意情形安排二次回访",
            entity_names: ["源麦甜(通州店)", "九棵树街道", "食品安全投诉"],
        },
        DemoCaseSeed {
            case_code: "DEMO-TZ-2026-0002",
            title: "110 接警信息关联工地劳资纠纷升级",
            source_type: "police_110",
            area_name: "马驹桥镇",
            risk_level: "high",
            risk_score: 91.2,
            status: "in_progress",
            alert_status: "open",
            assignee: "马驹桥镇综治办",
            risk_tags: "110警情,劳资欠薪,群体性风险,施工项目",
            reason: "接警内容与 12345 欠薪诉求、395 平台项目人员数据形成交叉印证，存在聚集讨薪和现场冲突风险。",
            advice: "立即核实总包和分包责任链 | 建立欠薪人员清单和工资支付台账 | 检察建议前置介入并同步属地公安维持现场秩序",
            entity_names: ["华丰建设项目", "马驹桥镇", "欠薪警情"],
        },
        DemoCaseSeed {
            case_code: "DEMO-TZ-2026-0003",
            title: "区综治信访重复反映土地补偿争议",
            source_type: "petitions",
            area_name: "潞城镇",
            risk_level: "medium",
            risk_score: 74.4,
            status: "pending_review",
            alert_status: "open",
            assignee: "区综治中心",
            risk_tags: "信访事项,土地补偿,重复访,历史遗留",
            reason: "信访人长期反映土地补偿和历史裁判问题，诉求跨度长、材料复杂，存在重复访和多部门流转低效风险。",
            advice: "梳理历史办理和裁判节点 | 明确属地、法院、行政机关责任边界 | 形成一次性告知与司法救助可行性评估",
            entity_names: ["潞城镇三元村", "土地补偿争议", "区综治中心"],
        },
        DemoCaseSeed {
            case_code: "DEMO-TZ-2026-0004",
            title: "区检察院信访反映民事裁判监督线索",
            source_type: "procuratorate_petition",
            area_name: "新华街道",
            risk_level: "medium",
            risk_score: 76.8,
            status: "pending_review",
            alert_status: "acknowledged",
            assignee: "民事检察部门",
            risk_tags: "民事检察,裁判监督,来信来访,程序审查",
            reason: "来信材料指向民事裁判监督和程序性异议，需要进行案件来源、裁判文书和当事人诉求聚合审查。",
            advice: "调取原案卷宗和裁判文书 | 校验是否符合监督受理条件 | 对不符合条件的事项生成释法说理答复要点",
            entity_names: ["北京市通州区人民检察院", "民事裁判监督线索", "新华街道"],
        },
        DemoCaseSeed {
            case_code: "DEMO-TZ-2026-0005",
            title: "395 平台项目人员数据提示欠薪企业风险",
            source_type: "platform_395",
            area_name: "台湖镇",
            risk_level: "high",
            risk_score: 88.9,
            status: "in_progress",
            alert_status: "open",
            assignee: "住建联络员",
            risk_tags: "395平台,工程建设,总包责任,工资支付",
            reason: "项目人员进退场、企业编码和总包信息与欠薪诉求高度相关，存在总包履责不到位和工程款链条风险。",
            advice: "核对实名制平台人员和工资专户流水 | 锁定总包联系人并要求限期反馈 | 对同企业历史项目进行图谱扩展排查",
            entity_names: ["北京华鼎新铭智能科技发展有限公司", "台湖镇项目", "总包工资责任"],
        },
    ]
}

async fn reset_demo_full_flow_data(db: &sqlx::PgPool) -> Result<(), AppError> {
    sqlx::query(
        r#"
        DELETE FROM graph_relations
        WHERE source_entity_id IN (
            SELECT ke.id FROM knowledge_entities ke
            JOIN risk_cases rc ON rc.id = ke.case_id
            WHERE rc.case_code LIKE 'DEMO-TZ-%'
        )
        OR target_entity_id IN (
            SELECT ke.id FROM knowledge_entities ke
            JOIN risk_cases rc ON rc.id = ke.case_id
            WHERE rc.case_code LIKE 'DEMO-TZ-%'
        )
        "#,
    )
    .execute(db)
    .await
    .map_err(|_| AppError::Internal)?;

    sqlx::query("DELETE FROM knowledge_entities WHERE case_id IN (SELECT id FROM risk_cases WHERE case_code LIKE 'DEMO-TZ-%')")
        .execute(db)
        .await
        .map_err(|_| AppError::Internal)?;
    sqlx::query("DELETE FROM dispatch_tasks WHERE case_id IN (SELECT id FROM risk_cases WHERE case_code LIKE 'DEMO-TZ-%')")
        .execute(db)
        .await
        .map_err(|_| AppError::Internal)?;
    sqlx::query("DELETE FROM alerts WHERE case_id IN (SELECT id FROM risk_cases WHERE case_code LIKE 'DEMO-TZ-%')")
        .execute(db)
        .await
        .map_err(|_| AppError::Internal)?;
    sqlx::query("DELETE FROM risk_cases WHERE case_code LIKE 'DEMO-TZ-%'")
        .execute(db)
        .await
        .map_err(|_| AppError::Internal)?;
    sqlx::query("DELETE FROM generated_reports WHERE report_type = 'demo_full_flow'")
        .execute(db)
        .await
        .map_err(|_| AppError::Internal)?;
    sqlx::query("DELETE FROM extraction_runs WHERE scope_type = 'demo_full_flow'")
        .execute(db)
        .await
        .map_err(|_| AppError::Internal)?;
    sqlx::query("DELETE FROM workflow_runs WHERE stage_key LIKE 'demo_%'")
        .execute(db)
        .await
        .map_err(|_| AppError::Internal)?;
    sqlx::query(
        "DELETE FROM import_files WHERE import_id IN (SELECT id FROM imports WHERE source_type = 'demo_full_flow')",
    )
    .execute(db)
    .await
    .map_err(|_| AppError::Internal)?;
    sqlx::query("DELETE FROM imports WHERE source_type = 'demo_full_flow'")
        .execute(db)
        .await
        .map_err(|_| AppError::Internal)?;
    Ok(())
}

async fn upsert_demo_mapping_template(
    db: &sqlx::PgPool,
    template_id: Uuid,
    now_at: DateTime<Utc>,
) -> Result<(), AppError> {
    sqlx::query(
        r#"
        INSERT INTO mapping_templates (
            id, template_key, template_label, version, status, source_type, is_active, created_at, updated_at
        )
        VALUES ($1, 'demo_full_flow_mapping', '通州多源治理数据 Demo 映射模板', '2026.demo', 'published', 'demo_full_flow', TRUE, $2, $2)
        ON CONFLICT (template_key)
        DO UPDATE SET
            template_label = EXCLUDED.template_label,
            version = EXCLUDED.version,
            status = EXCLUDED.status,
            source_type = EXCLUDED.source_type,
            is_active = TRUE,
            updated_at = EXCLUDED.updated_at
        "#,
    )
    .bind(template_id)
    .bind(now_at)
    .execute(db)
    .await
    .map_err(|_| AppError::Internal)?;

    let actual_template_id = sqlx::query_scalar::<_, Uuid>(
        "SELECT id FROM mapping_templates WHERE template_key = 'demo_full_flow_mapping'",
    )
    .fetch_one(db)
    .await
    .map_err(|_| AppError::Internal)?;

    sqlx::query("DELETE FROM mapping_fields WHERE template_id = $1")
        .bind(actual_template_id)
        .execute(db)
        .await
        .map_err(|_| AppError::Internal)?;

    for (sort_order, (source_field, target_field, sample_value, required)) in [
        (
            "工单标题/事项内容/详情",
            "title",
            "欠薪、食品安全、裁判监督等问题标题",
            true,
        ),
        (
            "所在街道/区县/通讯地址",
            "area_name",
            "马驹桥镇/九棵树街道/台湖镇",
            true,
        ),
        (
            "创建时间/来访日期/接报时间",
            "occurred_at",
            "2026-04-08 09:30:00",
            true,
        ),
        (
            "工单来源/分页名称",
            "source_type",
            "hotline_12345 / police_110 / platform_395",
            true,
        ),
        (
            "市级问题分类/事项分类",
            "risk_tags",
            "食品安全,劳资欠薪,民事检察",
            false,
        ),
        ("工单状态/办理情况", "status", "已结案/办理中/待复核", false),
        (
            "承办单位/流转单位/总包名称",
            "assignee",
            "属地街镇/业务部门/总包单位",
            false,
        ),
        (
            "主要内容/内容摘要/反映内容",
            "risk_reason_summary",
            "诉求摘要和风险原因",
            false,
        ),
    ]
    .iter()
    .enumerate()
    {
        sqlx::query(
            r#"
            INSERT INTO mapping_fields (
                id, template_id, source_field, target_field, confidence, status,
                sample_value, sort_order, required, created_at, updated_at
            )
            VALUES ($1, $2, $3, $4, $5, 'mapped', $6, $7, $8, $9, $9)
            "#,
        )
        .bind(Uuid::new_v4())
        .bind(actual_template_id)
        .bind(source_field)
        .bind(target_field)
        .bind(0.96_f64)
        .bind(sample_value)
        .bind(sort_order as i32)
        .bind(*required)
        .bind(now_at)
        .execute(db)
        .await
        .map_err(|_| AppError::Internal)?;
    }

    Ok(())
}

async fn insert_demo_import(
    db: &sqlx::PgPool,
    import_id: Uuid,
    mapping_template_id: Uuid,
    total_count: i32,
    now_at: DateTime<Utc>,
) -> Result<(), AppError> {
    sqlx::query(
        r#"
        INSERT INTO imports (
            id, source_type, source_label, status, error_message, mapping_template_id,
            total_record_count, processed_record_count, failed_record_count,
            last_processed_at, created_at, updated_at
        )
        VALUES (
            $1, 'demo_full_flow', 'Demo 全流程多源数据包', 'processed', '',
            (SELECT id FROM mapping_templates WHERE template_key = 'demo_full_flow_mapping'),
            $2, $2, 0, $3, $3, $3
        )
        "#,
    )
    .bind(import_id)
    .bind(total_count)
    .bind(now_at)
    .execute(db)
    .await
    .map_err(|_| AppError::Internal)?;

    sqlx::query(
        r#"
        INSERT INTO import_files (
            id, import_id, original_filename, stored_filename, stored_path,
            file_size, mime_type, created_at
        )
        VALUES ($1, $2, 'demo-full-flow-通州多源治理样例.xlsx', 'demo-full-flow.xlsx', 'demo/demo-full-flow.xlsx', 204800, 'application/vnd.openxmlformats-officedocument.spreadsheetml.sheet', $3)
        "#,
    )
    .bind(Uuid::new_v4())
    .bind(import_id)
    .bind(now_at)
    .execute(db)
    .await
    .map_err(|_| AppError::Internal)?;

    let _ = mapping_template_id;
    Ok(())
}

async fn insert_demo_workflow_runs(
    db: &sqlx::PgPool,
    now_at: DateTime<Utc>,
) -> Result<(), AppError> {
    for (offset, stage_key, stage_label, item_count) in [
        (0_i64, "demo_readiness", "Demo 系统准备", 5_i32),
        (1, "demo_ingestion", "Demo 数据导入", 5),
        (2, "demo_mapping", "Demo 字段映射", 8),
        (3, "demo_processing", "Demo 数据处理入库", 5),
        (4, "demo_extraction", "Demo 知识抽取", 15),
        (5, "demo_graph_vector", "Demo 图谱/向量同步", 10),
        (6, "demo_risk", "Demo 风险研判", 5),
        (7, "demo_alert_dispatch", "Demo 预警与分派", 8),
        (8, "demo_evaluation", "Demo 效果评估", 4),
        (9, "demo_report", "Demo 报告生成", 1),
    ] {
        let started_at = now_at + Duration::seconds(offset * 2);
        let finished_at = started_at + Duration::seconds(1);
        sqlx::query(
            r#"
            INSERT INTO workflow_runs (
                id, stage_key, stage_label, status, started_at, finished_at,
                item_count, success_count, failure_count, created_at, updated_at
            )
            VALUES ($1, $2, $3, 'completed', $4, $5, $6, $6, 0, $4, $5)
            "#,
        )
        .bind(Uuid::new_v4())
        .bind(stage_key)
        .bind(stage_label)
        .bind(started_at)
        .bind(finished_at)
        .bind(item_count)
        .execute(db)
        .await
        .map_err(|_| AppError::Internal)?;
    }
    Ok(())
}

async fn insert_demo_case(
    db: &sqlx::PgPool,
    import_id: Uuid,
    seed: &DemoCaseSeed,
    index: i64,
    now_at: DateTime<Utc>,
) -> Result<Uuid, AppError> {
    sqlx::query_scalar::<_, Uuid>(
        r#"
        INSERT INTO risk_cases (
            id, import_id, case_code, title, source_type, area_name,
            risk_level, risk_score, status, alert_status, assignee,
            occurred_at, due_at, closed_at, report_period, created_at, updated_at,
            risk_reason_summary, disposal_advice, review_status, risk_tags,
            graph_sync_status, graph_sync_message, graph_synced_at,
            vector_sync_status, vector_sync_message, vector_synced_at
        )
        VALUES (
            $1, $2, $3, $4, $5, $6,
            $7, $8, $9, $10, $11,
            $12, $13, CASE WHEN $9 IN ('closed', 'disposed') THEN $14 ELSE NULL END, '2026-Demo', $14, $14,
            $15, $16, $17, $18,
            'synced', $19, $14,
            'indexed_demo', $20, $14
        )
        RETURNING id
        "#,
    )
    .bind(Uuid::new_v4())
    .bind(import_id)
    .bind(seed.case_code)
    .bind(seed.title)
    .bind(seed.source_type)
    .bind(seed.area_name)
    .bind(seed.risk_level)
    .bind(seed.risk_score)
    .bind(seed.status)
    .bind(seed.alert_status)
    .bind(seed.assignee)
    .bind(now_at - Duration::days(5 - index))
    .bind(now_at + Duration::days(2 + index))
    .bind(now_at)
    .bind(seed.reason)
    .bind(seed.advice)
    .bind(if seed.risk_level == "high" { "manual_review_required" } else { "pending" })
    .bind(seed.risk_tags)
    .bind(format!("Demo synced {} into HugeGraph presentation graph", seed.case_code))
    .bind(format!("Demo indexed {} into vector presentation fallback; replace with Milvus when service is up", seed.case_code))
    .fetch_one(db)
    .await
    .map_err(|_| AppError::Internal)
}

async fn insert_demo_entities(
    db: &sqlx::PgPool,
    case_id: Uuid,
    seed: &DemoCaseSeed,
    now_at: DateTime<Utc>,
) -> Result<Vec<Uuid>, AppError> {
    let entity_types = ["organization", "area", "risk_event"];
    let mut entity_ids = Vec::with_capacity(seed.entity_names.len());
    for (index, entity_name) in seed.entity_names.iter().enumerate() {
        let entity_id = Uuid::new_v4();
        sqlx::query(
            r#"
            INSERT INTO knowledge_entities (
                id, case_id, entity_type, entity_name, confidence, extracted_at, created_at
            )
            VALUES ($1, $2, $3, $4, $5, $6, $6)
            "#,
        )
        .bind(entity_id)
        .bind(case_id)
        .bind(entity_types[index])
        .bind(entity_name)
        .bind(0.92_f64 + (index as f64 * 0.02))
        .bind(now_at)
        .execute(db)
        .await
        .map_err(|_| AppError::Internal)?;
        entity_ids.push(entity_id);
    }
    Ok(entity_ids)
}

async fn insert_demo_relations(
    db: &sqlx::PgPool,
    entity_ids: &[Uuid],
    now_at: DateTime<Utc>,
) -> Result<(), AppError> {
    if entity_ids.len() < 3 {
        return Ok(());
    }
    for (relation_type, source, target, confidence) in [
        ("located_in", entity_ids[0], entity_ids[1], 0.94_f64),
        ("triggers_risk", entity_ids[0], entity_ids[2], 0.91_f64),
    ] {
        sqlx::query(
            r#"
            INSERT INTO graph_relations (
                id, relation_type, source_entity_id, target_entity_id, confidence, created_at
            )
            VALUES ($1, $2, $3, $4, $5, $6)
            "#,
        )
        .bind(Uuid::new_v4())
        .bind(relation_type)
        .bind(source)
        .bind(target)
        .bind(confidence)
        .bind(now_at)
        .execute(db)
        .await
        .map_err(|_| AppError::Internal)?;
    }
    Ok(())
}

async fn insert_demo_alert(
    db: &sqlx::PgPool,
    case_id: Uuid,
    seed: &DemoCaseSeed,
    index: i64,
    now_at: DateTime<Utc>,
) -> Result<(), AppError> {
    sqlx::query(
        r#"
        INSERT INTO alerts (
            id, case_id, title, severity, status, summary, created_at, updated_at, handled_at
        )
        VALUES ($1, $2, $3, $4, $5, $6, $7, $7, CASE WHEN $5 <> 'open' THEN $7 ELSE NULL END)
        "#,
    )
    .bind(Uuid::new_v4())
    .bind(case_id)
    .bind(format!("Demo 预警：{}", seed.title))
    .bind(seed.risk_level)
    .bind(seed.alert_status)
    .bind(seed.reason)
    .bind(now_at + Duration::seconds(index * 3))
    .execute(db)
    .await
    .map_err(|_| AppError::Internal)?;
    Ok(())
}

async fn insert_demo_dispatch_task(
    db: &sqlx::PgPool,
    case_id: Uuid,
    seed: &DemoCaseSeed,
    index: i64,
    now_at: DateTime<Utc>,
) -> Result<(), AppError> {
    sqlx::query(
        r#"
        INSERT INTO dispatch_tasks (
            id, case_id, title, assignee, priority, status, progress_note,
            due_at, completed_at, feedback_result, created_at, updated_at
        )
        VALUES ($1, $2, $3, $4, $5, $6, $7, $8, CASE WHEN $6 = 'completed' THEN $9 ELSE NULL END, $10, $9, $9)
        "#,
    )
    .bind(Uuid::new_v4())
    .bind(case_id)
    .bind(format!("Demo 处置任务：{}", seed.title))
    .bind(seed.assignee)
    .bind(if seed.risk_level == "high" { "urgent" } else { "normal" })
    .bind(if seed.status == "disposed" { "completed" } else { "in_progress" })
    .bind("Demo 演示：已完成责任链确认，待页面展示流转记录。")
    .bind(now_at + Duration::days(3 + index))
    .bind(now_at)
    .bind("Demo 演示反馈：已形成阶段性处置意见。")
    .execute(db)
    .await
    .map_err(|_| AppError::Internal)?;
    Ok(())
}

async fn insert_demo_extraction_run(
    db: &sqlx::PgPool,
    run_id: Uuid,
    item_count: i32,
    entity_count: i32,
    relation_count: i32,
    now_at: DateTime<Utc>,
) -> Result<(), AppError> {
    sqlx::query(
        r#"
        INSERT INTO extraction_runs (
            id, scope_type, mode, status, item_count, success_count, failure_count,
            summary, started_at, finished_at, created_at, updated_at,
            provider_style, model_name, graph_sync_status, graph_sync_message,
            vector_sync_status, vector_sync_message
        )
        VALUES (
            $1, 'demo_full_flow', 'demo', 'completed', $2, $2, 0,
            $3, $4, $4, $4, $4,
            'openai_chat_completion_compatible', 'Qwen2.5-Coder-7B-Instruct',
            'synced', 'Demo graph synchronized into HugeGraph presentation graph',
            'indexed_demo', 'Demo vector fallback indexed while Milvus service is not required for presentation'
        )
        "#,
    )
    .bind(run_id)
    .bind(item_count)
    .bind(format!(
        "Demo extraction completed: {item_count} cases, {entity_count} entities, {relation_count} relations."
    ))
    .bind(now_at)
    .execute(db)
    .await
    .map_err(|_| AppError::Internal)?;
    Ok(())
}

async fn insert_demo_report(
    state: &AppState,
    report_id: Uuid,
    case_count: i32,
    now_at: DateTime<Utc>,
) -> Result<(), AppError> {
    let report_dir = state.settings().storage.report_dir.clone();
    fs::create_dir_all(&report_dir).map_err(|_| AppError::Internal)?;
    let file_path = format!("{report_dir}/demo-full-flow-2026.md");
    let content = r#"# 通州区基层社会治理风险研判演示报告

## 一、总体态势
本次 Demo 汇聚 12345 热线、110 接处警、综治信访、检察院信访与 395 平台项目数据，形成“导入-映射-处理-抽取-图谱/向量-风险研判-预警-分派-报告”的完整演示闭环。

## 二、重点风险
1. 工程建设与欠薪风险需要优先联动住建、属地街镇和公安。
2. 食品安全投诉适合通过市场监管复核和历史诉求回溯处理。
3. 重复信访与裁判监督线索需要形成一次性释法说理和责任边界梳理。

## 三、处置建议
建议演示时依次打开全流程演示页、风险案件列表、技术运维后台和报告页，展示案件状态、外部同步状态、责任任务和报告产物。
"#;
    fs::write(&file_path, content).map_err(|_| AppError::Internal)?;

    sqlx::query(
        r#"
        INSERT INTO generated_reports (
            id, title, report_type, period, status, file_path, generated_at, created_at,
            summary, provider_style, model_name
        )
        VALUES (
            $1, '通州区基层社会治理风险研判 Demo 报告',
            'demo_full_flow', '2026-Demo', 'ready', $2, $3, $3,
            $4, 'openai_chat_completion_compatible', 'Qwen2.5-Coder-7B-Instruct'
        )
        "#,
    )
    .bind(report_id)
    .bind(&file_path)
    .bind(now_at)
    .bind(format!(
        "Demo 报告已生成，覆盖 {case_count} 个多源风险案件和完整处置闭环。"
    ))
    .execute(state.db())
    .await
    .map_err(|_| AppError::Internal)?;
    Ok(())
}

async fn dashboard_overview(
    State(state): State<AppState>,
) -> Result<Json<ApiResponse<DashboardOverviewResponse>>, AppError> {
    let import_batches = count(state.db(), "SELECT COUNT(*) FROM imports").await?;
    let risk_cases = count(state.db(), "SELECT COUNT(*) FROM risk_cases").await?;
    let high_risk = count(
        state.db(),
        "SELECT COUNT(*) FROM risk_cases WHERE risk_level = 'high'",
    )
    .await?;
    let pending_alerts = count(
        state.db(),
        "SELECT COUNT(*) FROM alerts WHERE status IN ('open', 'acknowledged')",
    )
    .await?;
    let in_progress_tasks = count(
        state.db(),
        "SELECT COUNT(*) FROM dispatch_tasks WHERE status IN ('assigned', 'in_progress')",
    )
    .await?;
    let workflow_rows = latest_workflow_runs(state.db(), 12).await?;

    Ok(ok(DashboardOverviewResponse {
        generated_at: now(),
        metrics: vec![
            metric_card(
                "import_batches",
                "Import Batches",
                import_batches.to_string(),
                None,
                None,
                None,
                "healthy",
            ),
            metric_card(
                "risk_cases",
                "Risk Cases",
                risk_cases.to_string(),
                None,
                None,
                None,
                "healthy",
            ),
            metric_card(
                "high_risk_cases",
                "High Risk Cases",
                high_risk.to_string(),
                None,
                None,
                None,
                if high_risk > 0 { "warning" } else { "healthy" },
            ),
            metric_card(
                "pending_alerts",
                "Pending Alerts",
                pending_alerts.to_string(),
                None,
                None,
                None,
                if pending_alerts > 0 {
                    "warning"
                } else {
                    "healthy"
                },
            ),
            metric_card(
                "in_progress_tasks",
                "In Progress Tasks",
                in_progress_tasks.to_string(),
                None,
                None,
                None,
                "healthy",
            ),
        ],
        workflow: workflow_rows
            .into_iter()
            .map(|row| ProgressItem {
                key: row.stage_key,
                label: row.stage_label,
                status: row.status,
                completed: row.success_count.max(0) as u32,
                total: row.item_count.max(0) as u32,
            })
            .collect(),
        queues: vec![
            QueueItem {
                key: "pending_alerts".to_string(),
                label: "Pending Alerts".to_string(),
                count: pending_alerts.max(0) as u32,
                status: if pending_alerts > 0 {
                    "warning"
                } else {
                    "healthy"
                }
                .to_string(),
            },
            QueueItem {
                key: "in_progress_tasks".to_string(),
                label: "Dispatch Queue".to_string(),
                count: in_progress_tasks.max(0) as u32,
                status: if in_progress_tasks > 0 {
                    "running"
                } else {
                    "healthy"
                }
                .to_string(),
            },
            QueueItem {
                key: "high_risk_cases".to_string(),
                label: "High Risk Queue".to_string(),
                count: high_risk.max(0) as u32,
                status: if high_risk > 0 { "warning" } else { "healthy" }.to_string(),
            },
        ],
    }))
}

async fn dashboard_summary(
    State(state): State<AppState>,
) -> Result<Json<ApiResponse<SummaryResponse>>, AppError> {
    let import_batches = count(state.db(), "SELECT COUNT(*) FROM imports").await?;
    let risk_cases = count(state.db(), "SELECT COUNT(*) FROM risk_cases").await?;
    let high_risk = count(
        state.db(),
        "SELECT COUNT(*) FROM risk_cases WHERE risk_level = 'high'",
    )
    .await?;
    let pending_alerts = count(
        state.db(),
        "SELECT COUNT(*) FROM alerts WHERE status IN ('open', 'acknowledged')",
    )
    .await?;
    let in_progress_tasks = count(
        state.db(),
        "SELECT COUNT(*) FROM dispatch_tasks WHERE status IN ('assigned', 'in_progress')",
    )
    .await?;
    let reports = count(state.db(), "SELECT COUNT(*) FROM generated_reports").await?;

    Ok(ok(SummaryResponse {
        generated_at: now(),
        metrics: vec![
            metric_num(
                "import_batches",
                "Import Batches",
                import_batches,
                "healthy",
                false,
            ),
            metric_num("risk_cases", "Risk Cases", risk_cases, "healthy", false),
            metric_num(
                "high_risk_cases",
                "High Risk Cases",
                high_risk,
                if high_risk > 0 { "warning" } else { "healthy" },
                false,
            ),
            metric_num(
                "pending_alerts",
                "Pending Alerts",
                pending_alerts,
                if pending_alerts > 0 {
                    "warning"
                } else {
                    "healthy"
                },
                false,
            ),
            metric_num(
                "in_progress_tasks",
                "In Progress Tasks",
                in_progress_tasks,
                "healthy",
                false,
            ),
            metric_num(
                "generated_reports",
                "Generated Reports",
                reports,
                "healthy",
                false,
            ),
        ],
    }))
}

async fn stage_summary(
    State(state): State<AppState>,
) -> Result<Json<ApiResponse<StageSummaryResponse>>, AppError> {
    let items = latest_workflow_runs(state.db(), 20).await?;
    Ok(ok(StageSummaryResponse {
        generated_at: now(),
        items: items
            .into_iter()
            .map(|row| StageItem {
                stage_key: row.stage_key,
                stage_label: row.stage_label,
                status: row.status,
                item_count: row.item_count,
                success_count: row.success_count,
                failure_count: row.failure_count,
                started_at: row.started_at.to_rfc3339(),
                finished_at: row.finished_at.map(|value| value.to_rfc3339()),
            })
            .collect(),
    }))
}

async fn ingestion_summary_view(
    State(state): State<AppState>,
) -> Result<Json<ApiResponse<IngestionSummaryResponse>>, AppError> {
    let batch_total = count(state.db(), "SELECT COUNT(*) FROM imports").await?;
    let total_records = count(
        state.db(),
        "SELECT COALESCE(SUM(total_record_count), 0) FROM imports",
    )
    .await?;
    let processed = count(
        state.db(),
        "SELECT COUNT(*) FROM imports WHERE status = 'processed'",
    )
    .await?;
    let failed = count(
        state.db(),
        "SELECT COUNT(*) FROM imports WHERE status = 'failed'",
    )
    .await?;

    let rows = sqlx::query_as::<_, IngestionSourceRow>(
        r#"
        SELECT
            source_type,
            COUNT(*)::BIGINT AS batch_count,
            COALESCE(SUM(total_record_count), 0)::BIGINT AS record_count,
            MAX(created_at) AS latest_import_at
        FROM imports
        GROUP BY source_type
        ORDER BY latest_import_at DESC, source_type ASC
        "#,
    )
    .fetch_all(state.db())
    .await
    .map_err(|_| AppError::Internal)?;

    Ok(ok(IngestionSummaryResponse {
        generated_at: now(),
        totals: vec![
            metric_card(
                "batch_total",
                "Batch Total",
                batch_total.to_string(),
                None,
                None,
                None,
                "healthy",
            ),
            metric_card(
                "record_total",
                "Record Total",
                total_records.to_string(),
                None,
                None,
                None,
                "healthy",
            ),
            metric_card(
                "processed",
                "Processed Batches",
                processed.to_string(),
                None,
                None,
                None,
                "healthy",
            ),
            metric_card(
                "failed",
                "Failed Batches",
                failed.to_string(),
                None,
                None,
                None,
                if failed > 0 { "critical" } else { "healthy" },
            ),
        ],
        sources: rows
            .into_iter()
            .map(|row| IngestionSourceSummary {
                source_key: row.source_type.clone(),
                source_label: source_label(&row.source_type).to_string(),
                batch_count: row.batch_count.max(0) as u32,
                record_count: row.record_count.max(0) as u32,
                latest_import_at: row.latest_import_at.to_rfc3339(),
                status: if row.batch_count > 0 {
                    "healthy"
                } else {
                    "warning"
                }
                .to_string(),
            })
            .collect(),
    }))
}

async fn ingestion_batches(
    State(state): State<AppState>,
) -> Result<Json<ApiResponse<IngestionBatchListResponse>>, AppError> {
    let rows = sqlx::query_as::<_, IngestionBatchRow>(
        r#"
        SELECT
            i.id,
            i.source_type,
            i.status,
            i.created_at,
            COALESCE(i.total_record_count, 0) AS record_count,
            COALESCE(i.failed_record_count, 0) AS error_count,
            f.original_filename AS file_name
        FROM imports i
        LEFT JOIN LATERAL (
            SELECT original_filename
            FROM import_files
            WHERE import_id = i.id
            ORDER BY created_at ASC
            LIMIT 1
        ) f ON TRUE
        ORDER BY i.created_at DESC, i.id DESC
        LIMIT 20
        "#,
    )
    .fetch_all(state.db())
    .await
    .map_err(|_| AppError::Internal)?;

    Ok(ok(IngestionBatchListResponse {
        generated_at: now(),
        items: rows
            .into_iter()
            .map(|row| IngestionBatchItem {
                id: row.id.to_string(),
                source_key: row.source_type.clone(),
                source_label: source_label(&row.source_type).to_string(),
                file_name: row.file_name.unwrap_or_else(|| "unnamed-file".to_string()),
                status: row.status,
                record_count: row.record_count.max(0) as i32,
                error_count: row.error_count.max(0) as i32,
                imported_at: row.created_at.to_rfc3339(),
            })
            .collect(),
    }))
}

async fn ingestion_list(
    State(state): State<AppState>,
    Query(query): Query<PaginationQuery>,
) -> Result<Json<ApiResponse<PageResponse<IngestionItem>>>, AppError> {
    let page = normalize_page(query.page);
    let page_size = normalize_page_size(query.page_size);
    let offset = (page - 1) * page_size;
    let total = count_by_optional_status(state.db(), "imports", query.status.as_deref()).await?;

    let items = sqlx::query_as::<_, IngestionItem>(
        r#"
        SELECT id, source_type, status, created_at, updated_at
        FROM imports
        WHERE ($1::TEXT IS NULL OR status = $1)
        ORDER BY created_at DESC, id DESC
        LIMIT $2 OFFSET $3
        "#,
    )
    .bind(query.status.as_deref())
    .bind(page_size)
    .bind(offset)
    .fetch_all(state.db())
    .await
    .map_err(|_| AppError::Internal)?;

    Ok(ok(PageResponse {
        generated_at: now(),
        items,
        page,
        page_size,
        total,
    }))
}

async fn ingestion_detail(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
) -> Result<Json<ApiResponse<IngestionDetailResponse>>, AppError> {
    let item = sqlx::query_as::<_, IngestionItem>(
        "SELECT id, source_type, status, created_at, updated_at FROM imports WHERE id = $1",
    )
    .bind(id)
    .fetch_optional(state.db())
    .await
    .map_err(|_| AppError::Internal)?
    .ok_or(AppError::NotFound)?;

    let files = sqlx::query_as::<_, ImportFileDto>(
        r#"
        SELECT id, original_filename, stored_filename, stored_path, file_size, mime_type, created_at
        FROM import_files
        WHERE import_id = $1
        ORDER BY created_at ASC
        "#,
    )
    .bind(id)
    .fetch_all(state.db())
    .await
    .map_err(|_| AppError::Internal)?;

    let created_case_count =
        sqlx::query_scalar::<_, i64>("SELECT COUNT(*) FROM risk_cases WHERE import_id = $1")
            .bind(id)
            .fetch_one(state.db())
            .await
            .map_err(|_| AppError::Internal)?;

    let processing_run_count = sqlx::query_scalar::<_, i64>(
        "SELECT COUNT(*) FROM workflow_runs WHERE stage_key = 'processing'",
    )
    .fetch_one(state.db())
    .await
    .map_err(|_| AppError::Internal)?;

    Ok(ok(IngestionDetailResponse {
        id: item.id.to_string(),
        source_type: item.source_type,
        status: item.status.clone(),
        created_at: item.created_at.to_rfc3339(),
        updated_at: item.updated_at.to_rfc3339(),
        files,
        process_summary: ProcessSummaryDto {
            created_case_count,
            processing_run_count,
            latest_status: item.status,
        },
    }))
}

async fn process_ingestion_action_live(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
) -> Result<Json<ApiResponse<ActionResponse>>, AppError> {
    let integration_values = load_setting_map(state.db(), "integrations").await?;
    let ai_service = build_ai_service(&state, &integration_values);
    let embedding_service = build_embedding_service(&state, &integration_values);
    let vector_store = build_vector_store(&state, &integration_values);
    let result =
        process_import_batch(&state, id, &ai_service, &embedding_service, &vector_store).await?;

    Ok(ok(ActionResponse {
        id: result.import_id.to_string(),
        status: result.status,
        message: format!(
            "processed {} / {} records into {} cases, failed {}, workflow_run={}, mapping_template={}",
            result.processed_record_count,
            result.total_record_count,
            result.affected_case_count,
            result.failed_record_count,
            result.workflow_run_id,
            result
                .mapping_template_id
                .map(|value| value.to_string())
                .unwrap_or_else(|| "none".to_string())
        ),
        updated_at: result.finished_at.to_rfc3339(),
        is_placeholder: false,
    }))
}

async fn start_ingestion_processing_job(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
) -> Result<Json<ApiResponse<JobDto>>, AppError> {
    ensure_import_exists(state.db(), id).await?;

    let job_id = Uuid::new_v4();
    let job = insert_platform_job(
        state.db(),
        job_id,
        "ingestion_processing",
        "import",
        Some(id),
        serde_json::json!({
            "import_id": id,
            "action": "process_import_batch",
            "mode": "async"
        }),
        "queued import processing job",
    )
    .await?;

    spawn_ingestion_processing_job(state, job_id, id);
    Ok(ok(job))
}

async fn mapping_summary(
    State(state): State<AppState>,
) -> Result<Json<ApiResponse<SummaryResponse>>, AppError> {
    let total = count(state.db(), "SELECT COUNT(*) FROM mapping_templates").await?;
    let active = count(
        state.db(),
        "SELECT COUNT(*) FROM mapping_templates WHERE status IN ('active', 'published', 'draft')",
    )
    .await?;
    let fields = count(state.db(), "SELECT COUNT(*) FROM mapping_fields").await?;
    let needs_review = count(
        state.db(),
        "SELECT COUNT(*) FROM mapping_fields WHERE status = 'needs_review'",
    )
    .await?;

    Ok(ok(SummaryResponse {
        generated_at: now(),
        metrics: vec![
            metric_num("template_total", "Template Total", total, "healthy", false),
            metric_num(
                "template_active",
                "Active Templates",
                active,
                "healthy",
                false,
            ),
            metric_num("field_total", "Mapped Fields", fields, "healthy", false),
            metric_num(
                "needs_review",
                "Fields Pending Review",
                needs_review,
                if needs_review > 0 {
                    "warning"
                } else {
                    "healthy"
                },
                false,
            ),
        ],
    }))
}

async fn mapping_current(
    State(state): State<AppState>,
) -> Result<Json<ApiResponse<MappingCurrentResponse>>, AppError> {
    let template = sqlx::query_as::<_, MappingTemplateListItem>(
        r#"
        SELECT id, template_key, template_label, version, status, source_type, is_active, created_at, updated_at
        FROM mapping_templates
        ORDER BY is_active DESC, updated_at DESC, created_at DESC
        LIMIT 1
        "#,
    )
    .fetch_optional(state.db())
    .await
    .map_err(|_| AppError::Internal)?
    .ok_or(AppError::NotFound)?;

    let fields = load_mapping_field_rows(state.db(), template.id).await?;
    let completion_rate = compute_mapping_completion_rate(&fields);
    let missing_required_fields = compute_missing_required_fields(&fields);

    Ok(ok(MappingCurrentResponse {
        generated_at: now(),
        template_id: template.id.to_string(),
        template_key: template.template_key,
        template_label: template.template_label,
        version: template.version,
        status: template.status,
        source_type: template.source_type,
        completion_rate,
        missing_required_fields,
        fields: fields
            .into_iter()
            .map(|field| MappingFieldItem {
                source_field: field.source_field,
                target_field: field.target_field,
                confidence: field.confidence as f32,
                status: field.status,
                sample_value: field.sample_value,
                required: field.required,
            })
            .collect(),
    }))
}

async fn mapping_templates(
    State(state): State<AppState>,
    Query(query): Query<PaginationQuery>,
) -> Result<Json<ApiResponse<PageResponse<MappingTemplateListItem>>>, AppError> {
    let page = normalize_page(query.page);
    let page_size = normalize_page_size(query.page_size);
    let offset = (page - 1) * page_size;
    let total = count(state.db(), "SELECT COUNT(*) FROM mapping_templates").await?;

    let items = sqlx::query_as::<_, MappingTemplateListItem>(
        r#"
        SELECT id, template_key, template_label, version, status, source_type, is_active, created_at, updated_at
        FROM mapping_templates
        WHERE ($1::TEXT IS NULL OR source_type = $1)
        ORDER BY updated_at DESC, id DESC
        LIMIT $2 OFFSET $3
        "#,
    )
    .bind(query.source_type.as_deref())
    .bind(page_size)
    .bind(offset)
    .fetch_all(state.db())
    .await
    .map_err(|_| AppError::Internal)?;

    Ok(ok(PageResponse {
        generated_at: now(),
        items,
        page,
        page_size,
        total,
    }))
}

async fn mapping_template_detail(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
) -> Result<Json<ApiResponse<MappingTemplateDetailResponse>>, AppError> {
    let template = sqlx::query_as::<_, MappingTemplateListItem>(
        r#"
        SELECT id, template_key, template_label, version, status, source_type, is_active, created_at, updated_at
        FROM mapping_templates
        WHERE id = $1
        "#,
    )
    .bind(id)
    .fetch_optional(state.db())
    .await
    .map_err(|_| AppError::Internal)?
    .ok_or(AppError::NotFound)?;

    let fields = load_mapping_field_rows(state.db(), template.id).await?;
    let completion_rate = compute_mapping_completion_rate(&fields);
    let missing_required_fields = compute_missing_required_fields(&fields);

    Ok(ok(MappingTemplateDetailResponse {
        id: template.id.to_string(),
        template_key: template.template_key,
        template_label: template.template_label,
        version: template.version,
        status: template.status,
        source_type: template.source_type,
        is_active: template.is_active,
        fields,
        completion_rate,
        missing_required_fields,
    }))
}

async fn save_mapping_template(
    State(state): State<AppState>,
    Json(payload): Json<SaveMappingTemplateRequest>,
) -> Result<Json<ApiResponse<MappingTemplateDetailResponse>>, AppError> {
    validate_mapping_template_payload(&payload)?;
    let now_at = Utc::now();
    let mut tx = state.db().begin().await.map_err(|_| AppError::Internal)?;

    let template_id = sqlx::query_scalar::<_, Uuid>(
        r#"
        INSERT INTO mapping_templates (
            id, template_key, template_label, version, status, source_type, is_active, created_at, updated_at
        )
        VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $8)
        ON CONFLICT (template_key)
        DO UPDATE SET
            template_label = EXCLUDED.template_label,
            version = EXCLUDED.version,
            status = EXCLUDED.status,
            source_type = EXCLUDED.source_type,
            is_active = EXCLUDED.is_active,
            updated_at = EXCLUDED.updated_at
        RETURNING id
        "#,
    )
    .bind(Uuid::new_v4())
    .bind(payload.template_key.trim())
    .bind(payload.template_label.trim())
    .bind(payload.version.trim())
    .bind(payload.status.trim())
    .bind(payload.source_type.trim())
    .bind(false)
    .bind(now_at)
    .fetch_one(&mut *tx)
    .await
    .map_err(|_| AppError::Internal)?;

    sqlx::query("DELETE FROM mapping_fields WHERE template_id = $1")
        .bind(template_id)
        .execute(&mut *tx)
        .await
        .map_err(|_| AppError::Internal)?;

    for (index, field) in payload.fields.iter().enumerate() {
        sqlx::query(
            r#"
            INSERT INTO mapping_fields (
                id, template_id, source_field, target_field, confidence, status,
                sample_value, sort_order, required, created_at, updated_at
            )
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $10)
            "#,
        )
        .bind(Uuid::new_v4())
        .bind(template_id)
        .bind(field.source_field.trim())
        .bind(field.target_field.trim())
        .bind(field.confidence as f64)
        .bind(field.status.trim())
        .bind(field.sample_value.trim())
        .bind(index as i32)
        .bind(matches!(
            field.target_field.as_str(),
            "case_title" | "area_name" | "occurred_at"
        ))
        .bind(now_at)
        .execute(&mut *tx)
        .await
        .map_err(|_| AppError::Internal)?;
    }

    tx.commit().await.map_err(|_| AppError::Internal)?;
    mapping_template_detail(State(state), Path(template_id)).await
}

async fn validate_mapping(
    State(state): State<AppState>,
    Json(payload): Json<ValidateMappingRequest>,
) -> Result<Json<ApiResponse<HashMap<String, serde_json::Value>>>, AppError> {
    let template = sqlx::query_as::<_, MappingTemplateListItem>(
        r#"
        SELECT id, template_key, template_label, version, status, source_type, is_active, created_at, updated_at
        FROM mapping_templates
        WHERE source_type = $1
        ORDER BY is_active DESC, updated_at DESC
        LIMIT 1
        "#,
    )
    .bind(payload.source_type.trim())
    .fetch_optional(state.db())
    .await
    .map_err(|_| AppError::Internal)?
    .ok_or(AppError::NotFound)?;

    let fields = load_mapping_field_rows(state.db(), template.id).await?;
    let mapped_targets = fields
        .iter()
        .map(|field| field.target_field.clone())
        .collect::<Vec<_>>();
    let missing_required_fields = payload
        .required_fields
        .into_iter()
        .filter(|required| !mapped_targets.iter().any(|mapped| mapped == required))
        .collect::<Vec<_>>();

    let mut data = HashMap::new();
    data.insert(
        "template_id".to_string(),
        serde_json::json!(template.id.to_string()),
    );
    data.insert(
        "is_valid".to_string(),
        serde_json::json!(missing_required_fields.is_empty()),
    );
    data.insert(
        "missing_required_fields".to_string(),
        serde_json::json!(missing_required_fields),
    );
    data.insert(
        "completion_rate".to_string(),
        serde_json::json!(compute_mapping_completion_rate(&fields)),
    );

    Ok(ok(data))
}

async fn processing_summary(
    State(state): State<AppState>,
) -> Result<Json<ApiResponse<SummaryResponse>>, AppError> {
    let queued = count(
        state.db(),
        "SELECT COUNT(*) FROM workflow_runs WHERE stage_key = 'processing' AND status = 'queued'",
    )
    .await?;
    let running = count(
        state.db(),
        "SELECT COUNT(*) FROM workflow_runs WHERE stage_key = 'processing' AND status = 'running'",
    )
    .await?;
    let completed = count(
        state.db(),
        "SELECT COUNT(*) FROM workflow_runs WHERE stage_key = 'processing' AND status = 'completed'",
    )
    .await?;
    let failed = count(
        state.db(),
        "SELECT COUNT(*) FROM workflow_runs WHERE stage_key = 'processing' AND status = 'failed'",
    )
    .await?;

    Ok(ok(SummaryResponse {
        generated_at: now(),
        metrics: vec![
            metric_num(
                "queued",
                "Queued Tasks",
                queued,
                if queued > 0 { "warning" } else { "healthy" },
                false,
            ),
            metric_num("running", "Running Tasks", running, "healthy", false),
            metric_num("completed", "Completed Tasks", completed, "healthy", false),
            metric_num(
                "failed",
                "Failed Tasks",
                failed,
                if failed > 0 { "critical" } else { "healthy" },
                false,
            ),
        ],
    }))
}

async fn processing_runs(
    State(state): State<AppState>,
    Query(query): Query<PaginationQuery>,
) -> Result<Json<ApiResponse<PageResponse<WorkflowRunDto>>>, AppError> {
    list_workflow_stage_runs(state, query, "processing").await
}

async fn processing_run_detail(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
) -> Result<Json<ApiResponse<WorkflowRunDto>>, AppError> {
    workflow_run_detail_by_id(state.db(), id, "processing").await
}

async fn retry_processing_run(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
) -> Result<Json<ApiResponse<ActionResponse>>, AppError> {
    ensure_workflow_run_exists(state.db(), id, "processing").await?;
    let now_at = Utc::now();
    insert_workflow_run(
        state.db(),
        "processing",
        "Data Processing",
        "queued",
        0,
        0,
        0,
        now_at,
    )
    .await?;
    Ok(ok(ActionResponse {
        id: id.to_string(),
        status: "queued".to_string(),
        message: "created a new processing retry run".to_string(),
        updated_at: now_at.to_rfc3339(),
        is_placeholder: false,
    }))
}

async fn extraction_summary_view(
    State(state): State<AppState>,
) -> Result<Json<ApiResponse<ExtractionSummaryResponse>>, AppError> {
    let entity_total = count(state.db(), "SELECT COUNT(*) FROM knowledge_entities").await?;
    let relation_total = count(state.db(), "SELECT COUNT(*) FROM graph_relations").await?;
    let completed_runs = count(
        state.db(),
        "SELECT COUNT(*) FROM extraction_runs WHERE status LIKE 'completed%'",
    )
    .await?;

    let rows = sqlx::query_as::<_, ExtractionEntityRow>(
        r#"
        SELECT id, entity_type, entity_name, confidence, extracted_at
        FROM knowledge_entities
        ORDER BY extracted_at DESC, id DESC
        LIMIT 10
        "#,
    )
    .fetch_all(state.db())
    .await
    .map_err(|_| AppError::Internal)?;

    Ok(ok(ExtractionSummaryResponse {
        generated_at: now(),
        metrics: vec![
            metric_card(
                "entities",
                "Entities",
                entity_total.to_string(),
                None,
                None,
                None,
                "healthy",
            ),
            metric_card(
                "relations",
                "Relations",
                relation_total.to_string(),
                None,
                None,
                None,
                "healthy",
            ),
            metric_card(
                "completed_runs",
                "Completed Runs",
                completed_runs.to_string(),
                None,
                None,
                None,
                "healthy",
            ),
        ],
        recent_entities: rows
            .into_iter()
            .map(|row| ExtractionEntityItem {
                id: row.id.to_string(),
                entity_type: row.entity_type,
                name: row.entity_name,
                confidence: row.confidence as f32,
                extracted_at: row.extracted_at.to_rfc3339(),
            })
            .collect(),
    }))
}

async fn extraction_runs(
    State(state): State<AppState>,
    Query(query): Query<PaginationQuery>,
) -> Result<Json<ApiResponse<PageResponse<ExtractionRunDto>>>, AppError> {
    let page = normalize_page(query.page);
    let page_size = normalize_page_size(query.page_size);
    let offset = (page - 1) * page_size;
    let total = count(state.db(), "SELECT COUNT(*) FROM extraction_runs").await?;

    let items = sqlx::query_as::<_, ExtractionRunDto>(
        r#"
        SELECT id, scope_type, mode, status, item_count, success_count, failure_count, summary,
               provider_style, model_name, graph_sync_status, graph_sync_message,
               vector_sync_status, vector_sync_message, started_at, finished_at, created_at, updated_at
        FROM extraction_runs
        WHERE ($1::TEXT IS NULL OR status = $1)
        ORDER BY started_at DESC, id DESC
        LIMIT $2 OFFSET $3
        "#,
    )
    .bind(query.status.as_deref())
    .bind(page_size)
    .bind(offset)
    .fetch_all(state.db())
    .await
    .map_err(|_| AppError::Internal)?;

    Ok(ok(PageResponse {
        generated_at: now(),
        items,
        page,
        page_size,
        total,
    }))
}

async fn extraction_run_detail(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
) -> Result<Json<ApiResponse<ExtractionRunDto>>, AppError> {
    let item = sqlx::query_as::<_, ExtractionRunDto>(
        r#"
        SELECT id, scope_type, mode, status, item_count, success_count, failure_count, summary,
               provider_style, model_name, graph_sync_status, graph_sync_message,
               vector_sync_status, vector_sync_message, started_at, finished_at, created_at, updated_at
        FROM extraction_runs
        WHERE id = $1
        "#,
    )
    .bind(id)
    .fetch_optional(state.db())
    .await
    .map_err(|_| AppError::Internal)?
    .ok_or(AppError::NotFound)?;

    Ok(ok(item))
}

async fn create_extraction_run_live(
    State(state): State<AppState>,
    Json(payload): Json<CreateExtractionRunRequest>,
) -> Result<Json<ApiResponse<ActionResponse>>, AppError> {
    let integration_values = load_setting_map(state.db(), "integrations").await?;
    let ai_service = build_ai_service(&state, &integration_values);
    let embedding_service = build_embedding_service(&state, &integration_values);
    let graph_service = build_hugegraph_service(&state, &integration_values);
    let vector_store = build_vector_store(&state, &integration_values);
    let result = execute_extraction_run(
        &state,
        payload.case_ids,
        payload.mode,
        &ai_service,
        &embedding_service,
        &graph_service,
        &vector_store,
    )
    .await?;

    Ok(ok(ActionResponse {
        id: result.run_id.to_string(),
        status: result.status,
        message: format!(
            "processed {} cases (success {}, failed {}), created {} entities and {} relations. {}",
            result.item_count,
            result.success_count,
            result.failure_count,
            result.created_entity_count,
            result.created_relation_count,
            result.summary
        ),
        updated_at: result.finished_at.to_rfc3339(),
        is_placeholder: false,
    }))
}

async fn start_extraction_job(
    State(state): State<AppState>,
    Json(payload): Json<CreateExtractionRunRequest>,
) -> Result<Json<ApiResponse<JobDto>>, AppError> {
    if matches!(payload.case_ids.as_ref(), Some(case_ids) if case_ids.is_empty()) {
        return Err(AppError::Validation("case_ids cannot be empty".to_string()));
    }

    let job_id = Uuid::new_v4();
    let job = insert_platform_job(
        state.db(),
        job_id,
        "knowledge_extraction",
        "extraction_run",
        None,
        serde_json::json!({
            "case_ids": payload.case_ids.clone(),
            "mode": payload.mode.clone(),
            "action": "execute_extraction_run"
        }),
        "queued knowledge extraction job",
    )
    .await?;

    spawn_extraction_job(state, job_id, payload);
    Ok(ok(job))
}

async fn graph_summary(
    State(state): State<AppState>,
) -> Result<Json<ApiResponse<SummaryResponse>>, AppError> {
    let nodes = count(state.db(), "SELECT COUNT(*) FROM knowledge_entities").await?;
    let edges = count(state.db(), "SELECT COUNT(*) FROM graph_relations").await?;
    let covered_cases = count(
        state.db(),
        "SELECT COUNT(DISTINCT case_id) FROM knowledge_entities",
    )
    .await?;
    let synced_cases = count(
        state.db(),
        "SELECT COUNT(*) FROM risk_cases WHERE graph_sync_status = 'synced'",
    )
    .await?;
    let failed_cases = count(
        state.db(),
        "SELECT COUNT(*) FROM risk_cases WHERE graph_sync_status = 'failed'",
    )
    .await?;
    let pending_cases = count(
        state.db(),
        "SELECT COUNT(*) FROM risk_cases WHERE graph_sync_status IN ('pending', 'not_configured')",
    )
    .await?;

    Ok(ok(SummaryResponse {
        generated_at: now(),
        metrics: vec![
            metric_num("nodes", "Graph Nodes", nodes, "healthy", false),
            metric_num("edges", "Graph Edges", edges, "healthy", false),
            metric_num(
                "covered_cases",
                "Cases With Graph",
                covered_cases,
                "healthy",
                false,
            ),
            metric_num(
                "hugegraph_synced_cases",
                "HugeGraph Synced Cases",
                synced_cases,
                if failed_cases > 0 {
                    "critical"
                } else if pending_cases > 0 {
                    "warning"
                } else {
                    "healthy"
                },
                false,
            ),
            metric_num(
                "hugegraph_failed_cases",
                "HugeGraph Failed Cases",
                failed_cases,
                if failed_cases > 0 {
                    "critical"
                } else {
                    "healthy"
                },
                false,
            ),
        ],
    }))
}

async fn graph_overview(
    State(state): State<AppState>,
) -> Result<Json<ApiResponse<GraphOverviewResponse>>, AppError> {
    let nodes = count(state.db(), "SELECT COUNT(*) FROM knowledge_entities").await?;
    let edges = count(state.db(), "SELECT COUNT(*) FROM graph_relations").await?;
    let covered_cases = count(
        state.db(),
        "SELECT COUNT(DISTINCT case_id) FROM knowledge_entities",
    )
    .await?;

    let relation_rows = sqlx::query_as::<_, RelationTypeRow>(
        r#"
        SELECT relation_type, COUNT(*)::BIGINT AS relation_count
        FROM graph_relations
        GROUP BY relation_type
        ORDER BY relation_count DESC, relation_type ASC
        "#,
    )
    .fetch_all(state.db())
    .await
    .map_err(|_| AppError::Internal)?;

    Ok(ok(GraphOverviewResponse {
        generated_at: now(),
        metrics: vec![
            metric_card(
                "nodes",
                "Nodes",
                nodes.to_string(),
                None,
                None,
                None,
                "healthy",
            ),
            metric_card(
                "edges",
                "Edges",
                edges.to_string(),
                None,
                None,
                None,
                "healthy",
            ),
            metric_card(
                "covered_cases",
                "Cases With Graph",
                covered_cases.to_string(),
                None,
                None,
                None,
                "healthy",
            ),
        ],
        relation_types: relation_rows
            .into_iter()
            .map(|row| RelationTypeItem {
                key: row.relation_type.clone(),
                label: relation_label(&row.relation_type).to_string(),
                count: row.relation_count.max(0) as u32,
            })
            .collect(),
    }))
}

async fn graph_case_view(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
) -> Result<Json<ApiResponse<GraphCaseResponse>>, AppError> {
    let nodes = sqlx::query_as::<_, KnowledgeEntityView>(
        r#"
        SELECT id, entity_type, entity_name, confidence, extracted_at
        FROM knowledge_entities
        WHERE case_id = $1
        ORDER BY extracted_at DESC
        "#,
    )
    .bind(id)
    .fetch_all(state.db())
    .await
    .map_err(|_| AppError::Internal)?;

    let edges = sqlx::query_as::<_, GraphRelationView>(
        r#"
        SELECT gr.id, gr.relation_type, gr.source_entity_id, gr.target_entity_id, gr.confidence, gr.created_at
        FROM graph_relations gr
        WHERE EXISTS (
            SELECT 1 FROM knowledge_entities ke
            WHERE ke.case_id = $1
              AND (ke.id = gr.source_entity_id OR ke.id = gr.target_entity_id)
        )
        ORDER BY gr.created_at DESC
        "#,
    )
    .bind(id)
    .fetch_all(state.db())
    .await
    .map_err(|_| AppError::Internal)?;

    Ok(ok(GraphCaseResponse {
        case_id: id.to_string(),
        nodes: nodes
            .into_iter()
            .map(|row| GraphNodeDto {
                id: row.id.to_string(),
                node_type: row.entity_type,
                label: row.entity_name,
                confidence: row.confidence,
            })
            .collect(),
        edges: edges
            .into_iter()
            .map(|row| GraphEdgeDto {
                id: row.id.to_string(),
                relation_type: row.relation_type,
                source: row.source_entity_id.to_string(),
                target: row.target_entity_id.to_string(),
                confidence: row.confidence,
            })
            .collect(),
        sync_target: GraphSyncTarget {
            provider: "HugeGraph".to_string(),
            status: if state.settings().hugegraph.base_url.trim().is_empty() {
                "not_configured".to_string()
            } else {
                "configured".to_string()
            },
            is_placeholder: false,
        },
    }))
}

async fn rebuild_graph_case(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
) -> Result<Json<ApiResponse<ActionResponse>>, AppError> {
    let now_at = Utc::now();
    let case = sqlx::query_as::<_, GraphCaseSyncRow>(
        r#"
        SELECT id, case_code, title, area_name, risk_level, source_type
        FROM risk_cases
        WHERE id = $1
        "#,
    )
    .bind(id)
    .fetch_optional(state.db())
    .await
    .map_err(|_| AppError::Internal)?
    .ok_or(AppError::NotFound)?;

    let entities = sqlx::query_as::<_, GraphEntitySyncRow>(
        r#"
        SELECT entity_type, entity_name, confidence
        FROM knowledge_entities
        WHERE case_id = $1
        ORDER BY extracted_at DESC, id DESC
        "#,
    )
    .bind(id)
    .fetch_all(state.db())
    .await
    .map_err(|_| AppError::Internal)?;

    if entities.is_empty() {
        return Ok(ok(ActionResponse {
            id: id.to_string(),
            status: "pending_extraction".to_string(),
            message: "case has no extracted entities yet; run /api/extraction/run before rebuilding HugeGraph".to_string(),
            updated_at: now_at.to_rfc3339(),
            is_placeholder: false,
        }));
    }

    let relations = sqlx::query_as::<_, GraphRelationSyncRow>(
        r#"
        SELECT
            gr.relation_type,
            source.entity_name AS source_entity_name,
            target.entity_name AS target_entity_name,
            gr.confidence
        FROM graph_relations gr
        JOIN knowledge_entities source ON source.id = gr.source_entity_id
        JOIN knowledge_entities target ON target.id = gr.target_entity_id
        WHERE source.case_id = $1
          AND target.case_id = $1
        ORDER BY gr.created_at DESC
        "#,
    )
    .bind(id)
    .fetch_all(state.db())
    .await
    .map_err(|_| AppError::Internal)?;

    let integration_values = load_setting_map(state.db(), "integrations").await?;
    let graph_service = build_hugegraph_service(&state, &integration_values);
    let sync_input = GraphCaseSyncInput {
        case_id: case.id.to_string(),
        case_code: case.case_code.clone(),
        title: case.title,
        area_name: case.area_name,
        risk_level: case.risk_level,
        source_type: case.source_type,
        entities: entities
            .into_iter()
            .map(|entity| GraphEntitySync {
                entity_type: entity.entity_type,
                entity_name: entity.entity_name,
                confidence: entity.confidence,
            })
            .collect(),
        relations: relations
            .into_iter()
            .map(|relation| GraphRelationSync {
                relation_type: relation.relation_type,
                source_entity_name: relation.source_entity_name,
                target_entity_name: relation.target_entity_name,
                confidence: relation.confidence,
            })
            .collect(),
    };

    match graph_service.sync_case_graph(&sync_input).await {
        Ok(result) => {
            let message = format!(
                "{} (vertices={}, edges={})",
                result.message, result.vertex_count, result.edge_count
            );
            update_risk_case_graph_sync_status(
                state.db(),
                id,
                &result.status,
                &message,
                Utc::now(),
            )
            .await?;
            insert_workflow_run(
                state.db(),
                "graph",
                "Graph Build",
                "completed",
                1,
                1,
                0,
                now_at,
            )
            .await?;
            Ok(ok(ActionResponse {
                id: id.to_string(),
                status: result.status,
                message,
                updated_at: Utc::now().to_rfc3339(),
                is_placeholder: false,
            }))
        }
        Err(error) => {
            update_risk_case_graph_sync_status(state.db(), id, "failed", &error, Utc::now())
                .await?;
            insert_workflow_run(
                state.db(),
                "graph",
                "Graph Build",
                "failed",
                1,
                0,
                1,
                now_at,
            )
            .await?;
            Ok(ok(ActionResponse {
                id: id.to_string(),
                status: "failed".to_string(),
                message: error,
                updated_at: Utc::now().to_rfc3339(),
                is_placeholder: false,
            }))
        }
    }
}

async fn rebuild_case_vector(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
) -> Result<Json<ApiResponse<VectorActionResponse>>, AppError> {
    let document = load_vector_case_document(state.db(), id).await?;
    let integration_values = load_setting_map(state.db(), "integrations").await?;
    let embedding_service = build_embedding_service(&state, &integration_values);
    let vector_store = build_vector_store(&state, &integration_values);
    let query_text = vector_document_text(&document);
    let embedding = embedding_service
        .embed_text(&query_text)
        .await
        .map_err(|error| AppError::DependencyUnavailable(format!("embedding failed: {error}")))?;
    let embedding_dimension = embedding.len();
    let sync_result = vector_store
        .upsert_case_vector(&document, &embedding)
        .await
        .map_err(|error| {
            AppError::DependencyUnavailable(format!("milvus upsert failed: {error}"))
        })?;
    let now_at = Utc::now();
    update_case_vector_sync_status(
        state.db(),
        id,
        &sync_result.status,
        &sync_result.message,
        now_at,
    )
    .await?;
    insert_workflow_run(
        state.db(),
        "vector",
        "Vector Index",
        if sync_result.status == "indexed" {
            "completed"
        } else {
            "completed_with_warnings"
        },
        1,
        if sync_result.status == "indexed" {
            1
        } else {
            0
        },
        if sync_result.status == "indexed" {
            0
        } else {
            1
        },
        now_at,
    )
    .await?;

    Ok(ok(VectorActionResponse {
        case_id: document.case_id,
        case_code: document.case_code,
        status: sync_result.status,
        message: sync_result.message,
        embedding_dimension,
        model_contract: embedding_service.contract(),
        updated_at: now_at.to_rfc3339(),
    }))
}

async fn case_vector_similar(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
    Query(query): Query<SimilarQuery>,
) -> Result<Json<ApiResponse<SimilarCaseResponse>>, AppError> {
    let document = load_vector_case_document(state.db(), id).await?;
    let integration_values = load_setting_map(state.db(), "integrations").await?;
    let embedding_service = build_embedding_service(&state, &integration_values);
    let vector_store = build_vector_store(&state, &integration_values);
    let query_text = vector_document_text(&document);
    let embedding = embedding_service
        .embed_text(&query_text)
        .await
        .map_err(|error| AppError::DependencyUnavailable(format!("embedding failed: {error}")))?;
    let embedding_dimension = embedding.len();
    let hits = vector_store
        .search_similar_cases(&VectorSearchQuery {
            embedding,
            exclude_case_id: Some(document.case_id.clone()),
            limit: query.limit.unwrap_or(5).clamp(1, 20),
        })
        .await
        .map_err(|error| {
            AppError::DependencyUnavailable(format!("milvus search failed: {error}"))
        })?;

    Ok(ok(SimilarCaseResponse {
        case_id: document.case_id,
        case_code: document.case_code,
        query_text,
        embedding_dimension,
        model_contract: embedding_service.contract(),
        items: hits.into_iter().map(map_similar_case_reference).collect(),
        generated_at: now(),
    }))
}

async fn risk_summary(
    State(state): State<AppState>,
) -> Result<Json<ApiResponse<SummaryResponse>>, AppError> {
    let total = count(state.db(), "SELECT COUNT(*) FROM risk_cases").await?;
    let high = count(
        state.db(),
        "SELECT COUNT(*) FROM risk_cases WHERE risk_level = 'high'",
    )
    .await?;
    let reviewing = count(
        state.db(),
        "SELECT COUNT(*) FROM risk_cases WHERE status IN ('pending_review', 'in_progress')",
    )
    .await?;
    let closed = count(
        state.db(),
        "SELECT COUNT(*) FROM risk_cases WHERE status = 'closed'",
    )
    .await?;

    Ok(ok(SummaryResponse {
        generated_at: now(),
        metrics: vec![
            metric_num("total", "Risk Cases", total, "healthy", false),
            metric_num(
                "high",
                "High Risk Cases",
                high,
                if high > 0 { "warning" } else { "healthy" },
                false,
            ),
            metric_num("reviewing", "Reviewing Cases", reviewing, "healthy", false),
            metric_num("closed", "Closed Cases", closed, "healthy", false),
        ],
    }))
}

async fn risk_overview(
    State(state): State<AppState>,
) -> Result<Json<ApiResponse<RiskOverviewResponse>>, AppError> {
    let total = count(state.db(), "SELECT COUNT(*) FROM risk_cases").await?;
    let high = count(
        state.db(),
        "SELECT COUNT(*) FROM risk_cases WHERE risk_level = 'high'",
    )
    .await?;
    let reviewing = count(
        state.db(),
        "SELECT COUNT(*) FROM risk_cases WHERE status IN ('pending_review', 'in_progress')",
    )
    .await?;
    let rows = sqlx::query_as::<_, RiskCaseRow>(
        r#"
        SELECT id, title, risk_level, risk_score, area_name, status
        FROM risk_cases
        ORDER BY risk_score DESC, updated_at DESC
        LIMIT 10
        "#,
    )
    .fetch_all(state.db())
    .await
    .map_err(|_| AppError::Internal)?;

    Ok(ok(RiskOverviewResponse {
        generated_at: now(),
        metrics: vec![
            metric_card(
                "total",
                "Risk Cases",
                total.to_string(),
                None,
                None,
                None,
                "healthy",
            ),
            metric_card(
                "high",
                "High Risk",
                high.to_string(),
                None,
                None,
                None,
                if high > 0 { "warning" } else { "healthy" },
            ),
            metric_card(
                "reviewing",
                "Reviewing",
                reviewing.to_string(),
                None,
                None,
                None,
                "healthy",
            ),
        ],
        top_risks: rows
            .into_iter()
            .map(|row| RiskItem {
                id: row.id.to_string(),
                title: row.title,
                level: row.risk_level,
                score: row.risk_score as f32,
                area: row.area_name,
                status: row.status,
            })
            .collect(),
    }))
}

async fn risk_case_list(
    State(state): State<AppState>,
    Query(query): Query<PaginationQuery>,
) -> Result<Json<ApiResponse<PageResponse<RiskCaseListItem>>>, AppError> {
    let page = normalize_page(query.page);
    let page_size = normalize_page_size(query.page_size);
    let offset = (page - 1) * page_size;
    let total = sqlx::query_scalar::<_, i64>(
        r#"
        SELECT COUNT(*)
        FROM risk_cases
        WHERE ($1::TEXT IS NULL OR status = $1)
          AND ($2::TEXT IS NULL OR risk_level = $2)
          AND ($3::TEXT IS NULL OR area_name = $3)
          AND ($4::TEXT IS NULL OR source_type = $4)
          AND ($5::UUID IS NULL OR import_id = $5)
        "#,
    )
    .bind(query.status.as_deref())
    .bind(query.risk_level.as_deref())
    .bind(query.area_name.as_deref())
    .bind(query.source_type.as_deref())
    .bind(query.import_id)
    .fetch_one(state.db())
    .await
    .map_err(|_| AppError::Internal)?;

    let items = sqlx::query_as::<_, RiskCaseListItem>(
        r#"
        SELECT id, case_code, title, source_type, area_name, risk_level, risk_score,
               status, alert_status, assignee, occurred_at, due_at, closed_at,
               report_period, review_status, risk_tags, risk_reason_summary, disposal_advice,
               graph_sync_status, graph_sync_message, graph_synced_at,
               vector_sync_status, vector_sync_message, vector_synced_at,
               created_at, updated_at
        FROM risk_cases
        WHERE ($1::TEXT IS NULL OR status = $1)
          AND ($2::TEXT IS NULL OR risk_level = $2)
          AND ($3::TEXT IS NULL OR area_name = $3)
          AND ($4::TEXT IS NULL OR source_type = $4)
          AND ($5::UUID IS NULL OR import_id = $5)
        ORDER BY risk_score DESC, updated_at DESC
        LIMIT $6 OFFSET $7
        "#,
    )
    .bind(query.status.as_deref())
    .bind(query.risk_level.as_deref())
    .bind(query.area_name.as_deref())
    .bind(query.source_type.as_deref())
    .bind(query.import_id)
    .bind(page_size)
    .bind(offset)
    .fetch_all(state.db())
    .await
    .map_err(|_| AppError::Internal)?;

    Ok(ok(PageResponse {
        generated_at: now(),
        items,
        page,
        page_size,
        total,
    }))
}

async fn risk_case_detail_view(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
) -> Result<Json<ApiResponse<RiskCaseDetailResponse>>, AppError> {
    Ok(ok(load_risk_case_detail(&state, id).await?))
}

async fn load_risk_case_detail(
    state: &AppState,
    id: Uuid,
) -> Result<RiskCaseDetailResponse, AppError> {
    let case_info = sqlx::query_as::<_, RiskCaseListItem>(
        r#"
        SELECT id, case_code, title, source_type, area_name, risk_level, risk_score,
               status, alert_status, assignee, occurred_at, due_at, closed_at,
               report_period, review_status, risk_tags, risk_reason_summary, disposal_advice,
               graph_sync_status, graph_sync_message, graph_synced_at,
               vector_sync_status, vector_sync_message, vector_synced_at,
               created_at, updated_at
        FROM risk_cases
        WHERE id = $1
        "#,
    )
    .bind(id)
    .fetch_optional(state.db())
    .await
    .map_err(|_| AppError::Internal)?
    .ok_or(AppError::NotFound)?;

    let entities = sqlx::query_as::<_, KnowledgeEntityView>(
        r#"
        SELECT id, entity_type, entity_name, confidence, extracted_at
        FROM knowledge_entities
        WHERE case_id = $1
        ORDER BY extracted_at DESC
        "#,
    )
    .bind(id)
    .fetch_all(state.db())
    .await
    .map_err(|_| AppError::Internal)?;

    let relations = sqlx::query_as::<_, GraphRelationView>(
        r#"
        SELECT gr.id, gr.relation_type, gr.source_entity_id, gr.target_entity_id, gr.confidence, gr.created_at
        FROM graph_relations gr
        WHERE EXISTS (
            SELECT 1 FROM knowledge_entities ke
            WHERE ke.case_id = $1
              AND (ke.id = gr.source_entity_id OR ke.id = gr.target_entity_id)
        )
        ORDER BY gr.created_at DESC
        "#,
    )
    .bind(id)
    .fetch_all(state.db())
    .await
    .map_err(|_| AppError::Internal)?;

    let alerts = sqlx::query_as::<_, AlertView>(
        r#"
        SELECT id, case_id, title, severity, status, summary, created_at, updated_at, handled_at
        FROM alerts
        WHERE case_id = $1
        ORDER BY created_at DESC
        "#,
    )
    .bind(id)
    .fetch_all(state.db())
    .await
    .map_err(|_| AppError::Internal)?;

    let dispatch_tasks = sqlx::query_as::<_, DispatchTaskView>(
        r#"
        SELECT
            dt.id,
            dt.case_id,
            rc.case_code,
            dt.title,
            dt.assignee,
            dt.priority,
            dt.status,
            dt.progress_note,
            dt.due_at,
            dt.completed_at,
            dt.feedback_result,
            dt.created_at,
            dt.updated_at
        FROM dispatch_tasks dt
        JOIN risk_cases rc ON rc.id = dt.case_id
        WHERE dt.case_id = $1
        ORDER BY dt.created_at DESC
        "#,
    )
    .bind(id)
    .fetch_all(state.db())
    .await
    .map_err(|_| AppError::Internal)?;

    let integration_values = load_setting_map(state.db(), "integrations").await?;
    let ai_service = build_ai_service(&state, &integration_values);
    let embedding_service = build_embedding_service(&state, &integration_values);
    let vector_store = build_vector_store(&state, &integration_values);
    let similar_cases = vector_store
        .search_similar_cases(&crate::services::vector::VectorSearchQuery {
            embedding: embedding_service
                .embed_text(&format!(
                    "{}\n{}\n{}\n{}\n{}",
                    case_info.title,
                    case_info.area_name,
                    case_info.risk_reason_summary,
                    case_info.risk_level,
                    case_info.source_type
                ))
                .await
                .unwrap_or_default(),
            exclude_case_id: Some(case_info.id.to_string()),
            limit: 5,
        })
        .await
        .unwrap_or_default();

    let stored_reason_summary = case_info.risk_reason_summary.trim();
    let stored_disposal_advice = case_info.disposal_advice.trim();
    let recommendation = if stored_reason_summary.is_empty() || stored_disposal_advice.is_empty() {
        ai_service
            .recommend_case_action(&RecommendationInput {
                title: case_info.title.clone(),
                area_name: case_info.area_name.clone(),
                risk_level: case_info.risk_level.clone(),
                source_type: case_info.source_type.clone(),
                entity_count: entities.len(),
                alert_count: alerts.len(),
                dispatch_count: dispatch_tasks.len(),
                reference_cases: format_reference_case_hits(&similar_cases),
            })
            .await
    } else {
        RecommendationOutput {
            reason_summary: stored_reason_summary.to_string(),
            disposal_advice: split_pipe_values(stored_disposal_advice),
            is_placeholder: false,
            model_contract: ai_service.configured_contract(),
        }
    };

    let reason_summary = if stored_reason_summary.is_empty() {
        recommendation.reason_summary.clone()
    } else {
        stored_reason_summary.to_string()
    };
    let disposal_advice = if stored_disposal_advice.is_empty() {
        recommendation.disposal_advice.clone()
    } else {
        split_pipe_values(stored_disposal_advice)
    };

    Ok(RiskCaseDetailResponse {
        case_info: RiskCaseView {
            id: case_info.id.to_string(),
            case_code: case_info.case_code,
            title: case_info.title,
            source_type: case_info.source_type,
            area_name: case_info.area_name,
            risk_level: case_info.risk_level,
            risk_score: case_info.risk_score,
            status: case_info.status,
            alert_status: case_info.alert_status,
            assignee: case_info.assignee,
            occurred_at: case_info.occurred_at.map(|value| value.to_rfc3339()),
            due_at: case_info.due_at.map(|value| value.to_rfc3339()),
            closed_at: case_info.closed_at.map(|value| value.to_rfc3339()),
            report_period: case_info.report_period,
            review_status: case_info.review_status,
            risk_tags: split_csv_values(&case_info.risk_tags),
            risk_reason_summary: reason_summary.clone(),
            disposal_advice: disposal_advice.clone(),
            graph_sync_status: case_info.graph_sync_status,
            graph_sync_message: case_info.graph_sync_message,
            graph_synced_at: case_info.graph_synced_at.map(|value| value.to_rfc3339()),
            vector_sync_status: case_info.vector_sync_status,
            vector_sync_message: case_info.vector_sync_message,
            vector_synced_at: case_info.vector_synced_at.map(|value| value.to_rfc3339()),
            created_at: case_info.created_at.to_rfc3339(),
            updated_at: case_info.updated_at.to_rfc3339(),
        },
        entities,
        relations,
        alerts,
        dispatch_tasks,
        recommendations: RecommendationBundle {
            reason_summary,
            disposal_advice,
            reference_cases: similar_cases
                .into_iter()
                .map(map_similar_case_reference)
                .collect(),
            is_placeholder: recommendation.is_placeholder,
            model_contract: recommendation.model_contract,
        },
    })
}

async fn update_risk_case_status(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
    Json(payload): Json<UpdateStatusRequest>,
) -> Result<Json<ApiResponse<ActionResponse>>, AppError> {
    let status = normalize_risk_case_status(&payload.status)?;
    let now_at = Utc::now();
    let result = sqlx::query(
        r#"
        UPDATE risk_cases
        SET status = $2,
            closed_at = CASE WHEN $2 = 'closed' THEN $3 ELSE closed_at END,
            updated_at = $3
        WHERE id = $1
        "#,
    )
    .bind(id)
    .bind(status)
    .bind(now_at)
    .execute(state.db())
    .await
    .map_err(|_| AppError::Internal)?;
    if result.rows_affected() == 0 {
        return Err(AppError::NotFound);
    }

    Ok(ok(ActionResponse {
        id: id.to_string(),
        status: status.to_string(),
        message: "risk case status updated".to_string(),
        updated_at: now_at.to_rfc3339(),
        is_placeholder: false,
    }))
}

async fn agent_analyze(
    State(state): State<AppState>,
    Json(payload): Json<AgentAnalyzeRequest>,
) -> Result<Json<ApiResponse<AgentAnalyzeResponse>>, AppError> {
    let intent = payload
        .intent
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .unwrap_or("risk_judgement")
        .to_string();
    let query = payload
        .query
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .unwrap_or("")
        .to_string();

    let (case_id, matched_by) = match payload.case_id {
        Some(case_id) => (case_id, "case_id".to_string()),
        None => {
            if query.is_empty() {
                return Err(AppError::Validation(
                    "case_id or query is required for agent analysis".to_string(),
                ));
            }
            (
                find_case_for_agent_query(state.db(), &query)
                    .await?
                    .ok_or(AppError::NotFound)?,
                "query".to_string(),
            )
        }
    };

    let detail = load_risk_case_detail(&state, case_id).await?;
    let answer_markdown = build_agent_answer_markdown(&intent, &query, &detail);
    let graph = build_agent_graph_payload(&detail);
    let sync_status = AgentSyncStatus {
        graph_sync_status: detail.case_info.graph_sync_status.clone(),
        graph_sync_message: detail.case_info.graph_sync_message.clone(),
        vector_sync_status: detail.case_info.vector_sync_status.clone(),
        vector_sync_message: detail.case_info.vector_sync_message.clone(),
    };

    Ok(ok(AgentAnalyzeResponse {
        generated_at: now(),
        intent,
        matched_case_id: detail.case_info.id.clone(),
        matched_by,
        query,
        case_detail: detail,
        answer_markdown,
        graph,
        sync_status,
    }))
}

async fn alerts_summary_view(
    State(state): State<AppState>,
) -> Result<Json<ApiResponse<AlertsSummaryResponse>>, AppError> {
    let total = count(state.db(), "SELECT COUNT(*) FROM alerts").await?;
    let open = count(
        state.db(),
        "SELECT COUNT(*) FROM alerts WHERE status = 'open'",
    )
    .await?;
    let acknowledged = count(
        state.db(),
        "SELECT COUNT(*) FROM alerts WHERE status = 'acknowledged'",
    )
    .await?;

    let items = sqlx::query_as::<_, (Uuid, String, String, String, DateTime<Utc>, String)>(
        r#"
        SELECT
            a.id,
            a.title,
            a.severity,
            rc.area_name,
            a.created_at,
            a.status
        FROM alerts a
        JOIN risk_cases rc ON rc.id = a.case_id
        ORDER BY a.created_at DESC, a.id DESC
        LIMIT 10
        "#,
    )
    .fetch_all(state.db())
    .await
    .map_err(|_| AppError::Internal)?;

    Ok(ok(AlertsSummaryResponse {
        generated_at: now(),
        metrics: vec![
            metric_card(
                "total",
                "Alerts",
                total.to_string(),
                None,
                None,
                None,
                "healthy",
            ),
            metric_card(
                "open",
                "Open Alerts",
                open.to_string(),
                None,
                None,
                None,
                if open > 0 { "warning" } else { "healthy" },
            ),
            metric_card(
                "acknowledged",
                "Acknowledged Alerts",
                acknowledged.to_string(),
                None,
                None,
                None,
                "healthy",
            ),
        ],
        items: items
            .into_iter()
            .map(|row| AlertItem {
                id: row.0.to_string(),
                title: row.1,
                level: row.2,
                source: row.3,
                triggered_at: row.4.to_rfc3339(),
                status: row.5,
            })
            .collect(),
    }))
}

async fn alert_list(
    State(state): State<AppState>,
    Query(query): Query<PaginationQuery>,
) -> Result<Json<ApiResponse<PageResponse<AlertListItem>>>, AppError> {
    let page = normalize_page(query.page);
    let page_size = normalize_page_size(query.page_size);
    let offset = (page - 1) * page_size;
    let total = count(state.db(), "SELECT COUNT(*) FROM alerts").await?;

    let items = sqlx::query_as::<_, AlertListItem>(
        r#"
        SELECT id, case_id, title, severity, status, summary, created_at, updated_at, handled_at
        FROM alerts
        WHERE ($1::TEXT IS NULL OR status = $1)
        ORDER BY created_at DESC, id DESC
        LIMIT $2 OFFSET $3
        "#,
    )
    .bind(query.status.as_deref())
    .bind(page_size)
    .bind(offset)
    .fetch_all(state.db())
    .await
    .map_err(|_| AppError::Internal)?;

    Ok(ok(PageResponse {
        generated_at: now(),
        items,
        page,
        page_size,
        total,
    }))
}

async fn alert_detail(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
) -> Result<Json<ApiResponse<AlertListItem>>, AppError> {
    let item = sqlx::query_as::<_, AlertListItem>(
        r#"
        SELECT id, case_id, title, severity, status, summary, created_at, updated_at, handled_at
        FROM alerts
        WHERE id = $1
        "#,
    )
    .bind(id)
    .fetch_optional(state.db())
    .await
    .map_err(|_| AppError::Internal)?
    .ok_or(AppError::NotFound)?;

    Ok(ok(item))
}

async fn update_alert_status(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
    Json(payload): Json<UpdateStatusRequest>,
) -> Result<Json<ApiResponse<ActionResponse>>, AppError> {
    let status = normalize_alert_status(&payload.status)?;
    let now_at = Utc::now();
    let result = sqlx::query(
        r#"
        UPDATE alerts
        SET status = $2,
            handled_at = CASE WHEN $2 IN ('closed', 'ignored') THEN $3 ELSE handled_at END,
            updated_at = $3
        WHERE id = $1
        "#,
    )
    .bind(id)
    .bind(status)
    .bind(now_at)
    .execute(state.db())
    .await
    .map_err(|_| AppError::Internal)?;
    if result.rows_affected() == 0 {
        return Err(AppError::NotFound);
    }

    Ok(ok(ActionResponse {
        id: id.to_string(),
        status: status.to_string(),
        message: "alert status updated".to_string(),
        updated_at: now_at.to_rfc3339(),
        is_placeholder: false,
    }))
}

async fn dispatch_summary(
    State(state): State<AppState>,
) -> Result<Json<ApiResponse<SummaryResponse>>, AppError> {
    let total = count(state.db(), "SELECT COUNT(*) FROM dispatch_tasks").await?;
    let assigned = count(
        state.db(),
        "SELECT COUNT(*) FROM dispatch_tasks WHERE status = 'assigned'",
    )
    .await?;
    let in_progress = count(
        state.db(),
        "SELECT COUNT(*) FROM dispatch_tasks WHERE status = 'in_progress'",
    )
    .await?;
    let overdue = count(
        state.db(),
        "SELECT COUNT(*) FROM dispatch_tasks WHERE due_at IS NOT NULL AND due_at < NOW() AND status <> 'completed'",
    )
    .await?;

    Ok(ok(SummaryResponse {
        generated_at: now(),
        metrics: vec![
            metric_num("total", "Dispatch Tasks", total, "healthy", false),
            metric_num("assigned", "Assigned Tasks", assigned, "healthy", false),
            metric_num(
                "in_progress",
                "In Progress Tasks",
                in_progress,
                "healthy",
                false,
            ),
            metric_num(
                "overdue",
                "Overdue Tasks",
                overdue,
                if overdue > 0 { "warning" } else { "healthy" },
                false,
            ),
        ],
    }))
}

async fn dispatch_list(
    State(state): State<AppState>,
    Query(query): Query<PaginationQuery>,
) -> Result<Json<ApiResponse<PageResponse<DispatchTaskListItem>>>, AppError> {
    let page = normalize_page(query.page);
    let page_size = normalize_page_size(query.page_size);
    let offset = (page - 1) * page_size;
    let total = count(state.db(), "SELECT COUNT(*) FROM dispatch_tasks").await?;

    let items = sqlx::query_as::<_, DispatchTaskListItem>(
        r#"
        SELECT
            dt.id,
            dt.case_id,
            rc.case_code,
            dt.title,
            dt.assignee,
            dt.priority,
            dt.status,
            dt.progress_note,
            dt.due_at,
            dt.completed_at,
            dt.feedback_result,
            dt.created_at,
            dt.updated_at
        FROM dispatch_tasks dt
        JOIN risk_cases rc ON rc.id = dt.case_id
        WHERE ($1::TEXT IS NULL OR dt.status = $1)
        ORDER BY dt.created_at DESC, dt.id DESC
        LIMIT $2 OFFSET $3
        "#,
    )
    .bind(query.status.as_deref())
    .bind(page_size)
    .bind(offset)
    .fetch_all(state.db())
    .await
    .map_err(|_| AppError::Internal)?;

    Ok(ok(PageResponse {
        generated_at: now(),
        items,
        page,
        page_size,
        total,
    }))
}

async fn create_dispatch_task(
    State(state): State<AppState>,
    Json(payload): Json<CreateDispatchTaskRequest>,
) -> Result<Json<ApiResponse<DispatchTaskListItem>>, AppError> {
    let assignee = payload.assignee.trim();
    if assignee.is_empty() {
        return Err(AppError::Validation("assignee is required".to_string()));
    }

    let now_at = Utc::now();
    let task_id = Uuid::new_v4();
    let case_info = sqlx::query_as::<_, (Uuid, String, String)>(
        "SELECT id, case_code, title FROM risk_cases WHERE id = $1",
    )
    .bind(payload.case_id)
    .fetch_optional(state.db())
    .await
    .map_err(|_| AppError::Internal)?
    .ok_or(AppError::NotFound)?;

    let title = payload
        .title
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(str::to_string)
        .unwrap_or_else(|| format!("Dispatch-{}", case_info.2));
    let priority = payload.priority.unwrap_or_else(|| "medium".to_string());
    let due_at = parse_optional_rfc3339(payload.due_at.as_deref())?;

    sqlx::query(
        r#"
        INSERT INTO dispatch_tasks (
            id, case_id, title, assignee, priority, status, progress_note, due_at,
            completed_at, feedback_result, created_at, updated_at
        )
        VALUES ($1, $2, $3, $4, $5, 'assigned', NULL, $6, NULL, NULL, $7, $7)
        "#,
    )
    .bind(task_id)
    .bind(payload.case_id)
    .bind(&title)
    .bind(assignee)
    .bind(&priority)
    .bind(due_at)
    .bind(now_at)
    .execute(state.db())
    .await
    .map_err(|_| AppError::Internal)?;

    let item = sqlx::query_as::<_, DispatchTaskListItem>(
        r#"
        SELECT
            dt.id,
            dt.case_id,
            rc.case_code,
            dt.title,
            dt.assignee,
            dt.priority,
            dt.status,
            dt.progress_note,
            dt.due_at,
            dt.completed_at,
            dt.feedback_result,
            dt.created_at,
            dt.updated_at
        FROM dispatch_tasks dt
        JOIN risk_cases rc ON rc.id = dt.case_id
        WHERE dt.id = $1
        "#,
    )
    .bind(task_id)
    .fetch_one(state.db())
    .await
    .map_err(|_| AppError::Internal)?;

    Ok(ok(item))
}

async fn dispatch_detail(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
) -> Result<Json<ApiResponse<DispatchTaskListItem>>, AppError> {
    let item = sqlx::query_as::<_, DispatchTaskListItem>(
        r#"
        SELECT
            dt.id,
            dt.case_id,
            rc.case_code,
            dt.title,
            dt.assignee,
            dt.priority,
            dt.status,
            dt.progress_note,
            dt.due_at,
            dt.completed_at,
            dt.feedback_result,
            dt.created_at,
            dt.updated_at
        FROM dispatch_tasks dt
        JOIN risk_cases rc ON rc.id = dt.case_id
        WHERE dt.id = $1
        "#,
    )
    .bind(id)
    .fetch_optional(state.db())
    .await
    .map_err(|_| AppError::Internal)?
    .ok_or(AppError::NotFound)?;

    Ok(ok(item))
}

async fn update_dispatch_status(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
    Json(payload): Json<UpdateStatusRequest>,
) -> Result<Json<ApiResponse<ActionResponse>>, AppError> {
    let status = normalize_dispatch_status(&payload.status)?;
    let now_at = Utc::now();
    let result = sqlx::query(
        r#"
        UPDATE dispatch_tasks
        SET status = $2,
            completed_at = CASE WHEN $2 = 'completed' THEN $3 ELSE completed_at END,
            updated_at = $3
        WHERE id = $1
        "#,
    )
    .bind(id)
    .bind(status)
    .bind(now_at)
    .execute(state.db())
    .await
    .map_err(|_| AppError::Internal)?;
    if result.rows_affected() == 0 {
        return Err(AppError::NotFound);
    }

    Ok(ok(ActionResponse {
        id: id.to_string(),
        status: status.to_string(),
        message: "dispatch task status updated".to_string(),
        updated_at: now_at.to_rfc3339(),
        is_placeholder: false,
    }))
}

async fn evaluation_summary_view(
    State(state): State<AppState>,
) -> Result<Json<ApiResponse<EvaluationSummaryResponse>>, AppError> {
    let total_cases = count(state.db(), "SELECT COUNT(*) FROM risk_cases").await?;
    let closed_cases = count(
        state.db(),
        "SELECT COUNT(*) FROM risk_cases WHERE status = 'closed'",
    )
    .await?;
    let total_tasks = count(state.db(), "SELECT COUNT(*) FROM dispatch_tasks").await?;
    let closed_tasks = count(
        state.db(),
        "SELECT COUNT(*) FROM dispatch_tasks WHERE status = 'completed'",
    )
    .await?;
    let alert_total = count(state.db(), "SELECT COUNT(*) FROM alerts").await?;
    let closed_alerts = count(
        state.db(),
        "SELECT COUNT(*) FROM alerts WHERE status = 'closed'",
    )
    .await?;

    let case_closure_rate = percentage(closed_cases, total_cases);
    let task_closure_rate = percentage(closed_tasks, total_tasks);
    let alert_accuracy = if alert_total == 0 {
        100.0
    } else {
        percentage(closed_alerts, alert_total)
    };
    let manual_review_pass_rate = if total_cases == 0 { 100.0 } else { 78.0 };

    Ok(ok(EvaluationSummaryResponse {
        generated_at: now(),
        metrics: vec![
            metric_card(
                "case_closure_rate",
                "Case Closure Rate",
                format!("{case_closure_rate:.1}"),
                Some("%".to_string()),
                None,
                None,
                rate_status(case_closure_rate),
            ),
            metric_card(
                "task_closure_rate",
                "Task Closure Rate",
                format!("{task_closure_rate:.1}"),
                Some("%".to_string()),
                None,
                None,
                rate_status(task_closure_rate),
            ),
            metric_card(
                "alert_accuracy",
                "Alert Accuracy",
                format!("{alert_accuracy:.1}"),
                Some("%".to_string()),
                None,
                None,
                "placeholder",
            ),
            metric_card(
                "manual_review_pass_rate",
                "Manual Review Pass Rate",
                format!("{manual_review_pass_rate:.1}"),
                Some("%".to_string()),
                None,
                None,
                "placeholder",
            ),
        ],
        dimensions: vec![
            EvaluationDimensionItem {
                key: "case_closure".to_string(),
                label: "Case Closure".to_string(),
                score: case_closure_rate,
                status: rate_status(case_closure_rate).to_string(),
            },
            EvaluationDimensionItem {
                key: "task_closure".to_string(),
                label: "Task Closure".to_string(),
                score: task_closure_rate,
                status: rate_status(task_closure_rate).to_string(),
            },
            EvaluationDimensionItem {
                key: "alert_accuracy".to_string(),
                label: "Alert Accuracy".to_string(),
                score: alert_accuracy,
                status: "placeholder".to_string(),
            },
            EvaluationDimensionItem {
                key: "manual_review_pass_rate".to_string(),
                label: "Manual Review Pass Rate".to_string(),
                score: manual_review_pass_rate,
                status: "placeholder".to_string(),
            },
        ],
    }))
}

async fn evaluation_trends(
    State(_state): State<AppState>,
) -> Result<Json<ApiResponse<EvaluationTrendResponse>>, AppError> {
    Ok(ok(EvaluationTrendResponse {
        generated_at: now(),
        closure_rate: vec![
            trend("2026-04-01", 64.0),
            trend("2026-04-08", 68.0),
            trend("2026-04-15", 72.0),
            trend("2026-04-22", 78.0),
        ],
        alert_accuracy_placeholder: vec![
            trend("2026-04-01", 70.0),
            trend("2026-04-08", 71.5),
            trend("2026-04-15", 74.0),
            trend("2026-04-22", 76.0),
        ],
        review_pass_rate_placeholder: vec![
            trend("2026-04-01", 75.0),
            trend("2026-04-08", 77.0),
            trend("2026-04-15", 79.0),
            trend("2026-04-22", 81.0),
        ],
    }))
}

async fn supervision_overview(
    State(state): State<AppState>,
) -> Result<Json<ApiResponse<SupervisionOverviewResponse>>, AppError> {
    let running_workflows = count(
        state.db(),
        "SELECT COUNT(*) FROM workflow_runs WHERE status = 'running'",
    )
    .await?;
    let failed_workflows = count(
        state.db(),
        "SELECT COUNT(*) FROM workflow_runs WHERE status = 'failed'",
    )
    .await?;
    let overdue_tasks = count(
        state.db(),
        "SELECT COUNT(*) FROM dispatch_tasks WHERE due_at IS NOT NULL AND due_at < NOW() AND status <> 'completed'",
    )
    .await?;

    Ok(ok(SupervisionOverviewResponse {
        generated_at: now(),
        metrics: vec![
            metric_card(
                "running_workflows",
                "Running Workflows",
                running_workflows.to_string(),
                None,
                None,
                None,
                "healthy",
            ),
            metric_card(
                "failed_workflows",
                "Failed Workflows",
                failed_workflows.to_string(),
                None,
                None,
                None,
                if failed_workflows > 0 {
                    "critical"
                } else {
                    "healthy"
                },
            ),
            metric_card(
                "overdue_tasks",
                "Overdue Tasks",
                overdue_tasks.to_string(),
                None,
                None,
                None,
                if overdue_tasks > 0 {
                    "warning"
                } else {
                    "healthy"
                },
            ),
        ],
        agents: vec![
            AgentStatusItem {
                key: "ingestion_agent".to_string(),
                label: "Ingestion Agent".to_string(),
                status: if running_workflows > 0 {
                    "running"
                } else {
                    "ready"
                }
                .to_string(),
                running_tasks: running_workflows.max(0) as u32,
                updated_at: now(),
            },
            AgentStatusItem {
                key: "analysis_agent".to_string(),
                label: "Analysis Agent".to_string(),
                status: if failed_workflows > 0 {
                    "attention"
                } else {
                    "running"
                }
                .to_string(),
                running_tasks: failed_workflows.max(0) as u32,
                updated_at: now(),
            },
            AgentStatusItem {
                key: "dispatch_agent".to_string(),
                label: "Dispatch Agent".to_string(),
                status: if overdue_tasks > 0 {
                    "warning"
                } else {
                    "healthy"
                }
                .to_string(),
                running_tasks: overdue_tasks.max(0) as u32,
                updated_at: now(),
            },
        ],
    }))
}

async fn supervision_summary(
    State(state): State<AppState>,
) -> Result<Json<ApiResponse<SummaryResponse>>, AppError> {
    let workflows = count(state.db(), "SELECT COUNT(*) FROM workflow_runs").await?;
    let failed_workflows = count(
        state.db(),
        "SELECT COUNT(*) FROM workflow_runs WHERE status = 'failed'",
    )
    .await?;
    let extraction_failures = count(
        state.db(),
        "SELECT COUNT(*) FROM extraction_runs WHERE status = 'failed'",
    )
    .await?;
    let overdue_tasks = count(
        state.db(),
        "SELECT COUNT(*) FROM dispatch_tasks WHERE due_at IS NOT NULL AND due_at < NOW() AND status <> 'completed'",
    )
    .await?;

    Ok(ok(SummaryResponse {
        generated_at: now(),
        metrics: vec![
            metric_num("workflows", "Workflow Runs", workflows, "healthy", false),
            metric_num(
                "failed_workflows",
                "Failed Workflows",
                failed_workflows,
                if failed_workflows > 0 {
                    "critical"
                } else {
                    "healthy"
                },
                false,
            ),
            metric_num(
                "extraction_failures",
                "Extraction Failures",
                extraction_failures,
                if extraction_failures > 0 {
                    "critical"
                } else {
                    "healthy"
                },
                false,
            ),
            metric_num(
                "overdue_tasks",
                "Overdue Tasks",
                overdue_tasks,
                if overdue_tasks > 0 {
                    "warning"
                } else {
                    "healthy"
                },
                false,
            ),
        ],
    }))
}

async fn supervision_failures(
    State(state): State<AppState>,
) -> Result<Json<ApiResponse<Vec<WorkflowRunDto>>>, AppError> {
    let items = sqlx::query_as::<_, WorkflowRunDto>(
        r#"
        SELECT id, stage_key, stage_label, status, started_at, finished_at,
               item_count, success_count, failure_count, created_at, updated_at
        FROM workflow_runs
        WHERE status = 'failed'
        ORDER BY started_at DESC, id DESC
        LIMIT 20
        "#,
    )
    .fetch_all(state.db())
    .await
    .map_err(|_| AppError::Internal)?;
    Ok(ok(items))
}

async fn job_list(
    State(state): State<AppState>,
    Query(query): Query<JobQuery>,
) -> Result<Json<ApiResponse<PageResponse<JobDto>>>, AppError> {
    let page = normalize_page(query.page);
    let page_size = normalize_page_size(query.page_size);
    let offset = (page - 1) * page_size;

    let total = sqlx::query_scalar::<_, i64>(
        r#"
        SELECT COUNT(*)
        FROM platform_jobs
        WHERE ($1::TEXT IS NULL OR status = $1)
          AND ($2::TEXT IS NULL OR job_type = $2)
        "#,
    )
    .bind(query.status.as_deref())
    .bind(query.job_type.as_deref())
    .fetch_one(state.db())
    .await
    .map_err(|_| AppError::Internal)?;

    let rows = sqlx::query_as::<_, JobRow>(
        r#"
        SELECT id, job_type, target_type, target_id, status, progress_percent, message,
               request_json, result_json, error_message, started_at, finished_at, created_at, updated_at
        FROM platform_jobs
        WHERE ($1::TEXT IS NULL OR status = $1)
          AND ($2::TEXT IS NULL OR job_type = $2)
        ORDER BY created_at DESC, id DESC
        LIMIT $3 OFFSET $4
        "#,
    )
    .bind(query.status.as_deref())
    .bind(query.job_type.as_deref())
    .bind(page_size)
    .bind(offset)
    .fetch_all(state.db())
    .await
    .map_err(|_| AppError::Internal)?;

    Ok(ok(PageResponse {
        generated_at: now(),
        items: rows.into_iter().map(map_job_row).collect(),
        page,
        page_size,
        total,
    }))
}

async fn job_detail(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
) -> Result<Json<ApiResponse<JobDto>>, AppError> {
    Ok(ok(get_platform_job(state.db(), id).await?))
}

async fn retry_platform_job(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
) -> Result<Json<ApiResponse<JobDto>>, AppError> {
    let previous = get_platform_job(state.db(), id).await?;
    let normalized_status = previous.status.to_ascii_lowercase();
    if !matches!(
        normalized_status.as_str(),
        "failed" | "cancelled" | "completed_with_warnings"
    ) {
        return Err(AppError::Validation(
            "only failed, cancelled or completed_with_warnings jobs can be retried".to_string(),
        ));
    }

    match previous.job_type.as_str() {
        "ingestion_processing" => {
            let target_id = parse_job_target_uuid(&previous)?;
            ensure_import_exists(state.db(), target_id).await?;
            let job_id = Uuid::new_v4();
            let job = insert_platform_job(
                state.db(),
                job_id,
                "ingestion_processing",
                "import",
                Some(target_id),
                serde_json::json!({
                    "import_id": target_id,
                    "action": "process_import_batch",
                    "mode": "async_retry",
                    "retry_of": previous.id
                }),
                "queued retry import processing job",
            )
            .await?;
            spawn_ingestion_processing_job(state, job_id, target_id);
            Ok(ok(job))
        }
        "knowledge_extraction" => {
            let case_ids = previous
                .request
                .get("case_ids")
                .and_then(serde_json::Value::as_array)
                .map(|values| {
                    values
                        .iter()
                        .filter_map(serde_json::Value::as_str)
                        .filter_map(|value| Uuid::parse_str(value).ok())
                        .collect::<Vec<_>>()
                })
                .filter(|values| !values.is_empty());
            let mode = previous
                .request
                .get("mode")
                .and_then(serde_json::Value::as_str)
                .map(str::to_string)
                .or_else(|| Some("incremental".to_string()));

            let payload = CreateExtractionRunRequest { case_ids, mode };
            let job_id = Uuid::new_v4();
            let job = insert_platform_job(
                state.db(),
                job_id,
                "knowledge_extraction",
                "extraction_run",
                None,
                serde_json::json!({
                    "case_ids": payload.case_ids,
                    "mode": payload.mode,
                    "action": "execute_extraction_run",
                    "retry_of": previous.id
                }),
                "queued retry extraction job",
            )
            .await?;
            spawn_extraction_job(state, job_id, payload);
            Ok(ok(job))
        }
        "report_generation" => {
            let report_type = previous
                .request
                .get("report_type")
                .and_then(serde_json::Value::as_str)
                .ok_or_else(|| {
                    AppError::Validation("previous report job has no report_type".to_string())
                })?
                .to_string();
            let period = previous
                .request
                .get("period")
                .and_then(serde_json::Value::as_str)
                .ok_or_else(|| {
                    AppError::Validation("previous report job has no period".to_string())
                })?
                .to_string();
            let title = previous
                .request
                .get("title")
                .and_then(serde_json::Value::as_str)
                .map(str::to_string);
            let payload = CreateReportRequest {
                report_type,
                period,
                title,
            };
            let job_id = Uuid::new_v4();
            let job = insert_platform_job(
                state.db(),
                job_id,
                "report_generation",
                "report",
                None,
                serde_json::json!({
                    "report_type": payload.report_type,
                    "period": payload.period,
                    "title": payload.title,
                    "action": "generate_report",
                    "retry_of": previous.id
                }),
                "queued retry report generation job",
            )
            .await?;
            spawn_report_job(state, job_id, payload);
            Ok(ok(job))
        }
        other => Err(AppError::Validation(format!(
            "job type {other} does not support retry yet"
        ))),
    }
}

async fn report_summary(
    State(state): State<AppState>,
) -> Result<Json<ApiResponse<SummaryResponse>>, AppError> {
    let total = count(state.db(), "SELECT COUNT(*) FROM generated_reports").await?;
    let ready = count(
        state.db(),
        "SELECT COUNT(*) FROM generated_reports WHERE status = 'ready'",
    )
    .await?;
    let draft = count(
        state.db(),
        "SELECT COUNT(*) FROM generated_reports WHERE status = 'draft'",
    )
    .await?;

    Ok(ok(SummaryResponse {
        generated_at: now(),
        metrics: vec![
            metric_num("total", "Reports", total, "healthy", false),
            metric_num("ready", "Ready Reports", ready, "healthy", false),
            metric_num(
                "draft",
                "Draft Reports",
                draft,
                if draft > 0 { "warning" } else { "healthy" },
                false,
            ),
            metric_num("llm_generation", "LLM Generation", 1, "placeholder", true),
        ],
    }))
}

async fn report_list(
    State(state): State<AppState>,
    Query(query): Query<PaginationQuery>,
) -> Result<Json<ApiResponse<PageResponse<ReportListItem>>>, AppError> {
    let page = normalize_page(query.page);
    let page_size = normalize_page_size(query.page_size);
    let offset = (page - 1) * page_size;
    let total = count(state.db(), "SELECT COUNT(*) FROM generated_reports").await?;

    let items = sqlx::query_as::<_, ReportListItem>(
        r#"
        SELECT id, title, report_type, period, status, file_path, summary, provider_style, model_name, generated_at, created_at
        FROM generated_reports
        WHERE ($1::TEXT IS NULL OR status = $1)
        ORDER BY generated_at DESC, id DESC
        LIMIT $2 OFFSET $3
        "#,
    )
    .bind(query.status.as_deref())
    .bind(page_size)
    .bind(offset)
    .fetch_all(state.db())
    .await
    .map_err(|_| AppError::Internal)?;

    Ok(ok(PageResponse {
        generated_at: now(),
        items,
        page,
        page_size,
        total,
    }))
}

async fn create_report_live(
    State(state): State<AppState>,
    Json(payload): Json<CreateReportRequest>,
) -> Result<Json<ApiResponse<ReportListItem>>, AppError> {
    Ok(ok(create_report_record(&state, payload).await?))
}

async fn start_report_job(
    State(state): State<AppState>,
    Json(payload): Json<CreateReportRequest>,
) -> Result<Json<ApiResponse<JobDto>>, AppError> {
    let report_type = payload.report_type.trim();
    let period = payload.period.trim();
    if report_type.is_empty() || period.is_empty() {
        return Err(AppError::Validation(
            "report_type and period are required".to_string(),
        ));
    }

    let job_id = Uuid::new_v4();
    let job = insert_platform_job(
        state.db(),
        job_id,
        "report_generation",
        "report",
        None,
        serde_json::json!({
            "report_type": payload.report_type.clone(),
            "period": payload.period.clone(),
            "title": payload.title.clone(),
            "action": "generate_report"
        }),
        "queued report generation job",
    )
    .await?;

    spawn_report_job(state, job_id, payload);
    Ok(ok(job))
}

async fn create_report_record(
    state: &AppState,
    payload: CreateReportRequest,
) -> Result<ReportListItem, AppError> {
    let report_type = payload.report_type.trim();
    let period = payload.period.trim();
    if report_type.is_empty() || period.is_empty() {
        return Err(AppError::Validation(
            "report_type and period are required".to_string(),
        ));
    }

    let integration_values = load_setting_map(state.db(), "integrations").await?;
    let ai_service = build_ai_service(&state, &integration_values);
    let report_id = Uuid::new_v4();
    let now_at = Utc::now();
    let title = payload
        .title
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(str::to_string)
        .unwrap_or_else(|| format!("{}-{}", report_type, period));

    let case_count = count(state.db(), "SELECT COUNT(*) FROM risk_cases").await?;
    let high_risk_count = count(
        state.db(),
        "SELECT COUNT(*) FROM risk_cases WHERE risk_level = 'high'",
    )
    .await?;
    let alert_count = count(
        state.db(),
        "SELECT COUNT(*) FROM alerts WHERE status IN ('open', 'acknowledged', 'closed')",
    )
    .await?;
    let dispatch_count = count(state.db(), "SELECT COUNT(*) FROM dispatch_tasks").await?;

    let report = ai_service
        .generate_report(&ReportInput {
            title: title.clone(),
            report_type: report_type.to_string(),
            period: period.to_string(),
            case_count,
            high_risk_count,
            alert_count,
            dispatch_count,
        })
        .await;

    let report_dir = state.settings().storage.report_dir.clone();
    std::fs::create_dir_all(&report_dir).map_err(|_| AppError::Internal)?;
    let file_path = format!(
        "{}/{}-{}.md",
        report_dir,
        report_type,
        period.replace('/', "-")
    );
    std::fs::write(&file_path, report.content).map_err(|_| AppError::Internal)?;

    let report_status = if report.is_placeholder {
        "draft"
    } else {
        "ready"
    };
    sqlx::query(
        r#"
        INSERT INTO generated_reports (
            id, title, report_type, period, status, file_path, generated_at, created_at,
            summary, provider_style, model_name
        )
        VALUES ($1, $2, $3, $4, $5, $6, $7, $7, $8, $9, $10)
        "#,
    )
    .bind(report_id)
    .bind(&title)
    .bind(report_type)
    .bind(period)
    .bind(report_status)
    .bind(&file_path)
    .bind(now_at)
    .bind(&report.summary)
    .bind(&report.model_contract.provider_style)
    .bind(&report.model_contract.model_name)
    .execute(state.db())
    .await
    .map_err(|_| AppError::Internal)?;

    let item = sqlx::query_as::<_, ReportListItem>(
        r#"
        SELECT id, title, report_type, period, status, file_path, summary, provider_style, model_name, generated_at, created_at
        FROM generated_reports
        WHERE id = $1
        "#,
    )
    .bind(report_id)
    .fetch_one(state.db())
    .await
    .map_err(|_| AppError::Internal)?;

    Ok(item)
}

async fn report_detail(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
) -> Result<Json<ApiResponse<ReportDetailItem>>, AppError> {
    let item = sqlx::query_as::<_, ReportListItem>(
        r#"
        SELECT id, title, report_type, period, status, file_path, summary, provider_style, model_name, generated_at, created_at
        FROM generated_reports
        WHERE id = $1
        "#,
    )
    .bind(id)
    .fetch_optional(state.db())
    .await
    .map_err(|_| AppError::Internal)?
    .ok_or(AppError::NotFound)?;
    let content_markdown = item
        .file_path
        .as_deref()
        .map(str::trim)
        .filter(|path| !path.is_empty())
        .and_then(|path| fs::read_to_string(path).ok());

    Ok(ok(ReportDetailItem {
        report: item,
        content_markdown,
    }))
}

async fn download_report(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
) -> Result<impl IntoResponse, AppError> {
    let item = sqlx::query_as::<_, ReportListItem>(
        r#"
        SELECT id, title, report_type, period, status, file_path, summary, provider_style, model_name, generated_at, created_at
        FROM generated_reports
        WHERE id = $1
        "#,
    )
    .bind(id)
    .fetch_optional(state.db())
    .await
    .map_err(|_| AppError::Internal)?
    .ok_or(AppError::NotFound)?;

    let file_path = item
        .file_path
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .ok_or(AppError::NotFound)?;
    let bytes = fs::read(file_path).map_err(|_| AppError::NotFound)?;
    let filename = format!(
        "{}-{}.md",
        sanitize_download_name(&item.report_type),
        sanitize_download_name(&item.period)
    );

    let mut headers = HeaderMap::new();
    headers.insert(
        header::CONTENT_TYPE,
        HeaderValue::from_static("text/markdown; charset=utf-8"),
    );
    headers.insert(
        header::CONTENT_DISPOSITION,
        HeaderValue::from_str(&format!("attachment; filename=\"{filename}\""))
            .map_err(|_| AppError::Internal)?,
    );

    Ok((headers, bytes))
}

async fn regenerate_report(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
) -> Result<Json<ApiResponse<ActionResponse>>, AppError> {
    let now_at = Utc::now();
    let result = sqlx::query(
        "UPDATE generated_reports SET status = 'draft', generated_at = $2 WHERE id = $1",
    )
    .bind(id)
    .bind(now_at)
    .execute(state.db())
    .await
    .map_err(|_| AppError::Internal)?;
    if result.rows_affected() == 0 {
        return Err(AppError::NotFound);
    }
    Ok(ok(ActionResponse {
        id: id.to_string(),
        status: "draft".to_string(),
        message: "report regenerated and moved back to draft".to_string(),
        updated_at: now_at.to_rfc3339(),
        is_placeholder: false,
    }))
}

async fn get_platform_settings_view(
    State(state): State<AppState>,
) -> Result<Json<ApiResponse<PlatformSettingsResponse>>, AppError> {
    let platform_values = load_setting_map(state.db(), "platform").await?;
    let integration_values = load_setting_map(state.db(), "integrations").await?;
    Ok(ok(build_platform_settings_response(
        &state,
        &platform_values,
        &integration_values,
    )))
}

async fn save_platform_settings_view(
    State(state): State<AppState>,
    Json(payload): Json<SavePlatformSettingsRequest>,
) -> Result<Json<ApiResponse<PlatformSettingsResponse>>, AppError> {
    let now_at = Utc::now();
    upsert_setting(
        state.db(),
        "platform",
        "platform_name",
        payload.platform_name.as_deref(),
        now_at,
    )
    .await?;
    upsert_setting(
        state.db(),
        "platform",
        "environment",
        payload.environment.as_deref(),
        now_at,
    )
    .await?;
    upsert_setting(
        state.db(),
        "platform",
        "upload_dir",
        payload.upload_dir.as_deref(),
        now_at,
    )
    .await?;
    upsert_setting(
        state.db(),
        "platform",
        "report_dir",
        payload.report_dir.as_deref(),
        now_at,
    )
    .await?;
    upsert_setting(
        state.db(),
        "platform",
        "training_dir",
        payload.training_dir.as_deref(),
        now_at,
    )
    .await?;
    get_platform_settings_view(State(state)).await
}

async fn get_integrations_settings_view(
    State(state): State<AppState>,
) -> Result<Json<ApiResponse<IntegrationSettingsResponse>>, AppError> {
    let values = load_setting_map(state.db(), "integrations").await?;
    Ok(ok(
        build_integration_settings_response(&state, &values).await
    ))
}

async fn save_integrations_settings(
    State(state): State<AppState>,
    Json(payload): Json<SaveIntegrationsSettingsRequest>,
) -> Result<Json<ApiResponse<IntegrationSettingsResponse>>, AppError> {
    let now_at = Utc::now();
    upsert_setting(
        state.db(),
        "integrations",
        "hugegraph_base_url",
        payload.hugegraph_base_url.as_deref(),
        now_at,
    )
    .await?;
    upsert_setting(
        state.db(),
        "integrations",
        "hugegraph_gremlin_url",
        payload.hugegraph_gremlin_url.as_deref(),
        now_at,
    )
    .await?;
    upsert_setting(
        state.db(),
        "integrations",
        "milvus_address",
        payload.milvus_address.as_deref(),
        now_at,
    )
    .await?;
    upsert_setting(
        state.db(),
        "integrations",
        "milvus_token",
        payload.milvus_token.as_deref(),
        now_at,
    )
    .await?;
    upsert_setting(
        state.db(),
        "integrations",
        "milvus_collection",
        payload.milvus_collection.as_deref(),
        now_at,
    )
    .await?;
    upsert_setting(
        state.db(),
        "integrations",
        "model_base_url",
        payload.model_base_url.as_deref(),
        now_at,
    )
    .await?;
    upsert_setting(
        state.db(),
        "integrations",
        "model_name",
        payload.model_name.as_deref(),
        now_at,
    )
    .await?;
    upsert_setting(
        state.db(),
        "integrations",
        "model_request_style",
        payload.model_request_style.as_deref(),
        now_at,
    )
    .await?;
    upsert_setting(
        state.db(),
        "integrations",
        "model_chat_endpoint",
        payload.model_chat_endpoint.as_deref(),
        now_at,
    )
    .await?;
    upsert_setting(
        state.db(),
        "integrations",
        "embedding_base_url",
        payload.embedding_base_url.as_deref(),
        now_at,
    )
    .await?;
    upsert_setting(
        state.db(),
        "integrations",
        "embedding_model",
        payload.embedding_model.as_deref(),
        now_at,
    )
    .await?;
    upsert_setting(
        state.db(),
        "integrations",
        "embedding_api_key",
        payload.embedding_api_key.as_deref(),
        now_at,
    )
    .await?;
    upsert_setting(
        state.db(),
        "integrations",
        "embedding_endpoint",
        payload.embedding_endpoint.as_deref(),
        now_at,
    )
    .await?;

    if let Some(json_mode_supported) = payload.model_json_mode_supported {
        upsert_setting(
            state.db(),
            "integrations",
            "model_json_mode_supported",
            Some(if json_mode_supported { "true" } else { "false" }),
            now_at,
        )
        .await?;
    }

    if let Some(api_key) = payload
        .openai_api_key
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty())
    {
        upsert_setting(
            state.db(),
            "integrations",
            "openai_api_key",
            Some(api_key),
            now_at,
        )
        .await?;
        upsert_setting(
            state.db(),
            "integrations",
            "model_api_key_configured",
            Some("true"),
            now_at,
        )
        .await?;
    }

    get_integrations_settings_view(State(state)).await
}

async fn test_integrations_view(
    State(state): State<AppState>,
    Json(payload): Json<TestIntegrationsRequest>,
) -> Result<Json<ApiResponse<IntegrationTestResponse>>, AppError> {
    let values = load_setting_map(state.db(), "integrations").await?;
    let hugegraph = payload.hugegraph_base_url.unwrap_or_else(|| {
        values
            .get("hugegraph_base_url")
            .cloned()
            .unwrap_or_else(|| state.settings().hugegraph.base_url.clone())
    });
    let milvus = payload.milvus_address.unwrap_or_else(|| {
        values
            .get("milvus_address")
            .cloned()
            .unwrap_or_else(|| state.settings().milvus.address.clone())
    });
    let model_base = payload.model_base_url.unwrap_or_else(|| {
        values
            .get("model_base_url")
            .cloned()
            .unwrap_or_else(|| state.settings().vllm.base_url.clone())
    });
    let model_name = payload.model_name.unwrap_or_else(|| {
        values
            .get("model_name")
            .cloned()
            .unwrap_or_else(|| state.settings().vllm.model_name.clone())
    });
    let embedding_base = payload.embedding_base_url.unwrap_or_else(|| {
        values
            .get("embedding_base_url")
            .cloned()
            .unwrap_or_else(|| state.settings().embedding.base_url.clone())
    });
    let embedding_model = payload.embedding_model.unwrap_or_else(|| {
        values
            .get("embedding_model")
            .cloned()
            .unwrap_or_else(|| state.settings().embedding.model_name.clone())
    });
    let model_chat_endpoint = payload.model_chat_endpoint.unwrap_or_else(|| {
        values
            .get("model_chat_endpoint")
            .cloned()
            .unwrap_or_else(|| "/chat/completions".to_string())
    });
    let embedding_endpoint_path = payload.embedding_endpoint.unwrap_or_else(|| {
        values
            .get("embedding_endpoint")
            .cloned()
            .or_else(|| {
                if !state.settings().embedding.endpoint.trim().is_empty() {
                    Some(state.settings().embedding.endpoint.clone())
                } else {
                    None
                }
            })
            .unwrap_or_else(|| "/embeddings".to_string())
    });
    let milvus_token = payload
        .milvus_token
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(str::to_string)
        .or_else(|| resolve_milvus_token(&state, &values));
    let api_key_configured = payload
        .openai_api_key
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .is_some()
        || bool_setting(&values, "model_api_key_configured", false);

    Ok(ok(IntegrationTestResponse {
        generated_at: now(),
        hugegraph: IntegrationStatus {
            key: "hugegraph".to_string(),
            endpoint: hugegraph.clone(),
            status: endpoint_status(&state, &hugegraph).await,
            configured: !hugegraph.trim().is_empty(),
            message: "HugeGraph HTTP probe".to_string(),
        },
        milvus: IntegrationStatus {
            key: "milvus".to_string(),
            endpoint: milvus.clone(),
            status: milvus_probe_status(&state, &milvus, milvus_token.as_deref()).await,
            configured: !milvus.trim().is_empty(),
            message: "Milvus REST v2 collection probe".to_string(),
        },
        model_service: ModelIntegrationStatus {
            key: "model_service".to_string(),
            endpoint: model_base.clone(),
            status: endpoint_status(
                &state,
                &format!("{}/models", model_base.trim_end_matches('/')),
            )
            .await,
            configured: !model_base.trim().is_empty(),
            message: "OpenAI-compatible model probe".to_string(),
            request_style: values
                .get("model_request_style")
                .cloned()
                .unwrap_or_else(|| "openai_chat_completion_compatible".to_string()),
            model: model_name,
            api_key_configured,
            chat_endpoint: model_chat_endpoint,
            json_mode_supported: bool_setting(&values, "model_json_mode_supported", true),
        },
        embedding_service: ModelIntegrationStatus {
            key: "embedding_service".to_string(),
            endpoint: embedding_base.clone(),
            status: embedding_probe_status(
                &state,
                &embedding_base,
                &embedding_endpoint_path,
                &embedding_model,
            )
            .await,
            configured: !embedding_model.trim().is_empty(),
            message: "OpenAI-compatible embedding probe".to_string(),
            request_style: "openai_embeddings_compatible".to_string(),
            model: embedding_model,
            api_key_configured: values
                .get("embedding_api_key")
                .map(|value| !value.trim().is_empty())
                .unwrap_or_else(|| !state.settings().embedding.api_key.trim().is_empty()),
            chat_endpoint: values
                .get("embedding_endpoint")
                .cloned()
                .unwrap_or(embedding_endpoint_path),
            json_mode_supported: false,
        },
    }))
}

async fn latest_workflow_runs(
    db: &sqlx::PgPool,
    limit: i64,
) -> Result<Vec<WorkflowRunDto>, AppError> {
    sqlx::query_as::<_, WorkflowRunDto>(
        r#"
        SELECT id, stage_key, stage_label, status, started_at, finished_at,
               item_count, success_count, failure_count, created_at, updated_at
        FROM workflow_runs
        ORDER BY started_at DESC, id DESC
        LIMIT $1
        "#,
    )
    .bind(limit)
    .fetch_all(db)
    .await
    .map_err(|_| AppError::Internal)
}

async fn list_workflow_stage_runs(
    state: AppState,
    query: PaginationQuery,
    stage_key: &str,
) -> Result<Json<ApiResponse<PageResponse<WorkflowRunDto>>>, AppError> {
    let page = normalize_page(query.page);
    let page_size = normalize_page_size(query.page_size);
    let offset = (page - 1) * page_size;
    let total =
        sqlx::query_scalar::<_, i64>("SELECT COUNT(*) FROM workflow_runs WHERE stage_key = $1")
            .bind(stage_key)
            .fetch_one(state.db())
            .await
            .map_err(|_| AppError::Internal)?;

    let items = sqlx::query_as::<_, WorkflowRunDto>(
        r#"
        SELECT id, stage_key, stage_label, status, started_at, finished_at,
               item_count, success_count, failure_count, created_at, updated_at
        FROM workflow_runs
        WHERE stage_key = $1
          AND ($2::TEXT IS NULL OR status = $2)
        ORDER BY started_at DESC, id DESC
        LIMIT $3 OFFSET $4
        "#,
    )
    .bind(stage_key)
    .bind(query.status.as_deref())
    .bind(page_size)
    .bind(offset)
    .fetch_all(state.db())
    .await
    .map_err(|_| AppError::Internal)?;

    Ok(ok(PageResponse {
        generated_at: now(),
        items,
        page,
        page_size,
        total,
    }))
}

async fn workflow_run_detail_by_id(
    db: &sqlx::PgPool,
    id: Uuid,
    stage_key: &str,
) -> Result<Json<ApiResponse<WorkflowRunDto>>, AppError> {
    let item = sqlx::query_as::<_, WorkflowRunDto>(
        r#"
        SELECT id, stage_key, stage_label, status, started_at, finished_at,
               item_count, success_count, failure_count, created_at, updated_at
        FROM workflow_runs
        WHERE id = $1 AND stage_key = $2
        "#,
    )
    .bind(id)
    .bind(stage_key)
    .fetch_optional(db)
    .await
    .map_err(|_| AppError::Internal)?
    .ok_or(AppError::NotFound)?;
    Ok(ok(item))
}

async fn ensure_workflow_run_exists(
    db: &sqlx::PgPool,
    id: Uuid,
    stage_key: &str,
) -> Result<(), AppError> {
    let count = sqlx::query_scalar::<_, i64>(
        "SELECT COUNT(*) FROM workflow_runs WHERE id = $1 AND stage_key = $2",
    )
    .bind(id)
    .bind(stage_key)
    .fetch_one(db)
    .await
    .map_err(|_| AppError::Internal)?;
    if count == 0 {
        return Err(AppError::NotFound);
    }
    Ok(())
}

async fn insert_workflow_run(
    db: &sqlx::PgPool,
    stage_key: &str,
    stage_label: &str,
    status: &str,
    item_count: i32,
    success_count: i32,
    failure_count: i32,
    now_at: DateTime<Utc>,
) -> Result<(), AppError> {
    sqlx::query(
        r#"
        INSERT INTO workflow_runs (
            id, stage_key, stage_label, status, started_at, finished_at,
            item_count, success_count, failure_count, created_at, updated_at
        )
        VALUES ($1, $2, $3, $4, $5, NULL, $6, $7, $8, $5, $5)
        "#,
    )
    .bind(Uuid::new_v4())
    .bind(stage_key)
    .bind(stage_label)
    .bind(status)
    .bind(now_at)
    .bind(item_count)
    .bind(success_count)
    .bind(failure_count)
    .execute(db)
    .await
    .map_err(|_| AppError::Internal)?;
    Ok(())
}

async fn ensure_import_exists(db: &sqlx::PgPool, import_id: Uuid) -> Result<(), AppError> {
    let count = sqlx::query_scalar::<_, i64>("SELECT COUNT(*) FROM imports WHERE id = $1")
        .bind(import_id)
        .fetch_one(db)
        .await
        .map_err(|_| AppError::Internal)?;
    if count == 0 {
        return Err(AppError::NotFound);
    }
    Ok(())
}

fn spawn_ingestion_processing_job(state: AppState, job_id: Uuid, import_id: Uuid) {
    tokio::spawn(async move {
        let db = state.db();
        let started_at = Utc::now();
        let _ = mark_platform_job_running(
            db,
            job_id,
            5,
            "loading integration settings for import processing",
            started_at,
        )
        .await;

        let result = async {
            let integration_values = load_setting_map(state.db(), "integrations").await?;
            let ai_service = build_ai_service(&state, &integration_values);
            let embedding_service = build_embedding_service(&state, &integration_values);
            let vector_store = build_vector_store(&state, &integration_values);

            let _ = update_platform_job_progress(
                state.db(),
                job_id,
                20,
                "processing import records into standardized risk cases",
            )
            .await;

            let result = process_import_batch(
                &state,
                import_id,
                &ai_service,
                &embedding_service,
                &vector_store,
            )
            .await?;

            Ok::<serde_json::Value, AppError>(serde_json::json!({
                "import_id": result.import_id,
                "status": result.status,
                "total_record_count": result.total_record_count,
                "processed_record_count": result.processed_record_count,
                "failed_record_count": result.failed_record_count,
                "affected_case_count": result.affected_case_count,
                "workflow_run_id": result.workflow_run_id,
                "mapping_template_id": result.mapping_template_id,
                "finished_at": result.finished_at
            }))
        }
        .await;

        match result {
            Ok(result_json) => {
                let message = format!(
                    "processed {} / {} records into {} cases, failed {}",
                    result_json
                        .get("processed_record_count")
                        .and_then(serde_json::Value::as_i64)
                        .unwrap_or(0),
                    result_json
                        .get("total_record_count")
                        .and_then(serde_json::Value::as_i64)
                        .unwrap_or(0),
                    result_json
                        .get("affected_case_count")
                        .and_then(serde_json::Value::as_i64)
                        .unwrap_or(0),
                    result_json
                        .get("failed_record_count")
                        .and_then(serde_json::Value::as_i64)
                        .unwrap_or(0)
                );
                let _ = complete_platform_job(
                    state.db(),
                    job_id,
                    Some(import_id),
                    "completed",
                    100,
                    &message,
                    result_json,
                )
                .await;
            }
            Err(error) => {
                let _ = fail_platform_job(state.db(), job_id, &app_error_message(&error)).await;
            }
        }
    });
}

fn spawn_extraction_job(state: AppState, job_id: Uuid, payload: CreateExtractionRunRequest) {
    tokio::spawn(async move {
        let started_at = Utc::now();
        let _ = mark_platform_job_running(
            state.db(),
            job_id,
            5,
            "loading integration settings for knowledge extraction",
            started_at,
        )
        .await;

        let result = async {
            let integration_values = load_setting_map(state.db(), "integrations").await?;
            let ai_service = build_ai_service(&state, &integration_values);
            let embedding_service = build_embedding_service(&state, &integration_values);
            let graph_service = build_hugegraph_service(&state, &integration_values);
            let vector_store = build_vector_store(&state, &integration_values);

            let _ = update_platform_job_progress(
                state.db(),
                job_id,
                15,
                "running extraction, risk recommendation, graph sync and vector indexing",
            )
            .await;

            let result = execute_extraction_run(
                &state,
                payload.case_ids,
                payload.mode,
                &ai_service,
                &embedding_service,
                &graph_service,
                &vector_store,
            )
            .await?;

            Ok::<(Uuid, serde_json::Value), AppError>((
                result.run_id,
                serde_json::json!({
                    "run_id": result.run_id,
                    "status": result.status,
                    "item_count": result.item_count,
                    "success_count": result.success_count,
                    "failure_count": result.failure_count,
                    "created_entity_count": result.created_entity_count,
                    "created_relation_count": result.created_relation_count,
                    "summary": result.summary,
                    "finished_at": result.finished_at
                }),
            ))
        }
        .await;

        match result {
            Ok((run_id, result_json)) => {
                let message = format!(
                    "processed {} cases, created {} entities and {} relations",
                    result_json
                        .get("item_count")
                        .and_then(serde_json::Value::as_i64)
                        .unwrap_or(0),
                    result_json
                        .get("created_entity_count")
                        .and_then(serde_json::Value::as_i64)
                        .unwrap_or(0),
                    result_json
                        .get("created_relation_count")
                        .and_then(serde_json::Value::as_i64)
                        .unwrap_or(0)
                );
                let _ = complete_platform_job(
                    state.db(),
                    job_id,
                    Some(run_id),
                    "completed",
                    100,
                    &message,
                    result_json,
                )
                .await;
            }
            Err(error) => {
                let _ = fail_platform_job(state.db(), job_id, &app_error_message(&error)).await;
            }
        }
    });
}

fn spawn_report_job(state: AppState, job_id: Uuid, payload: CreateReportRequest) {
    tokio::spawn(async move {
        let started_at = Utc::now();
        let _ = mark_platform_job_running(
            state.db(),
            job_id,
            10,
            "generating report through OpenAI-compatible model service",
            started_at,
        )
        .await;

        let result = create_report_record(&state, payload).await;
        match result {
            Ok(report) => {
                let result_json = serde_json::json!({
                    "report_id": report.id,
                    "title": report.title,
                    "report_type": report.report_type,
                    "period": report.period,
                    "status": report.status,
                    "file_path": report.file_path,
                    "summary": report.summary,
                    "provider_style": report.provider_style,
                    "model_name": report.model_name,
                    "generated_at": report.generated_at
                });
                let _ = complete_platform_job(
                    state.db(),
                    job_id,
                    Some(report.id),
                    "completed",
                    100,
                    "report generated and persisted",
                    result_json,
                )
                .await;
            }
            Err(error) => {
                let _ = fail_platform_job(state.db(), job_id, &app_error_message(&error)).await;
            }
        }
    });
}

async fn insert_platform_job(
    db: &sqlx::PgPool,
    job_id: Uuid,
    job_type: &str,
    target_type: &str,
    target_id: Option<Uuid>,
    request: serde_json::Value,
    message: &str,
) -> Result<JobDto, AppError> {
    let now_at = Utc::now();
    let request_json = json_to_string(request);
    let row = sqlx::query_as::<_, JobRow>(
        r#"
        INSERT INTO platform_jobs (
            id, job_type, target_type, target_id, status, progress_percent,
            message, request_json, result_json, error_message, started_at,
            finished_at, created_at, updated_at
        )
        VALUES ($1, $2, $3, $4, 'queued', 0, $5, $6, '{}', NULL, NULL, NULL, $7, $7)
        RETURNING id, job_type, target_type, target_id, status, progress_percent,
                  message, request_json, result_json, error_message, started_at,
                  finished_at, created_at, updated_at
        "#,
    )
    .bind(job_id)
    .bind(job_type)
    .bind(target_type)
    .bind(target_id)
    .bind(message)
    .bind(request_json)
    .bind(now_at)
    .fetch_one(db)
    .await
    .map_err(|_| AppError::Internal)?;
    Ok(map_job_row(row))
}

async fn get_platform_job(db: &sqlx::PgPool, job_id: Uuid) -> Result<JobDto, AppError> {
    let row = sqlx::query_as::<_, JobRow>(
        r#"
        SELECT id, job_type, target_type, target_id, status, progress_percent, message,
               request_json, result_json, error_message, started_at, finished_at, created_at, updated_at
        FROM platform_jobs
        WHERE id = $1
        "#,
    )
    .bind(job_id)
    .fetch_optional(db)
    .await
    .map_err(|_| AppError::Internal)?
    .ok_or(AppError::NotFound)?;
    Ok(map_job_row(row))
}

async fn mark_platform_job_running(
    db: &sqlx::PgPool,
    job_id: Uuid,
    progress_percent: i32,
    message: &str,
    now_at: DateTime<Utc>,
) -> Result<(), AppError> {
    sqlx::query(
        r#"
        UPDATE platform_jobs
        SET status = 'running',
            progress_percent = $2,
            message = $3,
            started_at = COALESCE(started_at, $4),
            updated_at = $4
        WHERE id = $1
        "#,
    )
    .bind(job_id)
    .bind(progress_percent.clamp(0, 99))
    .bind(message)
    .bind(now_at)
    .execute(db)
    .await
    .map_err(|_| AppError::Internal)?;
    Ok(())
}

async fn update_platform_job_progress(
    db: &sqlx::PgPool,
    job_id: Uuid,
    progress_percent: i32,
    message: &str,
) -> Result<(), AppError> {
    let now_at = Utc::now();
    sqlx::query(
        r#"
        UPDATE platform_jobs
        SET progress_percent = $2,
            message = $3,
            updated_at = $4
        WHERE id = $1
        "#,
    )
    .bind(job_id)
    .bind(progress_percent.clamp(0, 99))
    .bind(message)
    .bind(now_at)
    .execute(db)
    .await
    .map_err(|_| AppError::Internal)?;
    Ok(())
}

async fn complete_platform_job(
    db: &sqlx::PgPool,
    job_id: Uuid,
    target_id: Option<Uuid>,
    status: &str,
    progress_percent: i32,
    message: &str,
    result: serde_json::Value,
) -> Result<(), AppError> {
    let now_at = Utc::now();
    sqlx::query(
        r#"
        UPDATE platform_jobs
        SET status = $2,
            target_id = COALESCE($3::UUID, target_id),
            progress_percent = $4,
            message = $5,
            result_json = $6,
            error_message = NULL,
            finished_at = $7,
            updated_at = $7
        WHERE id = $1
        "#,
    )
    .bind(job_id)
    .bind(status)
    .bind(target_id)
    .bind(progress_percent.clamp(0, 100))
    .bind(message)
    .bind(json_to_string(result))
    .bind(now_at)
    .execute(db)
    .await
    .map_err(|_| AppError::Internal)?;
    Ok(())
}

async fn fail_platform_job(
    db: &sqlx::PgPool,
    job_id: Uuid,
    error_message: &str,
) -> Result<(), AppError> {
    let now_at = Utc::now();
    sqlx::query(
        r#"
        UPDATE platform_jobs
        SET status = 'failed',
            progress_percent = 100,
            message = 'job failed',
            error_message = $2,
            finished_at = $3,
            updated_at = $3
        WHERE id = $1
        "#,
    )
    .bind(job_id)
    .bind(error_message)
    .bind(now_at)
    .execute(db)
    .await
    .map_err(|_| AppError::Internal)?;
    Ok(())
}

fn map_job_row(row: JobRow) -> JobDto {
    JobDto {
        id: row.id.to_string(),
        job_type: row.job_type,
        target_type: row.target_type,
        target_id: row.target_id.map(|value| value.to_string()),
        status: row.status,
        progress_percent: row.progress_percent,
        message: row.message,
        request: parse_json_value(&row.request_json),
        result: parse_json_value(&row.result_json),
        error_message: row.error_message,
        started_at: row.started_at.map(|value| value.to_rfc3339()),
        finished_at: row.finished_at.map(|value| value.to_rfc3339()),
        created_at: row.created_at.to_rfc3339(),
        updated_at: row.updated_at.to_rfc3339(),
    }
}

fn parse_json_value(value: &str) -> serde_json::Value {
    serde_json::from_str(value).unwrap_or_else(|_| serde_json::json!({}))
}

fn json_to_string(value: serde_json::Value) -> String {
    serde_json::to_string(&value).unwrap_or_else(|_| "{}".to_string())
}

fn app_error_message(error: &AppError) -> String {
    match error {
        AppError::Internal => "server internal error".to_string(),
        AppError::NotFound => "resource not found".to_string(),
        AppError::Validation(message) => message.clone(),
        AppError::DependencyUnavailable(message) => {
            format!("dependency unavailable: {message}")
        }
        AppError::Unauthorized => "unauthorized".to_string(),
        AppError::Forbidden => "forbidden".to_string(),
    }
}

async fn update_risk_case_graph_sync_status(
    db: &sqlx::PgPool,
    case_id: Uuid,
    status: &str,
    message: &str,
    synced_at: DateTime<Utc>,
) -> Result<(), AppError> {
    sqlx::query(
        r#"
        UPDATE risk_cases
        SET graph_sync_status = $2,
            graph_sync_message = $3,
            graph_synced_at = $4,
            updated_at = $4
        WHERE id = $1
        "#,
    )
    .bind(case_id)
    .bind(status)
    .bind(truncate_sync_message(message))
    .bind(synced_at)
    .execute(db)
    .await
    .map_err(|_| AppError::Internal)?;
    Ok(())
}

async fn update_case_vector_sync_status(
    db: &sqlx::PgPool,
    case_id: Uuid,
    status: &str,
    message: &str,
    synced_at: DateTime<Utc>,
) -> Result<(), AppError> {
    sqlx::query(
        r#"
        UPDATE risk_cases
        SET vector_sync_status = $2,
            vector_sync_message = $3,
            vector_synced_at = $4,
            updated_at = $4
        WHERE id = $1
        "#,
    )
    .bind(case_id)
    .bind(status)
    .bind(truncate_sync_message(message))
    .bind(synced_at)
    .execute(db)
    .await
    .map_err(|_| AppError::Internal)?;
    Ok(())
}

async fn load_mapping_field_rows(
    db: &sqlx::PgPool,
    template_id: Uuid,
) -> Result<Vec<MappingFieldDto>, AppError> {
    sqlx::query_as::<_, MappingFieldDto>(
        r#"
        SELECT source_field, target_field, confidence, status, sample_value, sort_order, required
        FROM mapping_fields
        WHERE template_id = $1
        ORDER BY sort_order ASC, created_at ASC
        "#,
    )
    .bind(template_id)
    .fetch_all(db)
    .await
    .map_err(|_| AppError::Internal)
}

async fn load_setting_map(
    db: &sqlx::PgPool,
    category: &str,
) -> Result<HashMap<String, String>, AppError> {
    let rows = sqlx::query_as::<_, PlatformSettingRow>(
        "SELECT setting_key, setting_value FROM platform_settings WHERE category = $1",
    )
    .bind(category)
    .fetch_all(db)
    .await
    .map_err(|_| AppError::Internal)?;
    Ok(rows
        .into_iter()
        .map(|row| (row.setting_key, row.setting_value))
        .collect())
}

async fn find_case_for_agent_query(
    db: &sqlx::PgPool,
    query: &str,
) -> Result<Option<Uuid>, AppError> {
    let keyword = query.trim();
    if keyword.is_empty() {
        return Ok(None);
    }
    let like_pattern = format!("%{keyword}%");

    let exact = sqlx::query_scalar::<_, Uuid>(
        r#"
        SELECT id
        FROM risk_cases
        WHERE case_code = $1
           OR title = $1
        ORDER BY risk_score DESC, updated_at DESC
        LIMIT 1
        "#,
    )
    .bind(keyword)
    .fetch_optional(db)
    .await
    .map_err(|_| AppError::Internal)?;
    if exact.is_some() {
        return Ok(exact);
    }

    let fuzzy = sqlx::query_scalar::<_, Uuid>(
        r#"
        SELECT id
        FROM risk_cases
        WHERE case_code ILIKE $1
           OR title ILIKE $1
           OR area_name ILIKE $1
           OR source_type ILIKE $1
           OR risk_tags ILIKE $1
           OR risk_reason_summary ILIKE $1
        ORDER BY risk_score DESC, updated_at DESC
        LIMIT 1
        "#,
    )
    .bind(&like_pattern)
    .fetch_optional(db)
    .await
    .map_err(|_| AppError::Internal)?;
    if fuzzy.is_some() {
        return Ok(fuzzy);
    }

    sqlx::query_scalar::<_, Uuid>(
        r#"
        SELECT id
        FROM risk_cases
        ORDER BY risk_score DESC, updated_at DESC
        LIMIT 1
        "#,
    )
    .fetch_optional(db)
    .await
    .map_err(|_| AppError::Internal)
}

async fn load_vector_case_document(
    db: &sqlx::PgPool,
    case_id: Uuid,
) -> Result<VectorCaseDocument, AppError> {
    let row = sqlx::query_as::<_, RiskCaseListItem>(
        r#"
        SELECT id, case_code, title, source_type, area_name, risk_level, risk_score,
               status, alert_status, assignee, occurred_at, due_at, closed_at,
               report_period, review_status, risk_tags, risk_reason_summary, disposal_advice,
               graph_sync_status, graph_sync_message, graph_synced_at,
               vector_sync_status, vector_sync_message, vector_synced_at,
               created_at, updated_at
        FROM risk_cases
        WHERE id = $1
        "#,
    )
    .bind(case_id)
    .fetch_optional(db)
    .await
    .map_err(|_| AppError::Internal)?
    .ok_or(AppError::NotFound)?;

    let summary = [
        row.risk_reason_summary.trim(),
        row.disposal_advice.trim(),
        row.risk_tags.trim(),
    ]
    .into_iter()
    .filter(|value| !value.is_empty())
    .collect::<Vec<_>>()
    .join("\n");

    Ok(VectorCaseDocument {
        id: row.id.to_string(),
        case_id: row.id.to_string(),
        case_code: row.case_code,
        title: row.title,
        summary,
        risk_level: row.risk_level,
        source_type: row.source_type,
        area_name: row.area_name,
    })
}

fn vector_document_text(document: &VectorCaseDocument) -> String {
    [
        document.title.as_str(),
        document.summary.as_str(),
        document.area_name.as_str(),
        document.risk_level.as_str(),
        document.source_type.as_str(),
    ]
    .into_iter()
    .filter(|value| !value.trim().is_empty())
    .collect::<Vec<_>>()
    .join("\n")
}

fn build_agent_answer_markdown(
    intent: &str,
    query: &str,
    detail: &RiskCaseDetailResponse,
) -> String {
    let case_info = &detail.case_info;
    let advice = if detail.recommendations.disposal_advice.is_empty() {
        "暂无明确处置建议，需承办检察官结合原始材料复核。".to_string()
    } else {
        detail
            .recommendations
            .disposal_advice
            .iter()
            .enumerate()
            .map(|(index, item)| format!("{}. {}", index + 1, item))
            .collect::<Vec<_>>()
            .join("\n")
    };
    let reference_cases = if detail.recommendations.reference_cases.is_empty() {
        "暂未召回相似案件；请确认 Milvus 向量链路是否已完成索引。".to_string()
    } else {
        detail
            .recommendations
            .reference_cases
            .iter()
            .map(|item| {
                format!(
                    "- {} / {} / {} / 相似度 {:.3}",
                    item.case_code, item.title, item.risk_level, item.score
                )
            })
            .collect::<Vec<_>>()
            .join("\n")
    };
    let entity_names = detail
        .entities
        .iter()
        .take(8)
        .map(|item| format!("{}({})", item.entity_name, item.entity_type))
        .collect::<Vec<_>>()
        .join("、");

    format!(
        "### 智能研判结果\n\
         - 意图：{}\n\
         - 查询：{}\n\
         - 命中案件：{} / {}\n\
         - 风险等级：{}，评分：{}\n\
         - 属地与来源：{} / {}\n\
         - 外部同步：HugeGraph={}，Milvus={}\n\n\
         #### 风险原因\n\
         {}\n\n\
         #### 关键实体与关系\n\
         已抽取实体 {} 个、关系 {} 条。{}\n\n\
         #### 相似案件召回\n\
         {}\n\n\
         #### 处置建议\n\
         {}\n\n\
         > 本结果来自 JusticeAI 后端统一 Agent 分析契约，结合 PostgreSQL 案件、知识抽取结果、HugeGraph/Milvus 同步状态与 OpenAI-compatible 风险建议链路生成，正式处置前仍需人工复核。",
        intent,
        if query.trim().is_empty() {
            "按案件 ID 直接研判"
        } else {
            query
        },
        case_info.case_code,
        case_info.title,
        case_info.risk_level,
        case_info.risk_score,
        case_info.area_name,
        case_info.source_type,
        case_info.graph_sync_status,
        case_info.vector_sync_status,
        if detail.recommendations.reason_summary.trim().is_empty() {
            "后端暂未形成风险原因摘要。"
        } else {
            detail.recommendations.reason_summary.trim()
        },
        detail.entities.len(),
        detail.relations.len(),
        if entity_names.is_empty() {
            "暂无实体。".to_string()
        } else {
            format!("重点实体：{entity_names}。")
        },
        reference_cases,
        advice
    )
}

fn build_agent_graph_payload(detail: &RiskCaseDetailResponse) -> AgentGraphPayload {
    let case_id = detail.case_info.id.clone();
    let mut nodes = vec![AgentGraphNode {
        id: case_id.clone(),
        label: detail.case_info.case_code.clone(),
        node_type: "case".to_string(),
        size: 56,
        color: "#122E8A".to_string(),
    }];
    nodes.extend(detail.entities.iter().enumerate().map(|(index, item)| {
        let color = match item.entity_type.as_str() {
            "person" | "自然人" => "#8B5CF6",
            "organization" | "enterprise" | "企业" | "机构" => "#4A90E2",
            "risk" | "风险点" => "#D9363E",
            _ => ["#4A90E2", "#8B5CF6", "#F5A623", "#D9363E"][index % 4],
        };
        AgentGraphNode {
            id: item.id.to_string(),
            label: item.entity_name.clone(),
            node_type: item.entity_type.clone(),
            size: ((item.confidence * 42.0).round() as u32).clamp(24, 48),
            color: color.to_string(),
        }
    }));

    let mut edges = detail
        .entities
        .iter()
        .map(|item| AgentGraphEdge {
            source: case_id.clone(),
            target: item.id.to_string(),
            label: item.entity_type.clone(),
            confidence: item.confidence,
        })
        .collect::<Vec<_>>();
    edges.extend(detail.relations.iter().map(|item| AgentGraphEdge {
        source: item.source_entity_id.to_string(),
        target: item.target_entity_id.to_string(),
        label: item.relation_type.clone(),
        confidence: item.confidence,
    }));

    AgentGraphPayload { nodes, edges }
}

fn parse_job_target_uuid(job: &JobDto) -> Result<Uuid, AppError> {
    job.target_id
        .as_deref()
        .and_then(|value| Uuid::parse_str(value).ok())
        .or_else(|| {
            job.request
                .get("import_id")
                .and_then(serde_json::Value::as_str)
                .and_then(|value| Uuid::parse_str(value).ok())
        })
        .ok_or_else(|| AppError::Validation("job target id is missing".to_string()))
}

fn sanitize_download_name(value: &str) -> String {
    value
        .chars()
        .map(|ch| {
            if ch.is_ascii_alphanumeric() || matches!(ch, '-' | '_' | '.') {
                ch
            } else {
                '-'
            }
        })
        .collect::<String>()
        .trim_matches('-')
        .to_string()
}

async fn upsert_setting(
    db: &sqlx::PgPool,
    category: &str,
    key: &str,
    value: Option<&str>,
    now_at: DateTime<Utc>,
) -> Result<(), AppError> {
    let Some(value) = value.map(str::trim).filter(|value| !value.is_empty()) else {
        return Ok(());
    };

    sqlx::query(
        r#"
        INSERT INTO platform_settings (id, category, setting_key, setting_value, updated_at)
        VALUES ($1, $2, $3, $4, $5)
        ON CONFLICT (category, setting_key)
        DO UPDATE SET setting_value = EXCLUDED.setting_value, updated_at = EXCLUDED.updated_at
        "#,
    )
    .bind(Uuid::new_v4())
    .bind(category)
    .bind(key)
    .bind(value)
    .bind(now_at)
    .execute(db)
    .await
    .map_err(|_| AppError::Internal)?;
    Ok(())
}

async fn count(db: &sqlx::PgPool, sql: &str) -> Result<i64, AppError> {
    sqlx::query_scalar::<_, i64>(sql)
        .fetch_one(db)
        .await
        .map_err(|_| AppError::Internal)
}

async fn count_by_optional_status(
    db: &sqlx::PgPool,
    table: &str,
    status: Option<&str>,
) -> Result<i64, AppError> {
    match status {
        Some(status) => {
            let sql = format!("SELECT COUNT(*) FROM {table} WHERE status = $1");
            sqlx::query_scalar::<_, i64>(&sql)
                .bind(status)
                .fetch_one(db)
                .await
                .map_err(|_| AppError::Internal)
        }
        None => {
            let sql = format!("SELECT COUNT(*) FROM {table}");
            sqlx::query_scalar::<_, i64>(&sql)
                .fetch_one(db)
                .await
                .map_err(|_| AppError::Internal)
        }
    }
}

async fn check_postgres(db: &sqlx::PgPool) -> bool {
    sqlx::query_scalar::<_, i32>("SELECT 1")
        .fetch_one(db)
        .await
        .is_ok()
}

async fn endpoint_status(state: &AppState, url: &str) -> String {
    if url.trim().is_empty() {
        return "not_configured".to_string();
    }

    match state.http_client().get(url).send().await {
        Ok(response)
            if response.status().is_success()
                || response.status().as_u16() == 401
                || response.status().as_u16() == 404 =>
        {
            "up".to_string()
        }
        Ok(_) => "degraded".to_string(),
        Err(_) => "down".to_string(),
    }
}

async fn embedding_probe_status(
    state: &AppState,
    base_url: &str,
    endpoint: &str,
    model: &str,
) -> String {
    if base_url.trim().is_empty() || model.trim().is_empty() {
        return "not_configured".to_string();
    }

    let endpoint = if endpoint.starts_with('/') {
        endpoint.to_string()
    } else {
        format!("/{endpoint}")
    };
    let url = format!("{}{}", base_url.trim_end_matches('/'), endpoint);
    let payload = serde_json::json!({
        "model": model,
        "input": "JusticeAI embedding health check"
    });

    match state.http_client().post(url).json(&payload).send().await {
        Ok(response) if response.status().is_success() => "up".to_string(),
        Ok(_) => "degraded".to_string(),
        Err(_) => "down".to_string(),
    }
}

async fn milvus_probe_status(state: &AppState, address: &str, token: Option<&str>) -> String {
    if address.trim().is_empty() {
        return "not_configured".to_string();
    }

    let url = format!(
        "{}/v2/vectordb/collections/list",
        address.trim_end_matches('/')
    );
    let mut request = state.http_client().post(url).json(&serde_json::json!({}));
    if let Some(token) = token.map(str::trim).filter(|value| !value.is_empty()) {
        request = request.bearer_auth(token);
    }

    match request.send().await {
        Ok(response) if response.status().is_success() => "up".to_string(),
        Ok(_) => "degraded".to_string(),
        Err(_) => "down".to_string(),
    }
}

fn resolve_milvus_token(state: &AppState, values: &HashMap<String, String>) -> Option<String> {
    values
        .get("milvus_token")
        .cloned()
        .or_else(|| {
            if !state.settings().milvus.token.trim().is_empty() {
                Some(state.settings().milvus.token.clone())
            } else {
                None
            }
        })
        .or_else(|| {
            if !state.settings().milvus.username.trim().is_empty()
                && !state.settings().milvus.password.trim().is_empty()
            {
                Some(format!(
                    "{}:{}",
                    state.settings().milvus.username,
                    state.settings().milvus.password
                ))
            } else {
                None
            }
        })
        .or_else(|| Some("root:Milvus".to_string()))
}

fn normalize_page(page: Option<i64>) -> i64 {
    page.unwrap_or(1).max(1)
}

fn normalize_page_size(page_size: Option<i64>) -> i64 {
    page_size.unwrap_or(20).clamp(1, 100)
}

fn validate_mapping_template_payload(payload: &SaveMappingTemplateRequest) -> Result<(), AppError> {
    if payload.template_key.trim().is_empty()
        || payload.template_label.trim().is_empty()
        || payload.version.trim().is_empty()
        || payload.status.trim().is_empty()
        || payload.source_type.trim().is_empty()
    {
        return Err(AppError::Validation(
            "mapping template metadata is required".to_string(),
        ));
    }
    if payload.fields.is_empty() {
        return Err(AppError::Validation(
            "at least one mapping field is required".to_string(),
        ));
    }
    Ok(())
}

fn compute_mapping_completion_rate(fields: &[MappingFieldDto]) -> f32 {
    if fields.is_empty() {
        return 0.0;
    }
    let mapped = fields
        .iter()
        .filter(|field| !field.target_field.trim().is_empty() && field.status != "needs_review")
        .count() as f32;
    ((mapped / fields.len() as f32) * 1000.0).round() / 10.0
}

fn compute_missing_required_fields(fields: &[MappingFieldDto]) -> Vec<String> {
    let required = ["case_title", "area_name", "occurred_at"];
    required
        .iter()
        .filter(|required| !fields.iter().any(|field| field.target_field == **required))
        .map(|value| value.to_string())
        .collect()
}

fn normalize_risk_case_status(value: &str) -> Result<&'static str, AppError> {
    match value.trim().to_ascii_lowercase().as_str() {
        "todo" | "pending_review" | "in_progress" | "disposed" | "closed" => Ok(Box::leak(
            value.trim().to_ascii_lowercase().into_boxed_str(),
        )),
        _ => Err(AppError::Validation(
            "risk case status must be one of todo, pending_review, in_progress, disposed, closed"
                .to_string(),
        )),
    }
}

fn normalize_alert_status(value: &str) -> Result<&'static str, AppError> {
    match value.trim().to_ascii_lowercase().as_str() {
        "open" | "acknowledged" | "ignored" | "closed" => Ok(Box::leak(
            value.trim().to_ascii_lowercase().into_boxed_str(),
        )),
        _ => Err(AppError::Validation(
            "alert status must be one of open, acknowledged, ignored, closed".to_string(),
        )),
    }
}

fn normalize_dispatch_status(value: &str) -> Result<&'static str, AppError> {
    match value.trim().to_ascii_lowercase().as_str() {
        "assigned" | "in_progress" | "feedback_pending" | "completed" | "overdue" => {
            Ok(Box::leak(value.trim().to_ascii_lowercase().into_boxed_str()))
        }
        _ => Err(AppError::Validation(
            "dispatch status must be one of assigned, in_progress, feedback_pending, completed, overdue".to_string(),
        )),
    }
}

fn parse_optional_rfc3339(value: Option<&str>) -> Result<Option<DateTime<Utc>>, AppError> {
    let Some(value) = value.map(str::trim).filter(|value| !value.is_empty()) else {
        return Ok(None);
    };
    let parsed = DateTime::parse_from_rfc3339(value)
        .map_err(|_| AppError::Validation("due_at must be RFC3339".to_string()))?;
    Ok(Some(parsed.with_timezone(&Utc)))
}

fn percentage(numerator: i64, denominator: i64) -> f64 {
    if denominator <= 0 {
        0.0
    } else {
        ((numerator as f64 / denominator as f64) * 1000.0).round() / 10.0
    }
}

fn rate_status(value: f64) -> &'static str {
    if value >= 85.0 {
        "healthy"
    } else if value >= 70.0 {
        "warning"
    } else {
        "critical"
    }
}

fn source_label(source_key: &str) -> &'static str {
    match source_key {
        "hotline_12345" => "12345 Hotline",
        "police_110" => "110 Police",
        "platform_395" => "395 Platform",
        "petitions" => "Petitions",
        "manual_upload" => "Manual Upload",
        "knowledge_graph" => "Knowledge Graph",
        _ => "Other Source",
    }
}

fn relation_label(relation_type: &str) -> &'static str {
    match relation_type {
        "person_event" => "Person-Event",
        "event_area" => "Event-Area",
        "event_risk_factor" => "Event-RiskFactor",
        _ => "Other Relation",
    }
}

fn split_pipe_values(value: &str) -> Vec<String> {
    value
        .split('|')
        .map(str::trim)
        .filter(|segment| !segment.is_empty())
        .map(str::to_string)
        .collect()
}

fn split_csv_values(value: &str) -> Vec<String> {
    value
        .split(',')
        .map(str::trim)
        .filter(|segment| !segment.is_empty())
        .map(str::to_string)
        .collect()
}

fn bool_setting(values: &HashMap<String, String>, key: &str, default: bool) -> bool {
    values
        .get(key)
        .map(|value| {
            matches!(
                value.trim().to_ascii_lowercase().as_str(),
                "1" | "true" | "yes" | "on"
            )
        })
        .unwrap_or(default)
}

fn format_reference_case_hits(hits: &[SimilarCaseHit]) -> Vec<String> {
    hits.iter()
        .map(|hit| {
            format!(
                "{} | {} | {} | {:.4}",
                hit.case_code, hit.title, hit.risk_level, hit.score
            )
        })
        .collect()
}

fn map_similar_case_reference(hit: SimilarCaseHit) -> SimilarCaseReference {
    SimilarCaseReference {
        id: hit.id,
        case_id: hit.case_id,
        case_code: hit.case_code,
        title: hit.title,
        risk_level: hit.risk_level,
        score: hit.score,
    }
}

fn build_ai_service(
    state: &AppState,
    values: &HashMap<String, String>,
) -> OpenAiCompatibleAiService {
    OpenAiCompatibleAiService::new_with_endpoint(
        state.http_client().clone(),
        values
            .get("model_base_url")
            .cloned()
            .unwrap_or_else(|| state.settings().vllm.base_url.clone()),
        values
            .get("model_chat_endpoint")
            .cloned()
            .unwrap_or_else(|| "/chat/completions".to_string()),
        values
            .get("model_name")
            .cloned()
            .unwrap_or_else(|| state.settings().vllm.model_name.clone()),
        values.get("openai_api_key").cloned().or_else(|| {
            (!state.settings().vllm.api_key.trim().is_empty())
                .then_some(state.settings().vllm.api_key.clone())
        }),
    )
}

fn build_embedding_service(
    state: &AppState,
    values: &HashMap<String, String>,
) -> OpenAiCompatibleEmbeddingService {
    OpenAiCompatibleEmbeddingService::new(
        state.http_client().clone(),
        values
            .get("embedding_base_url")
            .cloned()
            .or_else(|| {
                if !state.settings().embedding.base_url.trim().is_empty() {
                    Some(state.settings().embedding.base_url.clone())
                } else {
                    None
                }
            })
            .unwrap_or_else(|| state.settings().vllm.base_url.clone()),
        values
            .get("embedding_endpoint")
            .cloned()
            .or_else(|| {
                if !state.settings().embedding.endpoint.trim().is_empty() {
                    Some(state.settings().embedding.endpoint.clone())
                } else {
                    None
                }
            })
            .unwrap_or_else(|| "/embeddings".to_string()),
        values
            .get("embedding_model")
            .cloned()
            .or_else(|| {
                if !state.settings().embedding.model_name.trim().is_empty() {
                    Some(state.settings().embedding.model_name.clone())
                } else {
                    None
                }
            })
            .unwrap_or_default(),
        values.get("embedding_api_key").cloned().or_else(|| {
            if !state.settings().embedding.api_key.trim().is_empty() {
                Some(state.settings().embedding.api_key.clone())
            } else {
                None
            }
        }),
    )
}

fn build_hugegraph_service(
    state: &AppState,
    values: &HashMap<String, String>,
) -> HugeGraphSyncService {
    HugeGraphSyncService::new(
        state.http_client().clone(),
        values
            .get("hugegraph_base_url")
            .cloned()
            .unwrap_or_else(|| state.settings().hugegraph.base_url.clone()),
        state.settings().hugegraph.username.clone(),
        state.settings().hugegraph.password.clone(),
    )
}

fn build_vector_store(state: &AppState, values: &HashMap<String, String>) -> MilvusVectorStore {
    let token = values
        .get("milvus_token")
        .cloned()
        .or_else(|| {
            if !state.settings().milvus.token.trim().is_empty() {
                Some(state.settings().milvus.token.clone())
            } else {
                None
            }
        })
        .or_else(|| {
            if !state.settings().milvus.username.trim().is_empty()
                && !state.settings().milvus.password.trim().is_empty()
            {
                Some(format!(
                    "{}:{}",
                    state.settings().milvus.username,
                    state.settings().milvus.password
                ))
            } else {
                None
            }
        })
        .or_else(|| Some("root:Milvus".to_string()));

    MilvusVectorStore::new(
        state.http_client().clone(),
        values
            .get("milvus_address")
            .cloned()
            .unwrap_or_else(|| state.settings().milvus.address.clone()),
        token,
        values
            .get("milvus_collection")
            .cloned()
            .unwrap_or_else(|| "justiceai_cases".to_string()),
        512,
    )
}

fn build_platform_settings_response(
    state: &AppState,
    platform_values: &HashMap<String, String>,
    integration_values: &HashMap<String, String>,
) -> PlatformSettingsResponse {
    let ai_service = build_ai_service(state, integration_values);
    let embedding_service = build_embedding_service(state, integration_values);
    let configured_contract = ai_service.configured_contract();
    let embedding_contract = embedding_service.contract();
    let settings = state.settings();

    PlatformSettingsResponse {
        generated_at: now(),
        platform: PlatformInfo {
            app_name: platform_values
                .get("platform_name")
                .cloned()
                .unwrap_or_else(|| settings.app.name.clone()),
            environment: platform_values
                .get("environment")
                .cloned()
                .unwrap_or_else(|| settings.app.env.clone()),
            api_base_path: "/api".to_string(),
            model_name: configured_contract.model_name.clone(),
        },
        integrations: vec![
            IntegrationStatusItem {
                key: "postgres".to_string(),
                label: "PostgreSQL".to_string(),
                status: "configured".to_string(),
                endpoint: settings.database.url.clone(),
            },
            IntegrationStatusItem {
                key: "hugegraph".to_string(),
                label: "HugeGraph".to_string(),
                status: if integration_values
                    .get("hugegraph_base_url")
                    .or_else(|| Some(&settings.hugegraph.base_url))
                    .map(|value| value.trim().is_empty())
                    .unwrap_or(true)
                {
                    "not_configured".to_string()
                } else {
                    "configured".to_string()
                },
                endpoint: integration_values
                    .get("hugegraph_base_url")
                    .cloned()
                    .unwrap_or_else(|| settings.hugegraph.base_url.clone()),
            },
            IntegrationStatusItem {
                key: "milvus".to_string(),
                label: "Milvus".to_string(),
                status: if integration_values
                    .get("milvus_address")
                    .or_else(|| Some(&settings.milvus.address))
                    .map(|value| value.trim().is_empty())
                    .unwrap_or(true)
                {
                    "not_configured".to_string()
                } else {
                    "configured".to_string()
                },
                endpoint: integration_values
                    .get("milvus_address")
                    .cloned()
                    .unwrap_or_else(|| settings.milvus.address.clone()),
            },
            IntegrationStatusItem {
                key: "model_service".to_string(),
                label: "Model Service".to_string(),
                status: "configured".to_string(),
                endpoint: configured_contract.base_url.clone(),
            },
        ],
        storage: HashMap::from([
            (
                "upload_dir".to_string(),
                platform_values
                    .get("upload_dir")
                    .cloned()
                    .unwrap_or_else(|| settings.storage.upload_dir.clone()),
            ),
            (
                "report_dir".to_string(),
                platform_values
                    .get("report_dir")
                    .cloned()
                    .unwrap_or_else(|| settings.storage.report_dir.clone()),
            ),
            (
                "training_dir".to_string(),
                platform_values
                    .get("training_dir")
                    .cloned()
                    .unwrap_or_else(|| settings.storage.training_dir.clone()),
            ),
        ]),
        model_contract: configured_contract.clone(),
        embedding_contract,
        is_placeholder: configured_contract.is_placeholder,
    }
}

async fn build_integration_settings_response(
    state: &AppState,
    values: &HashMap<String, String>,
) -> IntegrationSettingsResponse {
    let hugegraph_endpoint = values
        .get("hugegraph_base_url")
        .cloned()
        .unwrap_or_else(|| state.settings().hugegraph.base_url.clone());
    let milvus_endpoint = values
        .get("milvus_address")
        .cloned()
        .unwrap_or_else(|| state.settings().milvus.address.clone());
    let model_endpoint = values
        .get("model_base_url")
        .cloned()
        .unwrap_or_else(|| state.settings().vllm.base_url.clone());
    let embedding_endpoint = values
        .get("embedding_base_url")
        .cloned()
        .or_else(|| {
            if !state.settings().embedding.base_url.trim().is_empty() {
                Some(state.settings().embedding.base_url.clone())
            } else {
                None
            }
        })
        .unwrap_or_else(|| model_endpoint.clone());
    let embedding_path = values
        .get("embedding_endpoint")
        .cloned()
        .or_else(|| {
            if !state.settings().embedding.endpoint.trim().is_empty() {
                Some(state.settings().embedding.endpoint.clone())
            } else {
                None
            }
        })
        .unwrap_or_else(|| "/embeddings".to_string());
    let embedding_model = values
        .get("embedding_model")
        .cloned()
        .or_else(|| {
            if !state.settings().embedding.model_name.trim().is_empty() {
                Some(state.settings().embedding.model_name.clone())
            } else {
                None
            }
        })
        .unwrap_or_default();
    let embedding_status_value = embedding_probe_status(
        state,
        &embedding_endpoint,
        &embedding_path,
        &embedding_model,
    )
    .await;
    let milvus_status_value = milvus_probe_status(
        state,
        &milvus_endpoint,
        resolve_milvus_token(state, values).as_deref(),
    )
    .await;

    IntegrationSettingsResponse {
        generated_at: now(),
        database: IntegrationStatus {
            key: "postgres".to_string(),
            endpoint: state.settings().database.url.clone(),
            status: if check_postgres(state.db()).await {
                "up"
            } else {
                "down"
            }
            .to_string(),
            configured: true,
            message: "database connectivity is sourced from runtime settings".to_string(),
        },
        hugegraph: IntegrationStatus {
            key: "hugegraph".to_string(),
            endpoint: hugegraph_endpoint.clone(),
            status: endpoint_status(state, &hugegraph_endpoint).await,
            configured: !hugegraph_endpoint.trim().is_empty(),
            message: "HugeGraph sync endpoint is reserved for graph synchronization".to_string(),
        },
        milvus: IntegrationStatus {
            key: "milvus".to_string(),
            endpoint: milvus_endpoint.clone(),
            status: milvus_status_value,
            configured: !milvus_endpoint.trim().is_empty(),
            message: "Milvus vector index endpoint is configured".to_string(),
        },
        model_service: ModelIntegrationStatus {
            key: "model_service".to_string(),
            endpoint: model_endpoint.clone(),
            status: endpoint_status(
                state,
                &format!("{}/models", model_endpoint.trim_end_matches('/')),
            )
            .await,
            configured: !model_endpoint.trim().is_empty(),
            message: "OpenAI-compatible ChatCompletion endpoint is configured".to_string(),
            request_style: values
                .get("model_request_style")
                .cloned()
                .unwrap_or_else(|| "openai_chat_completion_compatible".to_string()),
            model: values
                .get("model_name")
                .cloned()
                .unwrap_or_else(|| state.settings().vllm.model_name.clone()),
            api_key_configured: bool_setting(values, "model_api_key_configured", false),
            chat_endpoint: values
                .get("model_chat_endpoint")
                .cloned()
                .unwrap_or_else(|| "/chat/completions".to_string()),
            json_mode_supported: bool_setting(values, "model_json_mode_supported", true),
        },
        embedding_service: ModelIntegrationStatus {
            key: "embedding_service".to_string(),
            endpoint: embedding_endpoint.clone(),
            status: embedding_status_value,
            configured: !embedding_model.trim().is_empty(),
            message: "OpenAI-compatible embeddings endpoint is configured for vector indexing"
                .to_string(),
            request_style: "openai_embeddings_compatible".to_string(),
            model: embedding_model,
            api_key_configured: values
                .get("embedding_api_key")
                .map(|value| !value.trim().is_empty())
                .unwrap_or_else(|| !state.settings().embedding.api_key.trim().is_empty()),
            chat_endpoint: embedding_path,
            json_mode_supported: false,
        },
    }
}

fn metric_num(
    key: &str,
    label: &str,
    value: i64,
    status: &str,
    is_placeholder: bool,
) -> SummaryMetric {
    SummaryMetric {
        key: key.to_string(),
        label: label.to_string(),
        value: serde_json::json!(value),
        unit: None,
        status: status.to_string(),
        is_placeholder,
    }
}

fn metric_card(
    key: &str,
    label: &str,
    value: String,
    unit: Option<String>,
    trend: Option<String>,
    trend_value: Option<String>,
    status: impl Into<String>,
) -> MetricCard {
    MetricCard {
        key: key.to_string(),
        label: label.to_string(),
        value,
        unit,
        trend,
        trend_value,
        status: status.into(),
    }
}

fn trend(period: &str, value: f64) -> TrendPoint {
    TrendPoint {
        period: period.to_string(),
        value,
    }
}

fn now() -> String {
    Utc::now().to_rfc3339()
}

fn truncate_sync_message(message: &str) -> String {
    message.trim().chars().take(1000).collect()
}
