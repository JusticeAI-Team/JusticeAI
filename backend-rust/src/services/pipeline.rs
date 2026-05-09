use std::{
    collections::{HashMap, HashSet},
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
        embedding::OpenAiCompatibleEmbeddingService,
        graph::{GraphCaseSyncInput, GraphEntitySync, GraphRelationSync, HugeGraphSyncService},
        vector::{MilvusVectorStore, SimilarCaseHit, VectorCaseDocument, VectorSearchQuery},
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
    source_type: Option<String>,
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
    embedding_service: &OpenAiCompatibleEmbeddingService,
    vector_store: &MilvusVectorStore,
) -> Result<ProcessImportResult, AppError> {
    let import_row =
        sqlx::query_as::<_, ImportRow>("SELECT source_type FROM imports WHERE id = $1")
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
    let mapping_template_id =
        latest_mapping_template_id(state.db(), &import_row.source_type).await?;
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
            let case_source_type = context
                .source_type
                .clone()
                .unwrap_or_else(|| import_row.source_type.clone());
            let title = context
                .title
                .unwrap_or_else(|| format!("{}-lead-{}", file.original_filename, row_index));
            let area_name = context
                .area_name
                .unwrap_or_else(|| "unassigned-area".to_string());
            let occurred_at = context.occurred_at.unwrap_or(now_at);
            let risk_score = context
                .risk_score
                .unwrap_or_else(|| default_risk_score(row_index, &case_source_type))
                .clamp(0.0, 100.0);
            let risk_level = normalize_risk_level(
                context
                    .risk_level
                    .as_deref()
                    .unwrap_or(risk_level_from_score(risk_score)),
            );
            let status =
                normalize_case_status(context.status.as_deref().unwrap_or("pending_review"))?;
            let alert_status = normalize_alert_status(
                context
                    .alert_status
                    .as_deref()
                    .unwrap_or(default_alert_status(risk_level)),
            )?;
            let due_at = occurred_at + Duration::days(3 + i64::from(row_index % 4));
            let case_code = format!(
                "IMP-{}-{:04}",
                &import_id.simple().to_string()[..8],
                row_index
            );

            let similar_cases = vector_store
                .search_similar_cases(&VectorSearchQuery {
                    embedding: embed_for_similarity(
                        embedding_service,
                        &format!(
                            "{}\n{}\n{}\n{}",
                            title, area_name, case_source_type, risk_level
                        ),
                    )
                    .await
                    .unwrap_or_default(),
                    exclude_case_id: None,
                    limit: 3,
                })
                .await
                .unwrap_or_default();

            let recommendation = ai_service
                .recommend_case_action(&RecommendationInput {
                    title: title.clone(),
                    area_name: area_name.clone(),
                    risk_level: risk_level.to_string(),
                    source_type: case_source_type.clone(),
                    entity_count: 0,
                    alert_count: 0,
                    dispatch_count: 0,
                    reference_cases: format_reference_cases(&similar_cases),
                })
                .await;

            let risk_tags = compose_risk_tags(&case_source_type, risk_level, &context.risk_tags);

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
            .bind(&case_source_type)
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
                source_type: case_source_type.clone(),
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

    let final_status = if processed_record_count > 0 {
        "processed"
    } else {
        "failed"
    };
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
        let sync = sync_case_vector(embedding_service, vector_store, document).await;
        update_case_vector_sync(
            state.db(),
            parse_uuid(&document.case_id),
            &sync.status,
            &sync.message,
            Utc::now(),
        )
        .await?;
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
    embedding_service: &OpenAiCompatibleEmbeddingService,
    graph_service: &HugeGraphSyncService,
    vector_store: &MilvusVectorStore,
) -> Result<ExtractionResult, AppError> {
    let mode = mode
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty())
        .unwrap_or_else(|| "incremental".to_string());
    let selected_case_ids = case_ids.is_some();
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
    let scope_type = if selected_case_ids {
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
    let mut graph_sync_messages = Vec::new();
    let mut graph_sync_failures = Vec::new();
    let mut vector_sync_messages = Vec::new();
    let mut vector_sync_failures = Vec::new();
    let mut extraction_summaries = Vec::new();

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
            summary,
            model_contract,
            is_placeholder,
            ..
        } = extracted;
        extraction_summaries.push(if is_placeholder {
            format!("fallback: {summary}")
        } else {
            format!("{} via {}", summary, model_contract.model_name)
        });

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
            .search_similar_cases(&VectorSearchQuery {
                embedding: embed_for_similarity(
                    embedding_service,
                    &format!(
                        "{}\n{}\n{}\n{}",
                        case.title, case.area_name, case.source_type, case.risk_level
                    ),
                )
                .await
                .unwrap_or_default(),
                exclude_case_id: Some(case.id.to_string()),
                limit: 3,
            })
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
            if relation.source_index >= entity_ids.len()
                || relation.target_index >= entity_ids.len()
            {
                continue;
            }

            let relation_type = relation.relation_type.clone();
            let relation_confidence = relation.confidence;

            sqlx::query(
                r#"
                INSERT INTO graph_relations (
                    id, relation_type, source_entity_id, target_entity_id, confidence, created_at
                )
                VALUES ($1, $2, $3, $4, $5, $6)
                "#,
            )
            .bind(Uuid::new_v4())
            .bind(&relation_type)
            .bind(entity_ids[relation.source_index])
            .bind(entity_ids[relation.target_index])
            .bind(relation_confidence)
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
        "processed {} cases, created {} entities and {} relations. {}",
        success_count,
        created_entity_count,
        created_relation_count,
        extraction_summaries.join(" || ")
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
    .bind(if failure_count > 0 {
        "completed_with_warnings"
    } else {
        "completed"
    })
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
    .bind(if failure_count > 0 {
        "attention"
    } else {
        "completed"
    })
    .bind(now_at)
    .bind(cases.len() as i32)
    .bind(success_count)
    .bind(failure_count)
    .execute(&mut *tx)
    .await
    .map_err(|_| AppError::Internal)?;

    tx.commit().await.map_err(|_| AppError::Internal)?;

    for input in &graph_sync_inputs {
        match graph_service.sync_case_graph(input).await {
            Ok(result) => {
                let case_id = parse_uuid(&input.case_id);
                let sync_message = format!(
                    "{} (vertices={}, edges={})",
                    result.message, result.vertex_count, result.edge_count
                );
                graph_sync_messages.push(sync_message.clone());
                update_case_graph_sync(
                    state.db(),
                    case_id,
                    &result.status,
                    &sync_message,
                    Utc::now(),
                )
                .await?;
            }
            Err(error) => {
                let case_id = parse_uuid(&input.case_id);
                update_case_graph_sync(state.db(), case_id, "failed", &error, Utc::now()).await?;
                graph_sync_messages.push(error.clone());
                graph_sync_failures.push(error);
            }
        }
    }
    for document in &vector_documents {
        let sync = sync_case_vector(embedding_service, vector_store, document).await;
        let case_id = parse_uuid(&document.case_id);
        update_case_vector_sync(state.db(), case_id, &sync.status, &sync.message, Utc::now())
            .await?;
        vector_sync_messages.push(sync.message.clone());
        if sync.status != "indexed" {
            vector_sync_failures.push(sync.message);
        }
    }

    let graph_sync_message = if graph_sync_failures.is_empty() {
        join_messages(&graph_sync_messages)
    } else {
        join_messages(&graph_sync_failures)
    };
    let vector_sync_message = if vector_sync_failures.is_empty() {
        join_messages(&vector_sync_messages)
    } else {
        join_messages(&vector_sync_failures)
    };

    update_extraction_run_sync_status(
        state.db(),
        run_id,
        if graph_sync_failures.is_empty() {
            "synced"
        } else {
            "failed"
        },
        &graph_sync_message,
        if vector_sync_failures.is_empty() {
            "indexed"
        } else {
            "failed"
        },
        &vector_sync_message,
    )
    .await?;

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
    let file = File::open(path)
        .map_err(|_| AppError::Validation("failed to read csv file".to_string()))?;
    let mut reader = csv::Reader::from_reader(BufReader::new(file));
    let headers = reader
        .headers()
        .map_err(|_| AppError::Validation("failed to parse csv headers".to_string()))?
        .clone();

    let mut rows = Vec::new();
    for record in reader.records() {
        let record =
            record.map_err(|_| AppError::Validation("failed to parse csv row".to_string()))?;
        rows.push(build_row_map(&headers, &record));
    }

    if rows.is_empty() {
        rows.push(HashMap::new());
    }

    Ok(rows)
}

fn parse_excel_rows(path: &Path) -> Result<Vec<HashMap<String, String>>, AppError> {
    let mut workbook = open_workbook_auto(path)
        .map_err(|_| AppError::Validation("failed to read excel file".to_string()))?;
    let sheet_names = workbook.sheet_names().to_vec();
    if sheet_names.is_empty() {
        return Err(AppError::Validation(
            "excel workbook does not contain sheets".to_string(),
        ));
    }
    let mut rows = Vec::new();

    for (sheet_index, sheet_name) in sheet_names.iter().enumerate() {
        let range = workbook.worksheet_range(sheet_name).map_err(|_| {
            AppError::Validation(format!("failed to parse excel sheet: {sheet_name}"))
        })?;
        let sheet_rows = range
            .rows()
            .map(|row| row.iter().map(cell_to_string).collect::<Vec<_>>())
            .collect::<Vec<_>>();

        let Some(header_index) = detect_header_row(&sheet_rows) else {
            if let Some(row) = build_image_sheet_placeholder(sheet_name, sheet_index + 1) {
                rows.push(row);
            }
            continue;
        };

        let headers = normalize_excel_headers(&sheet_rows, header_index);
        let mut sheet_data_count = 0_usize;
        for (row_offset, row) in sheet_rows.iter().enumerate().skip(header_index + 1) {
            let mut values = row.clone();
            if values.iter().all(|value| value.trim().is_empty()) {
                continue;
            }
            if values.len() < headers.len() {
                values.resize(headers.len(), String::new());
            }
            let mut mapped = build_row_map_from_vecs(&headers, &values);
            enrich_excel_row_metadata(&mut mapped, sheet_name, sheet_index + 1, row_offset + 1);
            rows.push(mapped);
            sheet_data_count += 1;
        }

        if sheet_data_count == 0 {
            if let Some(row) = build_image_sheet_placeholder(sheet_name, sheet_index + 1) {
                rows.push(row);
            }
        }
    }

    if rows.is_empty() {
        rows.push(HashMap::new());
    }

    Ok(rows)
}

const HEADER_KEYWORDS: &[&str] = &[
    "序号",
    "创建时间",
    "外部工单编号",
    "区级工单编号",
    "派单类型",
    "工单来源",
    "来电主体",
    "工单类型",
    "工单标题",
    "主要内容",
    "所在街道",
    "镇_街道",
    "镇/街道",
    "所属问题点位",
    "来电人姓名",
    "来电人号码",
    "市级一级问题分类",
    "市级二级问题分类",
    "市级三级问题分类",
    "市级问题分类",
    "工单状态",
    "承办单位",
    "处理结果",
    "来访日期",
    "姓名",
    "性别",
    "证件号码",
    "身份证号",
    "联系电话",
    "手机号",
    "通讯地址",
    "是否初次来访",
    "随行人数",
    "事项类型",
    "内容摘要",
    "流转单位",
    "答复情况",
    "时间",
    "来信",
    "信访人姓名",
    "事项内容",
    "诉求",
    "涉四大检察情况",
    "事项分类",
    "内容分类",
    "涉及原案件名称",
    "办理情况",
    "是否集体访",
    "接警编号",
    "关联编号",
    "警情序",
    "报警人",
    "报警电话",
    "接报时间",
    "承办人",
    "反映内容",
    "企业编码",
    "企业名称",
    "项目编码",
    "项目名称",
    "进场时间",
    "退出时间",
    "总包联系人",
    "总包联系方式",
    "总包名称",
    "区县",
    "详情",
];

fn detect_header_row(rows: &[Vec<String>]) -> Option<usize> {
    let mut best: Option<(usize, i32)> = None;

    for (index, row) in rows.iter().take(20).enumerate() {
        let non_empty = row.iter().filter(|value| !value.trim().is_empty()).count() as i32;
        if non_empty < 2 {
            continue;
        }

        let keyword_score = row
            .iter()
            .map(|value| header_keyword_score(value))
            .sum::<i32>();
        let short_label_score = row
            .iter()
            .filter(|value| {
                let value = value.trim();
                !value.is_empty() && value.chars().count() <= 32 && !looks_like_data_value(value)
            })
            .count() as i32;
        let score = keyword_score * 5 + short_label_score + non_empty.min(20) - (index as i32 / 3);

        if best
            .map(|(_, best_score)| score > best_score)
            .unwrap_or(true)
        {
            best = Some((index, score));
        }
    }

    best.and_then(|(index, score)| (score >= 8).then_some(index))
}

fn header_keyword_score(value: &str) -> i32 {
    let normalized = normalize_lookup_key(value);
    if normalized.is_empty() {
        return 0;
    }

    let exact_match = HEADER_KEYWORDS
        .iter()
        .any(|candidate| normalize_lookup_key(candidate) == normalized);
    if exact_match {
        return 3;
    }

    if HEADER_KEYWORDS.iter().any(|candidate| {
        let candidate = normalize_lookup_key(candidate);
        normalized.contains(&candidate) || candidate.contains(&normalized)
    }) {
        return 1;
    }

    0
}

fn looks_like_data_value(value: &str) -> bool {
    let trimmed = value.trim();
    trimmed.parse::<f64>().is_ok()
        || trimmed.chars().count() > 80
        || trimmed.contains("市民反映")
        || trimmed.contains("工单来源：")
        || trimmed.contains("热线-")
        || trimmed.contains("网络-")
        || trimmed.contains("通州-")
}

fn normalize_excel_headers(rows: &[Vec<String>], header_index: usize) -> Vec<String> {
    let current = rows.get(header_index).map(Vec::as_slice).unwrap_or(&[]);
    if header_index == 0 {
        return normalize_headers(current);
    }

    let parent = rows.get(header_index - 1).map(Vec::as_slice).unwrap_or(&[]);
    let width = current.len().max(parent.len());
    let mut merged = Vec::with_capacity(width);
    for index in 0..width {
        let lower = current.get(index).map(String::as_str).unwrap_or("").trim();
        let upper = parent.get(index).map(String::as_str).unwrap_or("").trim();
        let header = if lower.is_empty() { upper } else { lower };
        merged.push(header.to_string());
    }

    normalize_headers(&merged)
}

fn normalize_headers(headers: &[String]) -> Vec<String> {
    let mut seen = HashMap::<String, usize>::new();
    headers
        .iter()
        .enumerate()
        .map(|(index, header)| {
            let base = header.trim();
            let base = if base.is_empty() {
                format!("column_{}", index + 1)
            } else {
                base.to_string()
            };
            let counter = seen.entry(base.clone()).or_insert(0);
            *counter += 1;
            if *counter == 1 {
                base
            } else {
                format!("{base}#{}", *counter)
            }
        })
        .collect()
}

fn enrich_excel_row_metadata(
    row: &mut HashMap<String, String>,
    sheet_name: &str,
    sheet_index: usize,
    excel_row_number: usize,
) {
    let source_channel = source_channel_from_sheet_name(sheet_name);
    row.insert("__sheet_name".to_string(), sheet_name.to_string());
    row.insert("__sheet_index".to_string(), sheet_index.to_string());
    row.insert(
        "__excel_row_number".to_string(),
        excel_row_number.to_string(),
    );
    row.insert("__source_channel".to_string(), source_channel.to_string());
    row.insert(
        "__source_label".to_string(),
        source_label_from_channel(source_channel).to_string(),
    );
    row.entry("source_type".to_string())
        .or_insert_with(|| source_channel.to_string());
}

fn build_image_sheet_placeholder(
    sheet_name: &str,
    sheet_index: usize,
) -> Option<HashMap<String, String>> {
    let source_channel = source_channel_from_sheet_name(sheet_name);
    if !matches!(source_channel, "police_110" | "platform_395") {
        return None;
    }

    let mut row = HashMap::new();
    let (title, summary, tags, fields, assignee, area_hint) = match source_channel {
        "police_110" => (
            "110接出警信息截图待OCR",
            "该分页为嵌入图片，已按截图表头识别为 110 接处警信息。截图字段包括序号、接警编号、关联编号、警情序号、报案人姓名、报警方式、报警电话、接报时间、接报单位、承办人、处理结果、处理时间、反馈内容；待后续 OCR 服务接入后拆分为逐条警情记录。",
            "image_sheet,ocr_pending,police_110,public_security,feedback_content",
            "序号,接警编号,关联编号,警情序号,报案人姓名,报警方式,报警电话,接报时间,接报单位,承办人,处理结果,处理时间,反馈内容",
            "110接处警平台",
            "截图包含多属地警情，待OCR识别",
        ),
        "platform_395" => (
            "395平台劳资/项目数据截图待OCR",
            "该分页为嵌入图片，已按截图表头识别为 395 平台项目人员与劳资风险数据。截图字段包括序号、姓名、身份证号、手机号、企业编码、企业名称、项目编码、项目名称、进场时间、退出时间、总包编码、总包联系人、总包联系方式、总包名称、区县、是否被高、详情；待后续 OCR 服务接入后拆分为项目、企业、人员和欠薪线索记录。",
            "image_sheet,ocr_pending,platform_395,construction_wage,project_personnel",
            "序号,姓名,身份证号,手机号,企业编码,企业名称,项目编码,项目名称,进场时间,退出时间,总包编码,总包联系人,总包联系方式,总包名称,区县,是否被高,详情",
            "395平台",
            "截图包含项目区县，待OCR识别",
        ),
        _ => unreachable!(),
    };

    row.insert("工单标题".to_string(), title.to_string());
    row.insert("主要内容".to_string(), summary.to_string());
    row.insert("反映内容".to_string(), summary.to_string());
    row.insert("详情".to_string(), summary.to_string());
    row.insert("所在街道".to_string(), area_hint.to_string());
    row.insert("区县".to_string(), area_hint.to_string());
    row.insert("承办单位".to_string(), assignee.to_string());
    row.insert("工单状态".to_string(), "待OCR".to_string());
    row.insert("risk_tags".to_string(), tags.to_string());
    row.insert("ocr_status".to_string(), "pending".to_string());
    row.insert("ocr_required".to_string(), "true".to_string());
    row.insert("ocr_required_fields".to_string(), fields.to_string());
    row.insert(
        "source_record_type".to_string(),
        "image_sheet_ocr_pending".to_string(),
    );
    row.insert("__image_sheet".to_string(), "true".to_string());
    enrich_excel_row_metadata(&mut row, sheet_name, sheet_index, 0);
    Some(row)
}

fn source_channel_from_sheet_name(sheet_name: &str) -> &'static str {
    if sheet_name.contains("110") {
        "police_110"
    } else if sheet_name.contains("395") {
        "platform_395"
    } else if sheet_name.contains("12345") || sheet_name.contains("12315") {
        "hotline_12345"
    } else if sheet_name.contains("检察院") && sheet_name.contains("信访") {
        "procuratorate_petition"
    } else if sheet_name.contains("综治") || sheet_name.contains("信访") {
        "petitions"
    } else {
        "manual_excel"
    }
}

fn source_label_from_channel(source_channel: &str) -> &'static str {
    match source_channel {
        "hotline_12345" => "12345/12315 热线工单",
        "police_110" => "110 接处警信息",
        "platform_395" => "395 平台数据",
        "procuratorate_petition" => "检察院信访数据",
        "petitions" => "综治/信访数据",
        _ => "人工导入表格",
    }
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
    row.insert(
        "title".to_string(),
        format!("{} generated placeholder lead", file_name),
    );
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
            "case_title" | "title" => context.title = Some(truncate_title(&value)),
            "source_type" => context.source_type = Some(value),
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

    if context.source_type.is_none() {
        context.source_type = derive_source_type(record);
    }
    if context.title.is_none() {
        context.title = extract_value(
            record,
            &[
                "title",
                "case_title",
                "subject",
                "summary",
                "工单标题",
                "主要内容",
                "事项内容",
                "内容摘要",
                "反映内容",
                "详情",
                "诉求",
                "项目名称",
            ],
        )
        .map(|value| truncate_title(&value));
    }
    if context.area_name.is_none() {
        context.area_name = extract_value(
            record,
            &[
                "area_name",
                "street",
                "district",
                "area",
                "所在街道",
                "镇_街道",
                "镇/街道",
                "街道",
                "区县",
                "通讯地址",
                "所属问题点位",
                "详细地址",
                "项目名称",
            ],
        );
    }
    if context.occurred_at.is_none() {
        context.occurred_at = extract_value(
            record,
            &[
                "occurred_at",
                "date",
                "time",
                "创建时间",
                "来访日期",
                "时间",
                "接报时",
                "接报时间",
                "市级派单时间",
                "区级派单时间",
                "办结时间",
                "进场时间",
                "退出时间",
            ],
        )
        .and_then(|value| parse_datetime_value(&value));
    }
    if context.risk_score.is_none() {
        context.risk_score = extract_value(record, &["risk_score", "score", "rating"])
            .and_then(|value| parse_score_value(&value))
            .or_else(|| derive_risk_score_from_record(record));
    }
    if context.risk_level.is_none() {
        context.risk_level =
            extract_value(record, &["risk_level", "level", "风险等级", "预警等级"]);
    }
    if context.status.is_none() {
        context.status = extract_value(
            record,
            &[
                "status",
                "case_status",
                "工单状态",
                "办理情况",
                "处理结果",
                "答复情况",
                "ocr_status",
            ],
        );
    }
    if context.alert_status.is_none() {
        context.alert_status =
            extract_value(record, &["alert_status", "warning_status", "预警状态"]);
    }
    if context.assignee.is_none() {
        context.assignee = extract_value(
            record,
            &[
                "assignee",
                "owner",
                "handler",
                "承办单位",
                "流转单位",
                "原案涉及部门及检察官",
                "线索分流去向",
                "处置人姓名",
                "总包名称",
                "承办人",
            ],
        );
    }
    if context.report_period.is_none() {
        context.report_period = extract_value(record, &["report_period", "period"]);
    }
    if context.risk_tags.is_empty() {
        context.risk_tags = extract_value(record, &["risk_tags", "tags"])
            .map(|value| {
                value
                    .split(|ch| ch == ',' || ch == ';' || ch == '|')
                    .map(str::trim)
                    .filter(|segment| !segment.is_empty())
                    .map(str::to_string)
                    .collect()
            })
            .unwrap_or_default();
    }
    context.risk_tags.extend(collect_risk_tags(record));
    context.risk_tags = dedupe_strings(context.risk_tags);

    context
}

fn extract_value(record: &HashMap<String, String>, candidates: &[&str]) -> Option<String> {
    for key in candidates {
        if let Some(value) = record
            .get(*key)
            .map(String::as_str)
            .map(str::trim)
            .filter(|value| !value.is_empty())
        {
            return Some(value.to_string());
        }
    }

    let normalized_candidates = candidates
        .iter()
        .map(|candidate| normalize_lookup_key(candidate))
        .collect::<Vec<_>>();

    record.iter().find_map(|(key, value)| {
        let normalized_key = normalize_lookup_key(key);
        normalized_candidates
            .iter()
            .any(|candidate| candidate == &normalized_key)
            .then(|| value.trim())
            .filter(|value| !value.is_empty())
            .map(str::to_string)
    })
}

fn normalize_lookup_key(value: &str) -> String {
    let without_duplicate_suffix = value.split('#').next().unwrap_or(value);
    without_duplicate_suffix
        .chars()
        .filter(|ch| {
            !ch.is_whitespace()
                && !matches!(
                    ch,
                    '_' | '-'
                        | '/'
                        | '\\'
                        | ':'
                        | '：'
                        | '('
                        | ')'
                        | '（'
                        | '）'
                        | '['
                        | ']'
                        | '【'
                        | '】'
                )
        })
        .flat_map(char::to_lowercase)
        .collect()
}

fn derive_source_type(record: &HashMap<String, String>) -> Option<String> {
    for candidate in [
        "__source_channel",
        "source_type",
        "工单来源",
        "__sheet_name",
    ] {
        let Some(value) = extract_value(record, &[candidate]) else {
            continue;
        };
        let normalized = value.to_ascii_lowercase();
        if normalized.contains("110") {
            return Some("police_110".to_string());
        }
        if normalized.contains("395") {
            return Some("platform_395".to_string());
        }
        if normalized.contains("12345") || normalized.contains("12315") || value.contains("热线")
        {
            return Some("hotline_12345".to_string());
        }
        if value.contains("检察院") && value.contains("信访") {
            return Some("procuratorate_petition".to_string());
        }
        if value.contains("综治") || value.contains("信访") {
            return Some("petitions".to_string());
        }
        if matches!(
            normalized.as_str(),
            "manual_excel"
                | "manual_upload"
                | "petitions"
                | "procuratorate_petition"
                | "hotline_12345"
                | "police_110"
                | "platform_395"
        ) {
            return Some(value);
        }
    }

    None
}

fn collect_risk_tags(record: &HashMap<String, String>) -> Vec<String> {
    let mut tags = Vec::new();
    for key in [
        "risk_tags",
        "tags",
        "__source_channel",
        "__sheet_name",
        "工单来源",
        "工单类型",
        "市级一级问题分类",
        "市级二级问题分类",
        "市级三级问题分类",
        "市级问题分类",
        "市级工单标签",
        "区级工单标签",
        "事项类型",
        "事项分类",
        "内容分类",
        "涉四大检察情况",
        "是否涉及非吸投资",
        "是否集体访",
    ] {
        if let Some(value) = extract_value(record, &[key]) {
            tags.extend(split_tag_value(&value));
        }
    }

    let text = record.values().cloned().collect::<Vec<_>>().join(" ");
    for (keyword, tag) in [
        ("欠薪", "劳资欠薪"),
        ("工资", "劳资欠薪"),
        ("恶意讨薪", "劳资纠纷"),
        ("集体访", "集体访"),
        ("群体", "群体性风险"),
        ("非吸", "非法吸收公众存款"),
        ("诈骗", "诈骗风险"),
        ("食品安全", "食品安全"),
        ("住房安全", "住房安全"),
        ("隔断", "违规群租"),
        ("犬", "犬类管理"),
        ("宠物", "犬类管理"),
        ("酒店", "文旅住宿安全"),
        ("报警", "警情关联"),
        ("110", "警情关联"),
        ("施工", "施工项目"),
        ("总包", "工程建设"),
        ("信访", "信访事项"),
        ("投诉", "投诉举报"),
    ] {
        if text.contains(keyword) {
            tags.push(tag.to_string());
        }
    }

    dedupe_strings(tags)
}

fn split_tag_value(value: &str) -> Vec<String> {
    value
        .replace("->", ",")
        .split(|ch| matches!(ch, ',' | '，' | ';' | '；' | '|' | '/' | '、'))
        .map(str::trim)
        .filter(|segment| {
            !segment.is_empty()
                && !matches!(*segment, "是" | "否" | "保密" | "不详" | "其他")
                && segment.chars().count() <= 64
        })
        .map(str::to_string)
        .collect()
}

fn derive_risk_score_from_record(record: &HashMap<String, String>) -> Option<f64> {
    let text = record.values().cloned().collect::<Vec<_>>().join(" ");
    if text.trim().is_empty() {
        return None;
    }

    let mut score = 58.0_f64;
    for keyword in [
        "欠薪",
        "恶意讨薪",
        "集体访",
        "群体",
        "非吸",
        "诈骗",
        "暴力",
        "极端",
        "报警",
        "110",
    ] {
        if text.contains(keyword) {
            score += 10.0;
        }
    }
    for keyword in [
        "纠纷",
        "投诉",
        "信访",
        "食品安全",
        "住房安全",
        "酒店",
        "施工",
        "工资",
        "隔断",
        "安全监管",
    ] {
        if text.contains(keyword) {
            score += 6.0;
        }
    }
    if extract_value(record, &["是否集体访"]).as_deref() == Some("是") {
        score += 15.0;
    }
    if extract_value(record, &["是否超区截止时间", "是否超市截止时间"]).as_deref() == Some("是")
    {
        score += 4.0;
    }

    Some(score.clamp(45.0, 96.0))
}

fn truncate_title(value: &str) -> String {
    let trimmed = value.trim();
    let mut title = trimmed.chars().take(80).collect::<String>();
    if trimmed.chars().count() > 80 {
        title.push('…');
    }
    title
}

fn dedupe_strings(values: Vec<String>) -> Vec<String> {
    let mut seen = HashSet::new();
    let mut deduped = Vec::new();
    for value in values {
        let value = value.trim();
        if value.is_empty() {
            continue;
        }
        let key = normalize_lookup_key(value);
        if seen.insert(key) {
            deduped.push(value.to_string());
        }
    }
    deduped
}

fn parse_datetime_value(value: &str) -> Option<DateTime<Utc>> {
    let value = value.trim();
    if value.is_empty() {
        return None;
    }

    if let Ok(parsed) = DateTime::parse_from_rfc3339(value) {
        return Some(parsed.with_timezone(&Utc));
    }

    if let Some(parsed) = parse_excel_serial_datetime(value) {
        return Some(parsed);
    }

    for format in [
        "%Y-%m-%d %H:%M:%S",
        "%Y-%m-%d %H:%M",
        "%Y/%m/%d %H:%M:%S",
        "%Y/%m/%d %H:%M",
        "%Y年%m月%d日 %H:%M:%S",
        "%Y年%m月%d日 %H:%M",
        "%Y-%m-%d",
        "%Y/%m/%d",
        "%Y年%m月%d日",
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

fn parse_excel_serial_datetime(value: &str) -> Option<DateTime<Utc>> {
    let serial = value.trim().parse::<f64>().ok()?;
    if !(20_000.0..=80_000.0).contains(&serial) {
        return None;
    }

    let epoch = chrono::NaiveDate::from_ymd_opt(1899, 12, 30)?.and_hms_opt(0, 0, 0)?;
    let whole_days = serial.trunc() as i64;
    let seconds = ((serial.fract() * 86_400.0).round()) as i64;
    epoch
        .checked_add_signed(Duration::days(whole_days))?
        .checked_add_signed(Duration::seconds(seconds))
        .map(|datetime| DateTime::<Utc>::from_naive_utc_and_offset(datetime, Utc))
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
        Data::DateTime(value) => {
            let (year, month, day, hour, minute, second, _) = value.to_ymd_hms_milli();
            format!("{year:04}-{month:02}-{day:02} {hour:02}:{minute:02}:{second:02}")
        }
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
    let normalized = value.trim().to_ascii_lowercase();
    match normalized.as_str() {
        "high" | "critical" => "high",
        "medium" | "mid" => "medium",
        "low" => "low",
        _ if value.contains("高") || value.contains("重大") || value.contains("严重") => {
            "high"
        }
        _ if value.contains("中") || value.contains("一般") => "medium",
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
    let normalized = value.trim().to_ascii_lowercase();
    match normalized.as_str() {
        "todo" => Ok("todo"),
        "pending_review" | "open" | "pending" | "ocr_pending" => Ok("pending_review"),
        "in_progress" | "processing" | "running" => Ok("in_progress"),
        "disposed" | "resolved" => Ok("disposed"),
        "closed" => Ok("closed"),
        _ if value.contains("待") || value.contains("未") => Ok("pending_review"),
        _ if value.contains("办理中") || value.contains("处理中") || value.contains("处置中") => {
            Ok("in_progress")
        }
        _ if value.contains("已处置") || value.contains("已解决") || value.contains("已化解") => {
            Ok("disposed")
        }
        _ if value.contains("已结案")
            || value.contains("已办结")
            || value.contains("办结")
            || value.contains("直接答复") =>
        {
            Ok("closed")
        }
        _ => Ok("pending_review"),
    }
}

fn normalize_alert_status(value: &str) -> Result<&'static str, AppError> {
    let normalized = value.trim().to_ascii_lowercase();
    match normalized.as_str() {
        "open" | "pending" | "pending_review" => Ok("open"),
        "acknowledged" | "confirmed" => Ok("acknowledged"),
        "ignored" => Ok("ignored"),
        "closed" => Ok("closed"),
        "resolved" => Ok("closed"),
        _ if value.contains("忽略") => Ok("ignored"),
        _ if value.contains("确认") || value.contains("已读") => Ok("acknowledged"),
        _ if value.contains("关闭") || value.contains("已结") || value.contains("办结") => {
            Ok("closed")
        }
        _ => Ok("open"),
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

fn compose_risk_tags(source_type: &str, risk_level: &str, tags: &[String]) -> String {
    let mut composed = tags.to_vec();
    composed.push(source_type.to_string());
    composed.push(format!("level:{risk_level}"));
    if risk_level == "high" {
        composed.push("escalation".to_string());
    }
    let composed = dedupe_strings(composed);
    if composed.is_empty() {
        default_risk_tags(source_type, risk_level)
    } else {
        composed.join(",")
    }
}

fn current_period() -> String {
    let now = Utc::now();
    format!("{}-{:02}", now.year(), now.month())
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

async fn sync_case_vector(
    embedding_service: &OpenAiCompatibleEmbeddingService,
    vector_store: &MilvusVectorStore,
    document: &VectorCaseDocument,
) -> crate::services::vector::VectorSyncResult {
    let embedding = match embedding_service
        .embed_text(&format!(
            "{}\n{}\n{}\n{}\n{}",
            document.title,
            document.summary,
            document.area_name,
            document.risk_level,
            document.source_type
        ))
        .await
    {
        Ok(value) => value,
        Err(error) => {
            return crate::services::vector::VectorSyncResult {
                status: "failed".to_string(),
                message: format!("embedding generation failed: {error}"),
            };
        }
    };

    match vector_store.upsert_case_vector(document, &embedding).await {
        Ok(result) => result,
        Err(error) => crate::services::vector::VectorSyncResult {
            status: "failed".to_string(),
            message: format!("milvus upsert failed: {error}"),
        },
    }
}

async fn embed_for_similarity(
    embedding_service: &OpenAiCompatibleEmbeddingService,
    text: &str,
) -> Result<Vec<f32>, String> {
    embedding_service.embed_text(text).await
}

async fn update_case_graph_sync(
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
    .bind(truncate_message(message))
    .bind(synced_at)
    .execute(db)
    .await
    .map_err(|_| AppError::Internal)?;
    Ok(())
}

async fn update_case_vector_sync(
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
    .bind(truncate_message(message))
    .bind(synced_at)
    .execute(db)
    .await
    .map_err(|_| AppError::Internal)?;
    Ok(())
}

async fn update_extraction_run_sync_status(
    db: &sqlx::PgPool,
    run_id: Uuid,
    graph_status: &str,
    graph_message: &str,
    vector_status: &str,
    vector_message: &str,
) -> Result<(), AppError> {
    sqlx::query(
        r#"
        UPDATE extraction_runs
        SET graph_sync_status = $2,
            graph_sync_message = $3,
            vector_sync_status = $4,
            vector_sync_message = $5,
            updated_at = $6
        WHERE id = $1
        "#,
    )
    .bind(run_id)
    .bind(graph_status)
    .bind(truncate_message(graph_message))
    .bind(vector_status)
    .bind(truncate_message(vector_message))
    .bind(Utc::now())
    .execute(db)
    .await
    .map_err(|_| AppError::Internal)?;
    Ok(())
}

fn join_messages(messages: &[String]) -> String {
    if messages.is_empty() {
        String::new()
    } else {
        messages.join(" | ")
    }
}

fn truncate_message(message: &str) -> String {
    let trimmed = message.trim();
    if trimmed.len() <= 1000 {
        trimmed.to_string()
    } else {
        trimmed[..1000].to_string()
    }
}

fn parse_uuid(value: &str) -> Uuid {
    Uuid::parse_str(value).unwrap_or_else(|_| Uuid::nil())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn detects_second_level_header_in_petition_sheet() {
        let rows = vec![
            vec![
                "序号".to_string(),
                "来访日期".to_string(),
                "来访人员信息".to_string(),
                "事件信息".to_string(),
            ],
            vec![
                "姓名".to_string(),
                "性别".to_string(),
                "证件号码".to_string(),
                "联系电话".to_string(),
                "通讯地址".to_string(),
                "是否 初次来访".to_string(),
                "随行人数".to_string(),
                "事项类型".to_string(),
                "内容摘要".to_string(),
                "流转单位".to_string(),
                "答复情况".to_string(),
            ],
            vec![
                "张三".to_string(),
                "男".to_string(),
                "110101199001010000".to_string(),
                "13800000000".to_string(),
                "通州区".to_string(),
                "是".to_string(),
                "0".to_string(),
                "求助".to_string(),
                "反映欠薪问题".to_string(),
                "综治中心".to_string(),
                "办理中".to_string(),
            ],
        ];

        assert_eq!(detect_header_row(&rows), Some(1));
    }

    #[test]
    fn second_level_header_keeps_parent_date_columns() {
        let rows = vec![
            vec![
                "序号".to_string(),
                "来访日期".to_string(),
                "来访人员信息".to_string(),
                "事件信息".to_string(),
            ],
            vec![
                "".to_string(),
                "".to_string(),
                "姓名".to_string(),
                "内容摘要".to_string(),
            ],
        ];

        let headers = normalize_excel_headers(&rows, 1);

        assert_eq!(headers[0], "序号");
        assert_eq!(headers[1], "来访日期");
        assert_eq!(headers[2], "姓名");
        assert_eq!(headers[3], "内容摘要");
    }

    #[test]
    fn context_extracts_chinese_hotline_fields() {
        let mut record = HashMap::new();
        record.insert("__sheet_name".to_string(), "12345示例数据".to_string());
        record.insert("__source_channel".to_string(), "hotline_12345".to_string());
        record.insert("工单标题".to_string(), "门店食品安全投诉".to_string());
        record.insert(
            "主要内容".to_string(),
            "市民反映蛋糕内有虫子，要求监管部门处理。".to_string(),
        );
        record.insert("所在街道".to_string(), "九棵树街道".to_string());
        record.insert("创建时间".to_string(), "46054.0045486111".to_string());
        record.insert("市级一级问题分类".to_string(), "公共安全".to_string());
        record.insert("市级二级问题分类".to_string(), "食品安全".to_string());
        record.insert("工单状态".to_string(), "已结案".to_string());
        record.insert("承办单位".to_string(), "区市场监管局".to_string());

        let context = build_import_record_context(&record, &[]);

        assert_eq!(context.source_type.as_deref(), Some("hotline_12345"));
        assert_eq!(context.title.as_deref(), Some("门店食品安全投诉"));
        assert_eq!(context.area_name.as_deref(), Some("九棵树街道"));
        assert_eq!(context.assignee.as_deref(), Some("区市场监管局"));
        assert_eq!(
            context.occurred_at.unwrap().date_naive().to_string(),
            "2026-02-01"
        );
        assert!(context.risk_tags.iter().any(|tag| tag == "食品安全"));
        assert_eq!(
            normalize_case_status(context.status.as_deref().unwrap()).unwrap(),
            "closed"
        );
    }

    #[test]
    fn image_sheet_placeholder_keeps_stable_ocr_contract() {
        let row = build_image_sheet_placeholder("110接出警信息", 4).unwrap();

        assert_eq!(
            row.get("__source_channel").map(String::as_str),
            Some("police_110")
        );
        assert_eq!(row.get("__image_sheet").map(String::as_str), Some("true"));
        assert_eq!(row.get("ocr_status").map(String::as_str), Some("pending"));
        assert!(row
            .get("主要内容")
            .map(|value| value.contains("接警编号"))
            .unwrap_or(false));
        assert_eq!(row.get("ocr_required").map(String::as_str), Some("true"));
        assert!(row
            .get("ocr_required_fields")
            .map(|value| value.contains("反馈内容"))
            .unwrap_or(false));
    }

    #[test]
    fn platform_395_image_placeholder_exposes_project_personnel_contract() {
        let row = build_image_sheet_placeholder("395平台数据", 5).unwrap();

        assert_eq!(
            row.get("__source_channel").map(String::as_str),
            Some("platform_395")
        );
        assert!(row
            .get("ocr_required_fields")
            .map(|value| value.contains("总包联系方式") && value.contains("项目名称"))
            .unwrap_or(false));
        assert!(row
            .get("risk_tags")
            .map(|value| value.contains("construction_wage"))
            .unwrap_or(false));
    }

    #[test]
    fn chinese_statuses_are_import_safe() {
        assert_eq!(normalize_case_status("已结案").unwrap(), "closed");
        assert_eq!(normalize_case_status("办理中").unwrap(), "in_progress");
        assert_eq!(normalize_case_status("待OCR").unwrap(), "pending_review");
        assert_eq!(normalize_alert_status("已确认").unwrap(), "acknowledged");
    }
}
