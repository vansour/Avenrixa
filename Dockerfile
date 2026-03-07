# Rust 后端完整构建（包含静态文件服务）
# 多阶段构建：后端构建 → 前端构建 → 运行时镜像

# ============================================
# 阶段 1: 构建
# ============================================
FROM rust:trixie AS backend-builder
WORKDIR /app
COPY Cargo.toml Cargo.lock ./
COPY src/ ./src/
RUN cargo build --release

# ============================================
# 阶段 2: 构建前端（SPA 模式）
# ============================================
FROM node:trixie AS frontend-builder
WORKDIR /app
COPY frontend-remix/ /app
RUN npm install && npm run build

# ============================================
# 阶段 3: 最终运行时环境
# ============================================
FROM debian:trixie-slim AS runtime
RUN apt-get update && apt-get install -y \
    ca-certificates \
    curl \
    && rm -rf /var/lib/apt/lists/*

# 创建运行用户（非 root 用户运行 Rust 服务）
RUN useradd -u 1000 -m -s /bin/bash -d /app vanapp

# 复制前端静态文件（包含自动生成的 index.html）
COPY --from=frontend-builder /app/build /app/frontend-remix/build

# 复制后端可执行文件
COPY --from=backend-builder /app/target/release/vansour-image /usr/local/bin/

WORKDIR /app
EXPOSE 8080
ENV RUST_LOG=info
ENV VITE_API_URL=http://localhost:8080

# 健康检查
HEALTHCHECK --interval=30s --timeout=10s --retries=3 --start-period=40s \
    CMD curl -f http://localhost:8080/health || exit 1

CMD ["vansour-image"]
