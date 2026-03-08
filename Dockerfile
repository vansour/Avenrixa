# ==========================================
# 阶段 1: 统一构建环境 (Builder)
# ==========================================
FROM rust:trixie AS builder
WORKDIR /app

# 1. 安装前端构建强依赖的目标架构
RUN rustup target add wasm32-unknown-unknown

# 2. 安装 Trunk（下载预编译二进制，避免每次 no-cache 都编译 trunk 源码）
ARG TARGETARCH=amd64
ARG TRUNK_VERSION=0.21.14
RUN set -eux; \
    case "${TARGETARCH}" in \
        "amd64") trunk_target="x86_64-unknown-linux-gnu" ;; \
        "arm64") trunk_target="aarch64-unknown-linux-gnu" ;; \
        *) echo "Unsupported TARGETARCH: ${TARGETARCH}" >&2; exit 1 ;; \
    esac; \
    curl -fsSL "https://github.com/trunk-rs/trunk/releases/download/v${TRUNK_VERSION}/trunk-${trunk_target}.tar.gz" \
    | tar -xz -C /usr/local/bin trunk; \
    trunk --version

# 3. 依赖缓存层 (仅复制配置文件，最大化利用 Docker 层缓存)
COPY Cargo.toml Cargo.lock ./
COPY backend/Cargo.toml ./backend/
COPY frontend/Cargo.toml ./frontend/

# 利用“假文件”技巧，预先下载和编译前后端的第三方依赖库
RUN mkdir -p backend/src frontend/src \
    && echo "fn main() {}" > backend/src/main.rs \
    && echo "fn main() {}" > frontend/src/main.rs \
    && echo "pub fn dummy() {}" > frontend/src/lib.rs

# 预编译后端依赖
RUN --mount=type=cache,target=/usr/local/cargo/registry \
    --mount=type=cache,target=/usr/local/cargo/git \
    --mount=type=cache,target=/app/target \
    cargo build --release --bin vansour-image

# 预编译前端依赖
# 注意: 如果你 frontend/Cargo.toml 里的 package name 不是 "frontend"，请将下面的 -p frontend 替换为实际名称
RUN --mount=type=cache,target=/usr/local/cargo/registry \
    --mount=type=cache,target=/usr/local/cargo/git \
    --mount=type=cache,target=/app/target \
    cargo build --release --target wasm32-unknown-unknown -p frontend

# 清理假文件
RUN rm -rf backend/src frontend/src

# 4. 复制真实的源码
# 这一步一旦源码有变动，Docker 会自动使后面的缓存失效，无需手动 echo 时间戳
COPY backend/ ./backend/
COPY frontend/ ./frontend/

# 5. 编译后端
RUN --mount=type=cache,target=/usr/local/cargo/registry \
    --mount=type=cache,target=/usr/local/cargo/git \
    --mount=type=cache,target=/app/target \
    touch backend/src/main.rs \
    && cargo build --release --bin vansour-image \
    && cp /app/target/release/vansour-image /app/vansour-image

# 6. 编译前端 (Dioxus WASM)
ENV API_BASE_URL=/
WORKDIR /app/frontend

# 核心修复: 挂载 Trunk 的本地缓存目录 (/root/.cache/trunk)
# 这样 Trunk 第一次自动下载工具后，后续构建会直接命中缓存，不再产生网络开销
RUN --mount=type=cache,target=/usr/local/cargo/registry \
    --mount=type=cache,target=/usr/local/cargo/git \
    --mount=type=cache,target=/app/target \
    --mount=type=cache,target=/root/.cache/trunk \
    find /app/frontend/src -type f -exec touch {} + \
    && touch /app/frontend/index.html /app/frontend/Cargo.toml \
    && trunk build --release --public-url / --dist /app/dist


# ==========================================
# 阶段 2: 最终运行时环境 (Runtime)
# ==========================================
FROM debian:trixie-slim AS runtime

# 安装运行时必需依赖（以 root 用户运行，避免卷权限问题）
RUN apt-get update && apt-get install -y --no-install-recommends \
    ca-certificates curl postgresql-client \
    && rm -rf /var/lib/apt/lists/* \
    && apt-get clean

WORKDIR /app

# 将构建阶段的产物复制过来
COPY --from=builder /app/vansour-image /usr/local/bin/vansour-image
RUN mkdir -p /app/frontend/dist
COPY --from=builder /app/dist /app/frontend/dist/

# 以 root 用户运行应用（确保可以管理卷权限）
# 注意：生产环境建议使用非 root 用户，并正确配置卷的所有者

EXPOSE 8080
ENV RUST_LOG=info

HEALTHCHECK --interval=30s --timeout=10s --retries=3 --start-period=40s \
    CMD curl -f http://localhost:8080/health || exit 1

CMD ["vansour-image"]
