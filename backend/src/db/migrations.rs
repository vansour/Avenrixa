use std::borrow::Cow;

use once_cell::sync::Lazy;
use sqlx::migrate::Migrator;
use sqlx::migrate::{Migration, MigrationType};

fn migration(version: i64, description: &'static str, sql: &'static str) -> Migration {
    Migration::new(
        version,
        Cow::Borrowed(description),
        MigrationType::Simple,
        Cow::Borrowed(sql),
        false,
    )
}

pub(super) fn postgres_migrator() -> &'static Migrator {
    &POSTGRES_MIGRATOR
}

static POSTGRES_MIGRATOR: Lazy<Migrator> = Lazy::new(|| Migrator {
    migrations: Cow::Owned(vec![
        migration(1, "initial schema", POSTGRES_0001_INITIAL_SCHEMA_SQL),
        migration(
            2,
            "drop unique filename index",
            POSTGRES_0002_DROP_UNIQUE_FILENAME_INDEX_SQL,
        ),
        migration(
            3,
            "add auth runtime state",
            POSTGRES_0003_ADD_AUTH_RUNTIME_STATE_SQL,
        ),
        migration(
            4,
            "remove unused image metadata",
            POSTGRES_0004_REMOVE_UNUSED_IMAGE_METADATA_SQL,
        ),
        migration(
            5,
            "add storage cleanup jobs",
            POSTGRES_0005_ADD_STORAGE_CLEANUP_JOBS_SQL,
        ),
        migration(6, "add media blobs", POSTGRES_0006_ADD_MEDIA_BLOBS_SQL),
        migration(
            7,
            "reconcile media blobs schema",
            POSTGRES_0007_RECONCILE_MEDIA_BLOBS_SCHEMA_SQL,
        ),
    ]),
    ..Migrator::DEFAULT
});

const POSTGRES_0001_INITIAL_SCHEMA_SQL: &str = r#"CREATE TABLE IF NOT EXISTS users (
    id UUID PRIMARY KEY,
    email VARCHAR(255) UNIQUE NOT NULL,
    email_verified_at TIMESTAMP WITH TIME ZONE,
    password_hash VARCHAR(255) NOT NULL,
    role VARCHAR(20) DEFAULT 'admin',
    created_at TIMESTAMP WITH TIME ZONE NOT NULL DEFAULT NOW()
);

CREATE TABLE IF NOT EXISTS categories (
    id UUID PRIMARY KEY,
    user_id UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    name VARCHAR(100) NOT NULL,
    created_at TIMESTAMP WITH TIME ZONE NOT NULL DEFAULT NOW(),
    UNIQUE(user_id, name)
);

CREATE TABLE IF NOT EXISTS images (
    id UUID PRIMARY KEY,
    user_id UUID NOT NULL,
    category_id UUID,
    filename VARCHAR(255) NOT NULL,
    thumbnail VARCHAR(255),
    original_filename VARCHAR(255),
    size BIGINT NOT NULL,
    hash VARCHAR(64) NOT NULL,
    format VARCHAR(20),
    views INTEGER DEFAULT 0,
    status VARCHAR(20) DEFAULT 'active',
    expires_at TIMESTAMP WITH TIME ZONE,
    deleted_at TIMESTAMP WITH TIME ZONE,
    created_at TIMESTAMP WITH TIME ZONE NOT NULL DEFAULT NOW()
);

CREATE TABLE IF NOT EXISTS image_tags (
    image_id UUID REFERENCES images(id) ON DELETE CASCADE,
    tag VARCHAR(50) NOT NULL
);

CREATE TABLE IF NOT EXISTS audit_logs (
    id UUID PRIMARY KEY,
    user_id UUID REFERENCES users(id) ON DELETE SET NULL,
    action VARCHAR(50) NOT NULL,
    target_type VARCHAR(20),
    target_id UUID,
    details JSONB,
    ip_address VARCHAR(45),
    created_at TIMESTAMP WITH TIME ZONE NOT NULL DEFAULT NOW()
);

CREATE TABLE IF NOT EXISTS settings (
    key VARCHAR(50) PRIMARY KEY,
    value TEXT NOT NULL,
    updated_at TIMESTAMP WITH TIME ZONE NOT NULL DEFAULT NOW()
);

CREATE TABLE IF NOT EXISTS password_reset_tokens (
    id UUID PRIMARY KEY,
    user_id UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    token_hash VARCHAR(128) NOT NULL UNIQUE,
    expires_at TIMESTAMP WITH TIME ZONE NOT NULL,
    used_at TIMESTAMP WITH TIME ZONE,
    created_at TIMESTAMP WITH TIME ZONE NOT NULL DEFAULT NOW()
);

CREATE TABLE IF NOT EXISTS email_verification_tokens (
    id UUID PRIMARY KEY,
    user_id UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    token_hash VARCHAR(128) NOT NULL UNIQUE,
    expires_at TIMESTAMP WITH TIME ZONE NOT NULL,
    used_at TIMESTAMP WITH TIME ZONE,
    created_at TIMESTAMP WITH TIME ZONE NOT NULL DEFAULT NOW()
);

CREATE INDEX IF NOT EXISTS idx_images_user_id ON images(user_id);
CREATE INDEX IF NOT EXISTS idx_images_category_id ON images(category_id);
CREATE INDEX IF NOT EXISTS idx_images_created_at ON images(created_at DESC);
CREATE INDEX IF NOT EXISTS idx_images_views ON images(views DESC);
CREATE INDEX IF NOT EXISTS idx_images_size ON images(size);
CREATE INDEX IF NOT EXISTS idx_images_deleted_at ON images(deleted_at);
CREATE INDEX IF NOT EXISTS idx_images_hash ON images(hash);
CREATE INDEX IF NOT EXISTS idx_images_status ON images(status);
CREATE INDEX IF NOT EXISTS idx_images_expires_at ON images(expires_at);
CREATE INDEX IF NOT EXISTS idx_categories_user_id ON categories(user_id);
CREATE INDEX IF NOT EXISTS idx_categories_created_at ON categories(created_at DESC);
CREATE INDEX IF NOT EXISTS idx_image_tags_image_id ON image_tags(image_id);
CREATE INDEX IF NOT EXISTS idx_image_tags_tag ON image_tags(tag);
CREATE INDEX IF NOT EXISTS idx_image_tags_image_tag ON image_tags(image_id, tag);
CREATE INDEX IF NOT EXISTS idx_audit_logs_user_id ON audit_logs(user_id);
CREATE INDEX IF NOT EXISTS idx_audit_logs_created_at ON audit_logs(created_at DESC);
CREATE INDEX IF NOT EXISTS idx_audit_logs_action ON audit_logs(action);
CREATE INDEX IF NOT EXISTS idx_password_reset_tokens_user_id ON password_reset_tokens(user_id);
CREATE INDEX IF NOT EXISTS idx_password_reset_tokens_expires_at ON password_reset_tokens(expires_at);
CREATE INDEX IF NOT EXISTS idx_email_verification_tokens_user_id ON email_verification_tokens(user_id);
CREATE INDEX IF NOT EXISTS idx_email_verification_tokens_expires_at ON email_verification_tokens(expires_at);

CREATE INDEX IF NOT EXISTS idx_images_user_status_created ON images(user_id, status, created_at DESC);
CREATE INDEX IF NOT EXISTS idx_images_status_expires ON images(status, expires_at) WHERE expires_at IS NOT NULL;
CREATE INDEX IF NOT EXISTS idx_images_hash_user_deleted ON images(hash, user_id, deleted_at) WHERE deleted_at IS NULL;

CREATE EXTENSION IF NOT EXISTS pg_trgm;
CREATE INDEX IF NOT EXISTS idx_images_filename_trgm ON images USING gin (filename gin_trgm_ops);
CREATE INDEX IF NOT EXISTS idx_images_original_filename_trgm ON images USING gin (original_filename gin_trgm_ops) WHERE original_filename IS NOT NULL;
CREATE INDEX IF NOT EXISTS idx_image_tags_tag_trgm ON image_tags USING gin (tag gin_trgm_ops);

CREATE INDEX IF NOT EXISTS idx_images_user_status_partial ON images(user_id, status) WHERE deleted_at IS NULL;
CREATE INDEX IF NOT EXISTS idx_images_user_category_status_partial ON images(user_id, category_id, status) WHERE deleted_at IS NULL;
CREATE INDEX IF NOT EXISTS idx_images_user_status_created_partial ON images(user_id, status, created_at DESC) WHERE deleted_at IS NULL;
CREATE INDEX IF NOT EXISTS idx_images_user_status_expires_partial ON images(user_id, status, expires_at) WHERE deleted_at IS NULL AND status = 'active';
CREATE INDEX IF NOT EXISTS idx_images_user_deleted_at_partial ON images(user_id, deleted_at DESC) WHERE deleted_at IS NOT NULL;
CREATE INDEX IF NOT EXISTS idx_images_user_category_deleted ON images(user_id, category_id, deleted_at);

CREATE UNIQUE INDEX IF NOT EXISTS uq_images_filename ON images(filename);
"#;

const POSTGRES_0002_DROP_UNIQUE_FILENAME_INDEX_SQL: &str = r#"DROP INDEX IF EXISTS uq_images_filename;

CREATE INDEX IF NOT EXISTS idx_images_filename_lookup ON images(filename);
"#;

const POSTGRES_0003_ADD_AUTH_RUNTIME_STATE_SQL: &str = r#"ALTER TABLE users
    ADD COLUMN token_version BIGINT NOT NULL DEFAULT 0;

CREATE TABLE IF NOT EXISTS auth_state (
    id SMALLINT PRIMARY KEY,
    session_epoch BIGINT NOT NULL DEFAULT 0,
    updated_at TIMESTAMP WITH TIME ZONE NOT NULL DEFAULT NOW(),
    CONSTRAINT chk_auth_state_singleton CHECK (id = 1)
);

INSERT INTO auth_state (id, session_epoch)
VALUES (1, 0)
ON CONFLICT (id) DO NOTHING;

CREATE TABLE IF NOT EXISTS revoked_tokens (
    token_hash VARCHAR(64) PRIMARY KEY,
    expires_at TIMESTAMP WITH TIME ZONE NOT NULL,
    created_at TIMESTAMP WITH TIME ZONE NOT NULL DEFAULT NOW()
);

CREATE INDEX IF NOT EXISTS idx_revoked_tokens_expires_at
    ON revoked_tokens(expires_at);
"#;

const POSTGRES_0004_REMOVE_UNUSED_IMAGE_METADATA_SQL: &str = r#"DROP TABLE IF EXISTS image_tags;
DROP TABLE IF EXISTS categories;

DROP INDEX IF EXISTS idx_images_category_id;
DROP INDEX IF EXISTS idx_images_deleted_at;
DROP INDEX IF EXISTS idx_images_hash_user_deleted;
DROP INDEX IF EXISTS idx_images_user_status_partial;
DROP INDEX IF EXISTS idx_images_user_category_status_partial;
DROP INDEX IF EXISTS idx_images_user_status_created_partial;
DROP INDEX IF EXISTS idx_images_user_status_expires_partial;
DROP INDEX IF EXISTS idx_images_user_deleted_at_partial;
DROP INDEX IF EXISTS idx_images_user_category_deleted;
DROP INDEX IF EXISTS idx_images_original_filename_trgm;

ALTER TABLE images
    DROP COLUMN IF EXISTS category_id,
    DROP COLUMN IF EXISTS original_filename,
    DROP COLUMN IF EXISTS deleted_at;
"#;

const POSTGRES_0005_ADD_STORAGE_CLEANUP_JOBS_SQL: &str = r#"CREATE TABLE IF NOT EXISTS storage_cleanup_jobs (
    id UUID PRIMARY KEY,
    file_key VARCHAR(255) NOT NULL,
    storage_signature CHAR(64) NOT NULL,
    storage_snapshot TEXT NOT NULL,
    attempts BIGINT NOT NULL DEFAULT 0,
    last_error TEXT NULL,
    next_attempt_at TIMESTAMPTZ NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE UNIQUE INDEX IF NOT EXISTS uq_storage_cleanup_jobs_signature_file
    ON storage_cleanup_jobs(storage_signature, file_key);

CREATE INDEX IF NOT EXISTS idx_storage_cleanup_jobs_next_attempt_at
    ON storage_cleanup_jobs(next_attempt_at);
"#;

const POSTGRES_0006_ADD_MEDIA_BLOBS_SQL: &str = r#"CREATE TABLE IF NOT EXISTS media_blobs (
    id UUID PRIMARY KEY,
    image_id UUID NOT NULL REFERENCES images(id) ON DELETE CASCADE,
    key VARCHAR(255) NOT NULL,
    size BIGINT NOT NULL,
    created_at TIMESTAMP WITH TIME ZONE NOT NULL DEFAULT NOW()
);

CREATE UNIQUE INDEX IF NOT EXISTS uq_media_blobs_image_key ON media_blobs(image_id, key);
CREATE INDEX IF NOT EXISTS idx_media_blobs_image_id ON media_blobs(image_id);
"#;

const POSTGRES_0007_RECONCILE_MEDIA_BLOBS_SCHEMA_SQL: &str = r#"ALTER TABLE media_blobs
    ADD COLUMN IF NOT EXISTS storage_key VARCHAR(255),
    ADD COLUMN IF NOT EXISTS media_kind VARCHAR(32) NOT NULL DEFAULT 'original',
    ADD COLUMN IF NOT EXISTS content_hash VARCHAR(64),
    ADD COLUMN IF NOT EXISTS ref_count BIGINT NOT NULL DEFAULT 0,
    ADD COLUMN IF NOT EXISTS status VARCHAR(32) NOT NULL DEFAULT 'ready',
    ADD COLUMN IF NOT EXISTS updated_at TIMESTAMP WITH TIME ZONE NOT NULL DEFAULT NOW();

DO $$
BEGIN
    IF EXISTS (
        SELECT 1
        FROM information_schema.columns
        WHERE table_schema = current_schema()
          AND table_name = 'media_blobs'
          AND column_name = 'key'
    ) THEN
        EXECUTE '
            UPDATE media_blobs
            SET storage_key = COALESCE(storage_key, key)
            WHERE storage_key IS NULL
        ';
    END IF;
END $$;

DELETE FROM media_blobs
WHERE storage_key IS NULL;

DELETE FROM media_blobs older
USING media_blobs newer
WHERE older.ctid < newer.ctid
  AND older.storage_key = newer.storage_key;

DROP INDEX IF EXISTS uq_media_blobs_image_key;
DROP INDEX IF EXISTS idx_media_blobs_image_id;

ALTER TABLE media_blobs
    DROP CONSTRAINT IF EXISTS media_blobs_pkey;

ALTER TABLE media_blobs
    DROP COLUMN IF EXISTS id,
    DROP COLUMN IF EXISTS image_id,
    DROP COLUMN IF EXISTS key,
    DROP COLUMN IF EXISTS size;

CREATE UNIQUE INDEX IF NOT EXISTS uq_media_blobs_storage_key
    ON media_blobs(storage_key);

WITH derived AS (
    SELECT
        refs.storage_key,
        MAX(refs.content_hash) AS content_hash,
        CASE
            WHEN BOOL_OR(refs.media_kind = 'original') THEN 'original'
            WHEN BOOL_OR(refs.media_kind = 'thumbnail') THEN 'thumbnail'
            ELSE 'original'
        END AS media_kind,
        COUNT(*)::BIGINT AS ref_count
    FROM (
        SELECT filename AS storage_key, hash AS content_hash, 'original' AS media_kind
        FROM images
        WHERE status = 'active'
        UNION ALL
        SELECT thumbnail AS storage_key, hash AS content_hash, 'thumbnail' AS media_kind
        FROM images
        WHERE status = 'active'
          AND thumbnail IS NOT NULL
    ) refs
    GROUP BY refs.storage_key
)
INSERT INTO media_blobs (
    storage_key,
    media_kind,
    content_hash,
    ref_count,
    status,
    created_at,
    updated_at
)
SELECT
    derived.storage_key,
    derived.media_kind,
    derived.content_hash,
    derived.ref_count,
    'ready',
    NOW(),
    NOW()
FROM derived
ON CONFLICT (storage_key) DO UPDATE
SET media_kind = EXCLUDED.media_kind,
    content_hash = COALESCE(EXCLUDED.content_hash, media_blobs.content_hash),
    ref_count = EXCLUDED.ref_count,
    status = 'ready',
    updated_at = NOW();

UPDATE media_blobs AS blobs
SET ref_count = 0,
    status = CASE
        WHEN blobs.status = 'pending_delete' THEN 'pending_delete'
        ELSE 'deleted'
    END,
    updated_at = NOW()
WHERE NOT EXISTS (
    SELECT 1
    FROM images
    WHERE status = 'active'
      AND (filename = blobs.storage_key OR thumbnail = blobs.storage_key)
);

ALTER TABLE media_blobs
    ALTER COLUMN storage_key SET NOT NULL;
"#;
