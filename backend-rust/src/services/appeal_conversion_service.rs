use std::collections::HashMap;

use chrono::Utc;
use serde::Serialize;
use sqlx::{FromRow, PgPool, Postgres, Transaction};
use uuid::Uuid;

use crate::{
    services::appeal_service::{
        self, clean_display_text, AppealRiskCaseLinkRow, ConvertRiskCaseInput, LinkRiskCaseInput,
    },
    services::appeal_standardization_service,
    shared::error::AppError,
};

#[derive(Debug, FromRow)]
struct ExistingLink {
    risk_case_id: Uuid,
}

pub async fn convert_to_risk_case(
    db: &PgPool,
    appeal_id: Uuid,
    staff_id: &str,
    input: ConvertRiskCaseInput,
) -> Result<ConversionResult, AppError> {
    if let Some(existing) = sqlx::query_as::<_, ExistingLink>(
        "SELECT risk_case_id FROM appeal_risk_case_links WHERE appeal_id = $1 LIMIT 1",
    )
    .bind(appeal_id)
    .fetch_optional(db)
    .await
    .map_err(|_| AppError::Internal)?
    {
        let link = load_link(db, appeal_id, existing.risk_case_id).await?;
        return Ok(ConversionResult {
            link,
            triggered_jobs: Vec::new(),
        });
    }

    let appeal = appeal_service::get_appeal(db, appeal_id).await?;
    let location = appeal_service::maybe_location(db, appeal_id).await?;
    let now = Utc::now();
    let risk_case_id = Uuid::new_v4();
    let standardization =
        appeal_standardization_service::preferred_standardization(db, appeal_id).await?;
    let mapping = standardization
        .as_ref()
        .map(|item| item.risk_case_mapping.clone())
        .unwrap_or_else(|| serde_json::json!({}));
    let risk_level = input.risk_level.unwrap_or_else(|| {
        mapping
            .get("risk_level")
            .and_then(serde_json::Value::as_str)
            .map(str::to_string)
            .unwrap_or_else(|| {
                if appeal.material_score >= 75 {
                    "medium".to_string()
                } else {
                    "low".to_string()
                }
            })
    });
    let risk_tags = input
        .risk_tags
        .unwrap_or_else(|| {
            mapping
                .get("risk_tags")
                .and_then(serde_json::Value::as_array)
                .map(|items| {
                    items
                        .iter()
                        .filter_map(serde_json::Value::as_str)
                        .map(str::to_string)
                        .collect::<Vec<_>>()
                })
                .filter(|items| !items.is_empty())
                .unwrap_or_else(|| {
                    vec![
                        "欠薪".to_string(),
                        "农民工".to_string(),
                        "工程建设".to_string(),
                    ]
                })
        })
        .join(",");
    let area_name = location
        .as_ref()
        .map(|item| item.area_name.clone())
        .filter(|value| !value.is_empty())
        .or_else(|| {
            mapping
                .get("area_name")
                .and_then(serde_json::Value::as_str)
                .map(str::to_string)
        })
        .unwrap_or_else(|| "北京市XX区".to_string());
    let area_name = clean_display_text(&area_name, "北京市XX区");
    let title = mapping
        .get("title")
        .and_then(serde_json::Value::as_str)
        .map(str::to_string)
        .filter(|value| !value.trim().is_empty())
        .unwrap_or_else(|| {
            if appeal.project_name.trim().is_empty() {
                format!("{}欠薪诉求", area_name)
            } else {
                format!(
                    "{}欠薪诉求",
                    clean_display_text(&appeal.project_name, "北京市XX区XX地点附近项目")
                )
            }
        });
    let title = clean_display_text(&title, "北京市XX区欠薪诉求");
    let summary = mapping
        .get("risk_reason_summary")
        .and_then(serde_json::Value::as_str)
        .map(str::to_string)
        .filter(|value| !value.trim().is_empty())
        .unwrap_or_else(|| {
            format!(
                "由移动端欠薪诉求 {} 转入。原始描述：{}",
                appeal.appeal_code,
                clean_display_text(
                    &appeal.oral_description,
                    "申请人反映存在欠薪问题，需进一步核实。"
                )
            )
        });
    let summary = clean_display_text(&summary, "移动端欠薪诉求转入风险案件，需进一步核实。");
    let disposal_advice = mapping
        .get("disposal_advice")
        .and_then(serde_json::Value::as_str)
        .unwrap_or("建议核实用工主体、欠薪金额、同项目类似线索并依法处置。")
        .to_string();

    let mut tx = db.begin().await.map_err(|_| AppError::Internal)?;
    sqlx::query(
        r#"
        INSERT INTO risk_cases (
            id, import_id, case_code, title, source_type, area_name, risk_level,
            risk_score, status, alert_status, assignee, occurred_at, due_at,
            closed_at, report_period, created_at, updated_at,
            risk_reason_summary, disposal_advice, review_status, risk_tags,
            graph_sync_status, graph_sync_message, vector_sync_status, vector_sync_message
        )
        VALUES (
            $1, NULL, $2, $3, 'mobile_labor_appeal', $4, $5,
            $6, 'pending_review', 'open', NULL, NULL, NULL,
            NULL, NULL, $7, $7,
            $8, $9, 'pending', $10,
            'pending', '', 'pending', ''
        )
        "#,
    )
    .bind(risk_case_id)
    .bind(format!("APPEAL-{}", appeal.appeal_code))
    .bind(&title)
    .bind(&area_name)
    .bind(&risk_level)
    .bind(appeal.material_score as f64)
    .bind(now)
    .bind(&summary)
    .bind(&disposal_advice)
    .bind(&risk_tags)
    .execute(&mut *tx)
    .await
    .map_err(|_| AppError::Internal)?;

    let link_id = Uuid::new_v4();
    sqlx::query(
        r#"
        INSERT INTO appeal_risk_case_links (
            id, appeal_id, risk_case_id, link_type, reason, created_by, created_at
        )
        VALUES ($1, $2, $3, 'converted', '检察院端将移动端线索转为风险案件', $4, $5)
        "#,
    )
    .bind(link_id)
    .bind(appeal_id)
    .bind(risk_case_id)
    .bind(staff_id)
    .bind(now)
    .execute(&mut *tx)
    .await
    .map_err(|_| AppError::Internal)?;

    sqlx::query(
        "UPDATE labor_appeals SET risk_case_status = 'converted', updated_at = $2 WHERE id = $1",
    )
    .bind(appeal_id)
    .bind(now)
    .execute(&mut *tx)
    .await
    .map_err(|_| AppError::Internal)?;

    appeal_service::insert_event_tx(
        &mut tx,
        appeal_id,
        "risk_case_converted",
        "staff",
        staff_id,
        "转为风险案件",
        "线索已纳入现有风险案件研判链路",
        true,
    )
    .await?;

    create_initial_case_graph_tx(
        &mut tx,
        risk_case_id,
        &title,
        &appeal.worker_name,
        &appeal.project_name,
        &appeal.employer_name,
        &appeal.contractor_name,
        &area_name,
        &risk_tags,
        now,
    )
    .await?;

    if input.create_alert.unwrap_or(false) {
        sqlx::query(
            r#"
            INSERT INTO alerts (id, case_id, title, severity, status, summary, created_at, updated_at)
            VALUES ($1, $2, $3, $4, 'open', $5, $6, $6)
            "#,
        )
        .bind(Uuid::new_v4())
        .bind(risk_case_id)
        .bind("农民工欠薪线索预警")
        .bind("medium")
        .bind("移动端欠薪诉求已转入风险案件，请及时核实。")
        .bind(now)
        .execute(&mut *tx)
        .await
        .map_err(|_| AppError::Internal)?;
    }

    if input.create_dispatch_task.unwrap_or(false) {
        sqlx::query(
            r#"
            INSERT INTO dispatch_tasks (id, case_id, title, assignee, priority, status, progress_note, created_at, updated_at)
            VALUES ($1, $2, '核实农民工欠薪线索', '承办检察官', 'medium', 'todo', '由移动端申诉转换生成', $3, $3)
            "#,
        )
        .bind(Uuid::new_v4())
        .bind(risk_case_id)
        .bind(now)
        .execute(&mut *tx)
        .await
        .map_err(|_| AppError::Internal)?;
    }

    let triggered_jobs =
        create_downstream_jobs_tx(&mut tx, appeal_id, risk_case_id, staff_id, now).await?;

    tx.commit().await.map_err(|_| AppError::Internal)?;
    Ok(ConversionResult {
        link: load_link(db, appeal_id, risk_case_id).await?,
        triggered_jobs,
    })
}

#[derive(Debug, Serialize)]
pub struct ConversionResult {
    pub link: AppealRiskCaseLinkRow,
    pub triggered_jobs: Vec<TriggeredJob>,
}

#[derive(Debug, Serialize)]
pub struct TriggeredJob {
    pub id: Uuid,
    pub job_type: String,
    pub target_type: String,
    pub target_id: Uuid,
}

pub async fn link_existing_risk_case(
    db: &PgPool,
    appeal_id: Uuid,
    staff_id: &str,
    input: LinkRiskCaseInput,
) -> Result<AppealRiskCaseLinkRow, AppError> {
    let risk_exists = sqlx::query_scalar::<_, i64>("SELECT COUNT(*) FROM risk_cases WHERE id = $1")
        .bind(input.risk_case_id)
        .fetch_one(db)
        .await
        .map_err(|_| AppError::Internal)?;
    if risk_exists == 0 {
        return Err(AppError::NotFound);
    }

    let now = Utc::now();
    let mut tx = db.begin().await.map_err(|_| AppError::Internal)?;
    sqlx::query(
        r#"
        INSERT INTO appeal_risk_case_links (
            id, appeal_id, risk_case_id, link_type, reason, created_by, created_at
        )
        VALUES ($1, $2, $3, $4, $5, $6, $7)
        ON CONFLICT (appeal_id, risk_case_id) DO NOTHING
        "#,
    )
    .bind(Uuid::new_v4())
    .bind(appeal_id)
    .bind(input.risk_case_id)
    .bind(input.link_type.unwrap_or_else(|| "linked".to_string()))
    .bind(input.reason.unwrap_or_default())
    .bind(staff_id)
    .bind(now)
    .execute(&mut *tx)
    .await
    .map_err(|_| AppError::Internal)?;
    sqlx::query(
        "UPDATE labor_appeals SET risk_case_status = 'linked', updated_at = $2 WHERE id = $1",
    )
    .bind(appeal_id)
    .bind(now)
    .execute(&mut *tx)
    .await
    .map_err(|_| AppError::Internal)?;
    appeal_service::insert_event_tx(
        &mut tx,
        appeal_id,
        "risk_case_linked",
        "staff",
        staff_id,
        "关联风险案件",
        "线索已关联到既有风险案件",
        true,
    )
    .await?;
    tx.commit().await.map_err(|_| AppError::Internal)?;
    load_link(db, appeal_id, input.risk_case_id).await
}

async fn create_initial_case_graph_tx(
    tx: &mut Transaction<'_, Postgres>,
    risk_case_id: Uuid,
    title: &str,
    worker_name: &str,
    project_name: &str,
    employer_name: &str,
    contractor_name: &str,
    area_name: &str,
    risk_tags: &str,
    now: chrono::DateTime<Utc>,
) -> Result<(), AppError> {
    let mut entity_specs = vec![
        ("case", title, 0.95),
        ("person", worker_name, 0.9),
        ("project", project_name, 0.86),
        ("organization", employer_name, 0.82),
        ("person", contractor_name, 0.78),
        ("location", area_name, 0.9),
    ];
    for tag in risk_tags
        .split([',', '，', '、'])
        .map(str::trim)
        .filter(|value| !value.is_empty())
    {
        entity_specs.push(("risk_tag", tag, 0.72));
    }

    let mut ids = HashMap::new();
    for (entity_type, entity_name, confidence) in entity_specs {
        let normalized = entity_name.trim();
        if normalized.is_empty() {
            continue;
        }
        let key = format!("{entity_type}:{normalized}");
        if ids.contains_key(&key) {
            continue;
        }
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
        .bind(risk_case_id)
        .bind(entity_type)
        .bind(normalized)
        .bind(confidence)
        .bind(now)
        .execute(&mut **tx)
        .await
        .map_err(|_| AppError::Internal)?;
        ids.insert(key, entity_id);
    }

    let case_id = entity_id(&ids, "case", title);
    let worker_id = entity_id(&ids, "person", worker_name);
    let project_id = entity_id(&ids, "project", project_name);
    let employer_id = entity_id(&ids, "organization", employer_name);
    let contractor_id = entity_id(&ids, "person", contractor_name);
    let area_id = entity_id(&ids, "location", area_name);

    insert_relation_if_present(tx, case_id, worker_id, "appeal_worker", 0.9, now).await?;
    insert_relation_if_present(tx, case_id, project_id, "appeal_project", 0.88, now).await?;
    insert_relation_if_present(tx, case_id, employer_id, "appeal_employer", 0.82, now).await?;
    insert_relation_if_present(tx, case_id, contractor_id, "appeal_contractor", 0.78, now).await?;
    insert_relation_if_present(tx, case_id, area_id, "appeal_area", 0.9, now).await?;
    insert_relation_if_present(tx, worker_id, project_id, "worked_at", 0.86, now).await?;
    insert_relation_if_present(tx, employer_id, project_id, "employing_entity", 0.8, now).await?;
    insert_relation_if_present(tx, contractor_id, project_id, "project_manager", 0.76, now).await?;
    insert_relation_if_present(tx, project_id, area_id, "located_in", 0.86, now).await?;

    sqlx::query(
        r#"
        UPDATE risk_cases
        SET graph_sync_status = 'synced',
            graph_sync_message = $2,
            graph_synced_at = $3,
            updated_at = $3
        WHERE id = $1
        "#,
    )
    .bind(risk_case_id)
    .bind("generated initial graph from labor appeal fields")
    .bind(now)
    .execute(&mut **tx)
    .await
    .map_err(|_| AppError::Internal)?;

    Ok(())
}

fn entity_id(ids: &HashMap<String, Uuid>, entity_type: &str, entity_name: &str) -> Option<Uuid> {
    ids.get(&format!("{entity_type}:{}", entity_name.trim()))
        .copied()
}

async fn insert_relation_if_present(
    tx: &mut Transaction<'_, Postgres>,
    source: Option<Uuid>,
    target: Option<Uuid>,
    relation_type: &str,
    confidence: f64,
    now: chrono::DateTime<Utc>,
) -> Result<(), AppError> {
    let (Some(source), Some(target)) = (source, target) else {
        return Ok(());
    };
    if source == target {
        return Ok(());
    }
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
    .bind(now)
    .execute(&mut **tx)
    .await
    .map_err(|_| AppError::Internal)?;
    Ok(())
}

async fn create_downstream_jobs_tx(
    tx: &mut Transaction<'_, Postgres>,
    appeal_id: Uuid,
    risk_case_id: Uuid,
    staff_id: &str,
    now: chrono::DateTime<Utc>,
) -> Result<Vec<TriggeredJob>, AppError> {
    let specs = [
        ("appeal_risk_case_agent_analyze", "risk_case"),
        ("appeal_risk_case_graph_rebuild", "risk_case"),
        ("appeal_risk_case_vector_rebuild", "risk_case"),
        ("appeal_similar_case_discovery", "labor_appeal"),
    ];
    let mut jobs = Vec::new();
    for (job_type, target_type) in specs {
        let job_id = Uuid::new_v4();
        let target_id = if target_type == "labor_appeal" {
            appeal_id
        } else {
            risk_case_id
        };
        sqlx::query(
            r#"
            INSERT INTO platform_jobs (
                id, job_type, target_type, target_id, status, progress_percent, message,
                request_json, result_json, created_at, updated_at
            )
            VALUES ($1, $2, $3, $4, 'queued', 0, $5, $6, '{}', $7, $7)
            "#,
        )
        .bind(job_id)
        .bind(job_type)
        .bind(target_type)
        .bind(target_id)
        .bind("created after labor appeal converted to risk_case")
        .bind(
            serde_json::json!({
                "appeal_id": appeal_id,
                "risk_case_id": risk_case_id,
                "created_by": staff_id
            })
            .to_string(),
        )
        .bind(now)
        .execute(&mut **tx)
        .await
        .map_err(|_| AppError::Internal)?;
        jobs.push(TriggeredJob {
            id: job_id,
            job_type: job_type.to_string(),
            target_type: target_type.to_string(),
            target_id,
        });
    }
    Ok(jobs)
}

async fn load_link(
    db: &PgPool,
    appeal_id: Uuid,
    risk_case_id: Uuid,
) -> Result<AppealRiskCaseLinkRow, AppError> {
    sqlx::query_as::<_, AppealRiskCaseLinkRow>(
        "SELECT * FROM appeal_risk_case_links WHERE appeal_id = $1 AND risk_case_id = $2",
    )
    .bind(appeal_id)
    .bind(risk_case_id)
    .fetch_optional(db)
    .await
    .map_err(|_| AppError::Internal)?
    .ok_or(AppError::NotFound)
}
