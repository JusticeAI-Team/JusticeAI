use std::{collections::HashMap, fs::File, io::BufReader, path::Path};

use axum::{
    extract::{Path as AxumPath, Query, State},
    routing::{get, post},
    Json, Router,
};
use calamine::{open_workbook_auto, Data, Reader};
use chrono::{DateTime, Datelike, Duration, Utc};
use csv::StringRecord;
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
        .route("/imports/:id/process", post(process_import))
        .route("/mapping/current", get(get_current_mapping).post(save_current_mapping))
        .route("/extraction/run", post(run_extraction))
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "snake_case")]
struct MappingFieldDto {
    source_field: String,
    target_field: String,
    confidence: f32,
    status: String,
    sample_value: String,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "snake_case")]
struct MappingTemplateResponse {
    generated_at: String,
    template_id: String,
    template_key: String,
    template_label: String,
    version: String,
    status: String,
    source_type: String,
    fields: Vec<MappingFieldDto>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "snake_case")]
struct ImportProcessResponse {
    import_id: String,
    status: String,
    created_case_count: u32,
    updated_at: String,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "snake_case")]
struct ExtractionRunResponse {
    generated_at: String,
    processed_case_count: u32,
    created_entity_count: u32,
    created_relation_count: u32,
    status: String,
}

#[derive(Debug, Deserialize)]
struct MappingQuery {
    source_type: Option<String>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "snake_case")]
struct SaveMappingRequest {
    template_key: String,
    template_label: String,
    version: String,
    status: String,
    source_type: String,
    fields: Vec<SaveMappingFieldRequest>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "snake_case")]
struct SaveMappingFieldRequest {
    source_field: String,
    target_field: String,
    confidence: f32,
    status: String,
    sample_value: String,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "snake_case")]
struct RunExtractionRequest {
    case_ids: Option<Vec<Uuid>>,
}

#[derive(Debug, FromRow)]
struct MappingTemplateRow {
    id: Uuid,
    template_key: String,
    template_label: String,
    version: String,
    status: String,
    source_type: String,
}

#[derive(Debug, FromRow)]
struct MappingFieldRow {
    source_field: String,
    target_field: String,
    confidence: f64,
    status: String,
    sample_value: String,
}

#[derive(Debug, Clone)]
struct MappingFieldConfig {
    source_field: String,
    target_field: String,
}

#[derive(Debug, FromRow)]
struct ImportProcessRow {
    source_type: String,
}

#[derive(Debug, FromRow)]
struct ImportFileRow {
    stored_path: String,
    original_filename: String,
}

#[derive(Debug, FromRow)]
struct RiskCaseSeedRow {
    id: Uuid,
    title: String,
    area_name: String,
}

#[derive(Debug, Default)]
struct ImportRecordContext {
    title: Option<String>,
    area_name: Option<String>,
    occurred_at: Option<DateTime<Utc>>,
    assignee: Option<String>,
    report_period: Option<String>,
    risk_score: Option<f64>,
    risk_level: Option<String>,
    status: Option<String>,
    alert_status: Option<String>,
}

async fn get_current_mapping(
    State(state): State<AppState>,
    Query(query): Query<MappingQuery>,
) -> Result<Json<ApiResponse<MappingTemplateResponse>>, AppError> {
    let source_type = query.source_type.as_deref().unwrap_or("manual_upload");

    let template = sqlx::query_as::<_, MappingTemplateRow>(
        r#"
        SELECT id, template_key, template_label, version, status, source_type
        FROM mapping_templates
        WHERE source_type = $1
        ORDER BY updated_at DESC
        LIMIT 1
        "#,
    )
    .bind(source_type)
    .fetch_optional(state.db())
    .await
    .map_err(|_| AppError::Internal)?
    .ok_or(AppError::NotFound)?;

    let fields = sqlx::query_as::<_, MappingFieldRow>(
        r#"
        SELECT source_field, target_field, confidence, status, sample_value
        FROM mapping_fields
        WHERE template_id = $1
        ORDER BY sort_order ASC, created_at ASC
        "#,
    )
    .bind(template.id)
    .fetch_all(state.db())
    .await
    .map_err(|_| AppError::Internal)?;

    Ok(ok(MappingTemplateResponse {
        generated_at: now(),
        template_id: template.id.to_string(),
        template_key: template.template_key,
        template_label: template.template_label,
        version: template.version,
        status: template.status,
        source_type: template.source_type,
        fields: fields
            .into_iter()
            .map(|field| MappingFieldDto {
                source_field: field.source_field,
                target_field: field.target_field,
                confidence: field.confidence as f32,
                status: field.status,
                sample_value: field.sample_value,
            })
            .collect(),
    }))
}

async fn save_current_mapping(
    State(state): State<AppState>,
    Json(payload): Json<SaveMappingRequest>,
) -> Result<Json<ApiResponse<MappingTemplateResponse>>, AppError> {
    let template_key = payload.template_key.trim();
    let template_label = payload.template_label.trim();
    let version = payload.version.trim();
    let status = payload.status.trim();
    let source_type = payload.source_type.trim();

    if template_key.is_empty()
        || template_label.is_empty()
        || version.is_empty()
        || status.is_empty()
        || source_type.is_empty()
    {
        return Err(AppError::Validation("映射模板关键信息不能为空".to_string()));
    }

    if payload.fields.is_empty() {
        return Err(AppError::Validation("至少需要一条映射字段定义".to_string()));
    }

    let now_at = Utc::now();
    let mut tx = state.db().begin().await.map_err(|_| AppError::Internal)?;

    let template_id = sqlx::query_scalar::<_, Uuid>(
        r#"
        INSERT INTO mapping_templates (
            id, template_key, template_label, version, status, source_type, created_at, updated_at
        )
        VALUES ($1, $2, $3, $4, $5, $6, $7, $8)
        ON CONFLICT (template_key)
        DO UPDATE SET
            template_label = EXCLUDED.template_label,
            version = EXCLUDED.version,
            status = EXCLUDED.status,
            source_type = EXCLUDED.source_type,
            updated_at = EXCLUDED.updated_at
        RETURNING id
        "#,
    )
    .bind(Uuid::new_v4())
    .bind(template_key)
    .bind(template_label)
    .bind(version)
    .bind(status)
    .bind(source_type)
    .bind(now_at)
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
                sample_value, sort_order, created_at, updated_at
            )
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10)
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
        .bind(now_at)
        .bind(now_at)
        .execute(&mut *tx)
        .await
        .map_err(|_| AppError::Internal)?;
    }

    tx.commit().await.map_err(|_| AppError::Internal)?;

    get_current_mapping(
        State(state),
        Query(MappingQuery {
            source_type: Some(source_type.to_string()),
        }),
    )
    .await
}

async fn process_import(
    State(state): State<AppState>,
    AxumPath(import_id): AxumPath<Uuid>,
) -> Result<Json<ApiResponse<ImportProcessResponse>>, AppError> {
    let import_row = sqlx::query_as::<_, ImportProcessRow>(
        r#"
        SELECT source_type
        FROM imports
        WHERE id = $1
        "#,
    )
    .bind(import_id)
    .fetch_optional(state.db())
    .await
    .map_err(|_| AppError::Internal)?
    .ok_or(AppError::NotFound)?;

    let files = sqlx::query_as::<_, ImportFileRow>(
        r#"
        SELECT stored_path, original_filename
        FROM import_files
        WHERE import_id = $1
        ORDER BY created_at ASC
        "#,
    )
    .bind(import_id)
    .fetch_all(state.db())
    .await
    .map_err(|_| AppError::Internal)?;

    if files.is_empty() {
        return Err(AppError::Validation("导入批次没有可处理文件".to_string()));
    }

    let mapping_fields = load_mapping_fields(state.db(), &import_row.source_type).await?;
    let base_upload_dir = state.settings().storage.upload_dir.clone();
    let mut created_case_count = 0_u32;
    let now_at = Utc::now();
    let mut tx = state.db().begin().await.map_err(|_| AppError::Internal)?;

    for file in files {
        let absolute_path = Path::new(&base_upload_dir).join(&file.stored_path);
        let records = parse_import_rows(&absolute_path, &file.original_filename)?;

        for (index, record) in records.into_iter().enumerate() {
            let context = build_import_record_context(&record, &mapping_fields);
            let title = context
                .title
                .unwrap_or_else(|| format!("{}-线索{}", file.original_filename, index + 1));
            let area_name = context.area_name.unwrap_or_else(|| "待补充街道".to_string());
            let occurred_at = context.occurred_at.unwrap_or(now_at);
            let assignee = context.assignee;
            let report_period = context.report_period.unwrap_or_else(current_period);
            let risk_score = context
                .risk_score
                .unwrap_or_else(|| 60.0 + ((index % 4) as f64) * 8.5)
                .clamp(0.0, 100.0);
            let risk_level = context
                .risk_level
                .unwrap_or_else(|| risk_level_from_score(risk_score).to_string());
            let status = context.status.unwrap_or_else(|| "todo".to_string());
            let alert_status = context.alert_status.unwrap_or_else(|| "open".to_string());
            let due_at = occurred_at + Duration::days(3 + index as i64);
            let case_code = format!("IMP-{}-{:03}", &import_id.simple().to_string()[..8], created_case_count + 1);

            let result = sqlx::query(
                r#"
                INSERT INTO risk_cases (
                    id, import_id, case_code, title, source_type, area_name, risk_level, risk_score,
                    status, alert_status, assignee, occurred_at, due_at, closed_at,
                    report_period, created_at, updated_at
                )
                VALUES (
                    $1, $2, $3, $4, $5, $6, $7, $8,
                    $9, $10, $11, $12, $13,
                    CASE WHEN $9 = 'closed' THEN $14 ELSE NULL END,
                    $15, $14, $14
                )
                ON CONFLICT (case_code) DO NOTHING
                "#,
            )
            .bind(Uuid::new_v4())
            .bind(import_id)
            .bind(case_code)
            .bind(title)
            .bind(import_row.source_type.as_str())
            .bind(area_name)
            .bind(normalize_risk_level(&risk_level))
            .bind(risk_score)
            .bind(normalize_case_status(&status)?)
            .bind(normalize_alert_status(&alert_status)?)
            .bind(assignee)
            .bind(occurred_at)
            .bind(due_at)
            .bind(now_at)
            .bind(report_period)
            .execute(&mut *tx)
            .await
            .map_err(|_| AppError::Internal)?;

            created_case_count += result.rows_affected() as u32;
        }
    }

    sqlx::query("UPDATE imports SET status = $2, updated_at = $3 WHERE id = $1")
        .bind(import_id)
        .bind("processed")
        .bind(now_at)
        .execute(&mut *tx)
        .await
        .map_err(|_| AppError::Internal)?;

    tx.commit().await.map_err(|_| AppError::Internal)?;

    Ok(ok(ImportProcessResponse {
        import_id: import_id.to_string(),
        status: "processed".to_string(),
        created_case_count,
        updated_at: now_at.to_rfc3339(),
    }))
}

async fn run_extraction(
    State(state): State<AppState>,
    Json(payload): Json<RunExtractionRequest>,
) -> Result<Json<ApiResponse<ExtractionRunResponse>>, AppError> {
    let case_rows = if let Some(case_ids) = payload.case_ids {
        if case_ids.is_empty() {
            return Err(AppError::Validation("case_ids 不能为空数组".to_string()));
        }

        sqlx::query_as::<_, RiskCaseSeedRow>(
            r#"
            SELECT id, title, area_name
            FROM risk_cases
            WHERE id = ANY($1)
            ORDER BY updated_at DESC
            "#,
        )
        .bind(&case_ids)
        .fetch_all(state.db())
        .await
        .map_err(|_| AppError::Internal)?
    } else {
        sqlx::query_as::<_, RiskCaseSeedRow>(
            r#"
            SELECT id, title, area_name
            FROM risk_cases
            ORDER BY updated_at DESC
            LIMIT 50
            "#,
        )
        .fetch_all(state.db())
        .await
        .map_err(|_| AppError::Internal)?
    };

    let now_at = Utc::now();
    let mut tx = state.db().begin().await.map_err(|_| AppError::Internal)?;
    let mut created_entity_count = 0_u32;
    let mut created_relation_count = 0_u32;

    for case in &case_rows {
        sqlx::query("DELETE FROM graph_relations WHERE source_entity_id IN (SELECT id FROM knowledge_entities WHERE case_id = $1) OR target_entity_id IN (SELECT id FROM knowledge_entities WHERE case_id = $1)")
            .bind(case.id)
            .execute(&mut *tx)
            .await
            .map_err(|_| AppError::Internal)?;

        sqlx::query("DELETE FROM knowledge_entities WHERE case_id = $1")
            .bind(case.id)
            .execute(&mut *tx)
            .await
            .map_err(|_| AppError::Internal)?;

        let person_id = Uuid::new_v4();
        let event_id = Uuid::new_v4();
        let area_id = Uuid::new_v4();

        let person_name = format!("{}相关主体", case.title);
        let event_name = case.title.clone();
        let area_name = case.area_name.clone();

        for (entity_id, entity_type, entity_name, confidence) in [
            (person_id, "person", person_name, 0.92_f64),
            (event_id, "event", event_name, 0.95_f64),
            (area_id, "area", area_name, 0.99_f64),
        ] {
            sqlx::query(
                r#"
                INSERT INTO knowledge_entities (id, case_id, entity_type, entity_name, confidence, extracted_at, created_at)
                VALUES ($1, $2, $3, $4, $5, $6, $7)
                "#,
            )
            .bind(entity_id)
            .bind(case.id)
            .bind(entity_type)
            .bind(entity_name)
            .bind(confidence)
            .bind(now_at)
            .bind(now_at)
            .execute(&mut *tx)
            .await
            .map_err(|_| AppError::Internal)?;

            created_entity_count += 1;
        }

        for (relation_type, source_entity_id, target_entity_id) in [
            ("person_event", person_id, event_id),
            ("person_area", person_id, area_id),
        ] {
            sqlx::query(
                r#"
                INSERT INTO graph_relations (id, relation_type, source_entity_id, target_entity_id, created_at)
                VALUES ($1, $2, $3, $4, $5)
                "#,
            )
            .bind(Uuid::new_v4())
            .bind(relation_type)
            .bind(source_entity_id)
            .bind(target_entity_id)
            .bind(now_at)
            .execute(&mut *tx)
            .await
            .map_err(|_| AppError::Internal)?;

            created_relation_count += 1;
        }
    }

    sqlx::query(
        r#"
        INSERT INTO workflow_runs (
            id, stage_key, stage_label, status, started_at, finished_at,
            item_count, success_count, failure_count, created_at, updated_at
        )
        VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11)
        "#,
    )
    .bind(Uuid::new_v4())
    .bind("extraction")
    .bind("知识抽取")
    .bind("completed")
    .bind(now_at)
    .bind(now_at)
    .bind(case_rows.len() as i32)
    .bind(case_rows.len() as i32)
    .bind(0_i32)
    .bind(now_at)
    .bind(now_at)
    .execute(&mut *tx)
    .await
    .map_err(|_| AppError::Internal)?;

    tx.commit().await.map_err(|_| AppError::Internal)?;

    Ok(ok(ExtractionRunResponse {
        generated_at: now_at.to_rfc3339(),
        processed_case_count: case_rows.len() as u32,
        created_entity_count,
        created_relation_count,
        status: "completed".to_string(),
    }))
}

async fn load_mapping_fields(
    db: &sqlx::PgPool,
    source_type: &str,
) -> Result<Vec<MappingFieldConfig>, AppError> {
    let template = sqlx::query_as::<_, MappingTemplateRow>(
        r#"
        SELECT id, template_key, template_label, version, status, source_type
        FROM mapping_templates
        WHERE source_type = $1
        ORDER BY updated_at DESC
        LIMIT 1
        "#,
    )
    .bind(source_type)
    .fetch_optional(db)
    .await
    .map_err(|_| AppError::Internal)?;

    let Some(template) = template else {
        return Ok(Vec::new());
    };

    let fields = sqlx::query_as::<_, MappingFieldRow>(
        r#"
        SELECT source_field, target_field, confidence, status, sample_value
        FROM mapping_fields
        WHERE template_id = $1
          AND status IN ('mapped', 'confirmed', 'active', 'approved', 'needs_review')
        ORDER BY sort_order ASC, created_at ASC
        "#,
    )
    .bind(template.id)
    .fetch_all(db)
    .await
    .map_err(|_| AppError::Internal)?;

    Ok(fields
        .into_iter()
        .filter_map(|field| {
            let source_field = field.source_field.trim().to_string();
            let target_field = field.target_field.trim().to_string();
            if source_field.is_empty() || target_field.is_empty() {
                None
            } else {
                Some(MappingFieldConfig {
                    source_field,
                    target_field,
                })
            }
        })
        .collect())
}

fn parse_import_rows(
    path: &Path,
    original_filename: &str,
) -> Result<Vec<HashMap<String, String>>, AppError> {
    let extension = path
        .extension()
        .and_then(|ext| ext.to_str())
        .map(|value| value.to_ascii_lowercase())
        .unwrap_or_default();

    match extension.as_str() {
        "csv" => parse_csv_rows(path),
        "xlsx" | "xls" => parse_excel_rows(path),
        _ => Ok(build_fallback_rows(original_filename)),
    }
}

fn parse_csv_rows(path: &Path) -> Result<Vec<HashMap<String, String>>, AppError> {
    let file = File::open(path).map_err(|_| AppError::Validation("读取导入文件失败".to_string()))?;
    let mut reader = csv::Reader::from_reader(BufReader::new(file));
    let headers = reader
        .headers()
        .map_err(|_| AppError::Validation("解析 CSV 表头失败".to_string()))?
        .clone();

    let mut rows = Vec::new();
    for record in reader.records() {
        let record = record.map_err(|_| AppError::Validation("解析 CSV 数据行失败".to_string()))?;
        rows.push(build_row_map(&headers, &record));
    }

    if rows.is_empty() {
        rows.push(HashMap::new());
    }

    Ok(rows)
}

fn parse_excel_rows(path: &Path) -> Result<Vec<HashMap<String, String>>, AppError> {
    let mut workbook = open_workbook_auto(path)
        .map_err(|_| AppError::Validation("读取 Excel 导入文件失败".to_string()))?;
    let sheet_name = workbook
        .sheet_names()
        .first()
        .cloned()
        .ok_or_else(|| AppError::Validation("Excel 文件没有可读取的工作表".to_string()))?;
    let range = workbook
        .worksheet_range(&sheet_name)
        .map_err(|_| AppError::Validation("解析 Excel 工作表失败".to_string()))?;

    let mut rows_iter = range.rows();
    let headers = rows_iter
        .next()
        .ok_or_else(|| AppError::Validation("Excel 文件缺少表头".to_string()))?
        .iter()
        .map(cell_to_string)
        .collect::<Vec<_>>();

    let mut rows = Vec::new();
    for row in rows_iter {
        let mut values = row.iter().map(cell_to_string).collect::<Vec<_>>();
        if values.iter().all(|value| value.trim().is_empty()) {
            continue;
        }
        if values.len() < headers.len() {
            values.resize(headers.len(), String::new());
        }
        rows.push(build_row_map_from_vecs(&headers, &values));
    }

    if rows.is_empty() {
        rows.push(HashMap::new());
    }

    Ok(rows)
}

fn build_row_map(headers: &StringRecord, record: &StringRecord) -> HashMap<String, String> {
    headers
        .iter()
        .zip(record.iter())
        .map(|(key, value)| (key.trim().to_string(), value.trim().to_string()))
        .collect()
}

fn build_row_map_from_vecs(headers: &[String], values: &[String]) -> HashMap<String, String> {
    headers
        .iter()
        .zip(values.iter())
        .map(|(key, value)| (key.trim().to_string(), value.trim().to_string()))
        .collect()
}

fn build_fallback_rows(file_name: &str) -> Vec<HashMap<String, String>> {
    let mut row = HashMap::new();
    row.insert("诉求标题".to_string(), format!("{} 自动生成风险线索", file_name));
    row.insert("发生街道".to_string(), "待研判街道".to_string());
    vec![row]
}

fn build_import_record_context(
    record: &HashMap<String, String>,
    mapping_fields: &[MappingFieldConfig],
) -> ImportRecordContext {
    let mut context = ImportRecordContext::default();

    for field in mapping_fields {
        let Some(value) = extract_value(record, &[field.source_field.as_str()]) else {
            continue;
        };

        match field.target_field.as_str() {
            "case_title" | "title" => context.title = Some(value),
            "area_name" | "street" => context.area_name = Some(value),
            "occurred_at" => {
                if let Some(parsed) = parse_datetime_value(&value) {
                    context.occurred_at = Some(parsed);
                }
            }
            "assignee" => context.assignee = Some(value),
            "report_period" => context.report_period = Some(value),
            "risk_score" => {
                if let Some(parsed) = parse_score_value(&value) {
                    context.risk_score = Some(parsed);
                }
            }
            "risk_level" => context.risk_level = Some(value),
            "status" => context.status = Some(value),
            "alert_status" => context.alert_status = Some(value),
            _ => {}
        }
    }

    if context.title.is_none() {
        context.title = extract_value(record, &["诉求标题", "标题", "title"]);
    }
    if context.area_name.is_none() {
        context.area_name = extract_value(record, &["发生街道", "街道", "area_name"]);
    }
    if context.occurred_at.is_none() {
        context.occurred_at = extract_value(record, &["发生时间", "occurred_at", "日期", "时间"])
            .and_then(|value| parse_datetime_value(&value));
    }
    if context.risk_score.is_none() {
        context.risk_score = extract_value(record, &["风险分数", "risk_score", "评分"])
            .and_then(|value| parse_score_value(&value));
    }
    if context.risk_level.is_none() {
        context.risk_level = extract_value(record, &["风险等级", "risk_level"]);
    }
    if context.status.is_none() {
        context.status = extract_value(record, &["状态", "status"]);
    }
    if context.alert_status.is_none() {
        context.alert_status = extract_value(record, &["预警状态", "alert_status"]);
    }
    if context.assignee.is_none() {
        context.assignee = extract_value(record, &["承办人", "assignee"]);
    }
    if context.report_period.is_none() {
        context.report_period = extract_value(record, &["期次", "report_period"]);
    }

    context
}

fn extract_value(record: &HashMap<String, String>, candidates: &[&str]) -> Option<String> {
    candidates.iter().find_map(|key| {
        record
            .get(*key)
            .map(String::as_str)
            .map(str::trim)
            .filter(|value| !value.is_empty())
            .map(str::to_string)
    })
}

fn parse_datetime_value(value: &str) -> Option<DateTime<Utc>> {
    if let Ok(parsed) = DateTime::parse_from_rfc3339(value) {
        return Some(parsed.with_timezone(&Utc));
    }

    for format in [
        "%Y-%m-%d %H:%M:%S",
        "%Y-%m-%d %H:%M",
        "%Y/%m/%d %H:%M:%S",
        "%Y/%m/%d %H:%M",
        "%Y-%m-%d",
        "%Y/%m/%d",
    ] {
        if let Ok(parsed) = chrono::NaiveDateTime::parse_from_str(value, format) {
            return Some(DateTime::<Utc>::from_naive_utc_and_offset(parsed, Utc));
        }
        if let Ok(parsed) = chrono::NaiveDate::parse_from_str(value, format) {
            return parsed
                .and_hms_opt(0, 0, 0)
                .map(|datetime| DateTime::<Utc>::from_naive_utc_and_offset(datetime, Utc));
        }
    }

    None
}

fn parse_score_value(value: &str) -> Option<f64> {
    value.trim().trim_end_matches('%').parse::<f64>().ok()
}

fn normalize_risk_level(value: &str) -> &'static str {
    match value.trim().to_ascii_lowercase().as_str() {
        "high" | "高" | "高风险" => "high",
        "medium" | "mid" | "中" | "中风险" => "medium",
        "low" | "低" | "低风险" => "low",
        _ => "low",
    }
}

fn normalize_case_status(value: &str) -> Result<&'static str, AppError> {
    match value.trim().to_ascii_lowercase().as_str() {
        "todo" | "待处理" => Ok("todo"),
        "open" | "已打开" => Ok("open"),
        "in_progress" | "处理中" | "办理中" => Ok("in_progress"),
        "closed" | "已办结" | "已关闭" => Ok("closed"),
        other => Err(AppError::Validation(format!("不支持的案件状态: {other}"))),
    }
}

fn normalize_alert_status(value: &str) -> Result<&'static str, AppError> {
    match value.trim().to_ascii_lowercase().as_str() {
        "open" | "待处理" | "未处理" => Ok("open"),
        "acknowledged" | "已确认" | "处理中" => Ok("acknowledged"),
        "resolved" | "已消警" | "已解除" => Ok("resolved"),
        other => Err(AppError::Validation(format!("不支持的预警状态: {other}"))),
    }
}

fn cell_to_string(cell: &Data) -> String {
    match cell {
        Data::Empty => String::new(),
        Data::String(value) => value.trim().to_string(),
        Data::Float(value) => {
            if value.fract() == 0.0 {
                format!("{value:.0}")
            } else {
                value.to_string()
            }
        }
        Data::Int(value) => value.to_string(),
        Data::Bool(value) => value.to_string(),
        Data::DateTime(value) => value.to_string(),
        Data::DateTimeIso(value) => value.trim().to_string(),
        Data::DurationIso(value) => value.trim().to_string(),
        Data::Error(_) => String::new(),
    }
}

fn risk_level_from_score(score: f64) -> &'static str {
    if score >= 85.0 {
        "high"
    } else if score >= 70.0 {
        "medium"
    } else {
        "low"
    }
}

fn current_period() -> String {
    let now = Utc::now();
    format!("{}-{:02}", now.year(), now.month())
}

fn now() -> String {
    Utc::now().to_rfc3339()
}
