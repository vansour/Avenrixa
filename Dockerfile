FROM ghcr.io/vansour/node:trixie AS frontend-builder
COPY frontend/ /app/frontend
WORKDIR /app/frontend
RUN npm install && npm run build

FROM ghcr.io/vansour/rust:trixie AS backend-builder
WORKDIR /app
COPY Cargo.toml Cargo.lock ./
COPY src/ ./src/
COPY --from=frontend-builder /app/frontend/dist ./frontend/dist
RUN cargo build --release

FROM ghcr.io/vansour/debian:trixie-slim
RUN apt-get update && apt-get install -y ca-certificates curl && rm -rf /var/lib/apt/lists/*
COPY --from=backend-builder /app/target/release/vansour-image /usr/local/bin/
COPY --from=backend-builder /app/frontend/dist /app/frontend/dist
WORKDIR /app
EXPOSE 8080
CMD ["vansour-image"]
