use axum::{
    extract::{Path, Query, State},
    routing::{get, post},
    Json, Router,
};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use uuid::Uuid;

use crate::{
    app::AppState,
    shared::{
        error::AppError,
        response::{ok, ApiResponse},
    },
};

pub fn routes() -> Router<AppState> {
    Router::new()
        .route("/dashboard/overview", get(dashboard_overview))
        .route("/ingestion/summary", get(ingestion_summary))
        .route("/ingestion/batches", get(ingestion_batches))
        .route("/extraction/summary", get(extraction_summary))
        .route("/graph/overview", get(graph_overview))
        .route("/risk/overview", get(risk_overview))
        .route("/risk/cases", get(list_risk_cases))
        .route("/risk/cases/:id", get(get_risk_case_detail))
        .route("/risk/cases/:id/status", post(update_risk_case_status))
        .route("/alerts/summary", get(alerts_summary))
        .route("/alerts/:id/acknowledge", post(acknowledge_alert))
        .route("/dispatch/tasks", get(dispatch_tasks))
        .route("/dispatch/tasks/:id/assign", post(assign_dispatch_task))
        .route("/evaluation/summary", get(evaluation_summary))
        .route("/supervision/overview", get(supervision_overview))
        .route("/reports", get(reports))
        .route("/reports/generate", post(generate_report))
        .route("/settings/platform", get(platform_settings))
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "snake_case")]
struct MetricCard {
    key: &'static str,
    label: &'static str,
    value: String,
    unit: Option<&'static str>,
    trend: Option<&'static str>,
    trend_value: Option<String>,
    status: &'static str,
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
    key: &'static str,
    label: &'static str,
    count: u32,
    status: &'static str,
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
    record_count: u32,
    error_count: u32,
    imported_at: String,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "snake_case")]
struct IngestionBatchListResponse {
    generated_at: String,
    items: Vec<IngestionBatchItem>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "snake_case")]
struct ExtractionSummaryResponse {
    generated_at: String,
    metrics: Vec<MetricCard>,
    recent_entities: Vec<ExtractionEntityItem>,
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
struct GraphOverviewResponse {
    generated_at: String,
    metrics: Vec<MetricCard>,
    relation_types: Vec<RelationTypeItem>,
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
struct RiskOverviewResponse {
    generated_at: String,
    metrics: Vec<MetricCard>,
    top_risks: Vec<RiskItem>,
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
struct RiskCaseListResponse {
    generated_at: String,
    items: Vec<RiskCaseDetail>,
    total: usize,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "snake_case")]
struct RiskCaseDetail {
    id: String,
    case_code: String,
    title: String,
    source_type: String,
    area_name: String,
    risk_level: String,
    risk_score: f32,
    status: String,
    alert_status: String,
    assignee: Option<String>,
    occurred_at: Option<String>,
    due_at: Option<String>,
    closed_at: Option<String>,
    report_period: Option<String>,
    created_at: String,
    updated_at: String,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "snake_case")]
struct AlertsSummaryResponse {
    generated_at: String,
    metrics: Vec<MetricCard>,
    items: Vec<AlertItem>,
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
struct DispatchTaskListResponse {
    generated_at: String,
    items: Vec<DispatchTaskItem>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "snake_case")]
struct DispatchTaskItem {
    id: String,
    case_code: String,
    title: String,
    assignee: String,
    priority: String,
    status: String,
    due_at: String,
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
struct EvaluationDimensionItem {
    key: &'static str,
    label: &'static str,
    score: f32,
    status: &'static str,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "snake_case")]
struct SupervisionOverviewResponse {
    generated_at: String,
    metrics: Vec<MetricCard>,
    agents: Vec<AgentStatusItem>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "snake_case")]
struct AgentStatusItem {
    key: &'static str,
    label: &'static str,
    status: &'static str,
    running_tasks: u32,
    updated_at: String,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "snake_case")]
struct ReportListResponse {
    generated_at: String,
    items: Vec<ReportItem>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "snake_case")]
struct ReportItem {
    id: String,
    title: String,
    report_type: String,
    period: String,
    generated_at: String,
    status: String,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "snake_case")]
struct ReportGenerateResponse {
    id: String,
    title: String,
    report_type: String,
    period: String,
    status: String,
    generated_at: String,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "snake_case")]
struct ActionResultResponse {
    id: String,
    status: String,
    updated_at: String,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "snake_case")]
struct PlatformSettingsResponse {
    generated_at: String,
    platform: PlatformInfo,
    integrations: Vec<IntegrationStatusItem>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "snake_case")]
struct PlatformInfo {
    app_name: String,
    environment: String,
    api_base_path: &'static str,
    model_name: String,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "snake_case")]
struct IntegrationStatusItem {
    key: &'static str,
    label: &'static str,
    status: &'static str,
    endpoint: String,
}

#[derive(Debug, Deserialize)]
struct RiskCaseListQuery {
    status: Option<String>,
    risk_level: Option<String>,
}

#[derive(Debug, Deserialize)]
struct UpdateRiskCaseStatusRequest {
    status: String,
}

#[derive(Debug, Deserialize)]
struct AssignDispatchTaskRequest {
    assignee: String,
}

#[derive(Debug, Deserialize)]
struct GenerateReportRequest {
    report_type: String,
    period: String,
    title: Option<String>,
}

#[derive(Debug, FromRow)]
struct WorkflowRunRow {
    stage_key: String,
    stage_label: String,
    status: String,
    item_count: i32,
    success_count: i32,
}

#[derive(Debug, FromRow)]
struct IngestionSourceRow {
    source_type: String,
    batch_count: i64,
    latest_import_at: DateTime<Utc>,
}

#[derive(Debug, FromRow)]
struct IngestionBatchRow {
    id: Uuid,
    source_type: String,
    status: String,
    created_at: DateTime<Utc>,
    file_name: Option<String>,
}

#[derive(Debug, FromRow)]
struct ExtractionEntityRow {
    id: Uuid,
    entity_type: String,
    entity_name: String,
    confidence: f64,
    extracted_at: DateTime<Utc>,
}

#[derive(Debug, FromRow)]
struct RelationTypeRow {
    relation_type: String,
    relation_count: i64,
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

#[derive(Debug, FromRow)]
struct RiskCaseDetailRow {
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
    created_at: DateTime<Utc>,
    updated_at: DateTime<Utc>,
}

#[derive(Debug, FromRow)]
struct DispatchTaskRow {
    id: Uuid,
    case_code: String,
    title: String,
    assignee: Option<String>,
    risk_level: String,
    status: String,
    due_at: Option<DateTime<Utc>>,
}

#[derive(Debug, FromRow)]
struct ReportRow {
    id: Uuid,
    title: String,
    report_type: String,
    period: String,
    generated_at: DateTime<Utc>,
    status: String,
}

async fn dashboard_overview(
    State(state): State<AppState>,
) -> Result<Json<ApiResponse<DashboardOverviewResponse>>, AppError> {
    let total_cases = count(state.db(), "SELECT COUNT(*) FROM risk_cases").await?;
    let high_risk_cases = count(
        state.db(),
        "SELECT COUNT(*) FROM risk_cases WHERE risk_level = 'high'",
    )
    .await?;
    let active_alerts = count(
        state.db(),
        "SELECT COUNT(*) FROM risk_cases WHERE alert_status IN ('open', 'acknowledged')",
    )
    .await?;
    let closed_cases = count(state.db(), "SELECT COUNT(*) FROM risk_cases WHERE status = 'closed'").await?;
    let closed_loop_rate = percentage(closed_cases, total_cases);

    let workflow_rows = sqlx::query_as::<_, WorkflowRunRow>(
        r#"
        SELECT stage_key, stage_label, status, item_count, success_count
        FROM workflow_runs
        ORDER BY started_at ASC
        "#,
    )
    .fetch_all(state.db())
    .await
    .map_err(|_| AppError::Internal)?;

    let pending_dispatch = count(
        state.db(),
        "SELECT COUNT(*) FROM risk_cases WHERE status IN ('todo', 'open')",
    )
    .await?;
    let pending_review = count(
        state.db(),
        "SELECT COUNT(*) FROM generated_reports WHERE status = 'draft'",
    )
    .await?;
    let agent_exceptions = count(
        state.db(),
        "SELECT COUNT(*) FROM workflow_runs WHERE status IN ('failed', 'attention')",
    )
    .await?;

    Ok(ok(DashboardOverviewResponse {
        generated_at: now(),
        metrics: vec![
            metric("total_cases", "案件总量", total_cases.to_string(), Some("件"), None, None, "healthy"),
            metric(
                "high_risk_cases",
                "高风险线索",
                high_risk_cases.to_string(),
                Some("条"),
                None,
                None,
                if high_risk_cases > 0 { "warning" } else { "healthy" },
            ),
            metric(
                "active_alerts",
                "待处理预警",
                active_alerts.to_string(),
                Some("条"),
                None,
                None,
                if active_alerts > 0 { "warning" } else { "healthy" },
            ),
            metric(
                "closed_loop_rate",
                "闭环处置率",
                format!("{closed_loop_rate:.1}"),
                Some("%"),
                None,
                None,
                if closed_loop_rate >= 80.0 { "healthy" } else { "warning" },
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
            queue("pending_dispatch", "待分派任务", pending_dispatch as u32, status_by_count(pending_dispatch)),
            queue("pending_review", "待复核报告", pending_review as u32, status_by_count(pending_review)),
            queue(
                "agent_exceptions",
                "Agent 异常",
                agent_exceptions as u32,
                if agent_exceptions > 0 { "critical" } else { "healthy" },
            ),
        ],
    }))
}

async fn ingestion_summary(
    State(state): State<AppState>,
) -> Result<Json<ApiResponse<IngestionSummaryResponse>>, AppError> {
    let batch_total = count(state.db(), "SELECT COUNT(*) FROM imports").await?;
    let record_total = count(state.db(), "SELECT COUNT(*) FROM import_files").await?;
    let failed_records = count(
        state.db(),
        "SELECT COUNT(*) FROM imports WHERE status NOT IN ('uploaded', 'completed')",
    )
    .await?;

    let source_rows = sqlx::query_as::<_, IngestionSourceRow>(
        r#"
        SELECT source_type, COUNT(*)::BIGINT AS batch_count, MAX(created_at) AS latest_import_at
        FROM imports
        GROUP BY source_type
        ORDER BY MAX(created_at) DESC, source_type ASC
        "#,
    )
    .fetch_all(state.db())
    .await
    .map_err(|_| AppError::Internal)?;

    Ok(ok(IngestionSummaryResponse {
        generated_at: now(),
        totals: vec![
            metric("batch_total", "批次总数", batch_total.to_string(), Some("批"), None, None, "healthy"),
            metric("record_total", "导入记录", record_total.to_string(), Some("条"), None, None, "healthy"),
            metric(
                "failed_records",
                "失败批次",
                failed_records.to_string(),
                Some("批"),
                None,
                None,
                status_by_count(failed_records),
            ),
        ],
        sources: source_rows
            .into_iter()
            .map(|row| IngestionSourceSummary {
                source_key: row.source_type.clone(),
                source_label: source_label(&row.source_type).to_string(),
                batch_count: row.batch_count.max(0) as u32,
                record_count: row.batch_count.max(0) as u32,
                latest_import_at: row.latest_import_at.to_rfc3339(),
                status: if row.batch_count > 0 { "healthy" } else { "warning" }.to_string(),
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
                file_name: row.file_name.unwrap_or_else(|| "未命名导入文件".to_string()),
                status: row.status,
                record_count: 1,
                error_count: 0,
                imported_at: row.created_at.to_rfc3339(),
            })
            .collect(),
    }))
}

async fn extraction_summary(
    State(state): State<AppState>,
) -> Result<Json<ApiResponse<ExtractionSummaryResponse>>, AppError> {
    let entity_total = count(state.db(), "SELECT COUNT(*) FROM knowledge_entities").await?;
    let relation_total = count(state.db(), "SELECT COUNT(*) FROM graph_relations").await?;
    let low_confidence = count(
        state.db(),
        "SELECT COUNT(*) FROM knowledge_entities WHERE confidence < 0.90",
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
            metric("entities", "抽取实体", entity_total.to_string(), Some("个"), None, None, "healthy"),
            metric("relations", "抽取关系", relation_total.to_string(), Some("条"), None, None, "healthy"),
            metric(
                "low_confidence",
                "低置信结果",
                low_confidence.to_string(),
                Some("条"),
                None,
                None,
                status_by_count(low_confidence),
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

async fn graph_overview(
    State(state): State<AppState>,
) -> Result<Json<ApiResponse<GraphOverviewResponse>>, AppError> {
    let nodes = count(state.db(), "SELECT COUNT(*) FROM knowledge_entities").await?;
    let edges = count(state.db(), "SELECT COUNT(*) FROM graph_relations").await?;
    let communities = count(state.db(), "SELECT COUNT(DISTINCT case_id) FROM knowledge_entities").await?;

    let rows = sqlx::query_as::<_, RelationTypeRow>(
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
            metric("nodes", "图谱节点", nodes.to_string(), Some("个"), None, None, "healthy"),
            metric("edges", "图谱边", edges.to_string(), Some("条"), None, None, "healthy"),
            metric(
                "communities",
                "风险团簇",
                communities.to_string(),
                Some("个"),
                None,
                None,
                if communities > 0 { "warning" } else { "healthy" },
            ),
        ],
        relation_types: rows
            .into_iter()
            .map(|row| RelationTypeItem {
                key: row.relation_type.clone(),
                label: relation_label(&row.relation_type).to_string(),
                count: row.relation_count.max(0) as u32,
            })
            .collect(),
    }))
}

async fn risk_overview(
    State(state): State<AppState>,
) -> Result<Json<ApiResponse<RiskOverviewResponse>>, AppError> {
    let high = count(state.db(), "SELECT COUNT(*) FROM risk_cases WHERE risk_level = 'high'").await?;
    let medium = count(state.db(), "SELECT COUNT(*) FROM risk_cases WHERE risk_level = 'medium'").await?;
    let disposed = count(state.db(), "SELECT COUNT(*) FROM risk_cases WHERE status = 'closed'").await?;

    let rows = sqlx::query_as::<_, RiskCaseRow>(
        r#"
        SELECT id, title, risk_level, risk_score, area_name, status
        FROM risk_cases
        ORDER BY risk_score DESC, created_at DESC
        LIMIT 10
        "#,
    )
    .fetch_all(state.db())
    .await
    .map_err(|_| AppError::Internal)?;

    Ok(ok(RiskOverviewResponse {
        generated_at: now(),
        metrics: vec![
            metric("high", "高风险", high.to_string(), Some("条"), None, None, if high > 0 { "critical" } else { "healthy" }),
            metric("medium", "中风险", medium.to_string(), Some("条"), None, None, if medium > 0 { "warning" } else { "healthy" }),
            metric("disposed", "已处置", disposed.to_string(), Some("条"), None, None, "healthy"),
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

async fn list_risk_cases(
    State(state): State<AppState>,
    Query(query): Query<RiskCaseListQuery>,
) -> Result<Json<ApiResponse<RiskCaseListResponse>>, AppError> {
    let rows = sqlx::query_as::<_, RiskCaseDetailRow>(
        r#"
        SELECT id, case_code, title, source_type, area_name, risk_level, risk_score,
               status, alert_status, assignee, occurred_at, due_at, closed_at,
               report_period, created_at, updated_at
        FROM risk_cases
        WHERE ($1::TEXT IS NULL OR status = $1)
          AND ($2::TEXT IS NULL OR risk_level = $2)
        ORDER BY updated_at DESC, risk_score DESC
        LIMIT 100
        "#,
    )
    .bind(query.status.as_deref())
    .bind(query.risk_level.as_deref())
    .fetch_all(state.db())
    .await
    .map_err(|_| AppError::Internal)?;

    let items: Vec<RiskCaseDetail> = rows.into_iter().map(map_risk_case_detail).collect();
    let total = items.len();

    Ok(ok(RiskCaseListResponse {
        generated_at: now(),
        items,
        total,
    }))
}

async fn get_risk_case_detail(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
) -> Result<Json<ApiResponse<RiskCaseDetail>>, AppError> {
    let row = query_risk_case_detail(state.db(), id).await?;
    Ok(ok(map_risk_case_detail(row)))
}

async fn update_risk_case_status(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
    Json(payload): Json<UpdateRiskCaseStatusRequest>,
) -> Result<Json<ApiResponse<ActionResultResponse>>, AppError> {
    let status = normalize_case_status(&payload.status)?;
    let now_at = Utc::now();

    let updated = sqlx::query(
        r#"
        UPDATE risk_cases
        SET status = $2,
            closed_at = CASE WHEN $2 = 'closed' THEN $3 ELSE NULL END,
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

    if updated.rows_affected() == 0 {
        return Err(AppError::NotFound);
    }

    Ok(ok(ActionResultResponse {
        id: id.to_string(),
        status: status.to_string(),
        updated_at: now_at.to_rfc3339(),
    }))
}

async fn alerts_summary(
    State(state): State<AppState>,
) -> Result<Json<ApiResponse<AlertsSummaryResponse>>, AppError> {
    let unread = count(state.db(), "SELECT COUNT(*) FROM risk_cases WHERE alert_status = 'open'").await?;
    let escalated = count(state.db(), "SELECT COUNT(*) FROM risk_cases WHERE risk_level = 'high' AND status <> 'closed'").await?;
    let resolved = count(state.db(), "SELECT COUNT(*) FROM risk_cases WHERE alert_status = 'resolved'").await?;

    let rows = sqlx::query_as::<_, RiskCaseRow>(
        r#"
        SELECT id, title, risk_level, risk_score, area_name, status
        FROM risk_cases
        WHERE alert_status IN ('open', 'acknowledged', 'resolved')
        ORDER BY updated_at DESC, risk_score DESC
        LIMIT 10
        "#,
    )
    .fetch_all(state.db())
    .await
    .map_err(|_| AppError::Internal)?;

    Ok(ok(AlertsSummaryResponse {
        generated_at: now(),
        metrics: vec![
            metric("unread", "未读预警", unread.to_string(), Some("条"), None, None, status_by_count(unread)),
            metric("escalated", "已升级", escalated.to_string(), Some("条"), None, None, if escalated > 0 { "critical" } else { "healthy" }),
            metric("resolved", "已消警", resolved.to_string(), Some("条"), None, None, "healthy"),
        ],
        items: rows
            .into_iter()
            .map(|row| AlertItem {
                id: row.id.to_string(),
                title: row.title,
                level: row.risk_level,
                source: row.area_name,
                triggered_at: now(),
                status: row.status,
            })
            .collect(),
    }))
}

async fn acknowledge_alert(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
) -> Result<Json<ApiResponse<ActionResultResponse>>, AppError> {
    let now_at = Utc::now();

    let updated = sqlx::query(
        r#"
        UPDATE risk_cases
        SET alert_status = 'acknowledged', updated_at = $2
        WHERE id = $1
        "#,
    )
    .bind(id)
    .bind(now_at)
    .execute(state.db())
    .await
    .map_err(|_| AppError::Internal)?;

    if updated.rows_affected() == 0 {
        return Err(AppError::NotFound);
    }

    Ok(ok(ActionResultResponse {
        id: id.to_string(),
        status: "acknowledged".to_string(),
        updated_at: now_at.to_rfc3339(),
    }))
}

async fn dispatch_tasks(
    State(state): State<AppState>,
) -> Result<Json<ApiResponse<DispatchTaskListResponse>>, AppError> {
    let rows = sqlx::query_as::<_, DispatchTaskRow>(
        r#"
        SELECT id, case_code, title, assignee, risk_level, status, due_at
        FROM risk_cases
        WHERE status IN ('todo', 'open', 'in_progress')
        ORDER BY COALESCE(due_at, created_at) ASC, risk_score DESC
        LIMIT 20
        "#,
    )
    .fetch_all(state.db())
    .await
    .map_err(|_| AppError::Internal)?;

    Ok(ok(DispatchTaskListResponse {
        generated_at: now(),
        items: rows
            .into_iter()
            .map(|row| DispatchTaskItem {
                id: row.id.to_string(),
                case_code: row.case_code,
                title: row.title,
                assignee: row.assignee.unwrap_or_else(|| "待分派".to_string()),
                priority: priority_from_level(&row.risk_level).to_string(),
                status: row.status,
                due_at: row.due_at.unwrap_or_else(Utc::now).to_rfc3339(),
            })
            .collect(),
    }))
}

async fn assign_dispatch_task(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
    Json(payload): Json<AssignDispatchTaskRequest>,
) -> Result<Json<ApiResponse<ActionResultResponse>>, AppError> {
    let assignee = payload.assignee.trim();
    if assignee.is_empty() {
        return Err(AppError::Validation("承办人不能为空".to_string()));
    }

    let now_at = Utc::now();
    let updated = sqlx::query(
        r#"
        UPDATE risk_cases
        SET assignee = $2,
            status = CASE WHEN status = 'todo' THEN 'in_progress' ELSE status END,
            updated_at = $3
        WHERE id = $1
        "#,
    )
    .bind(id)
    .bind(assignee)
    .bind(now_at)
    .execute(state.db())
    .await
    .map_err(|_| AppError::Internal)?;

    if updated.rows_affected() == 0 {
        return Err(AppError::NotFound);
    }

    Ok(ok(ActionResultResponse {
        id: id.to_string(),
        status: "assigned".to_string(),
        updated_at: now_at.to_rfc3339(),
    }))
}

async fn evaluation_summary(
    State(state): State<AppState>,
) -> Result<Json<ApiResponse<EvaluationSummaryResponse>>, AppError> {
    let total = count(state.db(), "SELECT COUNT(*) FROM risk_cases").await?;
    let closed = count(state.db(), "SELECT COUNT(*) FROM risk_cases WHERE status = 'closed'").await?;
    let mitigated = count(
        state.db(),
        "SELECT COUNT(*) FROM risk_cases WHERE alert_status = 'resolved' OR status = 'closed'",
    )
    .await?;
    let timely = count(
        state.db(),
        "SELECT COUNT(*) FROM risk_cases WHERE due_at IS NOT NULL AND (closed_at IS NULL OR closed_at <= due_at)",
    )
    .await?;

    let closure_rate = percentage(closed, total);
    let mitigation_rate = percentage(mitigated, total);
    let timeliness_rate = percentage(timely, total);
    let accuracy_score = if total == 0 { 100.0 } else { 82.0 + (mitigated as f32 / total as f32) * 18.0 };

    Ok(ok(EvaluationSummaryResponse {
        generated_at: now(),
        metrics: vec![
            metric("closure_rate", "办结率", format!("{closure_rate:.1}"), Some("%"), None, None, rate_status(closure_rate)),
            metric("mitigation_rate", "化解率", format!("{mitigation_rate:.1}"), Some("%"), None, None, rate_status(mitigation_rate)),
            metric("timeliness", "按期处置率", format!("{timeliness_rate:.1}"), Some("%"), None, None, rate_status(timeliness_rate)),
        ],
        dimensions: vec![
            EvaluationDimensionItem {
                key: "timeliness",
                label: "时效性",
                score: timeliness_rate,
                status: rate_status(timeliness_rate),
            },
            EvaluationDimensionItem {
                key: "accuracy",
                label: "研判准确性",
                score: accuracy_score,
                status: rate_status(accuracy_score),
            },
            EvaluationDimensionItem {
                key: "closure",
                label: "闭环成效",
                score: closure_rate,
                status: rate_status(closure_rate),
            },
        ],
    }))
}

async fn supervision_overview(
    State(state): State<AppState>,
) -> Result<Json<ApiResponse<SupervisionOverviewResponse>>, AppError> {
    let running_agents = count(
        state.db(),
        "SELECT COUNT(*) FROM workflow_runs WHERE status = 'running'",
    )
    .await?;
    let paused_agents = count(
        state.db(),
        "SELECT COUNT(*) FROM workflow_runs WHERE status = 'draft'",
    )
    .await?;
    let manual_interventions = count(
        state.db(),
        "SELECT COUNT(*) FROM risk_cases WHERE status IN ('todo', 'open') AND risk_level = 'high'",
    )
    .await?;

    Ok(ok(SupervisionOverviewResponse {
        generated_at: now(),
        metrics: vec![
            metric("running_agents", "运行中 Agent", running_agents.to_string(), Some("个"), None, None, "healthy"),
            metric("paused_agents", "暂停 Agent", paused_agents.to_string(), Some("个"), None, None, if paused_agents > 0 { "warning" } else { "healthy" }),
            metric("manual_interventions", "人工介入", manual_interventions.to_string(), Some("次"), None, None, if manual_interventions > 0 { "warning" } else { "healthy" }),
        ],
        agents: vec![
            AgentStatusItem {
                key: "lead_mining",
                label: "线索挖掘 Agent",
                status: if running_agents > 0 { "running" } else { "ready" },
                running_tasks: running_agents.max(0) as u32,
                updated_at: now(),
            },
            AgentStatusItem {
                key: "risk_assessment",
                label: "风险研判 Agent",
                status: if manual_interventions > 0 { "attention" } else { "running" },
                running_tasks: manual_interventions.max(0) as u32,
                updated_at: now(),
            },
            AgentStatusItem {
                key: "supervisor",
                label: "监督协调 Agent",
                status: if paused_agents > 0 { "attention" } else { "healthy" },
                running_tasks: paused_agents.max(0) as u32,
                updated_at: now(),
            },
        ],
    }))
}

async fn reports(
    State(state): State<AppState>,
) -> Result<Json<ApiResponse<ReportListResponse>>, AppError> {
    let rows = sqlx::query_as::<_, ReportRow>(
        r#"
        SELECT id, title, report_type, period, generated_at, status
        FROM generated_reports
        ORDER BY generated_at DESC, id DESC
        LIMIT 20
        "#,
    )
    .fetch_all(state.db())
    .await
    .map_err(|_| AppError::Internal)?;

    Ok(ok(ReportListResponse {
        generated_at: now(),
        items: rows
            .into_iter()
            .map(|row| ReportItem {
                id: row.id.to_string(),
                title: row.title,
                report_type: row.report_type,
                period: row.period,
                generated_at: row.generated_at.to_rfc3339(),
                status: row.status,
            })
            .collect(),
    }))
}

async fn generate_report(
    State(state): State<AppState>,
    Json(payload): Json<GenerateReportRequest>,
) -> Result<Json<ApiResponse<ReportGenerateResponse>>, AppError> {
    let report_type = payload.report_type.trim();
    let period = payload.period.trim();

    if report_type.is_empty() {
        return Err(AppError::Validation("报告类型不能为空".to_string()));
    }

    if period.is_empty() {
        return Err(AppError::Validation("报告周期不能为空".to_string()));
    }

    let now_at = Utc::now();
    let report_id = Uuid::new_v4();
    let title = payload
        .title
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(str::to_string)
        .unwrap_or_else(|| default_report_title(report_type, period));
    let file_path = format!("runtime/reports/{}-{}.md", report_type, period.replace('/', "-"));

    sqlx::query(
        r#"
        INSERT INTO generated_reports (id, title, report_type, period, status, file_path, generated_at, created_at)
        VALUES ($1, $2, $3, $4, $5, $6, $7, $8)
        "#,
    )
    .bind(report_id)
    .bind(&title)
    .bind(report_type)
    .bind(period)
    .bind("ready")
    .bind(&file_path)
    .bind(now_at)
    .bind(now_at)
    .execute(state.db())
    .await
    .map_err(|_| AppError::Internal)?;

    let report_body = format!(
        "# {}\n\n- 报告类型：{}\n- 周期：{}\n- 生成时间：{}\n- 状态：ready\n\n> 当前为后端占位生成结果，后续可替换为模板化正文与图表导出。\n",
        title,
        report_type,
        period,
        now_at.to_rfc3339(),
    );

    std::fs::write(&file_path, report_body).map_err(|_| AppError::Internal)?;

    Ok(ok(ReportGenerateResponse {
        id: report_id.to_string(),
        title,
        report_type: report_type.to_string(),
        period: period.to_string(),
        status: "ready".to_string(),
        generated_at: now_at.to_rfc3339(),
    }))
}

async fn platform_settings(
    State(state): State<AppState>,
) -> Json<ApiResponse<PlatformSettingsResponse>> {
    let settings = state.settings();

    ok(PlatformSettingsResponse {
        generated_at: now(),
        platform: PlatformInfo {
            app_name: settings.app.name.clone(),
            environment: settings.app.env.clone(),
            api_base_path: "/api",
            model_name: settings.vllm.model_name.clone(),
        },
        integrations: vec![
            IntegrationStatusItem {
                key: "postgres",
                label: "PostgreSQL",
                status: "configured",
                endpoint: settings.database.url.clone(),
            },
            IntegrationStatusItem {
                key: "hugegraph",
                label: "HugeGraph",
                status: "configured",
                endpoint: settings.hugegraph.base_url.clone(),
            },
            IntegrationStatusItem {
                key: "milvus",
                label: "Milvus",
                status: "configured",
                endpoint: settings.milvus.address.clone(),
            },
            IntegrationStatusItem {
                key: "vllm",
                label: "vLLM",
                status: "configured",
                endpoint: settings.vllm.base_url.clone(),
            },
        ],
    })
}

async fn query_risk_case_detail(db: &sqlx::PgPool, id: Uuid) -> Result<RiskCaseDetailRow, AppError> {
    sqlx::query_as::<_, RiskCaseDetailRow>(
        r#"
        SELECT id, case_code, title, source_type, area_name, risk_level, risk_score,
               status, alert_status, assignee, occurred_at, due_at, closed_at,
               report_period, created_at, updated_at
        FROM risk_cases
        WHERE id = $1
        "#,
    )
    .bind(id)
    .fetch_optional(db)
    .await
    .map_err(|_| AppError::Internal)?
    .ok_or(AppError::NotFound)
}

fn map_risk_case_detail(row: RiskCaseDetailRow) -> RiskCaseDetail {
    RiskCaseDetail {
        id: row.id.to_string(),
        case_code: row.case_code,
        title: row.title,
        source_type: row.source_type,
        area_name: row.area_name,
        risk_level: row.risk_level,
        risk_score: row.risk_score as f32,
        status: row.status,
        alert_status: row.alert_status,
        assignee: row.assignee,
        occurred_at: row.occurred_at.map(|value| value.to_rfc3339()),
        due_at: row.due_at.map(|value| value.to_rfc3339()),
        closed_at: row.closed_at.map(|value| value.to_rfc3339()),
        report_period: row.report_period,
        created_at: row.created_at.to_rfc3339(),
        updated_at: row.updated_at.to_rfc3339(),
    }
}

fn normalize_case_status(status: &str) -> Result<&str, AppError> {
    let normalized = status.trim().to_ascii_lowercase();

    match normalized.as_str() {
        "todo" | "open" | "in_progress" | "closed" => Ok(Box::leak(normalized.into_boxed_str())),
        _ => Err(AppError::Validation("案件状态仅支持 todo、open、in_progress、closed".to_string())),
    }
}

async fn count(db: &sqlx::PgPool, sql: &str) -> Result<i64, AppError> {
    sqlx::query_scalar::<_, i64>(sql)
        .fetch_one(db)
        .await
        .map_err(|_| AppError::Internal)
}

fn percentage(numerator: i64, denominator: i64) -> f32 {
    if denominator <= 0 {
        return 0.0;
    }

    ((numerator as f32 / denominator as f32) * 1000.0).round() / 10.0
}

fn status_by_count(count: i64) -> &'static str {
    if count > 0 {
        "warning"
    } else {
        "healthy"
    }
}

fn rate_status(rate: f32) -> &'static str {
    if rate >= 85.0 {
        "healthy"
    } else if rate >= 70.0 {
        "warning"
    } else {
        "critical"
    }
}

fn source_label(source_key: &str) -> &'static str {
    match source_key {
        "hotline_12345" => "12345 热线",
        "police_110" => "110 接警",
        "platform_395" => "395 平台",
        "petitions" => "信访数据",
        "manual_upload" => "手工导入",
        "knowledge_graph" => "知识图谱",
        _ => "其他来源",
    }
}

fn relation_label(relation_type: &str) -> &'static str {
    match relation_type {
        "person_event" => "人-事件",
        "event_department" => "事件-部门",
        "person_area" => "人-区域",
        _ => "其他关系",
    }
}

fn priority_from_level(level: &str) -> &'static str {
    match level {
        "high" => "high",
        "medium" => "medium",
        _ => "low",
    }
}

fn default_report_title(report_type: &str, period: &str) -> String {
    format!("{}报告-{}", report_type, period)
}

fn now() -> String {
    Utc::now().to_rfc3339()
}

fn metric(
    key: &'static str,
    label: &'static str,
    value: impl Into<String>,
    unit: Option<&'static str>,
    trend: Option<&'static str>,
    trend_value: Option<&'static str>,
    status: &'static str,
) -> MetricCard {
    MetricCard {
        key,
        label,
        value: value.into(),
        unit,
        trend,
        trend_value: trend_value.map(str::to_string),
        status,
    }
}

fn queue(key: &'static str, label: &'static str, count: u32, status: &'static str) -> QueueItem {
    QueueItem {
        key,
        label,
        count,
        status,
    }
}
