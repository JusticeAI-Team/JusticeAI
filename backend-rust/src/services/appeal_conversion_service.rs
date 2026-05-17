use chrono::Utc;
use sqlx::{FromRow, PgPool};
use uuid::Uuid;

use crate::{
    services::appeal_service::{
        self, AppealRiskCaseLinkRow, ConvertRiskCaseInput, LinkRiskCaseInput,
    },
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
) -> Result<AppealRiskCaseLinkRow, AppError> {
    if let Some(existing) = sqlx::query_as::<_, ExistingLink>(
        "SELECT risk_case_id FROM appeal_risk_case_links WHERE appeal_id = $1 LIMIT 1",
    )
    .bind(appeal_id)
    .fetch_optional(db)
    .await
    .map_err(|_| AppError::Internal)?
    {
        return load_link(db, appeal_id, existing.risk_case_id).await;
    }

    let appeal = appeal_service::get_appeal(db, appeal_id).await?;
    let location = appeal_service::maybe_location(db, appeal_id).await?;
    let now = Utc::now();
    let risk_case_id = Uuid::new_v4();
    let risk_level = input.risk_level.unwrap_or_else(|| {
        if appeal.material_score >= 75 {
            "medium".to_string()
        } else {
            "low".to_string()
        }
    });
    let risk_tags = input
        .risk_tags
        .unwrap_or_else(|| vec!["欠薪".to_string(), "农民工".to_string(), "工程建设".to_string()])
        .join(",");
    let area_name = location
        .as_ref()
        .map(|item| item.area_name.clone())
        .filter(|value| !value.is_empty())
        .unwrap_or_else(|| "北京市XX区".to_string());
    let title = if appeal.project_name.trim().is_empty() {
        format!("{}欠薪诉求", area_name)
    } else {
        format!("{}欠薪诉求", appeal.project_name)
    };
    let summary = format!(
        "由移动端欠薪诉求 {} 转入。原始描述：{}",
        appeal.appeal_code, appeal.oral_description
    );

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
            $8, '建议核实用工主体、欠薪金额、同项目类似线索并依法处置。', 'pending', $9,
            'pending', '', 'pending', ''
        )
        "#,
    )
    .bind(risk_case_id)
    .bind(format!("APPEAL-{}", appeal.appeal_code))
    .bind(&title)
    .bind(area_name)
    .bind(risk_level)
    .bind(appeal.material_score as f64)
    .bind(now)
    .bind(summary)
    .bind(risk_tags)
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

    tx.commit().await.map_err(|_| AppError::Internal)?;
    load_link(db, appeal_id, risk_case_id).await
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
