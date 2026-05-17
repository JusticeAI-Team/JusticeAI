use chrono::{DateTime, NaiveDate, Utc};
use serde::{Deserialize, Serialize};
use sqlx::{FromRow, PgPool, Postgres, Transaction};
use uuid::Uuid;

use crate::{
    domain::{
        appeal_score::{calculate_material_score, AppealScoreInput},
        appeal_status,
    },
    shared::error::AppError,
};

pub const DEV_APPLICANT_ID: Uuid = Uuid::from_u128(0x11111111111111111111111111111111);
const DEFAULT_ORAL_DESCRIPTION: &str =
    "申请人反映在北京市某项目务工期间存在欠薪问题，具体金额、用工主体和证据材料仍需进一步补充核实。";
const DEFAULT_PROJECT_NAME: &str = "北京市XX区XX地点附近项目";
const DEFAULT_EMPLOYER_NAME: &str = "待核实用工主体";
const DEFAULT_CONTRACTOR_NAME: &str = "待核实现场负责人";
const DEFAULT_DEMAND_TEXT: &str = "希望协助核实欠薪情况并依法处理。";
const DEFAULT_ADDRESS_TEXT: &str = "北京市XX区XX地点附近";
const DEFAULT_AREA_NAME: &str = "北京市XX区";

#[derive(Debug, Clone, Serialize, FromRow)]
pub struct ApplicantProfile {
    pub id: Uuid,
    pub display_name: String,
    pub real_name: String,
    pub phone: String,
    pub id_card_no: Option<String>,
    pub address_text: Option<String>,
    pub worker_type: String,
    pub auth_status: String,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, FromRow)]
pub struct LaborAppealRow {
    pub id: Uuid,
    pub applicant_id: Uuid,
    pub appeal_code: String,
    pub status: String,
    pub risk_case_status: String,
    pub oral_description: String,
    pub wage_amount_text: String,
    pub employer_name: String,
    pub contractor_name: String,
    pub project_name: String,
    pub work_period_text: String,
    pub coworker_count: Option<i32>,
    pub demand_text: String,
    pub worker_name: String,
    pub worker_phone: String,
    pub material_score: i32,
    pub missing_materials: String,
    pub submitted_at: Option<DateTime<Utc>>,
    pub accepted_at: Option<DateTime<Utc>>,
    pub resolved_at: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, FromRow)]
pub struct AppealMaterialRow {
    pub id: Uuid,
    pub appeal_id: Uuid,
    pub category: String,
    pub description: String,
    pub original_filename: String,
    #[allow(dead_code)]
    #[serde(skip_serializing)]
    pub stored_filename: String,
    pub file_size: i64,
    pub mime_type: Option<String>,
    #[allow(dead_code)]
    #[serde(skip_serializing)]
    pub deleted_at: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, FromRow)]
pub struct AppealLocationRow {
    pub id: Uuid,
    pub appeal_id: Uuid,
    pub latitude: f64,
    pub longitude: f64,
    pub address_text: String,
    pub area_code: String,
    pub area_name: String,
    pub confirmed_by_applicant: bool,
    pub geo_source: String,
    pub confidence: Option<f64>,
    pub conflict_flags: String,
    pub confirmed_by_staff: bool,
    pub staff_confirmed_by: String,
    pub staff_confirmed_at: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, FromRow)]
pub struct AppealEventRow {
    pub id: Uuid,
    pub appeal_id: Uuid,
    pub event_type: String,
    pub actor_type: String,
    pub actor_id: String,
    pub title: String,
    pub content: String,
    pub visible_to_applicant: bool,
    pub metadata_json: String,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, FromRow)]
pub struct AppealNotificationRow {
    pub id: Uuid,
    pub appeal_id: Uuid,
    pub applicant_id: Uuid,
    pub notification_type: String,
    pub title: String,
    pub content: String,
    pub read_at: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, FromRow)]
pub struct AppealRiskCaseLinkRow {
    pub id: Uuid,
    pub appeal_id: Uuid,
    pub risk_case_id: Uuid,
    pub link_type: String,
    pub reason: String,
    pub created_by: String,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct CreateDraftInput {
    pub client_request_id: Option<String>,
    pub oral_description: Option<String>,
    pub worker_name: Option<String>,
    pub worker_phone: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct SaveDraftInput {
    pub oral_description: Option<String>,
    pub wage_amount_text: Option<String>,
    pub employer_name: Option<String>,
    pub contractor_name: Option<String>,
    pub project_name: Option<String>,
    pub work_period_text: Option<String>,
    pub coworker_count: Option<i32>,
    pub demand_text: Option<String>,
    pub worker_name: Option<String>,
    pub worker_phone: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct SubmitInput {
    pub allow_incomplete: bool,
}

#[derive(Debug, Clone, Deserialize)]
pub struct SaveLocationInput {
    pub latitude: f64,
    pub longitude: f64,
    pub address_text: String,
    pub area_code: String,
    pub area_name: String,
    pub confirmed_by_applicant: bool,
}

#[derive(Debug, Clone, Deserialize)]
pub struct StaffConfirmLocationInput {
    pub area_code: Option<String>,
    pub area_name: Option<String>,
    pub address_text: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct RequestMaterialsInput {
    pub request_materials: Vec<String>,
    pub message: String,
    pub deadline: Option<NaiveDate>,
    pub internal_note: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct StatusActionInput {
    pub action: String,
    pub reason: Option<String>,
    pub notify_applicant: Option<bool>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct ConvertRiskCaseInput {
    pub risk_level: Option<String>,
    pub risk_tags: Option<Vec<String>>,
    pub create_alert: Option<bool>,
    pub create_dispatch_task: Option<bool>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct LinkRiskCaseInput {
    pub risk_case_id: Uuid,
    pub link_type: Option<String>,
    pub reason: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct ResolveInput {
    pub result_summary: String,
    pub notify_applicant: Option<bool>,
}

pub async fn ensure_dev_applicant(
    db: &PgPool,
    applicant_id: Uuid,
) -> Result<ApplicantProfile, AppError> {
    let now = Utc::now();
    sqlx::query(
        r#"
        INSERT INTO mobile_applicants (
            id, display_name, real_name, phone, id_card_no, address_text,
            worker_type, auth_status, created_at, updated_at
        )
        VALUES ($1, '张师傅', '张三', '13800001234', NULL, '北京市XX区XX地点', '建筑施工 / 临时用工', 'dev_verified', $2, $2)
        ON CONFLICT (id) DO NOTHING
        "#,
    )
    .bind(applicant_id)
    .bind(now)
    .execute(db)
    .await
    .map_err(|_| AppError::Internal)?;

    get_applicant(db, applicant_id).await
}

pub async fn get_applicant(db: &PgPool, applicant_id: Uuid) -> Result<ApplicantProfile, AppError> {
    sqlx::query_as::<_, ApplicantProfile>(
        r#"
        SELECT id, display_name, real_name, phone, id_card_no, address_text,
               worker_type, auth_status, created_at, updated_at
        FROM mobile_applicants
        WHERE id = $1
        "#,
    )
    .bind(applicant_id)
    .fetch_optional(db)
    .await
    .map_err(|_| AppError::Internal)?
    .ok_or(AppError::NotFound)
}

pub async fn create_draft(
    db: &PgPool,
    applicant_id: Uuid,
    input: CreateDraftInput,
) -> Result<LaborAppealRow, AppError> {
    ensure_dev_applicant(db, applicant_id).await?;
    let now = Utc::now();
    let id = Uuid::new_v4();
    let appeal_code = format!("BJ-XX-QX-{}-{}", now.format("%Y%m%d"), &id.to_string()[..8]);
    let worker_name = input.worker_name.unwrap_or_else(|| "张三".to_string());
    let worker_phone = input
        .worker_phone
        .unwrap_or_else(|| "13800001234".to_string());

    let mut tx = db.begin().await.map_err(|_| AppError::Internal)?;
    sqlx::query(
        r#"
        INSERT INTO labor_appeals (
            id, applicant_id, appeal_code, status, oral_description, worker_name,
            worker_phone, created_at, updated_at
        )
        VALUES ($1, $2, $3, $8, $4, $5, $6, $7, $7)
        "#,
    )
    .bind(id)
    .bind(applicant_id)
    .bind(&appeal_code)
    .bind(clean_display_text(
        &input.oral_description.unwrap_or_default(),
        DEFAULT_ORAL_DESCRIPTION,
    ))
    .bind(clean_display_text(&worker_name, "张三"))
    .bind(clean_display_text(&worker_phone, "13800001234"))
    .bind(now)
    .bind(appeal_status::DRAFT)
    .execute(&mut *tx)
    .await
    .map_err(|_| AppError::Internal)?;

    insert_event_tx(
        &mut tx,
        id,
        "draft_created",
        "applicant",
        &applicant_id.to_string(),
        "创建草稿",
        input
            .client_request_id
            .as_deref()
            .unwrap_or("移动端创建欠薪诉求草稿"),
        true,
    )
    .await?;
    tx.commit().await.map_err(|_| AppError::Internal)?;

    get_appeal(db, id).await
}

pub async fn save_draft(
    db: &PgPool,
    applicant_id: Option<Uuid>,
    appeal_id: Uuid,
    input: SaveDraftInput,
) -> Result<LaborAppealRow, AppError> {
    ensure_appeal_owner_if_needed(db, appeal_id, applicant_id).await?;
    let now = Utc::now();
    let mut tx = db.begin().await.map_err(|_| AppError::Internal)?;
    sqlx::query(
        r#"
        UPDATE labor_appeals
        SET oral_description = COALESCE($2, oral_description),
            wage_amount_text = COALESCE($3, wage_amount_text),
            employer_name = COALESCE($4, employer_name),
            contractor_name = COALESCE($5, contractor_name),
            project_name = COALESCE($6, project_name),
            work_period_text = COALESCE($7, work_period_text),
            coworker_count = COALESCE($8, coworker_count),
            demand_text = COALESCE($9, demand_text),
            worker_name = COALESCE($10, worker_name),
            worker_phone = COALESCE($11, worker_phone),
            updated_at = $12
        WHERE id = $1
        "#,
    )
    .bind(appeal_id)
    .bind(clean_optional_text(
        input.oral_description,
        DEFAULT_ORAL_DESCRIPTION,
    ))
    .bind(clean_optional_text(
        input.wage_amount_text,
        "待核实欠薪金额",
    ))
    .bind(clean_optional_text(
        input.employer_name,
        DEFAULT_EMPLOYER_NAME,
    ))
    .bind(clean_optional_text(
        input.contractor_name,
        DEFAULT_CONTRACTOR_NAME,
    ))
    .bind(clean_optional_text(
        input.project_name,
        DEFAULT_PROJECT_NAME,
    ))
    .bind(clean_optional_text(
        input.work_period_text,
        "待核实务工时间",
    ))
    .bind(input.coworker_count)
    .bind(clean_optional_text(input.demand_text, DEFAULT_DEMAND_TEXT))
    .bind(clean_optional_text(input.worker_name, "张三"))
    .bind(clean_optional_text(input.worker_phone, "13800001234"))
    .bind(now)
    .execute(&mut *tx)
    .await
    .map_err(|_| AppError::Internal)?;
    let actor_id = actor_id(applicant_id);
    insert_event_tx(
        &mut tx,
        appeal_id,
        "draft_saved",
        actor_type(applicant_id),
        &actor_id,
        "保存草稿",
        "已保存移动端补充信息",
        true,
    )
    .await?;
    tx.commit().await.map_err(|_| AppError::Internal)?;
    recompute_score(db, appeal_id).await?;
    get_appeal(db, appeal_id).await
}

pub async fn submit_appeal(
    db: &PgPool,
    applicant_id: Uuid,
    appeal_id: Uuid,
    input: SubmitInput,
) -> Result<LaborAppealRow, AppError> {
    ensure_appeal_owner_if_needed(db, appeal_id, Some(applicant_id)).await?;
    let score = recompute_score(db, appeal_id).await?;
    let next_status = if score.score >= 60 {
        appeal_status::SUBMITTED
    } else if input.allow_incomplete {
        appeal_status::SUBMITTED_INCOMPLETE
    } else {
        return Err(AppError::Validation(
            "材料完整度不足，需允许材料不完整提交或继续补充材料".to_string(),
        ));
    };
    let now = Utc::now();
    let mut tx = db.begin().await.map_err(|_| AppError::Internal)?;
    sqlx::query(
        r#"
        UPDATE labor_appeals
        SET status = $2, submitted_at = COALESCE(submitted_at, $3),
            material_score = $4, missing_materials = $5, updated_at = $3
        WHERE id = $1
        "#,
    )
    .bind(appeal_id)
    .bind(next_status)
    .bind(now)
    .bind(score.score)
    .bind(score.missing_materials.join("\n"))
    .execute(&mut *tx)
    .await
    .map_err(|_| AppError::Internal)?;
    insert_event_tx(
        &mut tx,
        appeal_id,
        "appeal_submitted",
        "applicant",
        &applicant_id.to_string(),
        "提交申诉",
        if next_status == appeal_status::SUBMITTED {
            "材料完整度达到提交标准"
        } else {
            "材料暂不完整，已按申请人确认先行提交"
        },
        true,
    )
    .await?;
    insert_notification_tx(
        &mut tx,
        appeal_id,
        applicant_id,
        "submitted",
        "线索已提交",
        "您的欠薪诉求已提交，后续办理进度会在消息中更新。",
    )
    .await?;
    tx.commit().await.map_err(|_| AppError::Internal)?;
    get_appeal(db, appeal_id).await
}

pub async fn save_location(
    db: &PgPool,
    applicant_id: Uuid,
    appeal_id: Uuid,
    input: SaveLocationInput,
) -> Result<AppealLocationRow, AppError> {
    ensure_appeal_owner_if_needed(db, appeal_id, Some(applicant_id)).await?;
    let now = Utc::now();
    let id = Uuid::new_v4();
    let validation = crate::services::geo::validate_beijing_location(
        &crate::services::geo::GeoValidationInput {
            latitude: input.latitude,
            longitude: input.longitude,
            address_text: clean_display_text(&input.address_text, DEFAULT_ADDRESS_TEXT),
            area_code: input.area_code.clone(),
            area_name: clean_display_text(&input.area_name, DEFAULT_AREA_NAME),
        },
    );
    let conflict_flags = crate::services::geo::conflict_flags_text(&validation.conflict_flags);
    let mut tx = db.begin().await.map_err(|_| AppError::Internal)?;
    sqlx::query(
        r#"
        INSERT INTO appeal_locations (
            id, appeal_id, latitude, longitude, address_text, area_code, area_name,
            confirmed_by_applicant, geo_source, confidence, conflict_flags, created_at, updated_at
        )
        VALUES ($1, $2, $3, $4, $5, $6, $7, $8, 'applicant_map', $9, $10, $11, $11)
        ON CONFLICT (appeal_id) DO UPDATE SET
            latitude = EXCLUDED.latitude,
            longitude = EXCLUDED.longitude,
            address_text = EXCLUDED.address_text,
            area_code = EXCLUDED.area_code,
            area_name = EXCLUDED.area_name,
            confirmed_by_applicant = EXCLUDED.confirmed_by_applicant,
            confidence = EXCLUDED.confidence,
            conflict_flags = EXCLUDED.conflict_flags,
            confirmed_by_staff = FALSE,
            staff_confirmed_by = '',
            staff_confirmed_at = NULL,
            updated_at = EXCLUDED.updated_at
        "#,
    )
    .bind(id)
    .bind(appeal_id)
    .bind(input.latitude)
    .bind(input.longitude)
    .bind(clean_display_text(
        &input.address_text,
        DEFAULT_ADDRESS_TEXT,
    ))
    .bind(input.area_code)
    .bind(clean_display_text(&input.area_name, DEFAULT_AREA_NAME))
    .bind(input.confirmed_by_applicant)
    .bind(validation.confidence)
    .bind(conflict_flags)
    .bind(now)
    .execute(&mut *tx)
    .await
    .map_err(|_| AppError::Internal)?;
    insert_event_tx(
        &mut tx,
        appeal_id,
        "location_saved",
        "applicant",
        &applicant_id.to_string(),
        "保存定位",
        "申请人确认了地图定位和行政区划信息",
        true,
    )
    .await?;
    tx.commit().await.map_err(|_| AppError::Internal)?;
    recompute_score(db, appeal_id).await?;
    get_location(db, appeal_id).await
}

pub async fn confirm_location_by_staff(
    db: &PgPool,
    appeal_id: Uuid,
    staff_id: &str,
    input: StaffConfirmLocationInput,
) -> Result<AppealLocationRow, AppError> {
    get_appeal(db, appeal_id).await?;
    let existing = get_location(db, appeal_id).await?;
    let area_code = input.area_code.unwrap_or(existing.area_code);
    let area_name = clean_display_text(
        &input.area_name.unwrap_or(existing.area_name),
        DEFAULT_AREA_NAME,
    );
    let address_text = clean_display_text(
        &input.address_text.unwrap_or(existing.address_text),
        DEFAULT_ADDRESS_TEXT,
    );
    let validation = crate::services::geo::validate_beijing_location(
        &crate::services::geo::GeoValidationInput {
            latitude: existing.latitude,
            longitude: existing.longitude,
            address_text: address_text.clone(),
            area_code: area_code.clone(),
            area_name: area_name.clone(),
        },
    );
    let now = Utc::now();
    let mut tx = db.begin().await.map_err(|_| AppError::Internal)?;
    sqlx::query(
        r#"
        UPDATE appeal_locations
        SET area_code = $2,
            area_name = $3,
            address_text = $4,
            confidence = $5,
            conflict_flags = $6,
            confirmed_by_staff = TRUE,
            staff_confirmed_by = $7,
            staff_confirmed_at = $8,
            updated_at = $8
        WHERE appeal_id = $1
        "#,
    )
    .bind(appeal_id)
    .bind(area_code)
    .bind(area_name)
    .bind(address_text)
    .bind(validation.confidence)
    .bind(crate::services::geo::conflict_flags_text(
        &validation.conflict_flags,
    ))
    .bind(staff_id)
    .bind(now)
    .execute(&mut *tx)
    .await
    .map_err(|_| AppError::Internal)?;
    insert_event_tx(
        &mut tx,
        appeal_id,
        "location_staff_confirmed",
        "staff",
        staff_id,
        "人工确认定位",
        "工作人员已确认定位点与行政区划",
        true,
    )
    .await?;
    tx.commit().await.map_err(|_| AppError::Internal)?;
    get_location(db, appeal_id).await
}

pub async fn list_mobile_appeals(
    db: &PgPool,
    applicant_id: Uuid,
) -> Result<Vec<LaborAppealRow>, AppError> {
    sqlx::query_as::<_, LaborAppealRow>(
        r#"
        SELECT * FROM labor_appeals
        WHERE applicant_id = $1
        ORDER BY updated_at DESC
        "#,
    )
    .bind(applicant_id)
    .fetch_all(db)
    .await
    .map_err(|_| AppError::Internal)
}

pub async fn get_appeal(db: &PgPool, appeal_id: Uuid) -> Result<LaborAppealRow, AppError> {
    sqlx::query_as::<_, LaborAppealRow>("SELECT * FROM labor_appeals WHERE id = $1")
        .bind(appeal_id)
        .fetch_optional(db)
        .await
        .map_err(|_| AppError::Internal)?
        .ok_or(AppError::NotFound)
}

pub async fn get_location(db: &PgPool, appeal_id: Uuid) -> Result<AppealLocationRow, AppError> {
    sqlx::query_as::<_, AppealLocationRow>("SELECT * FROM appeal_locations WHERE appeal_id = $1")
        .bind(appeal_id)
        .fetch_optional(db)
        .await
        .map_err(|_| AppError::Internal)?
        .ok_or(AppError::NotFound)
}

pub async fn maybe_location(
    db: &PgPool,
    appeal_id: Uuid,
) -> Result<Option<AppealLocationRow>, AppError> {
    sqlx::query_as::<_, AppealLocationRow>("SELECT * FROM appeal_locations WHERE appeal_id = $1")
        .bind(appeal_id)
        .fetch_optional(db)
        .await
        .map_err(|_| AppError::Internal)
}

pub async fn list_materials(
    db: &PgPool,
    appeal_id: Uuid,
) -> Result<Vec<AppealMaterialRow>, AppError> {
    sqlx::query_as::<_, AppealMaterialRow>(
        r#"
        SELECT id, appeal_id, category, description, original_filename, stored_filename,
               file_size, mime_type, deleted_at, created_at
        FROM appeal_materials
        WHERE appeal_id = $1 AND deleted_at IS NULL
        ORDER BY created_at DESC
        "#,
    )
    .bind(appeal_id)
    .fetch_all(db)
    .await
    .map_err(|_| AppError::Internal)
}

pub async fn list_events(
    db: &PgPool,
    appeal_id: Uuid,
    visible_only: bool,
) -> Result<Vec<AppealEventRow>, AppError> {
    sqlx::query_as::<_, AppealEventRow>(
        r#"
        SELECT * FROM appeal_events
        WHERE appeal_id = $1 AND ($2 = FALSE OR visible_to_applicant = TRUE)
        ORDER BY created_at ASC
        "#,
    )
    .bind(appeal_id)
    .bind(visible_only)
    .fetch_all(db)
    .await
    .map_err(|_| AppError::Internal)
}

pub async fn list_notifications(
    db: &PgPool,
    applicant_id: Uuid,
) -> Result<Vec<AppealNotificationRow>, AppError> {
    sqlx::query_as::<_, AppealNotificationRow>(
        r#"
        SELECT * FROM appeal_notifications
        WHERE applicant_id = $1
        ORDER BY created_at DESC
        "#,
    )
    .bind(applicant_id)
    .fetch_all(db)
    .await
    .map_err(|_| AppError::Internal)
}

pub async fn mark_notification_read(
    db: &PgPool,
    applicant_id: Uuid,
    notification_id: Uuid,
) -> Result<AppealNotificationRow, AppError> {
    let now = Utc::now();
    sqlx::query_as::<_, AppealNotificationRow>(
        r#"
        UPDATE appeal_notifications
        SET read_at = COALESCE(read_at, $3)
        WHERE id = $1 AND applicant_id = $2
        RETURNING *
        "#,
    )
    .bind(notification_id)
    .bind(applicant_id)
    .bind(now)
    .fetch_optional(db)
    .await
    .map_err(|_| AppError::Internal)?
    .ok_or(AppError::NotFound)
}

pub async fn supplement(
    db: &PgPool,
    applicant_id: Uuid,
    appeal_id: Uuid,
) -> Result<LaborAppealRow, AppError> {
    let appeal = get_appeal(db, appeal_id).await?;
    if appeal.applicant_id != applicant_id {
        return Err(AppError::Forbidden);
    }
    appeal_status::ensure_supplement_allowed(&appeal.status)?;
    let now = Utc::now();
    let mut tx = db.begin().await.map_err(|_| AppError::Internal)?;
    sqlx::query("UPDATE labor_appeals SET status = 'under_review', updated_at = $2 WHERE id = $1")
        .bind(appeal_id)
        .bind(now)
        .execute(&mut *tx)
        .await
        .map_err(|_| AppError::Internal)?;
    insert_event_tx(
        &mut tx,
        appeal_id,
        "supplement_submitted",
        "applicant",
        &applicant_id.to_string(),
        "补交材料",
        "申请人已补交材料并提交复核",
        true,
    )
    .await?;
    tx.commit().await.map_err(|_| AppError::Internal)?;
    recompute_score(db, appeal_id).await?;
    get_appeal(db, appeal_id).await
}

pub async fn recompute_score(
    db: &PgPool,
    appeal_id: Uuid,
) -> Result<crate::domain::appeal_score::AppealScore, AppError> {
    let appeal = get_appeal(db, appeal_id).await?;
    let location = maybe_location(db, appeal_id).await?;
    let categories = sqlx::query_scalar::<_, String>(
        "SELECT category FROM appeal_materials WHERE appeal_id = $1 AND deleted_at IS NULL",
    )
    .bind(appeal_id)
    .fetch_all(db)
    .await
    .map_err(|_| AppError::Internal)?;
    let score = calculate_material_score(&AppealScoreInput {
        oral_description: appeal.oral_description,
        worker_name: appeal.worker_name,
        worker_phone: appeal.worker_phone,
        project_name: appeal.project_name,
        employer_name: appeal.employer_name,
        contractor_name: appeal.contractor_name,
        wage_amount_text: appeal.wage_amount_text,
        area_name: location.map(|item| item.area_name),
        material_categories: categories,
    });
    sqlx::query(
        "UPDATE labor_appeals SET material_score = $2, missing_materials = $3, updated_at = $4 WHERE id = $1",
    )
    .bind(appeal_id)
    .bind(score.score)
    .bind(score.missing_materials.join("\n"))
    .bind(Utc::now())
    .execute(db)
    .await
    .map_err(|_| AppError::Internal)?;
    Ok(score)
}

pub async fn ensure_appeal_owner_if_needed(
    db: &PgPool,
    appeal_id: Uuid,
    applicant_id: Option<Uuid>,
) -> Result<(), AppError> {
    if let Some(applicant_id) = applicant_id {
        let owner =
            sqlx::query_scalar::<_, Uuid>("SELECT applicant_id FROM labor_appeals WHERE id = $1")
                .bind(appeal_id)
                .fetch_optional(db)
                .await
                .map_err(|_| AppError::Internal)?
                .ok_or(AppError::NotFound)?;
        if owner != applicant_id {
            return Err(AppError::Forbidden);
        }
    } else {
        get_appeal(db, appeal_id).await?;
    }
    Ok(())
}

pub async fn insert_event_tx(
    tx: &mut Transaction<'_, Postgres>,
    appeal_id: Uuid,
    event_type: &str,
    actor_type: &str,
    actor_id: &str,
    title: &str,
    content: &str,
    visible_to_applicant: bool,
) -> Result<(), AppError> {
    sqlx::query(
        r#"
        INSERT INTO appeal_events (
            id, appeal_id, event_type, actor_type, actor_id, title, content,
            visible_to_applicant, metadata_json, created_at
        )
        VALUES ($1, $2, $3, $4, $5, $6, $7, $8, '{}', $9)
        "#,
    )
    .bind(Uuid::new_v4())
    .bind(appeal_id)
    .bind(event_type)
    .bind(actor_type)
    .bind(actor_id)
    .bind(title)
    .bind(content)
    .bind(visible_to_applicant)
    .bind(Utc::now())
    .execute(&mut **tx)
    .await
    .map_err(|_| AppError::Internal)?;
    Ok(())
}

pub async fn insert_notification_tx(
    tx: &mut Transaction<'_, Postgres>,
    appeal_id: Uuid,
    applicant_id: Uuid,
    notification_type: &str,
    title: &str,
    content: &str,
) -> Result<(), AppError> {
    sqlx::query(
        r#"
        INSERT INTO appeal_notifications (
            id, appeal_id, applicant_id, notification_type, title, content, created_at
        )
        VALUES ($1, $2, $3, $4, $5, $6, $7)
        "#,
    )
    .bind(Uuid::new_v4())
    .bind(appeal_id)
    .bind(applicant_id)
    .bind(notification_type)
    .bind(title)
    .bind(content)
    .bind(Utc::now())
    .execute(&mut **tx)
    .await
    .map_err(|_| AppError::Internal)?;
    Ok(())
}

pub async fn insert_review_action_tx(
    tx: &mut Transaction<'_, Postgres>,
    appeal_id: Uuid,
    staff_id: &str,
    staff_role: &str,
    action_type: &str,
    from_status: &str,
    to_status: &str,
    request_materials: &str,
    message: &str,
    internal_note: &str,
    deadline: Option<NaiveDate>,
) -> Result<(), AppError> {
    sqlx::query(
        r#"
        INSERT INTO appeal_review_actions (
            id, appeal_id, staff_id, staff_role, action_type, from_status, to_status,
            request_materials, message, internal_note, deadline, created_at
        )
        VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12)
        "#,
    )
    .bind(Uuid::new_v4())
    .bind(appeal_id)
    .bind(staff_id)
    .bind(staff_role)
    .bind(action_type)
    .bind(from_status)
    .bind(to_status)
    .bind(request_materials)
    .bind(message)
    .bind(internal_note)
    .bind(deadline)
    .bind(Utc::now())
    .execute(&mut **tx)
    .await
    .map_err(|_| AppError::Internal)?;
    Ok(())
}

pub fn actor_type(applicant_id: Option<Uuid>) -> &'static str {
    if applicant_id.is_some() {
        "applicant"
    } else {
        "system"
    }
}

pub fn actor_id(applicant_id: Option<Uuid>) -> String {
    applicant_id.map(|id| id.to_string()).unwrap_or_default()
}

pub fn clean_display_text(value: &str, fallback: &str) -> String {
    let trimmed = value.trim();
    if trimmed.is_empty() || looks_like_corrupted_text(trimmed) {
        fallback.to_string()
    } else {
        trimmed.to_string()
    }
}

fn clean_optional_text(value: Option<String>, fallback: &str) -> Option<String> {
    value.map(|text| clean_display_text(&text, fallback))
}

fn looks_like_corrupted_text(value: &str) -> bool {
    if value.contains("???") || value.contains("�") {
        return true;
    }

    let total = value.chars().count();
    if total == 0 {
        return false;
    }
    let question_count = value.chars().filter(|ch| *ch == '?').count();
    question_count >= 2 && question_count * 5 >= total
}

pub fn mask_phone(phone: &str) -> String {
    if phone.len() >= 11 {
        format!("{}****{}", &phone[..3], &phone[7..])
    } else {
        phone.to_string()
    }
}

#[cfg(test)]
mod text_sanitizer_tests {
    use super::*;

    #[test]
    fn clean_display_text_keeps_normal_question_sentence() {
        assert_eq!(
            clean_display_text("老板是谁? 我只知道姓王", "fallback"),
            "老板是谁? 我只知道姓王"
        );
    }

    #[test]
    fn clean_display_text_replaces_replacement_question_runs() {
        assert_eq!(
            clean_display_text("???XX?XX??????欠薪诉求", "北京市XX区欠薪诉求"),
            "北京市XX区欠薪诉求"
        );
    }
}

pub fn mask_name(name: &str) -> String {
    let mut chars = name.chars();
    match chars.next() {
        Some(first) => format!("{first}*"),
        None => String::new(),
    }
}
