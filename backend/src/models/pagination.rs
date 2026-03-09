use chrono::{DateTime, TimeZone, Utc};
use serde::{Deserialize, Serialize, de};
use uuid::Uuid;

#[derive(Debug, Deserialize)]
#[serde(untagged)]
enum LegacyCursorValue {
    Tuple((DateTime<Utc>, String)),
    Raw(String),
}

fn parse_unix_cursor_time(raw: &str) -> Option<DateTime<Utc>> {
    let value = raw.trim().parse::<i64>().ok()?;
    if value.unsigned_abs() >= 1_000_000_000_000 {
        let seconds = value.div_euclid(1_000);
        let nanos = (value.rem_euclid(1_000) as u32) * 1_000_000;
        Utc.timestamp_opt(seconds, nanos).single()
    } else {
        Utc.timestamp_opt(value, 0).single()
    }
}

fn parse_legacy_cursor(raw: &str) -> Option<(DateTime<Utc>, String)> {
    let (time_raw, id_raw) = raw.split_once(',')?;
    let id = id_raw.trim();
    if id.is_empty() {
        return None;
    }

    let created_at = DateTime::parse_from_rfc3339(time_raw.trim())
        .map(|dt| dt.with_timezone(&Utc))
        .ok()
        .or_else(|| parse_unix_cursor_time(time_raw))?;

    Some((created_at, id.to_string()))
}

fn deserialize_legacy_cursor<'de, D>(
    deserializer: D,
) -> Result<Option<(DateTime<Utc>, String)>, D::Error>
where
    D: serde::Deserializer<'de>,
{
    let value = Option::<LegacyCursorValue>::deserialize(deserializer)?;
    match value {
        None => Ok(None),
        Some(LegacyCursorValue::Tuple(pair)) => Ok(Some(pair)),
        Some(LegacyCursorValue::Raw(raw)) => parse_legacy_cursor(&raw)
            .map(Some)
            .ok_or_else(|| de::Error::custom("invalid cursor format, expected 'time,id'")),
    }
}

#[derive(Debug, Deserialize)]
pub struct PaginationParams {
    pub page: Option<i32>,
    pub page_size: Option<i32>,
    pub sort_by: Option<String>,
    pub sort_order: Option<String>,
    pub search: Option<String>,
    pub category_id: Option<Uuid>,
    pub tag: Option<String>,
    pub cursor_created_at: Option<DateTime<Utc>>,
    pub cursor_id: Option<Uuid>,
    #[serde(default, deserialize_with = "deserialize_legacy_cursor")]
    pub cursor: Option<(DateTime<Utc>, String)>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Paginated<T> {
    pub data: Vec<T>,
    pub page: i32,
    pub page_size: i32,
    pub total: i64,
    pub has_next: bool,
}

#[derive(Debug, Serialize)]
pub struct CursorPaginated<T> {
    pub data: Vec<T>,
    pub next_cursor: Option<(DateTime<Utc>, String)>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_legacy_cursor_unix_seconds() {
        let id = Uuid::new_v4();
        let raw = format!("1700000000,{}", id);
        let parsed = parse_legacy_cursor(&raw).expect("should parse");
        assert_eq!(parsed.0.timestamp(), 1700000000);
        assert_eq!(parsed.1, id.to_string());
    }

    #[test]
    fn test_parse_legacy_cursor_rfc3339() {
        let id = Uuid::new_v4();
        let raw = format!("2026-03-09T12:34:56Z,{}", id);
        let parsed = parse_legacy_cursor(&raw).expect("should parse");
        assert_eq!(parsed.0.to_rfc3339(), "2026-03-09T12:34:56+00:00");
        assert_eq!(parsed.1, id.to_string());
    }

    #[test]
    fn test_pagination_params_deserialize_legacy_cursor_from_string() {
        let id = Uuid::new_v4();
        let value = serde_json::json!({
            "cursor": format!("1700000000,{}", id)
        });
        let parsed: PaginationParams = serde_json::from_value(value).expect("should deserialize");
        let cursor = parsed.cursor.expect("cursor should exist");
        assert_eq!(cursor.0.timestamp(), 1700000000);
        assert_eq!(cursor.1, id.to_string());
    }

    #[test]
    fn test_pagination_params_deserialize_legacy_cursor_invalid() {
        let value = serde_json::json!({
            "cursor": "bad-cursor-value"
        });
        let result = serde_json::from_value::<PaginationParams>(value);
        assert!(result.is_err());
    }
}
