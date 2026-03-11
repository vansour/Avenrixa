CREATE TABLE IF NOT EXISTS users (
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
