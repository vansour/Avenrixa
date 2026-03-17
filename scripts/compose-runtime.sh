#!/usr/bin/env bash

if [[ -z "${ROOT_DIR:-}" ]]; then
  ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
fi

: "${COMPOSE_PROJECT_NAME:=avenrixa}"
: "${COMPOSE_VARIANT:=postgres}"
: "${CACHE_MODE:=redis8}"
: "${COMPOSE_ENABLE_MAILPIT:=0}"
: "${APP_HOST_PORT:=8080}"
: "${APP_IMAGE_REF:=ghcr.io/vansour/avenrixa:latest}"
: "${POSTGRES_IMAGE:=postgres:18}"
: "${MAILPIT_HTTP_PORT:=18025}"
: "${MAILPIT_SMTP_PORT:=11025}"
: "${POSTGRES_ENABLE_WAL_ARCHIVE:=0}"
: "${POSTGRES_WAL_ARCHIVE_HOST_DIR:=}"
: "${POSTGRES_WAL_ARCHIVE_MOUNT_PATH:=/wal-archive}"
: "${POSTGRES_WAL_ARCHIVE_TIMEOUT:=60s}"

workspace_package_version() {
  sed -n 's/^version = "\(.*\)"/\1/p' "${ROOT_DIR}/Cargo.toml" | head -n 1
}

compose_variant_uses_mysql() {
  case "${COMPOSE_VARIANT}" in
    mysql|mariadb|mysql-ops|mariadb-ops)
      return 0
      ;;
    *)
      return 1
      ;;
  esac
}

compose_variant_uses_mariadb() {
  case "${COMPOSE_VARIANT}" in
    mariadb|mariadb-ops)
      return 0
      ;;
    *)
      return 1
      ;;
  esac
}

compose_variant_default_data_dir() {
  case "${COMPOSE_VARIANT}" in
    sqlite)
      printf '%s/data-sqlite' "${ROOT_DIR}"
      ;;
    mariadb|mariadb-ops)
      printf '%s/data-mariadb' "${ROOT_DIR}"
      ;;
    mysql|mysql-ops)
      printf '%s/data-mysql' "${ROOT_DIR}"
      ;;
    *)
      printf '%s/data' "${ROOT_DIR}"
      ;;
  esac
}

compose_variant_default_database_url() {
  case "${COMPOSE_VARIANT}" in
    postgres)
      printf 'postgresql://user:pass@postgres:5432/image'
      ;;
    sqlite)
      printf 'sqlite:///data/sqlite/app.db'
      ;;
    mysql)
      printf 'mysql://user:pass@mysql:3306/image'
      ;;
    mariadb)
      printf 'mariadb://user:pass@mysql:3306/image'
      ;;
    mysql-ops)
      printf 'mysql://vansour_image:replace-with-strong-app-password@mysql:3306/image'
      ;;
    mariadb-ops)
      printf 'mariadb://vansour_image:replace-with-strong-app-password@mysql:3306/image'
      ;;
    *)
      echo "Unsupported COMPOSE_VARIANT: ${COMPOSE_VARIANT}" >&2
      return 1
      ;;
  esac
}

compose_resolve_host_path() {
  local path="$1"

  if [[ "${path}" != /* ]]; then
    path="${ROOT_DIR}/${path}"
  fi

  printf '%s' "${path}"
}

compose_remove_host_path() {
  local target="$1"
  local resolved_target
  local helper_image

  resolved_target="$(compose_resolve_host_path "${target}")"

  case "${resolved_target}" in
    ""|"/")
      echo "Refusing to remove unsafe path: ${resolved_target}" >&2
      return 1
      ;;
  esac

  if [[ ! -e "${resolved_target}" ]]; then
    return 0
  fi

  if rm -rf "${resolved_target}" 2>/dev/null; then
    return 0
  fi

  if command -v sudo >/dev/null 2>&1 && sudo -n true >/dev/null 2>&1; then
    sudo rm -rf "${resolved_target}"
    return 0
  fi

  if [[ -d "${resolved_target}" ]] && command -v docker >/dev/null 2>&1; then
    helper_image="${COMPOSE_FS_HELPER_IMAGE:-busybox:1.37.0}"
    docker run --rm \
      -v "${resolved_target}:/target" \
      --entrypoint sh \
      "${helper_image}" \
      -c 'find /target -mindepth 1 -maxdepth 1 -exec rm -rf {} +'
    rmdir "${resolved_target}" 2>/dev/null || true
    rm -rf "${resolved_target}" 2>/dev/null || true
    if [[ ! -e "${resolved_target}" ]]; then
      return 0
    fi
  fi

  echo "Failed to remove host path: ${resolved_target}" >&2
  return 1
}

compose_reset_host_dir() {
  local target="$1"
  local resolved_target

  resolved_target="$(compose_resolve_host_path "${target}")"
  compose_remove_host_path "${resolved_target}"
  mkdir -p "${resolved_target}"
}

compose_host_path_relative_to_dir() {
  local base_dir="$1"
  local target="$2"
  local resolved_base_dir
  local resolved_target

  resolved_base_dir="$(compose_resolve_host_path "${base_dir}")"
  resolved_target="$(compose_resolve_host_path "${target}")"

  case "${resolved_target}" in
    "${resolved_base_dir}"/*)
      printf '%s' "${resolved_target#${resolved_base_dir}/}"
      ;;
    *)
      echo "Path ${resolved_target} is not within base dir ${resolved_base_dir}" >&2
      return 1
      ;;
  esac
}

compose_write_host_file() {
  local base_dir="$1"
  local target="$2"
  local content="$3"
  local resolved_base_dir
  local resolved_target
  local relative_target
  local helper_image

  resolved_base_dir="$(compose_resolve_host_path "${base_dir}")"
  resolved_target="$(compose_resolve_host_path "${target}")"
  relative_target="$(compose_host_path_relative_to_dir "${resolved_base_dir}" "${resolved_target}")"

  mkdir -p "$(dirname "${resolved_target}")" 2>/dev/null || true
  if (printf '%s\n' "${content}" > "${resolved_target}") 2>/dev/null; then
    return 0
  fi

  helper_image="${COMPOSE_FS_HELPER_IMAGE:-busybox:1.37.0}"
  printf '%s\n' "${content}" | docker run --rm -i \
    -v "${resolved_base_dir}:/target" \
    -e RELATIVE_TARGET="${relative_target}" \
    --entrypoint sh \
    "${helper_image}" \
    -lc 'set -eu; mkdir -p "$(dirname "/target/${RELATIVE_TARGET}")"; cat > "/target/${RELATIVE_TARGET}"'
}

compose_read_host_file() {
  local base_dir="$1"
  local target="$2"
  local resolved_base_dir
  local resolved_target
  local relative_target
  local helper_image

  resolved_base_dir="$(compose_resolve_host_path "${base_dir}")"
  resolved_target="$(compose_resolve_host_path "${target}")"
  relative_target="$(compose_host_path_relative_to_dir "${resolved_base_dir}" "${resolved_target}")"

  if cat "${resolved_target}" 2>/dev/null; then
    return 0
  fi

  helper_image="${COMPOSE_FS_HELPER_IMAGE:-busybox:1.37.0}"
  docker run --rm \
    -v "${resolved_base_dir}:/target" \
    -e RELATIVE_TARGET="${relative_target}" \
    --entrypoint sh \
    "${helper_image}" \
    -lc 'set -eu; cat "/target/${RELATIVE_TARGET}"'
}

compose_read_optional_host_file() {
  local base_dir="$1"
  local target="$2"
  local resolved_base_dir
  local resolved_target
  local relative_target
  local helper_image

  resolved_base_dir="$(compose_resolve_host_path "${base_dir}")"
  resolved_target="$(compose_resolve_host_path "${target}")"
  relative_target="$(compose_host_path_relative_to_dir "${resolved_base_dir}" "${resolved_target}")"

  if [[ -f "${resolved_target}" ]] && (tr -d '\r' < "${resolved_target}") 2>/dev/null; then
    return 0
  fi

  helper_image="${COMPOSE_FS_HELPER_IMAGE:-busybox:1.37.0}"
  docker run --rm \
    -v "${resolved_base_dir}:/target:ro" \
    -e RELATIVE_TARGET="${relative_target}" \
    --entrypoint sh \
    "${helper_image}" \
    -lc 'set -eu; target="/target/${RELATIVE_TARGET}"; if [ -f "${target}" ]; then tr -d "\r" < "${target}"; fi'
}

compose_directory_size_bytes() {
  local base_dir="$1"
  local target="$2"
  local resolved_base_dir
  local resolved_target
  local relative_target
  local helper_image
  local size

  resolved_base_dir="$(compose_resolve_host_path "${base_dir}")"
  resolved_target="$(compose_resolve_host_path "${target}")"
  relative_target="$(compose_host_path_relative_to_dir "${resolved_base_dir}" "${resolved_target}")"

  size="$(du -sb "${resolved_target}" 2>/dev/null | awk '{print $1}')" || true
  if [[ -n "${size}" ]]; then
    printf '%s' "${size}"
    return 0
  fi

  helper_image="${COMPOSE_FS_HELPER_IMAGE:-busybox:1.37.0}"
  docker run --rm \
    -v "${resolved_base_dir}:/target:ro" \
    -e RELATIVE_TARGET="${relative_target}" \
    --entrypoint sh \
    "${helper_image}" \
    -lc 'set -eu; du -sb "/target/${RELATIVE_TARGET}" | awk "{print \$1}"'
}

compose_variant_default_postgres_wal_archive_host_dir() {
  printf '%s/ops-backups/postgres-wal-archive' "${ROOT_DIR}"
}

compose_variant_resolved_postgres_wal_archive_host_dir() {
  local host_dir

  host_dir="${POSTGRES_WAL_ARCHIVE_HOST_DIR:-$(compose_variant_default_postgres_wal_archive_host_dir)}"
  if [[ "${host_dir}" != /* ]]; then
    host_dir="${ROOT_DIR}/${host_dir}"
  fi

  printf '%s' "${host_dir}"
}

compose_runtime_file_path() {
  local safe_project_name
  safe_project_name="$(printf '%s' "${COMPOSE_PROJECT_NAME}" | tr -c 'A-Za-z0-9._-' '-')"
  printf '/tmp/%s.compose.generated.yml' "${safe_project_name}"
}

yaml_double_quote() {
  local value="$1"
  value="${value//\\/\\\\}"
  value="${value//\"/\\\"}"
  printf '"%s"' "${value}"
}

compose_runtime_file="${COMPOSE_RUNTIME_FILE:-$(compose_runtime_file_path)}"
compose_files=("compose.yml")

compose_runtime_generate() {
  local app_container_name
  local app_data_dir
  local database_kind
  local database_url
  local jwt_secret
  local auth_cookie_secure
  local auth_cookie_same_site
  local app_depends_on
  local database_service_block=""
  local cache_service_block=""
  local mailpit_service_block=""
  local volumes_block=""
  local cache_url
  local app_database_url
  local app_redis_url
  local app_image_ref
  local app_version
  local app_revision
  local build_date
  local database_url_yaml
  local redis_url_yaml
  local jwt_secret_yaml
  local auth_cookie_secure_yaml
  local auth_cookie_same_site_yaml
  local app_image_ref_yaml
  local app_version_yaml
  local app_revision_yaml
  local build_date_yaml
  local postgres_volume_lines=""
  local postgres_command_block=""
  local postgres_wal_archive_host_dir=""
  local postgres_wal_archive_mount_path="${POSTGRES_WAL_ARCHIVE_MOUNT_PATH}"
  local postgres_archive_command=""
  local postgres_wal_archive_volume_yaml=""

  app_data_dir="${DATA_DIR:-${MYSQL_DATA_DIR:-$(compose_variant_default_data_dir)}}"
  if [[ "${app_data_dir}" != /* ]]; then
    app_data_dir="${ROOT_DIR}/${app_data_dir}"
  fi
  app_image_ref="${APP_IMAGE_REF}"
  app_version="${APP_VERSION:-$(workspace_package_version)}"
  app_revision="${APP_REVISION:-dev}"
  build_date="${BUILD_DATE:-unknown}"
  database_url="$(compose_variant_default_database_url)"
  jwt_secret="${JWT_SECRET:-your-secret-key-change-in-production}"
  auth_cookie_same_site="${AUTH_COOKIE_SAME_SITE:-Strict}"
  auth_cookie_secure="${AUTH_COOKIE_SECURE:-false}"

  case "${COMPOSE_VARIANT}" in
    postgres)
      app_container_name="vansour-image-app"
      database_kind="postgresql"
      app_depends_on=$'    depends_on:\n      postgres:\n        condition: service_healthy'
      volumes_block=$'volumes:\n  postgres_data:\n'
      postgres_volume_lines='      - "postgres_data:/var/lib/postgresql"'
      if [[ "${POSTGRES_ENABLE_WAL_ARCHIVE}" == "1" ]]; then
        postgres_wal_archive_host_dir="$(compose_variant_resolved_postgres_wal_archive_host_dir)"
        mkdir -p "${postgres_wal_archive_host_dir}"
        chmod 0777 "${postgres_wal_archive_host_dir}"
        postgres_archive_command="test ! -f ${postgres_wal_archive_mount_path}/%f && cp %p ${postgres_wal_archive_mount_path}/%f || test -f ${postgres_wal_archive_mount_path}/%f"
        postgres_wal_archive_volume_yaml="$(yaml_double_quote "${postgres_wal_archive_host_dir}:${postgres_wal_archive_mount_path}")"
        postgres_volume_lines+=$'\n'"      - ${postgres_wal_archive_volume_yaml}"
        postgres_command_block=$(cat <<EOF
    command:
      - postgres
      - -c
      - wal_level=replica
      - -c
      - archive_mode=on
      - -c
      - 'archive_timeout=${POSTGRES_WAL_ARCHIVE_TIMEOUT}'
      - -c
      - 'archive_command=${postgres_archive_command}'
EOF
)
        postgres_command_block+=$'\n'
      fi
      database_service_block=$(cat <<EOF
  postgres:
    container_name: vansour-image-postgres
    image: ${POSTGRES_IMAGE}
    environment:
      POSTGRES_DB: image
      POSTGRES_USER: user
      POSTGRES_PASSWORD: pass
    volumes:
${postgres_volume_lines}
${postgres_command_block}    healthcheck:
      test: pg_isready -U user -d image
      interval: 5s
      timeout: 5s
      retries: 5
    restart: unless-stopped
EOF
)
      database_service_block+=$'\n'
      ;;
    sqlite)
      app_container_name="vansour-image-sqlite-app"
      database_kind="sqlite"
      app_depends_on=""
      volumes_block=""
      ;;
    mysql)
      app_container_name="vansour-image-mysql-app"
      database_kind="mysql"
      app_depends_on=$'    depends_on:\n      mysql:\n        condition: service_healthy'
      volumes_block=$'volumes:\n  mysql_data:\n'
      database_service_block=$'  mysql:\n    container_name: vansour-image-mysql\n    image: mysql:8.4\n    environment:\n      MYSQL_DATABASE: image\n      MYSQL_USER: user\n      MYSQL_PASSWORD: pass\n      MYSQL_ROOT_PASSWORD: rootpass\n    volumes:\n      - "mysql_data:/var/lib/mysql"\n    healthcheck:\n      test: mysqladmin ping -h 127.0.0.1 -uuser -ppass\n      interval: 5s\n      timeout: 5s\n      retries: 10\n    restart: unless-stopped\n'
      ;;
    mariadb)
      app_container_name="vansour-image-mariadb-app"
      database_kind="mysql"
      app_depends_on=$'    depends_on:\n      mysql:\n        condition: service_healthy'
      volumes_block=$'volumes:\n  mariadb_data:\n'
      database_service_block=$'  mysql:\n    container_name: vansour-image-mariadb\n    image: mariadb:12\n    environment:\n      MARIADB_DATABASE: image\n      MARIADB_USER: user\n      MARIADB_PASSWORD: pass\n      MARIADB_ROOT_PASSWORD: rootpass\n      MYSQL_DATABASE: image\n      MYSQL_USER: user\n      MYSQL_PASSWORD: pass\n      MYSQL_ROOT_PASSWORD: rootpass\n    volumes:\n      - "mariadb_data:/var/lib/mysql"\n    healthcheck:\n      test:\n        - CMD-SHELL\n        - mariadb-admin ping -h 127.0.0.1 -uuser -ppass || mysqladmin ping -h 127.0.0.1 -uuser -ppass\n      interval: 5s\n      timeout: 5s\n      retries: 10\n    restart: unless-stopped\n'
      ;;
    mysql-ops)
      app_container_name="vansour-image-mysql-app"
      database_kind="mysql"
      app_depends_on=$'    depends_on:\n      mysql:\n        condition: service_healthy'
      volumes_block=$'volumes:\n  mysql_data:\n'
      auth_cookie_secure="${AUTH_COOKIE_SECURE:-true}"
      jwt_secret="${JWT_SECRET:-replace-with-a-random-secret-at-least-32-characters}"
      database_service_block=$'  mysql:\n    container_name: vansour-image-mysql\n    image: mysql:8.4\n    environment:\n      MYSQL_DATABASE: image\n      MYSQL_USER: vansour_image\n      MYSQL_PASSWORD: replace-with-strong-app-password\n      MYSQL_ROOT_PASSWORD: replace-with-strong-root-password\n    volumes:\n      - "mysql_data:/var/lib/mysql"\n    healthcheck:\n      test: mysqladmin ping -h 127.0.0.1 -uvansour_image -preplace-with-strong-app-password\n      interval: 5s\n      timeout: 5s\n      retries: 10\n    restart: unless-stopped\n'
      ;;
    mariadb-ops)
      app_container_name="vansour-image-mariadb-app"
      database_kind="mysql"
      app_depends_on=$'    depends_on:\n      mysql:\n        condition: service_healthy'
      volumes_block=$'volumes:\n  mariadb_data:\n'
      auth_cookie_secure="${AUTH_COOKIE_SECURE:-true}"
      jwt_secret="${JWT_SECRET:-replace-with-a-random-secret-at-least-32-characters}"
      database_service_block=$'  mysql:\n    container_name: vansour-image-mariadb\n    image: mariadb:12\n    environment:\n      MARIADB_DATABASE: image\n      MARIADB_USER: vansour_image\n      MARIADB_PASSWORD: replace-with-strong-app-password\n      MARIADB_ROOT_PASSWORD: replace-with-strong-root-password\n      MYSQL_DATABASE: image\n      MYSQL_USER: vansour_image\n      MYSQL_PASSWORD: replace-with-strong-app-password\n      MYSQL_ROOT_PASSWORD: replace-with-strong-root-password\n    volumes:\n      - "mariadb_data:/var/lib/mysql"\n    healthcheck:\n      test:\n        - CMD-SHELL\n        - mariadb-admin ping -h 127.0.0.1 -uvansour_image -preplace-with-strong-app-password || mysqladmin ping -h 127.0.0.1 -uvansour_image -preplace-with-strong-app-password\n      interval: 5s\n      timeout: 5s\n      retries: 10\n    restart: unless-stopped\n'
      ;;
    *)
      echo "Unsupported COMPOSE_VARIANT: ${COMPOSE_VARIANT}" >&2
      return 1
      ;;
  esac

  if [[ "${CACHE_MODE}" == "none" ]]; then
    cache_url=""
  else
    cache_url="redis://cache:6379"
    if [[ -z "${app_depends_on}" ]]; then
      app_depends_on=$'    depends_on:'
    fi
    app_depends_on="${app_depends_on}"$'\n      cache:\n        condition: service_healthy'
  fi

  if [[ -v DATABASE_URL ]]; then
    app_database_url="${DATABASE_URL}"
  else
    app_database_url="${database_url}"
  fi

  if [[ -v REDIS_URL ]]; then
    app_redis_url="${REDIS_URL}"
  else
    app_redis_url="${cache_url}"
  fi

  database_url_yaml="$(yaml_double_quote "${app_database_url}")"
  redis_url_yaml="$(yaml_double_quote "${app_redis_url}")"
  jwt_secret_yaml="$(yaml_double_quote "${jwt_secret}")"
  auth_cookie_secure_yaml="$(yaml_double_quote "${auth_cookie_secure}")"
  auth_cookie_same_site_yaml="$(yaml_double_quote "${auth_cookie_same_site}")"
  app_image_ref_yaml="$(yaml_double_quote "${app_image_ref}")"
  app_version_yaml="$(yaml_double_quote "${app_version}")"
  app_revision_yaml="$(yaml_double_quote "${app_revision}")"
  build_date_yaml="$(yaml_double_quote "${build_date}")"

  case "${CACHE_MODE}" in
    redis8)
      cache_service_block=$'  cache:\n    container_name: vansour-image-cache\n    image: redis:8\n    healthcheck:\n      test: redis-cli ping\n      interval: 5s\n      timeout: 5s\n      retries: 5\n    restart: unless-stopped\n'
      ;;
    dragonfly)
      cache_service_block=$'  cache:\n    container_name: vansour-image-cache\n    image: ghcr.io/dragonflydb/dragonfly:latest\n    command: --dir=/data\n    healthcheck:\n      test: redis-cli ping\n      interval: 5s\n      timeout: 5s\n      retries: 5\n    restart: unless-stopped\n'
      ;;
    none)
      cache_service_block=""
      ;;
    *)
      echo "Unsupported CACHE_MODE: ${CACHE_MODE}" >&2
      return 1
      ;;
  esac

  if [[ "${COMPOSE_ENABLE_MAILPIT}" == "1" ]]; then
    mailpit_service_block=$(cat <<EOF
  mailpit:
    container_name: vansour-image-sqlite-mailpit
    image: axllent/mailpit:latest
    ports:
      - "${MAILPIT_HTTP_PORT}:8025"
      - "${MAILPIT_SMTP_PORT}:1025"
    healthcheck:
      test: wget --quiet --output-document=- http://localhost:8025/api/v1/info || exit 1
      interval: 5s
      timeout: 5s
      retries: 10
    restart: unless-stopped
EOF
)
      mailpit_service_block+=$'\n'
    fi

  mkdir -p "$(dirname "${compose_runtime_file}")"
  cat > "${compose_runtime_file}" <<EOF
${volumes_block}services:
  app:
    container_name: ${app_container_name}
    build:
      context: ${ROOT_DIR}
      dockerfile: Dockerfile
      args:
        APP_VERSION: ${app_version_yaml}
        APP_REVISION: ${app_revision_yaml}
        BUILD_DATE: ${build_date_yaml}
    image: ${app_image_ref_yaml}
    ports:
      - "${APP_HOST_PORT}:8080"
${app_depends_on}
    environment:
      DATABASE_KIND: ${database_kind}
      DATABASE_URL: ${database_url_yaml}
      REDIS_URL: ${redis_url_yaml}
      JWT_SECRET: ${jwt_secret_yaml}
      AUTH_COOKIE_SECURE: ${auth_cookie_secure_yaml}
      AUTH_COOKIE_SAME_SITE: ${auth_cookie_same_site_yaml}
    volumes:
      - "${app_data_dir}:/data"
    restart: unless-stopped
    healthcheck:
      test: curl -f http://localhost:8080/health || exit 1
      interval: 30s
      timeout: 10s
      retries: 3
      start_period: 40s
${database_service_block}${cache_service_block}${mailpit_service_block}
EOF
}

compose() {
  compose_runtime_generate
  docker compose -p "${COMPOSE_PROJECT_NAME}" -f "${compose_runtime_file}" "$@"
}

if [[ "${BASH_SOURCE[0]}" == "$0" ]]; then
  set -euo pipefail

  if [[ $# -eq 0 ]]; then
    echo "Usage: $0 <docker-compose-args...>" >&2
    echo "   or: $0 path" >&2
    exit 1
  fi

  if [[ "$1" == "path" ]]; then
    compose_runtime_generate
    printf '%s\n' "${compose_runtime_file}"
    exit 0
  fi

  compose "$@"
fi
