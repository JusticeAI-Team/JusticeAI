use std::{
    collections::HashMap,
    fs::File,
    io::BufReader,
    path::Path,
};

use calamine::{open_workbook_auto, Data, Reader};
use chrono::{DateTime, Datelike, Duration, Utc};
use csv::StringRecord;
use sqlx::{FromRow, Postgres, Transaction};
use uuid::Uuid;

use crate::{
    app::AppState,
    services::{
        ai::{ExtractionInput, OpenAiCompatibleAiService, RecommendationInput},
        graph::{GraphCaseSyncInput, GraphEntitySync, GraphRelationSync, HugeGraphSyncService},
        vector::{MilvusVectorStore, SimilarCaseHit, VectorCaseDocument},
    },
    shared::error::AppError,
};

#[derive(Debug)]
pub struct ProcessImportResult {
    pub import_id: Uuid,
    pub status: String,
    pub total_record_count: i32,
    pub processed_record_count: i32,
    pub failed_record_count: i32,
    pub affected_case_count: i32,
    pub workflow_run_id: Uuid,
    pub mapping_template_id: Option<Uuid>,
    pub finished_at: DateTime<Utc>,
}

#[derive(Debug)]
pub struct ExtractionResult {
    pub run_id: Uuid,
    pub status: String,
    pub item_count: i32,
    pub success_count: i32,
    pub failure_count: i32,
    pub created_entity_count: i32,
    pub created_relation_count: i32,
    pub summary: String,
    pub finished_at: DateTime<Utc>,
}

#[derive(Debug, FromRow)]
struct ImportRow {
    id: Uuid,
    source_type: String,
}

#[derive(Debug, FromRow)]
struct ImportFileRow {
    stored_path: String,
    original_filename: String,
}

#[derive(Debug, FromRow)]
struct MappingTemplateRow {
    id: Uuid,
}

#[derive(Debug, FromRow)]
struct MappingFieldRow {
    source_field: String,
    target_field: String,
    status: String,
}

#[derive(Debug, Clone)]
struct MappingFieldConfig {
    source_field: String,
    target_field: String,
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
    risk_tags: Vec<String>,
}

#[derive(Debug, FromRow)]
struct RiskCaseSeedRow {
    id: Uuid,
    case_code: String,
    title: String,
    source_type: String,
    area_name: String,
    risk_level: String,
}

pub async fn process_import_batch(
    state: &AppState,
    import_id: Uuid,
    ai_service: &OpenAiCompatibleAiService,
    vector_store: &MilvusVectorStore,
) -> Result<ProcessImportResult, AppError> {
    let import_row = sqlx::query_as::<_, ImportRow>(
        "SELECT id, source_type FROM imports WHERE id = $1",
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
        return Err(AppError::Validation(
            "import batch does not contain any files".to_string(),
        ));
    }

    let now_at = Utc::now();
    let mapping_template_id = latest_mapping_template_id(state.db(), &import_row.source_type).await?;
    let mapping_fields = load_mapping_fields(state.db(), mapping_template_id).await?;
    let workflow_run_id = Uuid::new_v4();
    let upload_dir = state.settings().storage.upload_dir.clone();

    let mut tx = state.db().begin().await.map_err(|_| AppError::Internal)?;
    set_import_status(
        &mut tx,
        import_id,
        "processing",
        now_at,
        mapping_template_id,
        None,
        None,
        None,
    )
    .await?;

    insert_processing_workflow_run(&mut tx, workflow_run_id, now_at).await?;

    let mut total_record_count = 0_i32;
    let mut processed_record_count = 0_i32;
    let mut failed_record_count = 0_i32;
    let mut affected_case_count = 0_i32;
    let mut row_index = 0_i32;
    let mut vector_documents = Vec::new();

    for file in files {
        let absolute_path = Path::new(&upload_dir).join(&file.stored_path);
        let records = parse_import_rows(&absolute_path, &file.original_filename)?;
        total_record_count += records.len() as i32;

        for record in records {
            row_index += 1;
            let context = build_import_record_context(&record, &mapping_fields);
            let title = context
                .title
                .unwrap_or_else(|| format!("{}-lead-{}", file.original_filename, row_index));
            let area_name = context
                .area_name
                .unwrap_or_else(|| "unassigned-area".to_string());
            let occurred_at = context.occurred_at.unwrap_or(now_at);
            let risk_score = context
                .risk_score
                .unwrap_or_else(|| default_risk_score(row_index, &import_row.source_type))
                .clamp(0.0, 100.0);
            let risk_level = normalize_risk_level(
                context
                    .risk_level
                    .as_deref()
                    .unwrap_or(risk_level_from_score(risk_score)),
            );
            let status = normalize_case_status(context.status.as_deref().unwrap_or("pending_review"))?;
            let alert_status =
                normalize_alert_status(context.alert_status.as_deref().unwrap_or(default_alert_status(risk_level)))?;
            let due_at = occurred_at + Duration::days(3 + i64::from(row_index % 4));
            let case_code = format!(
                "IMP-{}-{:04}",
                &import_id.simple().to_string()[..8],
                row_index
            );

            let similar_cases = vector_store
                .search_similar_cases(
                    &format!(
                        "{}\n{}\n{}\n{}",
                        title, area_name, import_row.source_type, risk_level
                    ),
                    None,
                    3,
                )
                .await
                .unwrap_or_default();

            let recommendation = ai_service
                .recommend_case_action(&RecommendationInput {
                title: title.clone(),
                area_name: area_name.clone(),
                risk_level: risk_level.to_string(),
                source_type: import_row.source_type.clone(),
                entity_count: 0,
                alert_count: 0,
                dispatch_count: 0,
                reference_cases: format_reference_cases(&similar_cases),
                })
                .await;

            let risk_tags = if context.risk_tags.is_empty() {
                default_risk_tags(&import_row.source_type, risk_level)
            } else {
                context.risk_tags.join(",")
            };

            let case_id = sqlx::query_scalar::<_, Uuid>(
                r#"
                INSERT INTO risk_cases (
                    id, import_id, case_code, title, source_type, area_name,
                    risk_level, risk_score, status, alert_status, assignee,
                    occurred_at, due_at, closed_at, report_period, created_at, updated_at,
                    risk_reason_summary, disposal_advice, review_status, risk_tags
                )
                VALUES (
                    $1, $2, $3, $4, $5, $6,
                    $7, $8, $9, $10, $11,
                    $12, $13, CASE WHEN $9 = 'closed' THEN $14 ELSE NULL END, $15, $14, $14,
                    $16, $17, $18, $19
                )
                ON CONFLICT (case_code)
                DO UPDATE SET
                    title = EXCLUDED.title,
                    source_type = EXCLUDED.source_type,
                    area_name = EXCLUDED.area_name,
                    risk_level = EXCLUDED.risk_level,
                    risk_score = EXCLUDED.risk_score,
                    status = EXCLUDED.status,
                    alert_status = EXCLUDED.alert_status,
                    assignee = EXCLUDED.assignee,
                    occurred_at = EXCLUDED.occurred_at,
                    due_at = EXCLUDED.due_at,
                    closed_at = EXCLUDED.closed_at,
                    report_period = EXCLUDED.report_period,
                    updated_at = EXCLUDED.updated_at,
                    risk_reason_summary = EXCLUDED.risk_reason_summary,
                    disposal_advice = EXCLUDED.disposal_advice,
                    review_status = EXCLUDED.review_status,
                    risk_tags = EXCLUDED.risk_tags
                RETURNING id
                "#,
            )
            .bind(Uuid::new_v4())
            .bind(import_id)
            .bind(&case_code)
            .bind(&title)
            .bind(&import_row.source_type)
            .bind(&area_name)
            .bind(risk_level)
            .bind(risk_score)
            .bind(status)
            .bind(alert_status)
            .bind(context.assignee.clone())
            .bind(occurred_at)
            .bind(due_at)
            .bind(now_at)
            .bind(context.report_period.clone().unwrap_or_else(current_period))
            .bind(&recommendation.reason_summary)
            .bind(recommendation.disposal_advice.join(" | "))
            .bind(default_review_status(risk_level))
            .bind(&risk_tags)
            .fetch_one(&mut *tx)
            .await
            .map_err(|_| AppError::Internal)?;

            processed_record_count += 1;
            affected_case_count += 1;
            vector_documents.push(VectorCaseDocument {
                id: case_id.to_string(),
                case_id: case_id.to_string(),
                case_code: case_code.clone(),
                title: title.clone(),
                summary: recommendation.reason_summary.clone(),
                risk_level: risk_level.to_string(),
                source_type: import_row.source_type.clone(),
                area_name: area_name.clone(),
            });

            ensure_case_alert(
                &mut tx,
                case_id,
                &title,
                risk_level,
                alert_status,
                &recommendation.reason_summary,
                now_at,
            )
            .await?;
        }
    }

    let final_status = if processed_record_count > 0 { "processed" } else { "failed" };
    if processed_record_count == 0 {
        failed_record_count = total_record_count;
    }

    update_processing_workflow_run(
        &mut tx,
        workflow_run_id,
        final_status,
        total_record_count,
        processed_record_count,
        failed_record_count,
        now_at,
    )
    .await?;

    set_import_status(
        &mut tx,
        import_id,
        final_status,
        now_at,
        mapping_template_id,
        Some(total_record_count),
        Some(processed_record_count),
        Some(failed_record_count),
    )
    .await?;

    tx.commit().await.map_err(|_| AppError::Internal)?;

    for document in &vector_documents {
        let _ = vector_store.upsert_case_vector(document).await;
    }

    Ok(ProcessImportResult {
        import_id,
        status: final_status.to_string(),
        total_record_count,
        processed_record_count,
        failed_record_count,
        affected_case_count,
        workflow_run_id,
        mapping_template_id,
        finished_at: now_at,
    })
}

pub async fn execute_extraction_run(
    state: &AppState,
    case_ids: Option<Vec<Uuid>>,
    mode: Option<String>,
    ai_service: &OpenAiCompatibleAiService,
    graph_service: &HugeGraphSyncService,
    vector_store: &MilvusVectorStore,
) -> Result<ExtractionResult, AppError> {
    let mode = mode
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty())
        .unwrap_or_else(|| "incremental".to_string());
    let cases = if let Some(case_ids) = case_ids {
        if case_ids.is_empty() {
            return Err(AppError::Validation("case_ids cannot be empty".to_string()));
        }

        sqlx::query_as::<_, RiskCaseSeedRow>(
            r#"
            SELECT id, case_code, title, source_type, area_name, risk_level
            FROM risk_cases
            WHERE id = ANY($1)
            ORDER BY updated_at DESC, id DESC
            "#,
        )
        .bind(&case_ids)
        .fetch_all(state.db())
        .await
        .map_err(|_| AppError::Internal)?
    } else {
        sqlx::query_as::<_, RiskCaseSeedRow>(
            r#"
            SELECT id, case_code, title, source_type, area_name, risk_level
            FROM risk_cases
            ORDER BY updated_at DESC, id DESC
            LIMIT 50
            "#,
        )
        .fetch_all(state.db())
        .await
        .map_err(|_| AppError::Internal)?
    };

    let now_at = Utc::now();
    let run_id = Uuid::new_v4();
    let workflow_run_id = Uuid::new_v4();
    let scope_type = if case_ids_is_selected(&cases) {
        "selected_cases"
    } else {
        "all_recent_cases"
    };

    let mut tx = state.db().begin().await.map_err(|_| AppError::Internal)?;
    sqlx::query(
        r#"
        INSERT INTO extraction_runs (
            id, scope_type, mode, status, item_count, success_count, failure_count,
            summary, started_at, finished_at, created_at, updated_at, provider_style, model_name
        )
        VALUES ($1, $2, $3, 'running', $4, 0, 0, $5, $6, NULL, $6, $6, $7, $8)
        "#,
    )
    .bind(run_id)
    .bind(scope_type)
    .bind(&mode)
    .bind(cases.len() as i32)
    .bind("running OpenAI-compatible extraction pipeline")
    .bind(now_at)
    .bind(ai_service.configured_contract().provider_style)
    .bind(ai_service.configured_contract().model_name)
    .execute(&mut *tx)
    .await
    .map_err(|_| AppError::Internal)?;

    sqlx::query(
        r#"
        INSERT INTO workflow_runs (
            id, stage_key, stage_label, status, started_at, finished_at,
            item_count, success_count, failure_count, created_at, updated_at
        )
        VALUES ($1, 'extraction', 'Knowledge Extraction', 'running', $2, NULL, $3, 0, 0, $2, $2)
        "#,
    )
    .bind(workflow_run_id)
    .bind(now_at)
    .bind(cases.len() as i32)
    .execute(&mut *tx)
    .await
    .map_err(|_| AppError::Internal)?;

    let mut created_entity_count = 0_i32;
    let mut created_relation_count = 0_i32;
    let mut success_count = 0_i32;
    let mut graph_sync_inputs = Vec::new();
    let mut vector_documents = Vec::new();

    for case in &cases {
        delete_case_graph(&mut tx, case.id).await?;

        let extracted = ai_service
            .extract_case_graph(&ExtractionInput {
            title: case.title.clone(),
            area_name: case.area_name.clone(),
            source_type: case.source_type.clone(),
            risk_level: case.risk_level.clone(),
            })
            .await;
        let crate::services::ai::ExtractionOutput {
            entities,
            relations,
            ..
        } = extracted;

        let mut entity_ids = Vec::with_capacity(entities.len());
        let mut graph_entities = Vec::with_capacity(entities.len());
        for entity in entities {
            let entity_id = Uuid::new_v4();
            let entity_type = entity.entity_type.clone();
            let entity_name = entity.entity_name.clone();
            let entity_confidence = entity.confidence;
            sqlx::query(
                r#"
                INSERT INTO knowledge_entities (
                    id, case_id, entity_type, entity_name, confidence, extracted_at, created_at
                )
                VALUES ($1, $2, $3, $4, $5, $6, $6)
                "#,
            )
            .bind(entity_id)
            .bind(case.id)
            .bind(&entity_type)
            .bind(&entity_name)
            .bind(entity_confidence)
            .bind(now_at)
            .execute(&mut *tx)
            .await
            .map_err(|_| AppError::Internal)?;
            entity_ids.push(entity_id);
            graph_entities.push(GraphEntitySync {
                entity_type,
                entity_name,
                confidence: entity_confidence,
            });
            created_entity_count += 1;
        }

        let similar_cases = vector_store
            .search_similar_cases(
                &format!(
                    "{}\n{}\n{}\n{}",
                    case.title, case.area_name, case.source_type, case.risk_level
                ),
                Some(&case.id.to_string()),
                3,
            )
            .await
            .unwrap_or_default();

        let recommendation = ai_service
            .recommend_case_action(&RecommendationInput {
            title: case.title.clone(),
            area_name: case.area_name.clone(),
            risk_level: case.risk_level.clone(),
            source_type: case.source_type.clone(),
            entity_count: entity_ids.len(),
            alert_count: current_case_alert_count(&mut tx, case.id).await?,
            dispatch_count: current_case_dispatch_count(&mut tx, case.id).await?,
            reference_cases: format_reference_cases(&similar_cases),
            })
            .await;

        let reason_summary = recommendation.reason_summary.clone();
        let disposal_advice = recommendation.disposal_advice.join(" | ");
        sqlx::query(
            r#"
            UPDATE risk_cases
            SET risk_reason_summary = $2,
                disposal_advice = $3,
                updated_at = $4
            WHERE id = $1
            "#,
        )
        .bind(case.id)
        .bind(&reason_summary)
        .bind(&disposal_advice)
        .bind(now_at)
        .execute(&mut *tx)
        .await
        .map_err(|_| AppError::Internal)?;

        let mut graph_relations = Vec::new();
        for relation in relations {
            if relation.source_index >= entity_ids.len() || relation.target_index >= entity_ids.len() {
                continue;
            }

            let relation_type = relation.relation_type.clone();
            let relation_confidence = relation.confidence;

            sqlx::query(
                r#"
                INSERT INTO graph_relations (
                    id, relation_type, source_entity_id, target_entity_id, created_at
                )
                VALUES ($1, $2, $3, $4, $5)
                "#,
            )
            .bind(Uuid::new_v4())
            .bind(&relation_type)
            .bind(entity_ids[relation.source_index])
            .bind(entity_ids[relation.target_index])
            .bind(now_at)
            .execute(&mut *tx)
            .await
            .map_err(|_| AppError::Internal)?;
            graph_relations.push(GraphRelationSync {
                relation_type,
                source_entity_name: graph_entities[relation.source_index].entity_name.clone(),
                target_entity_name: graph_entities[relation.target_index].entity_name.clone(),
                confidence: relation_confidence,
            });
            created_relation_count += 1;
        }

        graph_sync_inputs.push(GraphCaseSyncInput {
            case_id: case.id.to_string(),
            case_code: case.case_code.clone(),
            title: case.title.clone(),
            area_name: case.area_name.clone(),
            risk_level: case.risk_level.clone(),
            source_type: case.source_type.clone(),
            entities: graph_entities.clone(),
            relations: graph_relations,
        });
        vector_documents.push(VectorCaseDocument {
            id: case.id.to_string(),
            case_id: case.id.to_string(),
            case_code: case.case_code.clone(),
            title: case.title.clone(),
            summary: reason_summary,
            risk_level: case.risk_level.clone(),
            source_type: case.source_type.clone(),
            area_name: case.area_name.clone(),
        });

        success_count += 1;
    }

    let failure_count = cases.len() as i32 - success_count;
    let summary = format!(
        "processed {} cases, created {} entities and {} relations via placeholder extraction service",
        success_count, created_entity_count, created_relation_count
    );

    sqlx::query(
        r#"
        UPDATE extraction_runs
        SET status = $2,
            item_count = $3,
            success_count = $4,
            failure_count = $5,
            summary = $6,
            finished_at = $7,
            updated_at = $7
        WHERE id = $1
        "#,
    )
    .bind(run_id)
    .bind(if failure_count > 0 { "completed_with_warnings" } else { "completed" })
    .bind(cases.len() as i32)
    .bind(success_count)
    .bind(failure_count)
    .bind(&summary)
    .bind(now_at)
    .execute(&mut *tx)
    .await
    .map_err(|_| AppError::Internal)?;

    sqlx::query(
        r#"
        UPDATE workflow_runs
        SET status = $2,
            finished_at = $3,
            item_count = $4,
            success_count = $5,
            failure_count = $6,
            updated_at = $3
        WHERE id = $1
        "#,
    )
    .bind(workflow_run_id)
    .bind(if failure_count > 0 { "attention" } else { "completed" })
    .bind(now_at)
    .bind(cases.len() as i32)
    .bind(success_count)
    .bind(failure_count)
    .execute(&mut *tx)
    .await
    .map_err(|_| AppError::Internal)?;

    tx.commit().await.map_err(|_| AppError::Internal)?;

    for input in &graph_sync_inputs {
        let _ = graph_service.sync_case_graph(input).await;
    }
    for document in &vector_documents {
        let _ = vector_store.upsert_case_vector(document).await;
    }

    Ok(ExtractionResult {
        run_id,
        status: if failure_count > 0 {
            "completed_with_warnings".to_string()
        } else {
            "completed".to_string()
        },
        item_count: cases.len() as i32,
        success_count,
        failure_count,
        created_entity_count,
        created_relation_count,
        summary,
        finished_at: now_at,
    })
}

async fn latest_mapping_template_id(
    db: &sqlx::PgPool,
    source_type: &str,
) -> Result<Option<Uuid>, AppError> {
    let template = sqlx::query_as::<_, MappingTemplateRow>(
        r#"
        SELECT id
        FROM mapping_templates
        WHERE source_type = $1
        ORDER BY is_active DESC, updated_at DESC, created_at DESC
        LIMIT 1
        "#,
    )
    .bind(source_type)
    .fetch_optional(db)
    .await
    .map_err(|_| AppError::Internal)?;

    Ok(template.map(|row| row.id))
}

async fn load_mapping_fields(
    db: &sqlx::PgPool,
    template_id: Option<Uuid>,
) -> Result<Vec<MappingFieldConfig>, AppError> {
    let Some(template_id) = template_id else {
        return Ok(Vec::new());
    };

    let fields = sqlx::query_as::<_, MappingFieldRow>(
        r#"
        SELECT source_field, target_field, status
        FROM mapping_fields
        WHERE template_id = $1
        ORDER BY sort_order ASC, created_at ASC
        "#,
    )
    .bind(template_id)
    .fetch_all(db)
    .await
    .map_err(|_| AppError::Internal)?;

    Ok(fields
        .into_iter()
        .filter(|field| {
            matches!(
                field.status.as_str(),
                "mapped" | "confirmed" | "active" | "approved" | "needs_review"
            )
        })
        .map(|field| MappingFieldConfig {
            source_field: field.source_field,
            target_field: field.target_field,
        })
        .collect())
}

async fn set_import_status(
    tx: &mut Transaction<'_, Postgres>,
    import_id: Uuid,
    status: &str,
    updated_at: DateTime<Utc>,
    mapping_template_id: Option<Uuid>,
    total_record_count: Option<i32>,
    processed_record_count: Option<i32>,
    failed_record_count: Option<i32>,
) -> Result<(), AppError> {
    sqlx::query(
        r#"
        UPDATE imports
        SET status = $2,
            mapping_template_id = COALESCE($3, mapping_template_id),
            total_record_count = COALESCE($4, total_record_count),
            processed_record_count = COALESCE($5, processed_record_count),
            failed_record_count = COALESCE($6, failed_record_count),
            last_processed_at = CASE WHEN $2 IN ('processing', 'processed', 'failed') THEN $7 ELSE last_processed_at END,
            updated_at = $7
        WHERE id = $1
        "#,
    )
    .bind(import_id)
    .bind(status)
    .bind(mapping_template_id)
    .bind(total_record_count)
    .bind(processed_record_count)
    .bind(failed_record_count)
    .bind(updated_at)
    .execute(&mut **tx)
    .await
    .map_err(|_| AppError::Internal)?;

    Ok(())
}

async fn insert_processing_workflow_run(
    tx: &mut Transaction<'_, Postgres>,
    workflow_run_id: Uuid,
    now_at: DateTime<Utc>,
) -> Result<(), AppError> {
    sqlx::query(
        r#"
        INSERT INTO workflow_runs (
            id, stage_key, stage_label, status, started_at, finished_at,
            item_count, success_count, failure_count, created_at, updated_at
        )
        VALUES ($1, 'processing', 'Data Processing', 'running', $2, NULL, 0, 0, 0, $2, $2)
        "#,
    )
    .bind(workflow_run_id)
    .bind(now_at)
    .execute(&mut **tx)
    .await
    .map_err(|_| AppError::Internal)?;

    Ok(())
}

async fn update_processing_workflow_run(
    tx: &mut Transaction<'_, Postgres>,
    workflow_run_id: Uuid,
    status: &str,
    item_count: i32,
    success_count: i32,
    failure_count: i32,
    finished_at: DateTime<Utc>,
) -> Result<(), AppError> {
    sqlx::query(
        r#"
        UPDATE workflow_runs
        SET status = $2,
            finished_at = $3,
            item_count = $4,
            success_count = $5,
            failure_count = $6,
            updated_at = $3
        WHERE id = $1
        "#,
    )
    .bind(workflow_run_id)
    .bind(status)
    .bind(finished_at)
    .bind(item_count)
    .bind(success_count)
    .bind(failure_count)
    .execute(&mut **tx)
    .await
    .map_err(|_| AppError::Internal)?;

    Ok(())
}

async fn ensure_case_alert(
    tx: &mut Transaction<'_, Postgres>,
    case_id: Uuid,
    title: &str,
    risk_level: &str,
    alert_status: &str,
    summary: &str,
    now_at: DateTime<Utc>,
) -> Result<(), AppError> {
    if matches!(risk_level, "low") {
        return Ok(());
    }

    sqlx::query(
        r#"
        INSERT INTO alerts (
            id, case_id, title, severity, status, summary, created_at, updated_at, handled_at
        )
        SELECT $1, $2, $3, $4, $5, $6, $7, $7,
               CASE WHEN $5 IN ('closed', 'ignored') THEN $7 ELSE NULL END
        WHERE NOT EXISTS (
            SELECT 1 FROM alerts WHERE case_id = $2 AND title = $3
        )
        "#,
    )
    .bind(Uuid::new_v4())
    .bind(case_id)
    .bind(format!("Alert-{}", title))
    .bind(risk_level)
    .bind(alert_status)
    .bind(summary)
    .bind(now_at)
    .execute(&mut **tx)
    .await
    .map_err(|_| AppError::Internal)?;

    Ok(())
}

async fn delete_case_graph(
    tx: &mut Transaction<'_, Postgres>,
    case_id: Uuid,
) -> Result<(), AppError> {
    sqlx::query(
        r#"
        DELETE FROM graph_relations
        WHERE source_entity_id IN (
                SELECT id FROM knowledge_entities WHERE case_id = $1
            )
           OR target_entity_id IN (
                SELECT id FROM knowledge_entities WHERE case_id = $1
            )
        "#,
    )
    .bind(case_id)
    .execute(&mut **tx)
    .await
    .map_err(|_| AppError::Internal)?;

    sqlx::query("DELETE FROM knowledge_entities WHERE case_id = $1")
        .bind(case_id)
        .execute(&mut **tx)
        .await
        .map_err(|_| AppError::Internal)?;

    Ok(())
}

async fn current_case_alert_count(
    tx: &mut Transaction<'_, Postgres>,
    case_id: Uuid,
) -> Result<usize, AppError> {
    sqlx::query_scalar::<_, i64>("SELECT COUNT(*) FROM alerts WHERE case_id = $1")
        .bind(case_id)
        .fetch_one(&mut **tx)
        .await
        .map(|value| value.max(0) as usize)
        .map_err(|_| AppError::Internal)
}

async fn current_case_dispatch_count(
    tx: &mut Transaction<'_, Postgres>,
    case_id: Uuid,
) -> Result<usize, AppError> {
    sqlx::query_scalar::<_, i64>("SELECT COUNT(*) FROM dispatch_tasks WHERE case_id = $1")
        .bind(case_id)
        .fetch_one(&mut **tx)
        .await
        .map(|value| value.max(0) as usize)
        .map_err(|_| AppError::Internal)
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
    let file = File::open(path).map_err(|_| AppError::Validation("failed to read csv file".to_string()))?;
    let mut reader = csv::Reader::from_reader(BufReader::new(file));
    let headers = reader
        .headers()
        .map_err(|_| AppError::Validation("failed to parse csv headers".to_string()))?
        .clone();

    let mut rows = Vec::new();
    for record in reader.records() {
        let record = record.map_err(|_| AppError::Validation("failed to parse csv row".to_string()))?;
        rows.push(build_row_map(&headers, &record));
    }

    if rows.is_empty() {
        rows.push(HashMap::new());
    }

    Ok(rows)
}

fn parse_excel_rows(path: &Path) -> Result<Vec<HashMap<String, String>>, AppError> {
    let mut workbook =
        open_workbook_auto(path).map_err(|_| AppError::Validation("failed to read excel file".to_string()))?;
    let sheet_name = workbook
        .sheet_names()
        .first()
        .cloned()
        .ok_or_else(|| AppError::Validation("excel workbook does not contain sheets".to_string()))?;
    let range = workbook
        .worksheet_range(&sheet_name)
        .map_err(|_| AppError::Validation("failed to parse excel sheet".to_string()))?;

    let mut rows_iter = range.rows();
    let headers = rows_iter
        .next()
        .ok_or_else(|| AppError::Validation("excel file does not contain a header row".to_string()))?
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
    row.insert("title".to_string(), format!("{} generated placeholder lead", file_name));
    row.insert("area_name".to_string(), "pending-area".to_string());
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
            "risk_tags" => {
                context.risk_tags = value
                    .split(|ch| ch == ',' || ch == ';' || ch == '|')
                    .map(str::trim)
                    .filter(|value| !value.is_empty())
                    .map(str::to_string)
                    .collect();
            }
            _ => {}
        }
    }

    if context.title.is_none() {
        context.title = extract_value(record, &["title", "case_title", "subject", "summary"]);
    }
    if context.area_name.is_none() {
        context.area_name = extract_value(record, &["area_name", "street", "district", "area"]);
    }
    if context.occurred_at.is_none() {
        context.occurred_at = extract_value(record, &["occurred_at", "date", "time"])
            .and_then(|value| parse_datetime_value(&value));
    }
    if context.risk_score.is_none() {
        context.risk_score = extract_value(record, &["risk_score", "score", "rating"])
            .and_then(|value| parse_score_value(&value));
    }
    if context.risk_level.is_none() {
        context.risk_level = extract_value(record, &["risk_level", "level"]);
    }
    if context.status.is_none() {
        context.status = extract_value(record, &["status", "case_status"]);
    }
    if context.alert_status.is_none() {
        context.alert_status = extract_value(record, &["alert_status", "warning_status"]);
    }
    if context.assignee.is_none() {
        context.assignee = extract_value(record, &["assignee", "owner", "handler"]);
    }
    if context.report_period.is_none() {
        context.report_period = extract_value(record, &["report_period", "period"]);
    }
    if context.risk_tags.is_empty() {
        context.risk_tags = extract_value(record, &["risk_tags", "tags"])
            .map(|value| {
                value.split(|ch| ch == ',' || ch == ';' || ch == '|')
                    .map(str::trim)
                    .filter(|segment| !segment.is_empty())
                    .map(str::to_string)
                    .collect()
            })
            .unwrap_or_default();
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

fn default_risk_score(row_index: i32, source_type: &str) -> f64 {
    let base = match source_type {
        "hotline_12345" => 78.0,
        "police_110" => 82.0,
        "petitions" => 86.0,
        _ => 72.0,
    };

    base + f64::from((row_index % 5) * 3)
}

fn normalize_risk_level(value: &str) -> &'static str {
    match value.trim().to_ascii_lowercase().as_str() {
        "high" | "critical" => "high",
        "medium" | "mid" => "medium",
        _ => "low",
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

fn normalize_case_status(value: &str) -> Result<&'static str, AppError> {
    match value.trim().to_ascii_lowercase().as_str() {
        "todo" | "pending_review" | "in_progress" | "disposed" | "closed" => {
            Ok(Box::leak(value.trim().to_ascii_lowercase().into_boxed_str()))
        }
        "open" => Ok("pending_review"),
        other => Err(AppError::Validation(format!("unsupported case status: {other}"))),
    }
}

fn normalize_alert_status(value: &str) -> Result<&'static str, AppError> {
    match value.trim().to_ascii_lowercase().as_str() {
        "open" | "acknowledged" | "ignored" | "closed" => {
            Ok(Box::leak(value.trim().to_ascii_lowercase().into_boxed_str()))
        }
        "resolved" => Ok("closed"),
        other => Err(AppError::Validation(format!("unsupported alert status: {other}"))),
    }
}

fn default_alert_status(risk_level: &str) -> &'static str {
    match risk_level {
        "high" | "medium" => "open",
        _ => "closed",
    }
}

fn default_review_status(risk_level: &str) -> &'static str {
    if risk_level == "high" {
        "manual_review_required"
    } else {
        "pending"
    }
}

fn default_risk_tags(source_type: &str, risk_level: &str) -> String {
    let mut tags = vec![source_type.to_string(), format!("level:{risk_level}")];
    if risk_level == "high" {
        tags.push("escalation".to_string());
    }
    tags.join(",")
}

fn current_period() -> String {
    let now = Utc::now();
    format!("{}-{:02}", now.year(), now.month())
}

fn case_ids_is_selected(cases: &[RiskCaseSeedRow]) -> bool {
    cases.len() < 50
}

fn format_reference_cases(hits: &[SimilarCaseHit]) -> Vec<String> {
    hits.iter()
        .map(|hit| {
            format!(
                "{} | {} | {} | {:.4}",
                hit.case_code, hit.title, hit.risk_level, hit.score
            )
        })
        .collect()
}
