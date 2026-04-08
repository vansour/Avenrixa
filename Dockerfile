# syntax=docker/dockerfile:1.7

# ==========================================
# 阶段 1: 前端构建 (Vue 3 + Vite)
# ==========================================
FROM node:22-bookworm-slim AS frontend-builder

WORKDIR /app/frontend

COPY frontend/package.json frontend/package-lock.json* ./

RUN --mount=type=cache,target=/root/.npm \
    npm ci

COPY frontend/ ./

RUN npm run build


# ==========================================
# 阶段 2: 后端构建 (Rust)
# ==========================================
FROM rust:trixie AS backend-builder
ARG APP_VERSION=dev
ARG APP_REVISION=dev
ARG BUILD_DATE=unknown
ENV APP_VERSION=${APP_VERSION}
ENV APP_REVISION=${APP_REVISION}
ENV BUILD_DATE=${BUILD_DATE}
WORKDIR /app

COPY Cargo.toml Cargo.lock ./
COPY backend/Cargo.toml ./backend/
COPY shared-types/Cargo.toml ./shared-types/

RUN mkdir -p backend/src shared-types/src \
    && echo "fn main() {}" > backend/src/main.rs \
    && echo "pub fn placeholder() {}" > shared-types/src/lib.rs

RUN --mount=type=cache,target=/usr/local/cargo/registry \
    --mount=type=cache,target=/usr/local/cargo/git \
    --mount=type=cache,target=/app/target \
    cargo build --release --bin avenrixa

RUN rm -rf backend/src shared-types/src

COPY backend/ ./backend/
COPY shared-types/ ./shared-types/

RUN --mount=type=cache,target=/usr/local/cargo/registry \
    --mount=type=cache,target=/usr/local/cargo/git \
    --mount=type=cache,target=/app/target \
    find backend/src shared-types/src -type f -exec touch {} + \
    && cargo build --release --bin avenrixa \
    && cp /app/target/release/avenrixa /app/avenrixa


# ==========================================
# 阶段 3: 最终运行时环境 (Runtime)
# ==========================================
FROM debian:trixie-slim AS runtime
ARG APP_VERSION=dev
ARG APP_REVISION=dev
ARG BUILD_DATE=unknown
LABEL org.opencontainers.image.title="Avenrixa" \
      org.opencontainers.image.version=${APP_VERSION} \
      org.opencontainers.image.revision=${APP_REVISION} \
      org.opencontainers.image.created=${BUILD_DATE}
ENV APP_VERSION=${APP_VERSION}
ENV APP_REVISION=${APP_REVISION}
ENV BUILD_DATE=${BUILD_DATE}

RUN set -eux; \
    apt-get update; \
    apt-get install -y --no-install-recommends ca-certificates curl default-mysql-client gnupg; \
    install -d -m 0755 /usr/share/keyrings; \
    curl -fsSL https://www.postgresql.org/media/keys/ACCC4CF8.asc \
      | gpg --dearmor -o /usr/share/keyrings/postgresql.gpg; \
    echo "deb [signed-by=/usr/share/keyrings/postgresql.gpg] https://apt.postgresql.org/pub/repos/apt trixie-pgdg main" \
      > /etc/apt/sources.list.d/pgdg.list; \
    apt-get update; \
    apt-get install -y --no-install-recommends postgresql-client-18; \
    rm -rf /var/lib/apt/lists/*; \
    apt-get clean

WORKDIR /app

COPY --from=backend-builder /app/avenrixa /usr/local/bin/avenrixa
RUN mkdir -p /app/frontend/dist
COPY --from=frontend-builder /app/frontend/dist /app/frontend/dist/

EXPOSE 8080
ENV RUST_LOG=info

HEALTHCHECK --interval=30s --timeout=10s --retries=3 --start-period=40s \
    CMD curl -f http://localhost:8080/health || exit 1

CMD ["avenrixa"]
