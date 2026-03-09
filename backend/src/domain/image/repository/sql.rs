pub(super) const IMAGE_SELECT_COLUMNS: &str = concat!(
    "id, user_id, category_id, filename, thumbnail, original_filename, ",
    "size, hash, format, views, status, expires_at, deleted_at, created_at"
);

pub(super) const IMAGE_SELECT_WITH_TOTAL_COUNT: &str = concat!(
    "id, user_id, category_id, filename, thumbnail, original_filename, ",
    "size, hash, format, views, status, expires_at, deleted_at, created_at, ",
    "COUNT(*) OVER() AS total_count"
);

pub(super) const CATEGORY_SELECT_COLUMNS: &str = "id, user_id, name, created_at";
