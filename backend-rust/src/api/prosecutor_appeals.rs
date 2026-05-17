use axum::{
    extract::{Path, Query, State},
    http::HeaderMap,
    routing::{get, post},
    Json, Router,
};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use uuid::Uuid;

use crate::{
    api::platform,
    app::AppState,
    domain::appeal_status,
    services::{
        appeal_conversion_service,
        appeal_conversion_service::ConversionResult,
        appeal_service::{
            self, mask_name, mask_phone, AppealEventRow, AppealLocationRow, AppealMaterialRow,
            AppealRiskCaseLinkRow, ConvertRiskCaseInput, LaborAppealRow, LinkRiskCaseInput,
            RequestMaterialsInput, ResolveInput, StaffConfirmLocationInput, StatusActionInput,
        },
    },
    shared::{
        error::AppError,
        response::{ok, ApiResponse},
    },
};

pub fn routes() -> Router<AppState> {
    Router::new()
        .route("/prosecutor/appeals", get(list_appeals))
        .route("/prosecutor/appeals/:id", get(appeal_detail))
        .route("/prosecutor/appeals/:id/graph", get(appeal_graph))
        .route("/prosecutor/appeals/:id/similar", get(appeal_similar))
        .route("/prosecutor/appeals/:id/accept", post(accept))
        .route(
            "/prosecutor/appeals/:id/request-materials",
            post(request_materials),
        )
        .route("/prosecutor/appeals/:id/status", post(status_action))
        .route(
            "/prosecutor/appeals/:id/convert-risk-case",
            post(convert_risk_case),
        )
        .route(
            "/prosecutor/appeals/:id/link-risk-case",
            post(link_risk_case),
        )
        .route("/prosecutor/appeals/:id/resolve", post(resolve))
        .route(
            "/prosecutor/appeals/:id/location/confirm",
            post(confirm_location),
        )
}

#[derive(Debug, Deserialize)]
struct AppealListQuery {
    status: Option<String>,
    area_code: Option<String>,
    keyword: Option<String>,
    submitted_from: Option<DateTime<Utc>>,
    submitted_to: Option<DateTime<Utc>>,
    page: Option<i64>,
    page_size: Option<i64>,
}

#[derive(Debug, Deserialize)]
struct SimilarQuery {
    limit: Option<i64>,
}

#[derive(Debug, Serialize)]
struct PageResponse<T> {
    items: Vec<T>,
    page: i64,
    page_size: i64,
    total: i64,
}

#[derive(Debug, Serialize, FromRow)]
struct AppealListRow {
    id: Uuid,
    appeal_code: String,
    status: String,
    risk_case_status: String,
    worker_name: String,
    worker_phone: String,
    project_name: String,
    wage_amount_text: String,
    material_score: i32,
    missing_materials: String,
    area_code: Option<String>,
    area_name: Option<String>,
    address_text: Option<String>,
    latitude: Option<f64>,
    longitude: Option<f64>,
    submitted_at: Option<DateTime<Utc>>,
    updated_at: DateTime<Utc>,
}

#[derive(Debug, Serialize)]
struct AppealListItem {
    id: Uuid,
    appeal_code: String,
    status: String,
    risk_case_status: String,
    worker_name: String,
    worker_phone: String,
    project_name: String,
    wage_amount_text: String,
    material_score: i32,
    missing_materials: Vec<String>,
    area_code: Option<String>,
    area_name: Option<String>,
    address_text: Option<String>,
    latitude: Option<f64>,
    longitude: Option<f64>,
    submitted_at: Option<DateTime<Utc>>,
    updated_at: DateTime<Utc>,
}

#[derive(Debug, Serialize)]
struct ProsecutorAppealDetail {
    appeal: LaborAppealRow,
    materials: Vec<AppealMaterialRow>,
    location: Option<AppealLocationRow>,
    timeline: Vec<AppealEventRow>,
    risk_case_links: Vec<AppealRiskCaseLinkRow>,
    available_actions: Vec<&'static str>,
}

#[derive(Debug, Serialize)]
struct AppealGraphResponse {
    appeal_id: Uuid,
    risk_case_id: Uuid,
    case_code: String,
    graph_sync_status: String,
    graph_sync_message: String,
    nodes: Vec<GraphNode>,
    edges: Vec<GraphEdge>,
}

#[derive(Debug, Serialize, FromRow)]
struct GraphNode {
    id: Uuid,
    node_type: String,
    label: String,
    confidence: f64,
}

#[derive(Debug, Serialize, FromRow)]
struct GraphEdge {
    id: Uuid,
    relation_type: String,
    source: Uuid,
    target: Uuid,
    confidence: f64,
}

#[derive(Debug, Serialize)]
struct AppealSimilarResponse {
    appeal_id: Uuid,
    risk_case_id: Uuid,
    case_code: String,
    vector_sync_status: String,
    vector_sync_message: String,
    items: Vec<SimilarRiskCaseItem>,
}

#[derive(Debug, Serialize, FromRow)]
struct SimilarRiskCaseItem {
    id: Uuid,
    case_code: String,
    title: String,
    area_name: String,
    risk_level: String,
    risk_score: f64,
    status: String,
    match_score: f64,
    matching_tags: i64,
}

#[derive(Debug, FromRow)]
struct LinkedRiskCase {
    id: Uuid,
    case_code: String,
    graph_sync_status: String,
    graph_sync_message: String,
    vector_sync_status: String,
    vector_sync_message: String,
}

async fn list_appeals(
    State(state): State<AppState>,
    headers: HeaderMap,
    Query(query): Query<AppealListQuery>,
) -> Result<Json<ApiResponse<PageResponse<AppealListItem>>>, AppError> {
    ensure_staff_allowed(&headers)?;
    let effective_area_code = effective_area_code(&headers, query.area_code.as_deref())?;
    let page = query.page.unwrap_or(1).max(1);
    let page_size = query.page_size.unwrap_or(20).clamp(1, 100);
    let offset = (page - 1) * page_size;
    let keyword = query.keyword.map(|value| format!("%{}%", value));

    let total = sqlx::query_scalar::<_, i64>(
        r#"
        SELECT COUNT(*)
        FROM labor_appeals la
        LEFT JOIN appeal_locations al ON al.appeal_id = la.id
        WHERE la.status <> 'draft'
          AND ($1::TEXT IS NULL OR la.status = $1)
          AND ($2::TEXT IS NULL OR al.area_code = $2)
          AND ($3::TEXT IS NULL OR la.worker_name ILIKE $3 OR la.worker_phone ILIKE $3
               OR la.project_name ILIKE $3 OR la.oral_description ILIKE $3)
          AND ($4::TIMESTAMPTZ IS NULL OR la.submitted_at >= $4)
          AND ($5::TIMESTAMPTZ IS NULL OR la.submitted_at <= $5)
        "#,
    )
    .bind(query.status.as_deref())
    .bind(effective_area_code.as_deref())
    .bind(keyword.as_deref())
    .bind(query.submitted_from)
    .bind(query.submitted_to)
    .fetch_one(state.db())
    .await
    .map_err(|_| AppError::Internal)?;

    let rows = sqlx::query_as::<_, AppealListRow>(
        r#"
        SELECT la.id, la.appeal_code, la.status, la.risk_case_status,
               la.worker_name, la.worker_phone, la.project_name, la.wage_amount_text,
               la.material_score, la.missing_materials, al.area_code, al.area_name,
               al.address_text, al.latitude, al.longitude, la.submitted_at, la.updated_at
        FROM labor_appeals la
        LEFT JOIN appeal_locations al ON al.appeal_id = la.id
        WHERE la.status <> 'draft'
          AND ($1::TEXT IS NULL OR la.status = $1)
          AND ($2::TEXT IS NULL OR al.area_code = $2)
          AND ($3::TEXT IS NULL OR la.worker_name ILIKE $3 OR la.worker_phone ILIKE $3
               OR la.project_name ILIKE $3 OR la.oral_description ILIKE $3)
          AND ($4::TIMESTAMPTZ IS NULL OR la.submitted_at >= $4)
          AND ($5::TIMESTAMPTZ IS NULL OR la.submitted_at <= $5)
        ORDER BY la.updated_at DESC
        LIMIT $6 OFFSET $7
        "#,
    )
    .bind(query.status.as_deref())
    .bind(effective_area_code.as_deref())
    .bind(keyword.as_deref())
    .bind(query.submitted_from)
    .bind(query.submitted_to)
    .bind(page_size)
    .bind(offset)
    .fetch_all(state.db())
    .await
    .map_err(|_| AppError::Internal)?;

    Ok(ok(PageResponse {
        items: rows.into_iter().map(mask_list_row).collect(),
        page,
        page_size,
        total,
    }))
}

async fn appeal_detail(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(id): Path<Uuid>,
) -> Result<Json<ApiResponse<ProsecutorAppealDetail>>, AppError> {
    ensure_staff_allowed(&headers)?;
    ensure_staff_area_access(state.db(), &headers, id).await?;
    Ok(ok(load_detail(state.db(), id).await?))
}

async fn appeal_graph(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(id): Path<Uuid>,
) -> Result<Json<ApiResponse<AppealGraphResponse>>, AppError> {
    ensure_staff_allowed(&headers)?;
    ensure_staff_area_access(state.db(), &headers, id).await?;
    let linked = load_primary_risk_case(state.db(), id).await?;
    let nodes = sqlx::query_as::<_, GraphNode>(
        r#"
        SELECT id, entity_type AS node_type, entity_name AS label, confidence
        FROM knowledge_entities
        WHERE case_id = $1
        ORDER BY extracted_at DESC, id DESC
        "#,
    )
    .bind(linked.id)
    .fetch_all(state.db())
    .await
    .map_err(|_| AppError::Internal)?;

    let edges = sqlx::query_as::<_, GraphEdge>(
        r#"
        SELECT gr.id, gr.relation_type, gr.source_entity_id AS source,
               gr.target_entity_id AS target, gr.confidence
        FROM graph_relations gr
        WHERE EXISTS (
            SELECT 1
            FROM knowledge_entities ke
            WHERE ke.case_id = $1
              AND (ke.id = gr.source_entity_id OR ke.id = gr.target_entity_id)
        )
        ORDER BY gr.created_at DESC
        "#,
    )
    .bind(linked.id)
    .fetch_all(state.db())
    .await
    .map_err(|_| AppError::Internal)?;

    Ok(ok(AppealGraphResponse {
        appeal_id: id,
        risk_case_id: linked.id,
        case_code: linked.case_code,
        graph_sync_status: linked.graph_sync_status,
        graph_sync_message: linked.graph_sync_message,
        nodes,
        edges,
    }))
}

async fn appeal_similar(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(id): Path<Uuid>,
    Query(query): Query<SimilarQuery>,
) -> Result<Json<ApiResponse<AppealSimilarResponse>>, AppError> {
    ensure_staff_allowed(&headers)?;
    ensure_staff_area_access(state.db(), &headers, id).await?;
    let linked = load_primary_risk_case(state.db(), id).await?;
    let limit = query.limit.unwrap_or(5).clamp(1, 20);
    let items = sqlx::query_as::<_, SimilarRiskCaseItem>(
        r#"
        WITH source_case AS (
            SELECT id, case_code, area_name, risk_level, risk_tags
            FROM risk_cases
            WHERE id = $1
        ),
        source_tags AS (
            SELECT DISTINCT trim(tag) AS tag
            FROM source_case,
                 regexp_split_to_table(COALESCE(source_case.risk_tags, ''), '[,，、\s]+') AS tag
            WHERE trim(tag) <> ''
        ),
        candidates AS (
            SELECT rc.id, rc.case_code, rc.title, rc.area_name, rc.risk_level,
                   rc.risk_score, rc.status,
                   COUNT(st.tag)::BIGINT AS matching_tags,
                   CASE WHEN rc.area_name = sc.area_name THEN 0.20::DOUBLE PRECISION ELSE 0::DOUBLE PRECISION END AS area_score,
                   CASE WHEN rc.risk_level = sc.risk_level THEN 0.10::DOUBLE PRECISION ELSE 0::DOUBLE PRECISION END AS level_score
            FROM risk_cases rc
            CROSS JOIN source_case sc
            LEFT JOIN source_tags st ON rc.risk_tags ILIKE ('%' || st.tag || '%')
            WHERE rc.id <> sc.id
            GROUP BY rc.id, rc.case_code, rc.title, rc.area_name, rc.risk_level,
                     rc.risk_score, rc.status, sc.area_name, sc.risk_level
        )
        SELECT id, case_code, title, area_name, risk_level, risk_score, status,
               LEAST(0.99::DOUBLE PRECISION, 0.35::DOUBLE PRECISION + (matching_tags::DOUBLE PRECISION * 0.12::DOUBLE PRECISION) + area_score + level_score) AS match_score,
               matching_tags
        FROM candidates
        WHERE matching_tags > 0 OR area_score > 0 OR level_score > 0
        ORDER BY match_score DESC, risk_score DESC, case_code ASC
        LIMIT $2
        "#,
    )
    .bind(linked.id)
    .bind(limit)
    .fetch_all(state.db())
    .await
    .map_err(|_| AppError::Internal)?;

    Ok(ok(AppealSimilarResponse {
        appeal_id: id,
        risk_case_id: linked.id,
        case_code: linked.case_code,
        vector_sync_status: linked.vector_sync_status,
        vector_sync_message: linked.vector_sync_message,
        items,
    }))
}

async fn accept(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(id): Path<Uuid>,
) -> Result<Json<ApiResponse<ProsecutorAppealDetail>>, AppError> {
    ensure_staff_allowed(&headers)?;
    ensure_staff_area_access(state.db(), &headers, id).await?;
    let staff_id = staff_id(&headers);
    let staff_role = staff_role(&headers);
    let appeal = appeal_service::get_appeal(state.db(), id).await?;
    appeal_status::ensure_accept_allowed(&appeal.status)?;
    let now = Utc::now();
    let mut tx = state.db().begin().await.map_err(|_| AppError::Internal)?;
    sqlx::query(
        "UPDATE labor_appeals SET status = 'accepted', accepted_at = COALESCE(accepted_at, $2), updated_at = $2 WHERE id = $1",
    )
    .bind(id)
    .bind(now)
    .execute(&mut *tx)
    .await
    .map_err(|_| AppError::Internal)?;
    appeal_service::insert_review_action_tx(
        &mut tx,
        id,
        &staff_id,
        &staff_role,
        "accept",
        &appeal.status,
        "accepted",
        "",
        "已接收线索并进入办理",
        "",
        None,
    )
    .await?;
    appeal_service::insert_event_tx(
        &mut tx,
        id,
        "appeal_accepted",
        "staff",
        &staff_id,
        "接收线索",
        "检察院端已接收线索",
        true,
    )
    .await?;
    appeal_service::insert_notification_tx(
        &mut tx,
        id,
        appeal.applicant_id,
        "accepted",
        "线索已接收",
        "工作人员已接收您的欠薪诉求，后续办理进度会继续同步。",
    )
    .await?;
    tx.commit().await.map_err(|_| AppError::Internal)?;
    Ok(ok(load_detail(state.db(), id).await?))
}

async fn request_materials(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(id): Path<Uuid>,
    Json(input): Json<RequestMaterialsInput>,
) -> Result<Json<ApiResponse<ProsecutorAppealDetail>>, AppError> {
    ensure_staff_allowed(&headers)?;
    ensure_staff_area_access(state.db(), &headers, id).await?;
    let staff_id = staff_id(&headers);
    let staff_role = staff_role(&headers);
    let appeal = appeal_service::get_appeal(state.db(), id).await?;
    appeal_status::ensure_request_materials_allowed(&appeal.status)?;
    let materials = input.request_materials.join("\n");
    let mut tx = state.db().begin().await.map_err(|_| AppError::Internal)?;
    sqlx::query(
        "UPDATE labor_appeals SET status = 'material_requested', updated_at = $2 WHERE id = $1",
    )
    .bind(id)
    .bind(Utc::now())
    .execute(&mut *tx)
    .await
    .map_err(|_| AppError::Internal)?;
    appeal_service::insert_review_action_tx(
        &mut tx,
        id,
        &staff_id,
        &staff_role,
        "request_materials",
        &appeal.status,
        "material_requested",
        &materials,
        &input.message,
        input.internal_note.as_deref().unwrap_or(""),
        input.deadline,
    )
    .await?;
    appeal_service::insert_event_tx(
        &mut tx,
        id,
        "materials_requested",
        "staff",
        &staff_id,
        "要求补证",
        &input.message,
        true,
    )
    .await?;
    appeal_service::insert_notification_tx(
        &mut tx,
        id,
        appeal.applicant_id,
        "materials_requested",
        "请补充材料",
        &input.message,
    )
    .await?;
    tx.commit().await.map_err(|_| AppError::Internal)?;
    Ok(ok(load_detail(state.db(), id).await?))
}

async fn status_action(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(id): Path<Uuid>,
    Json(input): Json<StatusActionInput>,
) -> Result<Json<ApiResponse<ProsecutorAppealDetail>>, AppError> {
    ensure_staff_allowed(&headers)?;
    ensure_staff_area_access(state.db(), &headers, id).await?;
    let staff_id = staff_id(&headers);
    let staff_role = staff_role(&headers);
    let appeal = appeal_service::get_appeal(state.db(), id).await?;
    let to_status = appeal_status::prosecutor_action_to_status(&input.action, &appeal.status)?;
    let reason = input.reason.unwrap_or_default();
    let mut tx = state.db().begin().await.map_err(|_| AppError::Internal)?;
    sqlx::query("UPDATE labor_appeals SET status = $2, updated_at = $3 WHERE id = $1")
        .bind(id)
        .bind(to_status)
        .bind(Utc::now())
        .execute(&mut *tx)
        .await
        .map_err(|_| AppError::Internal)?;
    appeal_service::insert_review_action_tx(
        &mut tx,
        id,
        &staff_id,
        &staff_role,
        &input.action,
        &appeal.status,
        to_status,
        "",
        &reason,
        "",
        None,
    )
    .await?;
    appeal_service::insert_event_tx(
        &mut tx,
        id,
        &input.action,
        "staff",
        &staff_id,
        "更新办理状态",
        &reason,
        true,
    )
    .await?;
    if input.notify_applicant.unwrap_or(false) {
        appeal_service::insert_notification_tx(
            &mut tx,
            id,
            appeal.applicant_id,
            to_status,
            "办理进度更新",
            &reason,
        )
        .await?;
    }
    tx.commit().await.map_err(|_| AppError::Internal)?;
    Ok(ok(load_detail(state.db(), id).await?))
}

async fn convert_risk_case(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(id): Path<Uuid>,
    Json(input): Json<ConvertRiskCaseInput>,
) -> Result<Json<ApiResponse<ConversionResult>>, AppError> {
    ensure_staff_allowed(&headers)?;
    ensure_staff_area_access(state.db(), &headers, id).await?;
    let staff_id = staff_id(&headers);
    let result =
        appeal_conversion_service::convert_to_risk_case(state.db(), id, &staff_id, input).await?;
    for job in &result.triggered_jobs {
        platform::start_existing_platform_job(state.clone(), job.id).await?;
    }
    Ok(ok(result))
}

async fn link_risk_case(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(id): Path<Uuid>,
    Json(input): Json<LinkRiskCaseInput>,
) -> Result<Json<ApiResponse<AppealRiskCaseLinkRow>>, AppError> {
    ensure_staff_allowed(&headers)?;
    ensure_staff_area_access(state.db(), &headers, id).await?;
    let staff_id = staff_id(&headers);
    Ok(ok(appeal_conversion_service::link_existing_risk_case(
        state.db(),
        id,
        &staff_id,
        input,
    )
    .await?))
}

async fn resolve(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(id): Path<Uuid>,
    Json(input): Json<ResolveInput>,
) -> Result<Json<ApiResponse<ProsecutorAppealDetail>>, AppError> {
    ensure_staff_allowed(&headers)?;
    ensure_staff_area_access(state.db(), &headers, id).await?;
    let staff_id = staff_id(&headers);
    let staff_role = staff_role(&headers);
    let appeal = appeal_service::get_appeal(state.db(), id).await?;
    let now = Utc::now();
    let mut tx = state.db().begin().await.map_err(|_| AppError::Internal)?;
    sqlx::query(
        "UPDATE labor_appeals SET status = 'resolved', resolved_at = $2, updated_at = $2 WHERE id = $1",
    )
    .bind(id)
    .bind(now)
    .execute(&mut *tx)
    .await
    .map_err(|_| AppError::Internal)?;
    appeal_service::insert_review_action_tx(
        &mut tx,
        id,
        &staff_id,
        &staff_role,
        "resolve",
        &appeal.status,
        "resolved",
        "",
        &input.result_summary,
        "",
        None,
    )
    .await?;
    appeal_service::insert_event_tx(
        &mut tx,
        id,
        "appeal_resolved",
        "staff",
        &staff_id,
        "办结",
        &input.result_summary,
        true,
    )
    .await?;
    if input.notify_applicant.unwrap_or(true) {
        appeal_service::insert_notification_tx(
            &mut tx,
            id,
            appeal.applicant_id,
            "resolved",
            "办理结果已更新",
            &input.result_summary,
        )
        .await?;
    }
    tx.commit().await.map_err(|_| AppError::Internal)?;
    Ok(ok(load_detail(state.db(), id).await?))
}

async fn confirm_location(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(id): Path<Uuid>,
    Json(input): Json<StaffConfirmLocationInput>,
) -> Result<Json<ApiResponse<ProsecutorAppealDetail>>, AppError> {
    ensure_staff_allowed(&headers)?;
    ensure_staff_area_access(state.db(), &headers, id).await?;
    let staff_id = staff_id(&headers);
    appeal_service::confirm_location_by_staff(state.db(), id, &staff_id, input).await?;
    Ok(ok(load_detail(state.db(), id).await?))
}

async fn load_detail(db: &sqlx::PgPool, id: Uuid) -> Result<ProsecutorAppealDetail, AppError> {
    let appeal = appeal_service::get_appeal(db, id).await?;
    let links = sqlx::query_as::<_, AppealRiskCaseLinkRow>(
        "SELECT * FROM appeal_risk_case_links WHERE appeal_id = $1 ORDER BY created_at DESC",
    )
    .bind(id)
    .fetch_all(db)
    .await
    .map_err(|_| AppError::Internal)?;
    Ok(ProsecutorAppealDetail {
        available_actions: available_actions(&appeal.status),
        appeal,
        materials: appeal_service::list_materials(db, id).await?,
        location: appeal_service::maybe_location(db, id).await?,
        timeline: appeal_service::list_events(db, id, false).await?,
        risk_case_links: links,
    })
}

async fn load_primary_risk_case(
    db: &sqlx::PgPool,
    appeal_id: Uuid,
) -> Result<LinkedRiskCase, AppError> {
    sqlx::query_as::<_, LinkedRiskCase>(
        r#"
        SELECT rc.id, rc.case_code,
               rc.graph_sync_status, rc.graph_sync_message,
               rc.vector_sync_status, rc.vector_sync_message
        FROM appeal_risk_case_links arcl
        JOIN risk_cases rc ON rc.id = arcl.risk_case_id
        WHERE arcl.appeal_id = $1
        ORDER BY arcl.created_at DESC
        LIMIT 1
        "#,
    )
    .bind(appeal_id)
    .fetch_optional(db)
    .await
    .map_err(|_| AppError::Internal)?
    .ok_or_else(|| AppError::Validation("appeal has no linked risk_case yet".to_string()))
}

fn mask_list_row(row: AppealListRow) -> AppealListItem {
    AppealListItem {
        id: row.id,
        appeal_code: row.appeal_code,
        status: row.status,
        risk_case_status: row.risk_case_status,
        worker_name: mask_name(&row.worker_name),
        worker_phone: mask_phone(&row.worker_phone),
        project_name: row.project_name,
        wage_amount_text: row.wage_amount_text,
        material_score: row.material_score,
        missing_materials: row
            .missing_materials
            .lines()
            .filter(|line| !line.trim().is_empty())
            .map(str::to_string)
            .collect(),
        area_code: row.area_code,
        area_name: row.area_name,
        address_text: row.address_text,
        latitude: row.latitude,
        longitude: row.longitude,
        submitted_at: row.submitted_at,
        updated_at: row.updated_at,
    }
}

fn available_actions(status: &str) -> Vec<&'static str> {
    let mut actions = Vec::new();
    if appeal_status::ensure_request_materials_allowed(status).is_ok() {
        actions.push("request_materials");
    }
    if appeal_status::ensure_accept_allowed(status).is_ok() {
        actions.push("accept");
    }
    if matches!(status, "accepted" | "under_review" | "transferred") {
        actions.push("start_processing");
    }
    if matches!(
        status,
        "accepted" | "processing" | "under_review" | "transferred"
    ) {
        actions.push("convert_risk_case");
        actions.push("resolve");
    }
    actions
}

fn staff_id(headers: &HeaderMap) -> String {
    headers
        .get("x-staff-id")
        .and_then(|value| value.to_str().ok())
        .unwrap_or("dev-staff")
        .to_string()
}

fn staff_role(headers: &HeaderMap) -> String {
    headers
        .get("x-staff-role")
        .and_then(|value| value.to_str().ok())
        .unwrap_or("prosecutor")
        .to_string()
}

fn ensure_staff_allowed(headers: &HeaderMap) -> Result<(), AppError> {
    let role = staff_role(headers);
    if matches!(
        role.as_str(),
        "prosecutor" | "prosecutor_reviewer" | "prosecutor_admin"
    ) {
        Ok(())
    } else {
        Err(AppError::Forbidden)
    }
}

fn staff_area_code(headers: &HeaderMap) -> Option<String> {
    headers
        .get("x-staff-area-code")
        .and_then(|value| value.to_str().ok())
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(str::to_string)
}

fn effective_area_code(
    headers: &HeaderMap,
    query_area_code: Option<&str>,
) -> Result<Option<String>, AppError> {
    if staff_role(headers) == "prosecutor_admin" {
        return Ok(query_area_code.map(str::to_string));
    }
    let staff_area = staff_area_code(headers);
    if let (Some(staff_area), Some(query_area)) = (staff_area.as_deref(), query_area_code) {
        if staff_area != query_area {
            return Err(AppError::Forbidden);
        }
    }
    Ok(staff_area.or_else(|| query_area_code.map(str::to_string)))
}

async fn ensure_staff_area_access(
    db: &sqlx::PgPool,
    headers: &HeaderMap,
    appeal_id: Uuid,
) -> Result<(), AppError> {
    if staff_role(headers) == "prosecutor_admin" {
        return Ok(());
    }
    let Some(staff_area) = staff_area_code(headers) else {
        return Ok(());
    };
    let area_code = sqlx::query_scalar::<_, Option<String>>(
        "SELECT area_code FROM appeal_locations WHERE appeal_id = $1 ORDER BY created_at DESC LIMIT 1",
    )
    .bind(appeal_id)
    .fetch_optional(db)
    .await
    .map_err(|_| AppError::Internal)?
    .flatten();
    if area_code.as_deref() == Some(staff_area.as_str()) {
        Ok(())
    } else {
        Err(AppError::Forbidden)
    }
}
