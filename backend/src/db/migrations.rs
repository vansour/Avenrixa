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

#[cfg(test)]
mod tests {
    use hex::encode;

    use super::{mysql_migrator, postgres_migrator, sqlite_migrator};

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
    }
}
