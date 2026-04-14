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
