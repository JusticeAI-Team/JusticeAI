use chrono::{Duration, Utc};
use sqlx::{Executor, PgPool};
use uuid::Uuid;

pub async fn initialize_workspace_schema(db: &PgPool) -> Result<(), sqlx::Error> {
    db.execute(include_str!("../sql/init_workspace_tables.sql"))
        .await?;
    recover_interrupted_platform_jobs(db).await?;
    seed_workspace_data(db).await?;
    Ok(())
}

async fn recover_interrupted_platform_jobs(db: &PgPool) -> Result<(), sqlx::Error> {
    let now = Utc::now();
    sqlx::query(
        r#"
        UPDATE platform_jobs
        SET status = 'failed',
            progress_percent = CASE WHEN progress_percent >= 100 THEN 99 ELSE progress_percent END,
            message = 'backend restarted before this in-process job completed',
            error_message = COALESCE(error_message, 'job was interrupted by backend restart and must be retried'),
            finished_at = COALESCE(finished_at, $1),
            updated_at = $1
        WHERE status IN ('queued', 'running')
        "#,
    )
    .bind(now)
    .execute(db)
    .await?;
    Ok(())
}

async fn seed_workspace_data(db: &PgPool) -> Result<(), sqlx::Error> {
    seed_mapping_templates(db).await?;
    seed_workflow_runs(db).await?;
    seed_risk_cases(db).await?;
    seed_knowledge_entities(db).await?;
    seed_graph_relations(db).await?;
    seed_generated_reports(db).await?;
    seed_extraction_runs(db).await?;
    seed_alerts(db).await?;
    seed_dispatch_tasks(db).await?;
    seed_platform_settings(db).await?;
    Ok(())
}

async fn seed_mapping_templates(db: &PgPool) -> Result<(), sqlx::Error> {
    let count = scalar_count(db, "SELECT COUNT(*) FROM mapping_templates").await?;
    if count > 0 {
        return Ok(());
    }

    let now = Utc::now();
    let template_id = Uuid::new_v4();

    sqlx::query(
        r#"
        INSERT INTO mapping_templates (
            id, template_key, template_label, version, status, source_type, created_at, updated_at
        )
        VALUES ($1, $2, $3, $4, $5, $6, $7, $8)
        "#,
    )
    .bind(template_id)
    .bind("petition-risk-v1")
    .bind("基层治理风险映射模板")
    .bind("v1.0.0")
    .bind("draft")
    .bind("manual_upload")
    .bind(now)
    .bind(now)
    .execute(db)
    .await?;

    let fields = [
        (
            "诉求标题",
            "case_title",
            0.98_f64,
            "mapped",
            "某小区物业纠纷持续升级",
            1_i32,
        ),
        (
            "发生街道",
            "area_name",
            0.93_f64,
            "mapped",
            "广安门内街道",
            2_i32,
        ),
        (
            "责任部门",
            "department_name",
            0.87_f64,
            "needs_review",
            "某街道办",
            3_i32,
        ),
    ];

    for (source_field, target_field, confidence, status, sample_value, sort_order) in fields {
        sqlx::query(
            r#"
            INSERT INTO mapping_fields (
                id, template_id, source_field, target_field, confidence, status, sample_value,
                sort_order, created_at, updated_at
            )
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10)
            "#,
        )
        .bind(Uuid::new_v4())
        .bind(template_id)
        .bind(source_field)
        .bind(target_field)
        .bind(confidence)
        .bind(status)
        .bind(sample_value)
        .bind(sort_order)
        .bind(now)
        .bind(now)
        .execute(db)
        .await?;
    }

    Ok(())
}

async fn seed_workflow_runs(db: &PgPool) -> Result<(), sqlx::Error> {
    let count = scalar_count(db, "SELECT COUNT(*) FROM workflow_runs").await?;
    if count > 0 {
        return Ok(());
    }

    let now = Utc::now();
    let stages = [
        ("ingestion", "数据归集", "running", 5, 4, 1, 6),
        ("mapping", "字段映射", "running", 24, 18, 6, 5),
        ("extraction", "知识抽取", "completed", 12, 12, 0, 4),
        ("risk_analysis", "风险研判", "running", 128, 96, 32, 3),
        ("reports", "报告生成", "draft", 8, 6, 2, 2),
    ];

    for (stage_key, stage_label, status, item_count, success_count, failure_count, hours_ago) in
        stages
    {
        let started_at = now - Duration::hours(hours_ago);
        let finished_at = if status == "completed" {
            Some(now - Duration::hours(1))
        } else {
            None
        };

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
        .bind(stage_key)
        .bind(stage_label)
        .bind(status)
        .bind(started_at)
        .bind(finished_at)
        .bind(item_count)
        .bind(success_count)
        .bind(failure_count)
        .bind(started_at)
        .bind(now)
        .execute(db)
        .await?;
    }

    Ok(())
}

async fn seed_risk_cases(db: &PgPool) -> Result<(), sqlx::Error> {
    let count = scalar_count(db, "SELECT COUNT(*) FROM risk_cases").await?;
    if count > 0 {
        return Ok(());
    }

    let now = Utc::now();
    let rows = [
        (
            "JA-2026-0001",
            "物业纠纷聚集性升级",
            "hotline_12345",
            "广安门内街道",
            "high",
            92.4_f64,
            "open",
            "open",
            Some("张检察官"),
            Some(now - Duration::days(3)),
            Some(now + Duration::days(2)),
            None,
            Some("2026-W16"),
        ),
        (
            "JA-2026-0002",
            "重复信访跨部门推诿",
            "petitions",
            "金融街街道",
            "high",
            89.1_f64,
            "in_progress",
            "acknowledged",
            Some("李检察官"),
            Some(now - Duration::days(5)),
            Some(now + Duration::days(1)),
            None,
            Some("2026-W16"),
        ),
        (
            "JA-2026-0003",
            "校园周边治安隐患持续上升",
            "police_110",
            "什刹海街道",
            "medium",
            76.8_f64,
            "todo",
            "open",
            Some("王检察官"),
            Some(now - Duration::days(2)),
            Some(now + Duration::days(4)),
            None,
            Some("2026-W16"),
        ),
        (
            "JA-2026-0004",
            "历史纠纷主体再次出现",
            "knowledge_graph",
            "月坛街道",
            "medium",
            71.2_f64,
            "closed",
            "resolved",
            Some("赵检察官"),
            Some(now - Duration::days(12)),
            Some(now - Duration::days(6)),
            Some(now - Duration::days(5)),
            Some("2026-04"),
        ),
    ];

    for row in rows {
        sqlx::query(
            r#"
            INSERT INTO risk_cases (
                id, import_id, case_code, title, source_type, area_name, risk_level, risk_score,
                status, alert_status, assignee, occurred_at, due_at, closed_at,
                report_period, created_at, updated_at, risk_reason_summary, disposal_advice,
                review_status, risk_tags
            )
            VALUES (
                $1, NULL, $2, $3, $4, $5, $6, $7,
                $8, $9, $10, $11, $12, $13,
                $14, $15, $16, $17, $18, $19, $20
            )
            "#,
        )
        .bind(Uuid::new_v4())
        .bind(row.0)
        .bind(row.1)
        .bind(row.2)
        .bind(row.3)
        .bind(row.4)
        .bind(row.5)
        .bind(row.6)
        .bind(row.7)
        .bind(row.8)
        .bind(row.9)
        .bind(row.10)
        .bind(row.11)
        .bind(row.12)
        .bind(now - Duration::days(10))
        .bind(now)
        .bind(format!("Seeded summary for {}", row.1))
        .bind(
            "Verify recurrence, coordinate disposal, and keep weekly supervision updates."
                .to_string(),
        )
        .bind(if row.4 == "high" {
            "manual_review_required"
        } else {
            "pending"
        })
        .bind(format!("{},level:{}", row.2, row.4))
        .execute(db)
        .await?;
    }

    Ok(())
}

async fn seed_knowledge_entities(db: &PgPool) -> Result<(), sqlx::Error> {
    let count = scalar_count(db, "SELECT COUNT(*) FROM knowledge_entities").await?;
    if count > 0 {
        return Ok(());
    }

    let cases = sqlx::query_as::<_, (Uuid, String)>(
        "SELECT id, case_code FROM risk_cases ORDER BY created_at ASC",
    )
    .fetch_all(db)
    .await?;

    let now = Utc::now();

    for (case_id, case_code) in cases {
        let people_name = format!("{}-主体", case_code);
        let event_name = format!("{}-事件", case_code);

        sqlx::query(
            r#"
            INSERT INTO knowledge_entities (id, case_id, entity_type, entity_name, confidence, extracted_at, created_at)
            VALUES ($1, $2, $3, $4, $5, $6, $7), ($8, $2, $9, $10, $11, $12, $13)
            "#,
        )
        .bind(Uuid::new_v4())
        .bind(case_id)
        .bind("person")
        .bind(people_name)
        .bind(0.95_f64)
        .bind(now - Duration::hours(8))
        .bind(now - Duration::hours(8))
        .bind(Uuid::new_v4())
        .bind("event")
        .bind(event_name)
        .bind(0.91_f64)
        .bind(now - Duration::hours(7))
        .bind(now - Duration::hours(7))
        .execute(db)
        .await?;
    }

    Ok(())
}

async fn seed_graph_relations(db: &PgPool) -> Result<(), sqlx::Error> {
    let count = scalar_count(db, "SELECT COUNT(*) FROM graph_relations").await?;
    if count > 0 {
        return Ok(());
    }

    let grouped = sqlx::query_as::<_, (Option<Uuid>, Vec<Uuid>)>(
        r#"
        SELECT case_id, array_agg(id ORDER BY created_at ASC)
        FROM knowledge_entities
        GROUP BY case_id
        "#,
    )
    .fetch_all(db)
    .await?;

    let now = Utc::now();

    for (_case_id, entity_ids) in grouped {
        if entity_ids.len() < 2 {
            continue;
        }

        sqlx::query(
            r#"
            INSERT INTO graph_relations (id, relation_type, source_entity_id, target_entity_id, created_at)
            VALUES ($1, $2, $3, $4, $5)
            "#,
        )
        .bind(Uuid::new_v4())
        .bind("person_event")
        .bind(entity_ids[0])
        .bind(entity_ids[1])
        .bind(now - Duration::hours(6))
        .execute(db)
        .await?;
    }

    Ok(())
}

async fn seed_generated_reports(db: &PgPool) -> Result<(), sqlx::Error> {
    let count = scalar_count(db, "SELECT COUNT(*) FROM generated_reports").await?;
    if count > 0 {
        return Ok(());
    }

    let now = Utc::now();
    let reports = [
        (
            "西城区基层治理风险周报",
            "weekly_risk",
            "2026-W16",
            "ready",
            Some("runtime/reports/weekly-risk-w16.pdf"),
            "已生成周报摘要占位内容",
            now - Duration::hours(12),
        ),
        (
            "重点领域预警处置月报",
            "monthly_disposal",
            "2026-04",
            "draft",
            None,
            "待生成 AI 总结与处置分析",
            now - Duration::hours(4),
        ),
    ];

    for (title, report_type, period, status, file_path, summary, generated_at) in reports {
        sqlx::query(
            r#"
            INSERT INTO generated_reports (
                id, title, report_type, period, status, file_path, generated_at, created_at,
                summary, provider_style, model_name
            )
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11)
            "#,
        )
        .bind(Uuid::new_v4())
        .bind(title)
        .bind(report_type)
        .bind(period)
        .bind(status)
        .bind(file_path)
        .bind(generated_at)
        .bind(generated_at)
        .bind(summary)
        .bind("openai_chat_completion_compatible")
        .bind("placeholder-seeded-model")
        .execute(db)
        .await?;
    }

    Ok(())
}

async fn seed_extraction_runs(db: &PgPool) -> Result<(), sqlx::Error> {
    let count = scalar_count(db, "SELECT COUNT(*) FROM extraction_runs").await?;
    if count > 0 {
        return Ok(());
    }

    let now = Utc::now();
    let runs = [
        (
            "all_recent_cases",
            "incremental",
            "completed",
            12_i32,
            12_i32,
            0_i32,
            Some("最近案件已完成规则抽取，待接入模型增强"),
            now - Duration::hours(10),
            Some(now - Duration::hours(9)),
        ),
        (
            "selected_cases",
            "full",
            "failed",
            3_i32,
            2_i32,
            1_i32,
            Some("1 条案件文本缺失，需补录后重试"),
            now - Duration::hours(5),
            Some(now - Duration::hours(4)),
        ),
    ];

    for (
        scope_type,
        mode,
        status,
        item_count,
        success_count,
        failure_count,
        summary,
        started_at,
        finished_at,
    ) in runs
    {
        sqlx::query(
            r#"
            INSERT INTO extraction_runs (
                id, scope_type, mode, status, item_count, success_count, failure_count,
                summary, started_at, finished_at, created_at, updated_at, provider_style, model_name
            )
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $9, $9, $11, $12)
            "#,
        )
        .bind(Uuid::new_v4())
        .bind(scope_type)
        .bind(mode)
        .bind(status)
        .bind(item_count)
        .bind(success_count)
        .bind(failure_count)
        .bind(summary)
        .bind(started_at)
        .bind(finished_at)
        .bind("openai_chat_completion_compatible")
        .bind("placeholder-seeded-model")
        .execute(db)
        .await?;
    }

    Ok(())
}

async fn seed_alerts(db: &PgPool) -> Result<(), sqlx::Error> {
    let count = scalar_count(db, "SELECT COUNT(*) FROM alerts").await?;
    if count > 0 {
        return Ok(());
    }

    let cases = sqlx::query_as::<_, (Uuid, String, String)>(
        "SELECT id, title, risk_level FROM risk_cases ORDER BY created_at ASC LIMIT 3",
    )
    .fetch_all(db)
    .await?;

    let now = Utc::now();
    for (index, (case_id, title, risk_level)) in cases.into_iter().enumerate() {
        let severity = if risk_level == "high" {
            "high"
        } else {
            "medium"
        };
        let status = if index == 0 {
            "open"
        } else if index == 1 {
            "acknowledged"
        } else {
            "closed"
        };
        sqlx::query(
            r#"
            INSERT INTO alerts (
                id, case_id, title, severity, status, summary, created_at, updated_at, handled_at
            )
            VALUES (
                $1, $2, $3, $4, $5, $6, $7, $7,
                CASE WHEN $5 = 'closed' THEN $7 ELSE NULL END
            )
            "#,
        )
        .bind(Uuid::new_v4())
        .bind(case_id)
        .bind(format!("预警-{}", title))
        .bind(severity)
        .bind(status)
        .bind(format!(
            "案件『{}』触发{}级预警，当前为平台占位摘要。",
            title, severity
        ))
        .bind(now - Duration::hours((index as i64 + 1) * 2))
        .execute(db)
        .await?;
    }

    Ok(())
}

async fn seed_dispatch_tasks(db: &PgPool) -> Result<(), sqlx::Error> {
    let count = scalar_count(db, "SELECT COUNT(*) FROM dispatch_tasks").await?;
    if count > 0 {
        return Ok(());
    }

    let cases = sqlx::query_as::<_, (Uuid, String, Option<String>, Option<chrono::DateTime<Utc>>)>(
        "SELECT id, title, assignee, due_at FROM risk_cases ORDER BY created_at ASC LIMIT 3",
    )
    .fetch_all(db)
    .await?;

    let now = Utc::now();
    for (index, (case_id, title, assignee, due_at)) in cases.into_iter().enumerate() {
        let status = match index {
            0 => "assigned",
            1 => "in_progress",
            _ => "completed",
        };
        sqlx::query(
            r#"
            INSERT INTO dispatch_tasks (
                id, case_id, title, assignee, priority, status, progress_note, due_at,
                completed_at, created_at, updated_at, feedback_result
            )
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $10, $11)
            "#,
        )
        .bind(Uuid::new_v4())
        .bind(case_id)
        .bind(format!("处置任务-{}", title))
        .bind(assignee.unwrap_or_else(|| "待分派".to_string()))
        .bind(if index == 0 { "high" } else { "medium" })
        .bind(status)
        .bind(Some(format!(
            "任务状态为 {}，用于前端协同处置页占位展示",
            status
        )))
        .bind(due_at)
        .bind(if status == "completed" {
            Some(now - Duration::days(1))
        } else {
            None
        })
        .bind(now - Duration::hours((index as i64 + 1) * 3))
        .bind(if status == "completed" {
            Some("closed-loop feedback recorded".to_string())
        } else {
            None
        })
        .execute(db)
        .await?;
    }

    Ok(())
}

async fn seed_platform_settings(db: &PgPool) -> Result<(), sqlx::Error> {
    let count = scalar_count(db, "SELECT COUNT(*) FROM platform_settings").await?;
    if count > 0 {
        return Ok(());
    }

    let now = Utc::now();
    let settings = [
        ("platform", "platform_name", "JusticeAI"),
        ("platform", "environment", "development"),
        ("platform", "upload_dir", "runtime/uploads"),
        ("platform", "report_dir", "runtime/reports"),
        ("platform", "training_dir", "runtime/training"),
        (
            "integrations",
            "hugegraph_base_url",
            "http://localhost:8080",
        ),
        (
            "integrations",
            "hugegraph_gremlin_url",
            "ws://localhost:8182/gremlin",
        ),
        ("integrations", "milvus_address", "http://localhost:19530"),
        ("integrations", "model_base_url", "http://localhost:8000"),
        ("integrations", "model_name", "justiceai-placeholder"),
        (
            "integrations",
            "model_request_style",
            "openai_chat_completion_compatible",
        ),
        ("integrations", "model_chat_endpoint", "/chat/completions"),
        ("integrations", "model_json_mode_supported", "true"),
        ("integrations", "model_api_key_configured", "false"),
    ];

    for (category, key, value) in settings {
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
        .bind(now)
        .execute(db)
        .await?;
    }

    Ok(())
}

async fn scalar_count(db: &PgPool, sql: &str) -> Result<i64, sqlx::Error> {
    sqlx::query_scalar::<_, i64>(sql).fetch_one(db).await
}
