pub(super) const IMAGE_SELECT_COLUMNS: &str = concat!(
    "id, user_id, category_id, filename, thumbnail, original_filename, ",
    "size, hash, format, views, status, expires_at, deleted_at, created_at"
);

pub(super) const IMAGE_SELECT_WITH_TOTAL_COUNT: &str = concat!(
    "id, user_id, category_id, filename, thumbnail, original_filename, ",
    "size, hash, format, views, status, expires_at, deleted_at, created_at, ",
    "COUNT(*) OVER() AS total_count"
);

pub(super) const MYSQL_IMAGE_SELECT_WITH_TOTAL_COUNT: &str = concat!(
    "id, user_id, category_id, filename, thumbnail, original_filename, ",
    "size, hash, format, views, status, expires_at, deleted_at, created_at, ",
    "CAST(COUNT(*) OVER() AS SIGNED) AS total_count"
);
