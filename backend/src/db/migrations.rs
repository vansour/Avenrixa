use std::borrow::Cow;

use once_cell::sync::Lazy;
use sqlx::migrate::{Migration, MigrationType};
use sqlx::migrate::Migrator;

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
        migration(
            6,
            "add media blobs",
            POSTGRES_0006_ADD_MEDIA_BLOBS_SQL,
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

#[cfg(test)]
mod tests {
    use std::{borrow::Cow, process::Command, time::Duration};

    use chrono::{TimeZone, Utc};
    use sqlx::postgres::PgPoolOptions;
    use tokio::time::{sleep, timeout};
    use uuid::Uuid;

    use super::postgres_migrator;

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

    fn postgres_legacy_migrator() -> Migrator {
        legacy_migrator(postgres_migrator())
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

    async fn start_postgres_test_pool() -> (DockerContainer, sqlx::PgPool) {
        let (container, host_port) = DockerContainer::start(
            "avenrixa-pg-upgrade",
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
        assert_eq!(applied_versions, vec![1, 2, 3, 4, 5, 6]);

        pool.close().await;
    }
}
