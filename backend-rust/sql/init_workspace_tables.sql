CREATE EXTENSION IF NOT EXISTS "pgcrypto";

CREATE TABLE IF NOT EXISTS imports (
    id UUID PRIMARY KEY,
    source_type TEXT NOT NULL,
    status TEXT NOT NULL,
    created_at TIMESTAMPTZ NOT NULL,
    updated_at TIMESTAMPTZ NOT NULL
);

CREATE TABLE IF NOT EXISTS import_files (
    id UUID PRIMARY KEY,
    import_id UUID NOT NULL REFERENCES imports(id),
    original_filename TEXT NOT NULL,
    stored_filename TEXT NOT NULL,
    stored_path TEXT NOT NULL,
    file_size BIGINT NOT NULL,
    mime_type TEXT,
    created_at TIMESTAMPTZ NOT NULL
);

CREATE INDEX IF NOT EXISTS idx_import_files_import_id
    ON import_files (import_id);

CREATE TABLE IF NOT EXISTS mapping_templates (
    id UUID PRIMARY KEY,
    template_key TEXT NOT NULL UNIQUE,
    template_label TEXT NOT NULL,
    version TEXT NOT NULL,
    status TEXT NOT NULL,
    source_type TEXT NOT NULL,
    created_at TIMESTAMPTZ NOT NULL,
    updated_at TIMESTAMPTZ NOT NULL
);

CREATE TABLE IF NOT EXISTS mapping_fields (
    id UUID PRIMARY KEY,
    template_id UUID NOT NULL REFERENCES mapping_templates(id) ON DELETE CASCADE,
    source_field TEXT NOT NULL,
    target_field TEXT NOT NULL,
    confidence DOUBLE PRECISION NOT NULL,
    status TEXT NOT NULL,
    sample_value TEXT NOT NULL,
    sort_order INTEGER NOT NULL DEFAULT 0,
    created_at TIMESTAMPTZ NOT NULL,
    updated_at TIMESTAMPTZ NOT NULL
);

CREATE INDEX IF NOT EXISTS idx_mapping_fields_template_id
    ON mapping_fields (template_id);

CREATE TABLE IF NOT EXISTS workflow_runs (
    id UUID PRIMARY KEY,
    stage_key TEXT NOT NULL,
    stage_label TEXT NOT NULL,
    status TEXT NOT NULL,
    started_at TIMESTAMPTZ NOT NULL,
    finished_at TIMESTAMPTZ,
    item_count INTEGER NOT NULL DEFAULT 0,
    success_count INTEGER NOT NULL DEFAULT 0,
    failure_count INTEGER NOT NULL DEFAULT 0,
    created_at TIMESTAMPTZ NOT NULL,
    updated_at TIMESTAMPTZ NOT NULL
);

CREATE INDEX IF NOT EXISTS idx_workflow_runs_stage_key
    ON workflow_runs (stage_key);

CREATE INDEX IF NOT EXISTS idx_workflow_runs_status
    ON workflow_runs (status);

CREATE TABLE IF NOT EXISTS risk_cases (
    id UUID PRIMARY KEY,
    import_id UUID REFERENCES imports(id) ON DELETE SET NULL,
    case_code TEXT NOT NULL UNIQUE,
    title TEXT NOT NULL,
    source_type TEXT NOT NULL,
    area_name TEXT NOT NULL,
    risk_level TEXT NOT NULL,
    risk_score DOUBLE PRECISION NOT NULL,
    status TEXT NOT NULL,
    alert_status TEXT NOT NULL,
    assignee TEXT,
    occurred_at TIMESTAMPTZ,
    due_at TIMESTAMPTZ,
    closed_at TIMESTAMPTZ,
    report_period TEXT,
    created_at TIMESTAMPTZ NOT NULL,
    updated_at TIMESTAMPTZ NOT NULL
);

CREATE INDEX IF NOT EXISTS idx_risk_cases_import_id
    ON risk_cases (import_id);

CREATE INDEX IF NOT EXISTS idx_risk_cases_status
    ON risk_cases (status);

CREATE INDEX IF NOT EXISTS idx_risk_cases_risk_level
    ON risk_cases (risk_level);

CREATE INDEX IF NOT EXISTS idx_risk_cases_alert_status
    ON risk_cases (alert_status);

CREATE INDEX IF NOT EXISTS idx_risk_cases_created_at
    ON risk_cases (created_at DESC);

CREATE TABLE IF NOT EXISTS knowledge_entities (
    id UUID PRIMARY KEY,
    case_id UUID REFERENCES risk_cases(id) ON DELETE SET NULL,
    entity_type TEXT NOT NULL,
    entity_name TEXT NOT NULL,
    confidence DOUBLE PRECISION NOT NULL,
    extracted_at TIMESTAMPTZ NOT NULL,
    created_at TIMESTAMPTZ NOT NULL
);

CREATE INDEX IF NOT EXISTS idx_knowledge_entities_case_id
    ON knowledge_entities (case_id);

CREATE INDEX IF NOT EXISTS idx_knowledge_entities_entity_type
    ON knowledge_entities (entity_type);

CREATE TABLE IF NOT EXISTS graph_relations (
    id UUID PRIMARY KEY,
    relation_type TEXT NOT NULL,
    source_entity_id UUID REFERENCES knowledge_entities(id) ON DELETE CASCADE,
    target_entity_id UUID REFERENCES knowledge_entities(id) ON DELETE CASCADE,
    created_at TIMESTAMPTZ NOT NULL
);

CREATE INDEX IF NOT EXISTS idx_graph_relations_relation_type
    ON graph_relations (relation_type);

CREATE TABLE IF NOT EXISTS generated_reports (
    id UUID PRIMARY KEY,
    title TEXT NOT NULL,
    report_type TEXT NOT NULL,
    period TEXT NOT NULL,
    status TEXT NOT NULL,
    file_path TEXT,
    generated_at TIMESTAMPTZ NOT NULL,
    created_at TIMESTAMPTZ NOT NULL
);

CREATE INDEX IF NOT EXISTS idx_generated_reports_type_period
    ON generated_reports (report_type, period);

CREATE TABLE IF NOT EXISTS extraction_runs (
    id UUID PRIMARY KEY,
    scope_type TEXT NOT NULL,
    mode TEXT NOT NULL,
    status TEXT NOT NULL,
    item_count INTEGER NOT NULL DEFAULT 0,
    success_count INTEGER NOT NULL DEFAULT 0,
    failure_count INTEGER NOT NULL DEFAULT 0,
    summary TEXT,
    started_at TIMESTAMPTZ NOT NULL,
    finished_at TIMESTAMPTZ,
    created_at TIMESTAMPTZ NOT NULL,
    updated_at TIMESTAMPTZ NOT NULL
);

CREATE INDEX IF NOT EXISTS idx_extraction_runs_status
    ON extraction_runs (status);

CREATE INDEX IF NOT EXISTS idx_extraction_runs_started_at
    ON extraction_runs (started_at DESC);

CREATE TABLE IF NOT EXISTS alerts (
    id UUID PRIMARY KEY,
    case_id UUID NOT NULL REFERENCES risk_cases(id) ON DELETE CASCADE,
    title TEXT NOT NULL,
    severity TEXT NOT NULL,
    status TEXT NOT NULL,
    summary TEXT NOT NULL,
    created_at TIMESTAMPTZ NOT NULL,
    updated_at TIMESTAMPTZ NOT NULL
);

CREATE INDEX IF NOT EXISTS idx_alerts_case_id
    ON alerts (case_id);

CREATE INDEX IF NOT EXISTS idx_alerts_status
    ON alerts (status);

CREATE TABLE IF NOT EXISTS dispatch_tasks (
    id UUID PRIMARY KEY,
    case_id UUID NOT NULL REFERENCES risk_cases(id) ON DELETE CASCADE,
    title TEXT NOT NULL,
    assignee TEXT NOT NULL,
    priority TEXT NOT NULL,
    status TEXT NOT NULL,
    progress_note TEXT,
    due_at TIMESTAMPTZ,
    completed_at TIMESTAMPTZ,
    created_at TIMESTAMPTZ NOT NULL,
    updated_at TIMESTAMPTZ NOT NULL
);

CREATE INDEX IF NOT EXISTS idx_dispatch_tasks_case_id
    ON dispatch_tasks (case_id);

CREATE INDEX IF NOT EXISTS idx_dispatch_tasks_status
    ON dispatch_tasks (status);

CREATE TABLE IF NOT EXISTS platform_settings (
    id UUID PRIMARY KEY,
    category TEXT NOT NULL,
    setting_key TEXT NOT NULL,
    setting_value TEXT NOT NULL,
    updated_at TIMESTAMPTZ NOT NULL,
    UNIQUE (category, setting_key)
);

CREATE INDEX IF NOT EXISTS idx_platform_settings_category
    ON platform_settings (category);

CREATE INDEX IF NOT EXISTS idx_platform_settings_key
    ON platform_settings (setting_key);

CREATE TABLE IF NOT EXISTS platform_jobs (
    id UUID PRIMARY KEY,
    job_type TEXT NOT NULL,
    target_type TEXT NOT NULL,
    target_id UUID,
    status TEXT NOT NULL,
    progress_percent INTEGER NOT NULL DEFAULT 0,
    message TEXT NOT NULL DEFAULT '',
    request_json TEXT NOT NULL DEFAULT '{}',
    result_json TEXT NOT NULL DEFAULT '{}',
    error_message TEXT,
    started_at TIMESTAMPTZ,
    finished_at TIMESTAMPTZ,
    created_at TIMESTAMPTZ NOT NULL,
    updated_at TIMESTAMPTZ NOT NULL
);

CREATE INDEX IF NOT EXISTS idx_platform_jobs_type_status
    ON platform_jobs (job_type, status);

CREATE INDEX IF NOT EXISTS idx_platform_jobs_target
    ON platform_jobs (target_type, target_id);

CREATE INDEX IF NOT EXISTS idx_platform_jobs_created_at
    ON platform_jobs (created_at DESC);

ALTER TABLE imports
    ADD COLUMN IF NOT EXISTS source_label TEXT,
    ADD COLUMN IF NOT EXISTS error_message TEXT,
    ADD COLUMN IF NOT EXISTS mapping_template_id UUID REFERENCES mapping_templates(id) ON DELETE SET NULL,
    ADD COLUMN IF NOT EXISTS total_record_count INTEGER NOT NULL DEFAULT 0,
    ADD COLUMN IF NOT EXISTS processed_record_count INTEGER NOT NULL DEFAULT 0,
    ADD COLUMN IF NOT EXISTS failed_record_count INTEGER NOT NULL DEFAULT 0,
    ADD COLUMN IF NOT EXISTS last_processed_at TIMESTAMPTZ;

ALTER TABLE generated_reports
    ADD COLUMN IF NOT EXISTS summary TEXT,
    ADD COLUMN IF NOT EXISTS provider_style TEXT NOT NULL DEFAULT 'openai_chat_completion_compatible',
    ADD COLUMN IF NOT EXISTS model_name TEXT;

ALTER TABLE risk_cases
    ADD COLUMN IF NOT EXISTS risk_reason_summary TEXT,
    ADD COLUMN IF NOT EXISTS disposal_advice TEXT,
    ADD COLUMN IF NOT EXISTS review_status TEXT NOT NULL DEFAULT 'pending',
    ADD COLUMN IF NOT EXISTS risk_tags TEXT NOT NULL DEFAULT '',
    ADD COLUMN IF NOT EXISTS graph_sync_status TEXT NOT NULL DEFAULT 'pending',
    ADD COLUMN IF NOT EXISTS graph_sync_message TEXT NOT NULL DEFAULT '',
    ADD COLUMN IF NOT EXISTS graph_synced_at TIMESTAMPTZ,
    ADD COLUMN IF NOT EXISTS vector_sync_status TEXT NOT NULL DEFAULT 'pending',
    ADD COLUMN IF NOT EXISTS vector_sync_message TEXT NOT NULL DEFAULT '',
    ADD COLUMN IF NOT EXISTS vector_synced_at TIMESTAMPTZ;

ALTER TABLE extraction_runs
    ADD COLUMN IF NOT EXISTS provider_style TEXT NOT NULL DEFAULT 'openai_chat_completion_compatible',
    ADD COLUMN IF NOT EXISTS model_name TEXT,
    ADD COLUMN IF NOT EXISTS graph_sync_status TEXT NOT NULL DEFAULT 'pending',
    ADD COLUMN IF NOT EXISTS graph_sync_message TEXT NOT NULL DEFAULT '',
    ADD COLUMN IF NOT EXISTS vector_sync_status TEXT NOT NULL DEFAULT 'pending',
    ADD COLUMN IF NOT EXISTS vector_sync_message TEXT NOT NULL DEFAULT '';

ALTER TABLE alerts
    ADD COLUMN IF NOT EXISTS handled_at TIMESTAMPTZ;

ALTER TABLE dispatch_tasks
    ADD COLUMN IF NOT EXISTS feedback_result TEXT;

ALTER TABLE graph_relations
    ADD COLUMN IF NOT EXISTS confidence DOUBLE PRECISION NOT NULL DEFAULT 1.0;

ALTER TABLE mapping_templates
    ADD COLUMN IF NOT EXISTS is_active BOOLEAN NOT NULL DEFAULT FALSE;

ALTER TABLE mapping_fields
    ADD COLUMN IF NOT EXISTS required BOOLEAN NOT NULL DEFAULT FALSE;

UPDATE imports SET
    source_label = COALESCE(source_label, source_type),
    error_message = COALESCE(error_message, ''),
    total_record_count = COALESCE(total_record_count, 0),
    processed_record_count = COALESCE(processed_record_count, 0),
    failed_record_count = COALESCE(failed_record_count, 0);

UPDATE generated_reports SET
    summary = COALESCE(summary, ''),
    provider_style = COALESCE(provider_style, 'openai_chat_completion_compatible'),
    model_name = COALESCE(model_name, '');
UPDATE risk_cases SET
    risk_reason_summary = COALESCE(risk_reason_summary, ''),
    disposal_advice = COALESCE(disposal_advice, ''),
    review_status = COALESCE(review_status, 'pending'),
    risk_tags = COALESCE(risk_tags, ''),
    graph_sync_status = COALESCE(graph_sync_status, 'pending'),
    graph_sync_message = COALESCE(graph_sync_message, ''),
    vector_sync_status = COALESCE(vector_sync_status, 'pending'),
    vector_sync_message = COALESCE(vector_sync_message, '');
UPDATE extraction_runs SET
    provider_style = COALESCE(provider_style, 'openai_chat_completion_compatible'),
    model_name = COALESCE(model_name, ''),
    graph_sync_status = COALESCE(graph_sync_status, 'pending'),
    graph_sync_message = COALESCE(graph_sync_message, ''),
    vector_sync_status = COALESCE(vector_sync_status, 'pending'),
    vector_sync_message = COALESCE(vector_sync_message, '');
UPDATE graph_relations SET confidence = COALESCE(confidence, 1.0);
UPDATE mapping_templates SET is_active = COALESCE(is_active, FALSE);
UPDATE mapping_fields SET required = COALESCE(required, FALSE);
UPDATE platform_jobs SET
    progress_percent = COALESCE(progress_percent, 0),
    message = COALESCE(message, ''),
    request_json = COALESCE(request_json, '{}'),
    result_json = COALESCE(result_json, '{}');
