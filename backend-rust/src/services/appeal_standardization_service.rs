use std::{
    collections::hash_map::DefaultHasher,
    hash::{Hash, Hasher},
};

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::{FromRow, PgPool};
use uuid::Uuid;

use crate::{
    app::AppState,
    domain::appeal_status,
    services::{
        ai::{AppealStandardizationInput, OpenAiCompatibleAiService},
        appeal_service::{self, mask_phone},
    },
    shared::error::AppError,
};

pub const PROMPT_VERSION: &str = "labor_appeal_standardization_v1";

pub async fn enqueue_standardization_job(
    db: &PgPool,
    appeal_id: Uuid,
    reason: &str,
) -> Result<Uuid, AppError> {
    let appeal = appeal_service::get_appeal(db, appeal_id).await?;
    if let Some(existing_id) = sqlx::query_scalar::<_, Uuid>(
        r#"
        SELECT id
        FROM platform_jobs
        WHERE job_type = 'appeal_standardization'
          AND target_type = 'labor_appeal'
          AND target_id = $1
          AND status IN ('queued', 'running')
        ORDER BY created_at DESC
        LIMIT 1
        "#,
    )
    .bind(appeal_id)
    .fetch_optional(db)
    .await
    .map_err(|_| AppError::Internal)?
    {
        return Ok(existing_id);
    }

    if !matches!(
        appeal.status.as_str(),
        appeal_status::SUBMITTED
            | appeal_status::SUBMITTED_INCOMPLETE
            | appeal_status::UNDER_REVIEW
            | appeal_status::STANDARDIZING
    ) {
        return Err(AppError::Conflict(format!(
            "current status {} cannot start standardization",
            appeal.status
        )));
    }

    let now = Utc::now();
    let job_id = Uuid::new_v4();
    let mut tx = db.begin().await.map_err(|_| AppError::Internal)?;
    sqlx::query(
        r#"
        UPDATE labor_appeals
        SET status = 'standardizing',
            updated_at = $2
        WHERE id = $1
          AND status IN ('submitted', 'submitted_incomplete', 'under_review')
        "#,
    )
    .bind(appeal_id)
    .bind(now)
    .execute(&mut *tx)
    .await
    .map_err(|_| AppError::Internal)?;
    appeal_service::insert_event_tx(
        &mut tx,
        appeal_id,
        "standardization_started",
        "system",
        "appeal_standardization",
        "智能整理开始",
        "系统正在将口语化诉求整理为检察院可审核的标准材料。",
        true,
    )
    .await?;
    sqlx::query(
        r#"
        INSERT INTO platform_jobs (
            id, job_type, target_type, target_id, status, progress_percent, message,
            request_json, result_json, error_message, started_at, finished_at, created_at, updated_at
        )
        VALUES ($1, 'appeal_standardization', 'labor_appeal', $2, 'queued', 0, $3,
            $4, '{}', NULL, NULL, NULL, $5, $5)
        "#,
    )
    .bind(job_id)
    .bind(appeal_id)
    .bind("queued labor appeal standardization job")
    .bind(
        serde_json::json!({
            "appeal_id": appeal_id,
            "prompt_version": PROMPT_VERSION,
            "reason": reason
        })
        .to_string(),
    )
    .bind(now)
    .execute(&mut *tx)
    .await
    .map_err(|_| AppError::Internal)?;
    tx.commit().await.map_err(|_| AppError::Internal)?;
    Ok(job_id)
}

#[derive(Debug, Clone, Serialize, FromRow)]
pub struct AppealStandardizationRow {
    pub id: Uuid,
    pub appeal_id: Uuid,
    pub status: String,
    pub provider_style: String,
    pub model_name: String,
    pub prompt_version: String,
    pub input_digest: String,
    #[allow(dead_code)]
    #[serde(skip_serializing)]
    pub input_snapshot: serde_json::Value,
    pub output_json: serde_json::Value,
    pub standardized_title: String,
    pub standard_summary: String,
    pub standardized_text: String,
    pub extracted_fields: serde_json::Value,
    pub missing_materials: serde_json::Value,
    pub conflict_items: serde_json::Value,
    pub evidence_assessment: serde_json::Value,
    pub risk_case_mapping: serde_json::Value,
    pub confidence: Option<f64>,
    pub human_revision_json: serde_json::Value,
    pub reviewed_by: String,
    pub reviewed_at: Option<DateTime<Utc>>,
    pub error_message: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct ReviewStandardizationInput {
    pub human_revision_json: Option<serde_json::Value>,
    pub reviewed_by: Option<String>,
}

pub async fn standardize_appeal(
    state: &AppState,
    appeal_id: Uuid,
) -> Result<AppealStandardizationRow, AppError> {
    let appeal = appeal_service::get_appeal(state.db(), appeal_id).await?;
    let materials = appeal_service::list_materials(state.db(), appeal_id).await?;
    let location = appeal_service::maybe_location(state.db(), appeal_id).await?;
    let missing_materials = split_missing_materials(&appeal.missing_materials);
    let sanitized_oral_description = mask_phone_like_text(&appeal.oral_description);
    let structured_fields = serde_json::json!({
        "worker_name": appeal.worker_name,
        "worker_phone": mask_phone(&appeal.worker_phone),
        "wage_amount_text": appeal.wage_amount_text,
        "employer_name": appeal.employer_name,
        "contractor_name": appeal.contractor_name,
        "project_name": appeal.project_name,
        "work_period_text": appeal.work_period_text,
        "coworker_count": appeal.coworker_count,
        "demand_text": appeal.demand_text
    });
    let material_snapshot = serde_json::Value::Array(
        materials
            .iter()
            .map(|item| {
                serde_json::json!({
                    "id": item.id,
                    "category": item.category,
                    "description": item.description,
                    "original_filename": item.original_filename,
                    "file_size": item.file_size,
                    "mime_type": item.mime_type
                })
            })
            .collect(),
    );
    let location_snapshot = location
        .map(|item| {
            serde_json::json!({
                "latitude": item.latitude,
                "longitude": item.longitude,
                "address_text": item.address_text,
                "area_code": item.area_code,
                "area_name": item.area_name,
                "confirmed_by_applicant": item.confirmed_by_applicant
            })
        })
        .unwrap_or_else(|| serde_json::json!({}));
    let input_snapshot = serde_json::json!({
        "appeal_id": appeal.id,
        "appeal_code": appeal.appeal_code,
        "oral_description": sanitized_oral_description,
        "structured_fields": structured_fields,
        "materials": material_snapshot,
        "location": location_snapshot,
        "material_score": appeal.material_score,
        "missing_materials": missing_materials
    });
    let input_digest = digest_json(&input_snapshot);

    if let Some(existing) = latest_standardization(state.db(), appeal_id).await? {
        if existing.status == "completed" && existing.input_digest == input_digest {
            mark_standardization_done(state.db(), appeal_id).await?;
            return Ok(existing);
        }
    }

    let now = Utc::now();
    let id = Uuid::new_v4();
    let service = OpenAiCompatibleAiService::new_with_endpoint(
        state.http_client().clone(),
        state.settings().vllm.base_url.clone(),
        "/chat/completions",
        state.settings().vllm.model_name.clone(),
        Some(state.settings().vllm.api_key.clone()),
    );
    let output = service
        .standardize_appeal(&AppealStandardizationInput {
            oral_description: input_snapshot
                .get("oral_description")
                .and_then(serde_json::Value::as_str)
                .unwrap_or("")
                .to_string(),
            structured_fields: input_snapshot["structured_fields"].clone(),
            materials: input_snapshot["materials"].clone(),
            location: input_snapshot["location"].clone(),
            material_score: appeal.material_score,
            missing_materials,
        })
        .await;
    let contract = output
        .model_contract
        .clone()
        .unwrap_or_else(|| service.configured_contract());
    let output_json = serde_json::to_value(&output).map_err(|_| AppError::Internal)?;
    let standard_summary = output
        .standardized_text
        .lines()
        .next()
        .unwrap_or("")
        .to_string();

    let mut tx = state.db().begin().await.map_err(|_| AppError::Internal)?;
    sqlx::query(
        r#"
        INSERT INTO appeal_standardizations (
            id, appeal_id, status, provider_style, model_name, prompt_version, input_digest,
            input_snapshot, output_json, standardized_title, standard_summary, standardized_text,
            extracted_fields, missing_materials, conflict_items, evidence_assessment,
            risk_case_mapping, confidence, human_revision_json, reviewed_by, error_message,
            created_at, updated_at
        )
        VALUES (
            $1, $2, 'completed', $3, $4, $5, $6,
            $7, $8, $9, $10, $11,
            $12, $13, $14, $15, $16, $17, '{}'::jsonb, '', NULL, $18, $18
        )
        "#,
    )
    .bind(id)
    .bind(appeal_id)
    .bind(&contract.provider_style)
    .bind(&contract.model_name)
    .bind(PROMPT_VERSION)
    .bind(input_digest)
    .bind(input_snapshot)
    .bind(output_json)
    .bind(&output.standardized_title)
    .bind(standard_summary)
    .bind(&output.standardized_text)
    .bind(output.extracted_fields)
    .bind(output.missing_materials)
    .bind(output.conflict_items)
    .bind(output.evidence_assessment)
    .bind(output.risk_case_mapping)
    .bind(output.confidence)
    .bind(now)
    .execute(&mut *tx)
    .await
    .map_err(|_| AppError::Internal)?;

    sqlx::query(
        r#"
        INSERT INTO platform_jobs (
            id, job_type, target_type, target_id, status, progress_percent, message,
            request_json, result_json, started_at, finished_at, created_at, updated_at
        )
        VALUES ($1, 'appeal_standardization', 'labor_appeal', $2, 'completed', 100,
            'appeal standardization completed', $3, $4, $5, $5, $5, $5)
        "#,
    )
    .bind(Uuid::new_v4())
    .bind(appeal_id)
    .bind(serde_json::json!({ "prompt_version": PROMPT_VERSION }).to_string())
    .bind(serde_json::json!({ "standardization_id": id }).to_string())
    .bind(now)
    .execute(&mut *tx)
    .await
    .map_err(|_| AppError::Internal)?;

    appeal_service::insert_event_tx(
        &mut tx,
        appeal_id,
        "standardization_completed",
        "ai",
        &contract.model_name,
        "智能标准化完成",
        "系统已生成标准化摘要、缺失材料、证据强度和风险案件映射建议",
        false,
    )
    .await?;
    mark_standardization_done_tx(&mut tx, appeal_id, now).await?;

    tx.commit().await.map_err(|_| AppError::Internal)?;
    load_standardization(state.db(), id).await
}

async fn mark_standardization_done(db: &PgPool, appeal_id: Uuid) -> Result<(), AppError> {
    let mut tx = db.begin().await.map_err(|_| AppError::Internal)?;
    mark_standardization_done_tx(&mut tx, appeal_id, Utc::now()).await?;
    tx.commit().await.map_err(|_| AppError::Internal)
}

async fn mark_standardization_done_tx(
    tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    appeal_id: Uuid,
    now: DateTime<Utc>,
) -> Result<(), AppError> {
    sqlx::query(
        r#"
        UPDATE labor_appeals
        SET status = 'under_review',
            updated_at = $2
        WHERE id = $1
          AND status = 'standardizing'
        "#,
    )
    .bind(appeal_id)
    .bind(now)
    .execute(&mut **tx)
    .await
    .map_err(|_| AppError::Internal)?;
    Ok(())
}

pub async fn mark_standardization_failed(
    db: &PgPool,
    appeal_id: Uuid,
    error_message: &str,
) -> Result<(), AppError> {
    let now = Utc::now();
    let mut tx = db.begin().await.map_err(|_| AppError::Internal)?;
    sqlx::query(
        r#"
        INSERT INTO appeal_standardizations (
            id, appeal_id, status, provider_style, model_name, prompt_version, input_digest,
            input_snapshot, output_json, standardized_title, standard_summary, standardized_text,
            extracted_fields, missing_materials, conflict_items, evidence_assessment,
            risk_case_mapping, confidence, human_revision_json, reviewed_by, error_message,
            created_at, updated_at
        )
        VALUES (
            $1, $2, 'failed', 'openai-compatible', '', $3, '',
            '{}'::jsonb, '{}'::jsonb, '', '', '',
            '{}'::jsonb, '[]'::jsonb, '[]'::jsonb, '{}'::jsonb,
            '{}'::jsonb, NULL, '{}'::jsonb, '', $4, $5, $5
        )
        "#,
    )
    .bind(Uuid::new_v4())
    .bind(appeal_id)
    .bind(PROMPT_VERSION)
    .bind(error_message.chars().take(500).collect::<String>())
    .bind(now)
    .execute(&mut *tx)
    .await
    .map_err(|_| AppError::Internal)?;
    sqlx::query(
        r#"
        UPDATE labor_appeals
        SET status = 'submitted_incomplete',
            updated_at = $2
        WHERE id = $1
          AND status = 'standardizing'
        "#,
    )
    .bind(appeal_id)
    .bind(now)
    .execute(&mut *tx)
    .await
    .map_err(|_| AppError::Internal)?;
    appeal_service::insert_event_tx(
        &mut tx,
        appeal_id,
        "standardization_failed",
        "system",
        "appeal_standardization",
        "智能整理未完成",
        "系统暂未完成标准化整理，线索仍保留为已提交状态，工作人员可继续人工审核。",
        true,
    )
    .await?;
    tx.commit().await.map_err(|_| AppError::Internal)?;
    Ok(())
}

pub async fn list_standardizations(
    db: &PgPool,
    appeal_id: Uuid,
) -> Result<Vec<AppealStandardizationRow>, AppError> {
    sqlx::query_as::<_, AppealStandardizationRow>(
        "SELECT * FROM appeal_standardizations WHERE appeal_id = $1 ORDER BY created_at DESC",
    )
    .bind(appeal_id)
    .fetch_all(db)
    .await
    .map_err(|_| AppError::Internal)
}

pub async fn latest_standardization(
    db: &PgPool,
    appeal_id: Uuid,
) -> Result<Option<AppealStandardizationRow>, AppError> {
    sqlx::query_as::<_, AppealStandardizationRow>(
        "SELECT * FROM appeal_standardizations WHERE appeal_id = $1 ORDER BY created_at DESC LIMIT 1",
    )
    .bind(appeal_id)
    .fetch_optional(db)
    .await
    .map_err(|_| AppError::Internal)
}

pub async fn review_standardization(
    db: &PgPool,
    appeal_id: Uuid,
    standardization_id: Uuid,
    input: ReviewStandardizationInput,
) -> Result<AppealStandardizationRow, AppError> {
    let reviewed_by = input.reviewed_by.unwrap_or_else(|| "dev-staff".to_string());
    sqlx::query_as::<_, AppealStandardizationRow>(
        r#"
        UPDATE appeal_standardizations
        SET human_revision_json = $3,
            reviewed_by = $4,
            reviewed_at = $5,
            updated_at = $5
        WHERE id = $1 AND appeal_id = $2
        RETURNING *
        "#,
    )
    .bind(standardization_id)
    .bind(appeal_id)
    .bind(
        input
            .human_revision_json
            .unwrap_or_else(|| serde_json::json!({})),
    )
    .bind(reviewed_by)
    .bind(Utc::now())
    .fetch_optional(db)
    .await
    .map_err(|_| AppError::Internal)?
    .ok_or(AppError::NotFound)
}

async fn load_standardization(db: &PgPool, id: Uuid) -> Result<AppealStandardizationRow, AppError> {
    sqlx::query_as::<_, AppealStandardizationRow>(
        "SELECT * FROM appeal_standardizations WHERE id = $1",
    )
    .bind(id)
    .fetch_optional(db)
    .await
    .map_err(|_| AppError::Internal)?
    .ok_or(AppError::NotFound)
}

fn split_missing_materials(value: &str) -> Vec<String> {
    value
        .lines()
        .map(str::trim)
        .filter(|line| !line.is_empty())
        .map(str::to_string)
        .collect()
}

fn mask_phone_like_text(value: &str) -> String {
    let chars: Vec<char> = value.chars().collect();
    let mut output = String::new();
    let mut index = 0;
    while index < chars.len() {
        if chars[index].is_ascii_digit() {
            let start = index;
            while index < chars.len() && chars[index].is_ascii_digit() {
                index += 1;
            }
            if index - start == 11 {
                let token: String = chars[start..index].iter().collect();
                output.push_str(&mask_phone(&token));
            } else {
                for ch in &chars[start..index] {
                    output.push(*ch);
                }
            }
        } else {
            output.push(chars[index]);
            index += 1;
        }
    }
    output
}

fn digest_json(value: &serde_json::Value) -> String {
    let mut hasher = DefaultHasher::new();
    value.to_string().hash(&mut hasher);
    format!("{:016x}", hasher.finish())
}
