use std::borrow::Cow;

use once_cell::sync::Lazy;
use sqlx::migrate::{Migration, MigrationType, Migrator};

fn migration(version: i64, description: &'static str, sql: &'static str) -> Migration {
    Migration::new(
        version,
        Cow::Borrowed(description),
        MigrationType::Simple,
        Cow::Borrowed(sql),
        false,
    )
}

pub(super) fn mysql_migrator() -> &'static Migrator {
    &MYSQL_MIGRATOR
}

pub(super) fn postgres_migrator() -> &'static Migrator {
    &POSTGRES_MIGRATOR
}

pub(super) fn sqlite_migrator() -> &'static Migrator {
    &SQLITE_MIGRATOR
}

static MYSQL_MIGRATOR: Lazy<Migrator> = Lazy::new(|| Migrator {
    migrations: Cow::Owned(vec![
        migration(1, "initial schema", MYSQL_0001_INITIAL_SCHEMA_SQL),
        migration(
            2,
            "drop unique filename index",
            MYSQL_0002_DROP_UNIQUE_FILENAME_INDEX_SQL,
        ),
        migration(
            3,
            "add auth runtime state",
            MYSQL_0003_ADD_AUTH_RUNTIME_STATE_SQL,
        ),
        migration(
            4,
            "remove unused image metadata",
            MYSQL_0004_REMOVE_UNUSED_IMAGE_METADATA_SQL,
        ),
        migration(
            5,
            "add storage cleanup jobs",
            MYSQL_0005_ADD_STORAGE_CLEANUP_JOBS_SQL,
        ),
    ]),
    ..Migrator::DEFAULT
});

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
    ]),
    ..Migrator::DEFAULT
});

static SQLITE_MIGRATOR: Lazy<Migrator> = Lazy::new(|| Migrator {
    migrations: Cow::Owned(vec![
        migration(1, "initial schema", SQLITE_0001_INITIAL_SCHEMA_SQL),
        migration(
            2,
            "drop unique filename index",
            SQLITE_0002_DROP_UNIQUE_FILENAME_INDEX_SQL,
        ),
        migration(
            3,
            "add auth runtime state",
            SQLITE_0003_ADD_AUTH_RUNTIME_STATE_SQL,
        ),
        migration(
            4,
            "remove unused image metadata",
            SQLITE_0004_REMOVE_UNUSED_IMAGE_METADATA_SQL,
        ),
        migration(
            5,
            "add storage cleanup jobs",
            SQLITE_0005_ADD_STORAGE_CLEANUP_JOBS_SQL,
        ),
    ]),
    ..Migrator::DEFAULT
});

const MYSQL_0001_INITIAL_SCHEMA_SQL: &str = r#"CREATE TABLE IF NOT EXISTS users (
    id BINARY(16) PRIMARY KEY,
    email VARCHAR(255) NOT NULL UNIQUE,
    email_verified_at DATETIME(6) NULL,
    password_hash VARCHAR(255) NOT NULL,
    role VARCHAR(20) NOT NULL DEFAULT 'admin',
    created_at DATETIME(6) NOT NULL DEFAULT CURRENT_TIMESTAMP(6)
);

CREATE TABLE IF NOT EXISTS categories (
    id BINARY(16) PRIMARY KEY,
    user_id BINARY(16) NOT NULL,
    name VARCHAR(100) NOT NULL,
    created_at DATETIME(6) NOT NULL DEFAULT CURRENT_TIMESTAMP(6),
    CONSTRAINT fk_categories_user
        FOREIGN KEY (user_id) REFERENCES users(id) ON DELETE CASCADE,
    UNIQUE KEY uq_categories_user_name (user_id, name)
);

CREATE TABLE IF NOT EXISTS images (
    id BINARY(16) PRIMARY KEY,
    user_id BINARY(16) NOT NULL,
    category_id BINARY(16) NULL,
    filename VARCHAR(255) NOT NULL,
    thumbnail VARCHAR(255) NULL,
    original_filename VARCHAR(255) NULL,
    size BIGINT NOT NULL,
    hash VARCHAR(64) NOT NULL,
    format VARCHAR(20) NULL,
    views INT NOT NULL DEFAULT 0,
    status VARCHAR(20) NOT NULL DEFAULT 'active',
    expires_at DATETIME(6) NULL,
    deleted_at DATETIME(6) NULL,
    created_at DATETIME(6) NOT NULL DEFAULT CURRENT_TIMESTAMP(6)
);

CREATE TABLE IF NOT EXISTS image_tags (
    image_id BINARY(16) NOT NULL,
    tag VARCHAR(50) NOT NULL,
    CONSTRAINT fk_image_tags_image
        FOREIGN KEY (image_id) REFERENCES images(id) ON DELETE CASCADE
);

CREATE TABLE IF NOT EXISTS audit_logs (
    id BINARY(16) PRIMARY KEY,
    user_id BINARY(16) NULL,
    action VARCHAR(50) NOT NULL,
    target_type VARCHAR(20) NULL,
    target_id BINARY(16) NULL,
    details JSON NULL,
    ip_address VARCHAR(45) NULL,
    created_at DATETIME(6) NOT NULL DEFAULT CURRENT_TIMESTAMP(6),
    CONSTRAINT fk_audit_logs_user
        FOREIGN KEY (user_id) REFERENCES users(id) ON DELETE SET NULL
);

CREATE TABLE IF NOT EXISTS settings (
    `key` VARCHAR(50) PRIMARY KEY,
    `value` TEXT NOT NULL,
    updated_at DATETIME(6) NOT NULL DEFAULT CURRENT_TIMESTAMP(6) ON UPDATE CURRENT_TIMESTAMP(6)
);

CREATE TABLE IF NOT EXISTS password_reset_tokens (
    id BINARY(16) PRIMARY KEY,
    user_id BINARY(16) NOT NULL,
    token_hash VARCHAR(128) NOT NULL UNIQUE,
    expires_at DATETIME(6) NOT NULL,
    used_at DATETIME(6) NULL,
    created_at DATETIME(6) NOT NULL DEFAULT CURRENT_TIMESTAMP(6),
    CONSTRAINT fk_password_reset_tokens_user
        FOREIGN KEY (user_id) REFERENCES users(id) ON DELETE CASCADE
);

CREATE TABLE IF NOT EXISTS email_verification_tokens (
    id BINARY(16) PRIMARY KEY,
    user_id BINARY(16) NOT NULL,
    token_hash VARCHAR(128) NOT NULL UNIQUE,
    expires_at DATETIME(6) NOT NULL,
    used_at DATETIME(6) NULL,
    created_at DATETIME(6) NOT NULL DEFAULT CURRENT_TIMESTAMP(6),
    CONSTRAINT fk_email_verification_tokens_user
        FOREIGN KEY (user_id) REFERENCES users(id) ON DELETE CASCADE
);

CREATE INDEX idx_images_user_id ON images(user_id);
CREATE INDEX idx_images_category_id ON images(category_id);
CREATE INDEX idx_images_created_at ON images(created_at DESC);
CREATE INDEX idx_images_views ON images(views DESC);
CREATE INDEX idx_images_size ON images(size);
CREATE INDEX idx_images_deleted_at ON images(deleted_at);
CREATE INDEX idx_images_hash ON images(hash);
CREATE INDEX idx_images_status ON images(status);
CREATE INDEX idx_images_expires_at ON images(expires_at);
CREATE INDEX idx_categories_user_id ON categories(user_id);
CREATE INDEX idx_categories_created_at ON categories(created_at DESC);
CREATE INDEX idx_image_tags_image_id ON image_tags(image_id);
CREATE INDEX idx_image_tags_tag ON image_tags(tag);
CREATE INDEX idx_image_tags_image_tag ON image_tags(image_id, tag);
CREATE INDEX idx_audit_logs_user_id ON audit_logs(user_id);
CREATE INDEX idx_audit_logs_created_at ON audit_logs(created_at DESC);
CREATE INDEX idx_audit_logs_action ON audit_logs(action);
CREATE INDEX idx_password_reset_tokens_user_id ON password_reset_tokens(user_id);
CREATE INDEX idx_password_reset_tokens_expires_at ON password_reset_tokens(expires_at);
CREATE INDEX idx_email_verification_tokens_user_id ON email_verification_tokens(user_id);
CREATE INDEX idx_email_verification_tokens_expires_at ON email_verification_tokens(expires_at);
CREATE INDEX idx_images_user_status_created ON images(user_id, status, created_at DESC);
CREATE INDEX idx_images_status_expires ON images(status, expires_at);
CREATE INDEX idx_images_hash_user_deleted ON images(hash, user_id, deleted_at);
CREATE INDEX idx_images_user_status_partial ON images(user_id, status, deleted_at);
CREATE INDEX idx_images_user_category_status_partial ON images(user_id, category_id, status, deleted_at);
CREATE INDEX idx_images_user_status_created_partial ON images(user_id, status, deleted_at, created_at DESC);
CREATE INDEX idx_images_user_status_expires_partial ON images(user_id, status, deleted_at, expires_at);
CREATE INDEX idx_images_user_deleted_at_partial ON images(user_id, deleted_at DESC);
CREATE INDEX idx_images_user_category_deleted ON images(user_id, category_id, deleted_at);

CREATE UNIQUE INDEX uq_images_filename ON images(filename);
"#;

const MYSQL_0002_DROP_UNIQUE_FILENAME_INDEX_SQL: &str = r#"DROP INDEX uq_images_filename ON images;

CREATE INDEX idx_images_filename_lookup ON images(filename);
"#;

const MYSQL_0003_ADD_AUTH_RUNTIME_STATE_SQL: &str = r#"ALTER TABLE users
    ADD COLUMN token_version BIGINT NOT NULL DEFAULT 0;

CREATE TABLE IF NOT EXISTS auth_state (
    id TINYINT PRIMARY KEY,
    session_epoch BIGINT NOT NULL DEFAULT 0,
    updated_at DATETIME(6) NOT NULL DEFAULT CURRENT_TIMESTAMP(6)
);

INSERT INTO auth_state (id, session_epoch, updated_at)
VALUES (1, 0, CURRENT_TIMESTAMP(6))
ON DUPLICATE KEY UPDATE
    session_epoch = session_epoch;

CREATE TABLE IF NOT EXISTS revoked_tokens (
    token_hash CHAR(64) PRIMARY KEY,
    expires_at DATETIME(6) NOT NULL,
    created_at DATETIME(6) NOT NULL DEFAULT CURRENT_TIMESTAMP(6)
);

CREATE INDEX idx_revoked_tokens_expires_at ON revoked_tokens(expires_at);
"#;

const MYSQL_0004_REMOVE_UNUSED_IMAGE_METADATA_SQL: &str = r#"DROP TABLE IF EXISTS image_tags;
DROP TABLE IF EXISTS categories;

DROP INDEX idx_images_category_id ON images;
DROP INDEX idx_images_deleted_at ON images;
DROP INDEX idx_images_hash_user_deleted ON images;
DROP INDEX idx_images_user_status_partial ON images;
DROP INDEX idx_images_user_category_status_partial ON images;
DROP INDEX idx_images_user_status_created_partial ON images;
DROP INDEX idx_images_user_status_expires_partial ON images;
DROP INDEX idx_images_user_deleted_at_partial ON images;
DROP INDEX idx_images_user_category_deleted ON images;

ALTER TABLE images
    DROP COLUMN category_id,
    DROP COLUMN original_filename,
    DROP COLUMN deleted_at;
"#;

const MYSQL_0005_ADD_STORAGE_CLEANUP_JOBS_SQL: &str = r#"CREATE TABLE IF NOT EXISTS storage_cleanup_jobs (
    id BINARY(16) PRIMARY KEY,
    file_key VARCHAR(255) NOT NULL,
    storage_signature CHAR(64) NOT NULL,
    storage_snapshot LONGTEXT NOT NULL,
    attempts BIGINT NOT NULL DEFAULT 0,
    last_error LONGTEXT NULL,
    next_attempt_at DATETIME(6) NOT NULL,
    created_at DATETIME(6) NOT NULL DEFAULT CURRENT_TIMESTAMP(6),
    updated_at DATETIME(6) NOT NULL DEFAULT CURRENT_TIMESTAMP(6)
);

CREATE UNIQUE INDEX uq_storage_cleanup_jobs_signature_file
    ON storage_cleanup_jobs(storage_signature, file_key);

CREATE INDEX idx_storage_cleanup_jobs_next_attempt_at
    ON storage_cleanup_jobs(next_attempt_at);
"#;

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

const SQLITE_0001_INITIAL_SCHEMA_SQL: &str = r#"CREATE TABLE IF NOT EXISTS users (
    id TEXT PRIMARY KEY,
    email TEXT UNIQUE NOT NULL,
    email_verified_at TEXT,
    password_hash TEXT NOT NULL,
    role TEXT DEFAULT 'admin',
    created_at TEXT NOT NULL DEFAULT (STRFTIME('%Y-%m-%dT%H:%M:%fZ', 'now'))
);

CREATE TABLE IF NOT EXISTS categories (
    id TEXT PRIMARY KEY,
    user_id TEXT NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    name TEXT NOT NULL,
    created_at TEXT NOT NULL DEFAULT (STRFTIME('%Y-%m-%dT%H:%M:%fZ', 'now')),
    UNIQUE(user_id, name)
);

CREATE TABLE IF NOT EXISTS images (
    id TEXT PRIMARY KEY,
    user_id TEXT NOT NULL,
    category_id TEXT,
    filename TEXT NOT NULL,
    thumbnail TEXT,
    original_filename TEXT,
    size INTEGER NOT NULL,
    hash TEXT NOT NULL,
    format TEXT,
    views INTEGER DEFAULT 0,
    status TEXT DEFAULT 'active',
    expires_at TEXT,
    deleted_at TEXT,
    created_at TEXT NOT NULL DEFAULT (STRFTIME('%Y-%m-%dT%H:%M:%fZ', 'now'))
);

CREATE TABLE IF NOT EXISTS image_tags (
    image_id TEXT REFERENCES images(id) ON DELETE CASCADE,
    tag TEXT NOT NULL
);

CREATE TABLE IF NOT EXISTS audit_logs (
    id TEXT PRIMARY KEY,
    user_id TEXT REFERENCES users(id) ON DELETE SET NULL,
    action TEXT NOT NULL,
    target_type TEXT,
    target_id TEXT,
    details TEXT,
    ip_address TEXT,
    created_at TEXT NOT NULL DEFAULT (STRFTIME('%Y-%m-%dT%H:%M:%fZ', 'now'))
);

CREATE TABLE IF NOT EXISTS settings (
    key TEXT PRIMARY KEY,
    value TEXT NOT NULL,
    updated_at TEXT NOT NULL DEFAULT (STRFTIME('%Y-%m-%dT%H:%M:%fZ', 'now'))
);

CREATE TABLE IF NOT EXISTS password_reset_tokens (
    id TEXT PRIMARY KEY,
    user_id TEXT NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    token_hash TEXT NOT NULL UNIQUE,
    expires_at TEXT NOT NULL,
    used_at TEXT,
    created_at TEXT NOT NULL DEFAULT (STRFTIME('%Y-%m-%dT%H:%M:%fZ', 'now'))
);

CREATE TABLE IF NOT EXISTS email_verification_tokens (
    id TEXT PRIMARY KEY,
    user_id TEXT NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    token_hash TEXT NOT NULL UNIQUE,
    expires_at TEXT NOT NULL,
    used_at TEXT,
    created_at TEXT NOT NULL DEFAULT (STRFTIME('%Y-%m-%dT%H:%M:%fZ', 'now'))
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
CREATE INDEX IF NOT EXISTS idx_images_user_status_partial ON images(user_id, status) WHERE deleted_at IS NULL;
CREATE INDEX IF NOT EXISTS idx_images_user_category_status_partial ON images(user_id, category_id, status) WHERE deleted_at IS NULL;
CREATE INDEX IF NOT EXISTS idx_images_user_status_created_partial ON images(user_id, status, created_at DESC) WHERE deleted_at IS NULL;
CREATE INDEX IF NOT EXISTS idx_images_user_status_expires_partial ON images(user_id, status, expires_at) WHERE deleted_at IS NULL AND status = 'active';
CREATE INDEX IF NOT EXISTS idx_images_user_deleted_at_partial ON images(user_id, deleted_at DESC) WHERE deleted_at IS NOT NULL;
CREATE INDEX IF NOT EXISTS idx_images_user_category_deleted ON images(user_id, category_id, deleted_at);
CREATE UNIQUE INDEX IF NOT EXISTS uq_images_filename ON images(filename);
"#;

const SQLITE_0002_DROP_UNIQUE_FILENAME_INDEX_SQL: &str = r#"DROP INDEX IF EXISTS uq_images_filename;

CREATE INDEX IF NOT EXISTS idx_images_filename_lookup ON images(filename);
"#;

const SQLITE_0003_ADD_AUTH_RUNTIME_STATE_SQL: &str = r#"ALTER TABLE users
    ADD COLUMN token_version INTEGER NOT NULL DEFAULT 0;

CREATE TABLE IF NOT EXISTS auth_state (
    id INTEGER PRIMARY KEY CHECK (id = 1),
    session_epoch INTEGER NOT NULL DEFAULT 0,
    updated_at TEXT NOT NULL DEFAULT (STRFTIME('%Y-%m-%dT%H:%M:%fZ', 'now'))
);

INSERT OR IGNORE INTO auth_state (id, session_epoch)
VALUES (1, 0);

CREATE TABLE IF NOT EXISTS revoked_tokens (
    token_hash TEXT PRIMARY KEY,
    expires_at TEXT NOT NULL,
    created_at TEXT NOT NULL DEFAULT (STRFTIME('%Y-%m-%dT%H:%M:%fZ', 'now'))
);

CREATE INDEX IF NOT EXISTS idx_revoked_tokens_expires_at
    ON revoked_tokens(expires_at);
"#;

const SQLITE_0004_REMOVE_UNUSED_IMAGE_METADATA_SQL: &str = r#"PRAGMA foreign_keys = OFF;

DROP INDEX IF EXISTS idx_images_category_id;
DROP INDEX IF EXISTS idx_images_deleted_at;
DROP INDEX IF EXISTS idx_images_hash_user_deleted;
DROP INDEX IF EXISTS idx_images_user_status_partial;
DROP INDEX IF EXISTS idx_images_user_category_status_partial;
DROP INDEX IF EXISTS idx_images_user_status_created_partial;
DROP INDEX IF EXISTS idx_images_user_status_expires_partial;
DROP INDEX IF EXISTS idx_images_user_deleted_at_partial;
DROP INDEX IF EXISTS idx_images_user_category_deleted;

DROP TABLE IF EXISTS image_tags;
DROP TABLE IF EXISTS categories;

ALTER TABLE images RENAME TO images_old;

CREATE TABLE images (
    id TEXT PRIMARY KEY,
    user_id TEXT NOT NULL,
    filename TEXT NOT NULL,
    thumbnail TEXT,
    size INTEGER NOT NULL,
    hash TEXT NOT NULL,
    format TEXT,
    views INTEGER DEFAULT 0,
    status TEXT DEFAULT 'active',
    expires_at TEXT,
    created_at TEXT NOT NULL DEFAULT (STRFTIME('%Y-%m-%dT%H:%M:%fZ', 'now'))
);

INSERT INTO images (
    id,
    user_id,
    filename,
    thumbnail,
    size,
    hash,
    format,
    views,
    status,
    expires_at,
    created_at
)
SELECT
    id,
    user_id,
    filename,
    thumbnail,
    size,
    hash,
    format,
    views,
    status,
    expires_at,
    created_at
FROM images_old;

DROP TABLE images_old;

CREATE INDEX IF NOT EXISTS idx_images_user_id ON images(user_id);
CREATE INDEX IF NOT EXISTS idx_images_created_at ON images(created_at DESC);
CREATE INDEX IF NOT EXISTS idx_images_views ON images(views DESC);
CREATE INDEX IF NOT EXISTS idx_images_size ON images(size);
CREATE INDEX IF NOT EXISTS idx_images_hash ON images(hash);
CREATE INDEX IF NOT EXISTS idx_images_status ON images(status);
CREATE INDEX IF NOT EXISTS idx_images_expires_at ON images(expires_at);
CREATE INDEX IF NOT EXISTS idx_images_user_status_created ON images(user_id, status, created_at DESC);
CREATE INDEX IF NOT EXISTS idx_images_status_expires ON images(status, expires_at) WHERE expires_at IS NOT NULL;
CREATE INDEX IF NOT EXISTS idx_images_filename_lookup ON images(filename);

PRAGMA foreign_keys = ON;
"#;

const SQLITE_0005_ADD_STORAGE_CLEANUP_JOBS_SQL: &str = r#"CREATE TABLE IF NOT EXISTS storage_cleanup_jobs (
    id TEXT PRIMARY KEY,
    file_key TEXT NOT NULL,
    storage_signature TEXT NOT NULL,
    storage_snapshot TEXT NOT NULL,
    attempts INTEGER NOT NULL DEFAULT 0,
    last_error TEXT,
    next_attempt_at TEXT NOT NULL,
    created_at TEXT NOT NULL DEFAULT (STRFTIME('%Y-%m-%dT%H:%M:%fZ', 'now')),
    updated_at TEXT NOT NULL DEFAULT (STRFTIME('%Y-%m-%dT%H:%M:%fZ', 'now'))
);

CREATE UNIQUE INDEX IF NOT EXISTS uq_storage_cleanup_jobs_signature_file
    ON storage_cleanup_jobs(storage_signature, file_key);

CREATE INDEX IF NOT EXISTS idx_storage_cleanup_jobs_next_attempt_at
    ON storage_cleanup_jobs(next_attempt_at);
"#;

#[cfg(test)]
mod tests {
    use std::{borrow::Cow, process::Command, time::Duration};

    use chrono::{NaiveDate, TimeZone, Utc};
    use hex::encode;
    use sqlx::{
        migrate::Migrator,
        mysql::MySqlPoolOptions,
        postgres::PgPoolOptions,
        sqlite::{SqliteConnectOptions, SqlitePoolOptions},
    };
    use tokio::time::{sleep, timeout};
    use uuid::Uuid;

    use super::{mysql_migrator, postgres_migrator, sqlite_migrator};

    const EXPECTED_IMAGE_COLUMNS: [&str; 11] = [
        "id",
        "user_id",
        "filename",
        "thumbnail",
        "size",
        "hash",
        "format",
        "views",
        "status",
        "expires_at",
        "created_at",
    ];

    fn legacy_migrator(migrator: &'static Migrator) -> Migrator {
        Migrator {
            migrations: Cow::Owned(migrator.migrations[..3].to_vec()),
            ..Migrator::DEFAULT
        }
    }

    fn mysql_legacy_migrator() -> Migrator {
        legacy_migrator(mysql_migrator())
    }

    fn postgres_legacy_migrator() -> Migrator {
        legacy_migrator(postgres_migrator())
    }

    fn sqlite_legacy_migrator() -> Migrator {
        legacy_migrator(sqlite_migrator())
    }

    async fn sqlite_test_pool() -> (tempfile::TempDir, sqlx::SqlitePool) {
        let temp_dir = tempfile::tempdir().expect("temp dir should be created");
        let database_path = temp_dir.path().join("migrations.db");
        let pool = SqlitePoolOptions::new()
            .max_connections(1)
            .connect_with(
                SqliteConnectOptions::new()
                    .filename(&database_path)
                    .create_if_missing(true)
                    .foreign_keys(true),
            )
            .await
            .expect("sqlite pool should be created");

        (temp_dir, pool)
    }

    async fn sqlite_table_exists(pool: &sqlx::SqlitePool, table_name: &str) -> bool {
        sqlx::query_scalar::<_, i64>(
            "SELECT EXISTS(
                SELECT 1
                FROM sqlite_master
                WHERE type = 'table' AND name = ?1
            )",
        )
        .bind(table_name)
        .fetch_one(pool)
        .await
        .expect("sqlite_master query should succeed")
            == 1
    }

    fn docker_output(args: &[String]) -> String {
        let output = Command::new("docker")
            .args(args)
            .output()
            .expect("docker command should start");
        let stdout = String::from_utf8_lossy(&output.stdout).trim().to_string();
        let stderr = String::from_utf8_lossy(&output.stderr).trim().to_string();
        assert!(
            output.status.success(),
            "docker {} failed\nstdout: {}\nstderr: {}",
            args.join(" "),
            stdout,
            stderr
        );
        stdout
    }

    struct DockerContainer {
        name: String,
    }

    impl DockerContainer {
        fn start(
            name_prefix: &str,
            image: &str,
            container_port: u16,
            env: &[(&str, &str)],
        ) -> (Self, u16) {
            let name = format!("{}-{}", name_prefix, Uuid::new_v4().simple());
            let mut args = vec![
                "run".to_string(),
                "-d".to_string(),
                "--rm".to_string(),
                "--name".to_string(),
                name.clone(),
                "-p".to_string(),
                format!("127.0.0.1::{}", container_port),
            ];

            for (key, value) in env {
                args.push("-e".to_string());
                args.push(format!("{}={}", key, value));
            }

            args.push(image.to_string());
            docker_output(&args);

            let container = Self { name };
            let host_port = container.host_port(container_port);
            (container, host_port)
        }

        fn host_port(&self, container_port: u16) -> u16 {
            let template = format!(
                "{{{{(index (index .NetworkSettings.Ports \"{}/tcp\") 0).HostPort}}}}",
                container_port
            );
            let output = docker_output(&[
                "inspect".to_string(),
                "-f".to_string(),
                template,
                self.name.clone(),
            ]);
            output
                .parse::<u16>()
                .expect("docker host port should be numeric")
        }
    }

    impl Drop for DockerContainer {
        fn drop(&mut self) {
            let _ = Command::new("docker")
                .args(["rm", "-f", &self.name])
                .output();
        }
    }

    async fn wait_for_postgres_pool(database_url: &str) -> sqlx::PgPool {
        for _ in 0..90 {
            if let Ok(Ok(pool)) = timeout(
                Duration::from_secs(2),
                PgPoolOptions::new()
                    .max_connections(1)
                    .connect(database_url),
            )
            .await
            {
                if sqlx::query_scalar::<_, i32>("SELECT 1")
                    .fetch_one(&pool)
                    .await
                    .is_ok()
                {
                    return pool;
                }

                pool.close().await;
            }

            sleep(Duration::from_secs(1)).await;
        }

        panic!("postgres container did not become ready in time");
    }

    async fn wait_for_mysql_pool(database_url: &str) -> sqlx::MySqlPool {
        for _ in 0..120 {
            if let Ok(Ok(pool)) = timeout(
                Duration::from_secs(2),
                MySqlPoolOptions::new()
                    .max_connections(1)
                    .connect(database_url),
            )
            .await
            {
                if sqlx::query_scalar::<_, i32>("SELECT 1")
                    .fetch_one(&pool)
                    .await
                    .is_ok()
                {
                    return pool;
                }

                pool.close().await;
            }

            sleep(Duration::from_secs(1)).await;
        }

        panic!("mysql container did not become ready in time");
    }

    async fn start_postgres_test_pool() -> (DockerContainer, sqlx::PgPool) {
        let (container, host_port) = DockerContainer::start(
            "vansour-image-pg-upgrade",
            "postgres:18",
            5432,
            &[
                ("POSTGRES_DB", "image"),
                ("POSTGRES_USER", "user"),
                ("POSTGRES_PASSWORD", "pass"),
            ],
        );
        let database_url = format!("postgresql://user:pass@127.0.0.1:{}/image", host_port);
        let pool = wait_for_postgres_pool(&database_url).await;
        (container, pool)
    }

    async fn start_mysql_test_pool() -> (DockerContainer, sqlx::MySqlPool) {
        let (container, host_port) = DockerContainer::start(
            "vansour-image-mysql-upgrade",
            "mysql:8.4",
            3306,
            &[
                ("MYSQL_DATABASE", "image"),
                ("MYSQL_USER", "user"),
                ("MYSQL_PASSWORD", "pass"),
                ("MYSQL_ROOT_PASSWORD", "rootpass"),
            ],
        );
        let database_url = format!("mysql://user:pass@127.0.0.1:{}/image", host_port);
        let pool = wait_for_mysql_pool(&database_url).await;
        (container, pool)
    }

    #[test]
    fn embedded_migration_checksums_match_previous_files() {
        assert_eq!(
            encode(mysql_migrator().migrations[0].checksum.as_ref()),
            "25b439d83f52283e4c160e277cba817138791adae4211fd281d40826a5906d745fd153dceecd8d6c8476a4bf0cd6d71d"
        );
        assert_eq!(
            encode(mysql_migrator().migrations[1].checksum.as_ref()),
            "aeeaa4793ee9d45d3ad71b108bf72276b3f77739ee82e0ab1dc20711d47804cfdbb1a6b7c790cba0f87a3545eae43a97"
        );
        assert_eq!(
            encode(mysql_migrator().migrations[2].checksum.as_ref()),
            "a964ea0b3b6b90ae31d10ec63ad30ecdd21a37e13c815e5834a54bf73996e0cd0ac98ced6434a9f2e6754b23d5efc82d"
        );
        assert_eq!(
            encode(mysql_migrator().migrations[3].checksum.as_ref()),
            "b7e5b2b4dcba5ecaff6716bc1bf713e9b2084fc6aafa335b88fd73c93da1dc29e357542f3c49b0a4db07dee1fa0164ed"
        );

        assert_eq!(
            encode(postgres_migrator().migrations[0].checksum.as_ref()),
            "e3d46ad709293dccd10547fadd6cde8ce8d4874417609b40088540b5c0e817ae5f1ef737d7853defcd7298ff4a2e7e93"
        );
        assert_eq!(
            encode(postgres_migrator().migrations[1].checksum.as_ref()),
            "9c089c99b82b83f67e4a9fe2ff47b4483c1475ba7be56e320a2910efd939504c9a57b87f390151db31da31fa0ea79dc8"
        );
        assert_eq!(
            encode(postgres_migrator().migrations[2].checksum.as_ref()),
            "f331d638badd2003158f875db2ec16a71a6b0e8721e5b605be725ee7c0f327eeaa32e65ccd2c2be3e4caa53c4cac516a"
        );
        assert_eq!(
            encode(postgres_migrator().migrations[3].checksum.as_ref()),
            "6929d35f692353eb93012002a4e4b54cac8d87f58afcb18248f908bf7a1a336e353a24cc834d52e3861790324062160a"
        );

        assert_eq!(
            encode(sqlite_migrator().migrations[0].checksum.as_ref()),
            "ea349ef314931d72e59185b5b55c5e25e85be3ec477a69a957b986ea8e131087693339f2094bc7bfb8edf055b24fa422"
        );
        assert_eq!(
            encode(sqlite_migrator().migrations[1].checksum.as_ref()),
            "9c089c99b82b83f67e4a9fe2ff47b4483c1475ba7be56e320a2910efd939504c9a57b87f390151db31da31fa0ea79dc8"
        );
        assert_eq!(
            encode(sqlite_migrator().migrations[2].checksum.as_ref()),
            "d65117bcefb41cc52dd4f1cffee63ecdd86a5111e2e8bb919be2367982ec12919fa0cdff4c053fa82f1e0390ef7c4aa0"
        );
        assert_eq!(
            encode(sqlite_migrator().migrations[3].checksum.as_ref()),
            "461c2880e14f936f080c5e2fc5b2cb500d6fc6f32c91f622be5755812d487655af784c0a039e7d19409b5cb3a6f98032"
        );
    }

    #[tokio::test]
    async fn sqlite_upgrade_from_legacy_schema_removes_unused_metadata_and_preserves_image_data() {
        let (_temp_dir, pool) = sqlite_test_pool().await;
        sqlite_legacy_migrator()
            .run(&pool)
            .await
            .expect("legacy migrations should succeed");

        let user_id = "00000000-0000-0000-0000-000000000001";
        let category_id = "00000000-0000-0000-0000-000000000010";
        let image_id = "00000000-0000-0000-0000-000000000100";
        let revoked_token_hash = "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa";
        let expires_at = "2030-02-03T04:05:06.000Z";
        let created_at = "2025-01-02T03:04:05.000Z";

        sqlx::query(
            "INSERT INTO users (
                id,
                email,
                password_hash,
                role,
                created_at,
                token_version
            )
            VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
        )
        .bind(user_id)
        .bind("legacy@example.com")
        .bind("password-hash")
        .bind("admin")
        .bind(created_at)
        .bind(2_i64)
        .execute(&pool)
        .await
        .expect("legacy user should be inserted");

        sqlx::query(
            "INSERT INTO categories (id, user_id, name, created_at)
             VALUES (?1, ?2, ?3, ?4)",
        )
        .bind(category_id)
        .bind(user_id)
        .bind("legacy-category")
        .bind(created_at)
        .execute(&pool)
        .await
        .expect("legacy category should be inserted");

        sqlx::query(
            "INSERT INTO images (
                id,
                user_id,
                category_id,
                filename,
                thumbnail,
                original_filename,
                size,
                hash,
                format,
                views,
                status,
                expires_at,
                deleted_at,
                created_at
            )
            VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14)",
        )
        .bind(image_id)
        .bind(user_id)
        .bind(category_id)
        .bind("legacy-image.png")
        .bind("legacy-image-thumb.webp")
        .bind("legacy-original-name.png")
        .bind(123_456_i64)
        .bind("legacy-hash")
        .bind("png")
        .bind(42_i64)
        .bind("archived")
        .bind(expires_at)
        .bind("2025-01-03T03:04:05.000Z")
        .bind(created_at)
        .execute(&pool)
        .await
        .expect("legacy image should be inserted");

        sqlx::query("INSERT INTO image_tags (image_id, tag) VALUES (?1, ?2)")
            .bind(image_id)
            .bind("legacy-tag")
            .execute(&pool)
            .await
            .expect("legacy image tag should be inserted");

        sqlx::query(
            "UPDATE auth_state
             SET session_epoch = ?1, updated_at = ?2
             WHERE id = 1",
        )
        .bind(7_i64)
        .bind(created_at)
        .execute(&pool)
        .await
        .expect("auth state should be updated");

        sqlx::query(
            "INSERT INTO revoked_tokens (token_hash, expires_at, created_at)
             VALUES (?1, ?2, ?3)",
        )
        .bind(revoked_token_hash)
        .bind(expires_at)
        .bind(created_at)
        .execute(&pool)
        .await
        .expect("revoked token should be inserted");

        sqlite_migrator()
            .run(&pool)
            .await
            .expect("current migrations should upgrade legacy schema");

        let image_columns = sqlx::query_scalar::<_, String>(
            "SELECT name FROM pragma_table_info('images') ORDER BY cid",
        )
        .fetch_all(&pool)
        .await
        .expect("image columns should be listed");
        assert_eq!(
            image_columns,
            vec![
                "id",
                "user_id",
                "filename",
                "thumbnail",
                "size",
                "hash",
                "format",
                "views",
                "status",
                "expires_at",
                "created_at",
            ]
        );

        assert!(!sqlite_table_exists(&pool, "categories").await);
        assert!(!sqlite_table_exists(&pool, "image_tags").await);

        let image = sqlx::query_as::<
            _,
            (
                String,
                String,
                String,
                Option<String>,
                i64,
                String,
                Option<String>,
                i64,
                String,
                Option<String>,
                String,
            ),
        >(
            "SELECT
                id,
                user_id,
                filename,
                thumbnail,
                size,
                hash,
                format,
                views,
                status,
                expires_at,
                created_at
             FROM images
             WHERE id = ?1",
        )
        .bind(image_id)
        .fetch_one(&pool)
        .await
        .expect("image should remain after migration");
        assert_eq!(
            image,
            (
                image_id.to_string(),
                user_id.to_string(),
                "legacy-image.png".to_string(),
                Some("legacy-image-thumb.webp".to_string()),
                123_456,
                "legacy-hash".to_string(),
                Some("png".to_string()),
                42,
                "archived".to_string(),
                Some(expires_at.to_string()),
                created_at.to_string(),
            )
        );

        let session_epoch =
            sqlx::query_scalar::<_, i64>("SELECT session_epoch FROM auth_state WHERE id = 1")
                .fetch_one(&pool)
                .await
                .expect("auth state should remain");
        assert_eq!(session_epoch, 7);

        let revoked_token_count = sqlx::query_scalar::<_, i64>(
            "SELECT COUNT(*)
             FROM revoked_tokens
             WHERE token_hash = ?1",
        )
        .bind(revoked_token_hash)
        .fetch_one(&pool)
        .await
        .expect("revoked token count should load");
        assert_eq!(revoked_token_count, 1);

        let applied_versions =
            sqlx::query_scalar::<_, i64>("SELECT version FROM _sqlx_migrations ORDER BY version")
                .fetch_all(&pool)
                .await
                .expect("applied migrations should be listed");
        assert_eq!(applied_versions, vec![1, 2, 3, 4, 5]);
    }

    #[tokio::test]
    #[ignore = "requires docker"]
    async fn mysql_upgrade_from_legacy_schema_removes_unused_metadata_and_preserves_image_data() {
        let (_container, pool) = start_mysql_test_pool().await;
        mysql_legacy_migrator()
            .run(&pool)
            .await
            .expect("legacy migrations should succeed");

        let user_id = Uuid::parse_str("00000000-0000-0000-0000-000000000001")
            .expect("user uuid should parse");
        let category_id = Uuid::parse_str("00000000-0000-0000-0000-000000000010")
            .expect("category uuid should parse");
        let image_id = Uuid::parse_str("00000000-0000-0000-0000-000000000100")
            .expect("image uuid should parse");
        let revoked_token_hash = "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa";
        let created_at = NaiveDate::from_ymd_opt(2025, 1, 2)
            .expect("valid date")
            .and_hms_micro_opt(3, 4, 5, 123_000)
            .expect("valid timestamp");
        let expires_at = NaiveDate::from_ymd_opt(2030, 2, 3)
            .expect("valid date")
            .and_hms_micro_opt(4, 5, 6, 456_000)
            .expect("valid timestamp");
        let deleted_at = NaiveDate::from_ymd_opt(2025, 1, 3)
            .expect("valid date")
            .and_hms_micro_opt(3, 4, 5, 789_000)
            .expect("valid timestamp");

        sqlx::query(
            "INSERT INTO users (
                id,
                email,
                password_hash,
                role,
                created_at,
                token_version
            )
            VALUES (?, ?, ?, ?, ?, ?)",
        )
        .bind(user_id)
        .bind("legacy@example.com")
        .bind("password-hash")
        .bind("admin")
        .bind(created_at)
        .bind(2_i64)
        .execute(&pool)
        .await
        .expect("legacy user should be inserted");

        sqlx::query(
            "INSERT INTO categories (id, user_id, name, created_at)
             VALUES (?, ?, ?, ?)",
        )
        .bind(category_id)
        .bind(user_id)
        .bind("legacy-category")
        .bind(created_at)
        .execute(&pool)
        .await
        .expect("legacy category should be inserted");

        sqlx::query(
            "INSERT INTO images (
                id,
                user_id,
                category_id,
                filename,
                thumbnail,
                original_filename,
                size,
                hash,
                format,
                views,
                status,
                expires_at,
                deleted_at,
                created_at
            )
            VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)",
        )
        .bind(image_id)
        .bind(user_id)
        .bind(category_id)
        .bind("legacy-image.png")
        .bind("legacy-image-thumb.webp")
        .bind("legacy-original-name.png")
        .bind(123_456_i64)
        .bind("legacy-hash")
        .bind("png")
        .bind(42_i32)
        .bind("archived")
        .bind(expires_at)
        .bind(deleted_at)
        .bind(created_at)
        .execute(&pool)
        .await
        .expect("legacy image should be inserted");

        sqlx::query("INSERT INTO image_tags (image_id, tag) VALUES (?, ?)")
            .bind(image_id)
            .bind("legacy-tag")
            .execute(&pool)
            .await
            .expect("legacy image tag should be inserted");

        sqlx::query(
            "UPDATE auth_state
             SET session_epoch = ?, updated_at = ?
             WHERE id = 1",
        )
        .bind(7_i64)
        .bind(created_at)
        .execute(&pool)
        .await
        .expect("auth state should be updated");

        sqlx::query(
            "INSERT INTO revoked_tokens (token_hash, expires_at, created_at)
             VALUES (?, ?, ?)",
        )
        .bind(revoked_token_hash)
        .bind(expires_at)
        .bind(created_at)
        .execute(&pool)
        .await
        .expect("revoked token should be inserted");

        mysql_migrator()
            .run(&pool)
            .await
            .expect("current migrations should upgrade legacy schema");

        let image_columns = sqlx::query_scalar::<_, String>(
            "SELECT COLUMN_NAME
             FROM INFORMATION_SCHEMA.COLUMNS
             WHERE TABLE_SCHEMA = DATABASE()
               AND TABLE_NAME = 'images'
             ORDER BY ORDINAL_POSITION",
        )
        .fetch_all(&pool)
        .await
        .expect("image columns should be listed");
        assert_eq!(image_columns, EXPECTED_IMAGE_COLUMNS);

        let categories_exists = sqlx::query_scalar::<_, i64>(
            "SELECT COUNT(*)
             FROM INFORMATION_SCHEMA.TABLES
             WHERE TABLE_SCHEMA = DATABASE()
               AND TABLE_NAME = ?",
        )
        .bind("categories")
        .fetch_one(&pool)
        .await
        .expect("table existence should load");
        assert_eq!(categories_exists, 0);

        let image_tags_exists = sqlx::query_scalar::<_, i64>(
            "SELECT COUNT(*)
             FROM INFORMATION_SCHEMA.TABLES
             WHERE TABLE_SCHEMA = DATABASE()
               AND TABLE_NAME = ?",
        )
        .bind("image_tags")
        .fetch_one(&pool)
        .await
        .expect("table existence should load");
        assert_eq!(image_tags_exists, 0);

        let image = sqlx::query_as::<
            _,
            (
                Uuid,
                Uuid,
                String,
                Option<String>,
                i64,
                String,
                Option<String>,
                i32,
                String,
                Option<chrono::NaiveDateTime>,
                chrono::NaiveDateTime,
            ),
        >(
            "SELECT
                id,
                user_id,
                filename,
                thumbnail,
                size,
                hash,
                format,
                views,
                status,
                expires_at,
                created_at
             FROM images
             WHERE id = ?",
        )
        .bind(image_id)
        .fetch_one(&pool)
        .await
        .expect("image should remain after migration");
        assert_eq!(
            image,
            (
                image_id,
                user_id,
                "legacy-image.png".to_string(),
                Some("legacy-image-thumb.webp".to_string()),
                123_456,
                "legacy-hash".to_string(),
                Some("png".to_string()),
                42,
                "archived".to_string(),
                Some(expires_at),
                created_at,
            )
        );

        let session_epoch =
            sqlx::query_scalar::<_, i64>("SELECT session_epoch FROM auth_state WHERE id = 1")
                .fetch_one(&pool)
                .await
                .expect("auth state should remain");
        assert_eq!(session_epoch, 7);

        let revoked_token_count = sqlx::query_scalar::<_, i64>(
            "SELECT COUNT(*)
             FROM revoked_tokens
             WHERE token_hash = ?",
        )
        .bind(revoked_token_hash)
        .fetch_one(&pool)
        .await
        .expect("revoked token count should load");
        assert_eq!(revoked_token_count, 1);

        let applied_versions =
            sqlx::query_scalar::<_, i64>("SELECT version FROM _sqlx_migrations ORDER BY version")
                .fetch_all(&pool)
                .await
                .expect("applied migrations should be listed");
        assert_eq!(applied_versions, vec![1, 2, 3, 4, 5]);

        pool.close().await;
    }

    #[tokio::test]
    #[ignore = "requires docker"]
    async fn postgres_upgrade_from_legacy_schema_removes_unused_metadata_and_preserves_image_data()
    {
        let (_container, pool) = start_postgres_test_pool().await;
        postgres_legacy_migrator()
            .run(&pool)
            .await
            .expect("legacy migrations should succeed");

        let user_id = Uuid::parse_str("00000000-0000-0000-0000-000000000001")
            .expect("user uuid should parse");
        let category_id = Uuid::parse_str("00000000-0000-0000-0000-000000000010")
            .expect("category uuid should parse");
        let image_id = Uuid::parse_str("00000000-0000-0000-0000-000000000100")
            .expect("image uuid should parse");
        let revoked_token_hash = "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa";
        let created_at = Utc
            .with_ymd_and_hms(2025, 1, 2, 3, 4, 5)
            .single()
            .expect("valid timestamp");
        let expires_at = Utc
            .with_ymd_and_hms(2030, 2, 3, 4, 5, 6)
            .single()
            .expect("valid timestamp");
        let deleted_at = Utc
            .with_ymd_and_hms(2025, 1, 3, 3, 4, 5)
            .single()
            .expect("valid timestamp");

        sqlx::query(
            "INSERT INTO users (
                id,
                email,
                password_hash,
                role,
                created_at,
                token_version
            )
            VALUES ($1, $2, $3, $4, $5, $6)",
        )
        .bind(user_id)
        .bind("legacy@example.com")
        .bind("password-hash")
        .bind("admin")
        .bind(created_at)
        .bind(2_i64)
        .execute(&pool)
        .await
        .expect("legacy user should be inserted");

        sqlx::query(
            "INSERT INTO categories (id, user_id, name, created_at)
             VALUES ($1, $2, $3, $4)",
        )
        .bind(category_id)
        .bind(user_id)
        .bind("legacy-category")
        .bind(created_at)
        .execute(&pool)
        .await
        .expect("legacy category should be inserted");

        sqlx::query(
            "INSERT INTO images (
                id,
                user_id,
                category_id,
                filename,
                thumbnail,
                original_filename,
                size,
                hash,
                format,
                views,
                status,
                expires_at,
                deleted_at,
                created_at
            )
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13, $14)",
        )
        .bind(image_id)
        .bind(user_id)
        .bind(category_id)
        .bind("legacy-image.png")
        .bind("legacy-image-thumb.webp")
        .bind("legacy-original-name.png")
        .bind(123_456_i64)
        .bind("legacy-hash")
        .bind("png")
        .bind(42_i32)
        .bind("archived")
        .bind(expires_at)
        .bind(deleted_at)
        .bind(created_at)
        .execute(&pool)
        .await
        .expect("legacy image should be inserted");

        sqlx::query("INSERT INTO image_tags (image_id, tag) VALUES ($1, $2)")
            .bind(image_id)
            .bind("legacy-tag")
            .execute(&pool)
            .await
            .expect("legacy image tag should be inserted");

        sqlx::query(
            "UPDATE auth_state
             SET session_epoch = $1, updated_at = $2
             WHERE id = 1",
        )
        .bind(7_i64)
        .bind(created_at)
        .execute(&pool)
        .await
        .expect("auth state should be updated");

        sqlx::query(
            "INSERT INTO revoked_tokens (token_hash, expires_at, created_at)
             VALUES ($1, $2, $3)",
        )
        .bind(revoked_token_hash)
        .bind(expires_at)
        .bind(created_at)
        .execute(&pool)
        .await
        .expect("revoked token should be inserted");

        postgres_migrator()
            .run(&pool)
            .await
            .expect("current migrations should upgrade legacy schema");

        let image_columns = sqlx::query_scalar::<_, String>(
            "SELECT column_name
             FROM information_schema.columns
             WHERE table_schema = 'public'
               AND table_name = 'images'
             ORDER BY ordinal_position",
        )
        .fetch_all(&pool)
        .await
        .expect("image columns should be listed");
        assert_eq!(image_columns, EXPECTED_IMAGE_COLUMNS);

        let categories_exists =
            sqlx::query_scalar::<_, bool>("SELECT to_regclass('public.categories') IS NOT NULL")
                .fetch_one(&pool)
                .await
                .expect("table existence should load");
        assert!(!categories_exists);

        let image_tags_exists =
            sqlx::query_scalar::<_, bool>("SELECT to_regclass('public.image_tags') IS NOT NULL")
                .fetch_one(&pool)
                .await
                .expect("table existence should load");
        assert!(!image_tags_exists);

        let image = sqlx::query_as::<
            _,
            (
                Uuid,
                Uuid,
                String,
                Option<String>,
                i64,
                String,
                Option<String>,
                i32,
                String,
                Option<chrono::DateTime<Utc>>,
                chrono::DateTime<Utc>,
            ),
        >(
            "SELECT
                id,
                user_id,
                filename,
                thumbnail,
                size,
                hash,
                format,
                views,
                status,
                expires_at,
                created_at
             FROM images
             WHERE id = $1",
        )
        .bind(image_id)
        .fetch_one(&pool)
        .await
        .expect("image should remain after migration");
        assert_eq!(
            image,
            (
                image_id,
                user_id,
                "legacy-image.png".to_string(),
                Some("legacy-image-thumb.webp".to_string()),
                123_456,
                "legacy-hash".to_string(),
                Some("png".to_string()),
                42,
                "archived".to_string(),
                Some(expires_at),
                created_at,
            )
        );

        let session_epoch =
            sqlx::query_scalar::<_, i64>("SELECT session_epoch FROM auth_state WHERE id = 1")
                .fetch_one(&pool)
                .await
                .expect("auth state should remain");
        assert_eq!(session_epoch, 7);

        let revoked_token_count = sqlx::query_scalar::<_, i64>(
            "SELECT COUNT(*)
             FROM revoked_tokens
             WHERE token_hash = $1",
        )
        .bind(revoked_token_hash)
        .fetch_one(&pool)
        .await
        .expect("revoked token count should load");
        assert_eq!(revoked_token_count, 1);

        let applied_versions =
            sqlx::query_scalar::<_, i64>("SELECT version FROM _sqlx_migrations ORDER BY version")
                .fetch_all(&pool)
                .await
                .expect("applied migrations should be listed");
        assert_eq!(applied_versions, vec![1, 2, 3, 4, 5]);

        pool.close().await;
    }
}
