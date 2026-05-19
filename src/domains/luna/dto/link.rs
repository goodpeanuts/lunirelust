use chrono::NaiveDate;
use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;
use validator::Validate;

use crate::domains::luna::domain::Link;

// Keep deserialization aligned with the link-placeholder contract so omitted
// names enter the system as the same sentinel used by update-mode backfill.
fn default_link_name() -> String {
    "None".to_owned()
}

// Link DTOs
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct LinkDto {
    pub id: i64,
    pub record_id: String,
    pub name: String,
    #[schema(value_type = String)]
    pub size: Decimal,
    pub date: Date,
    pub link: String,
    pub star: bool,
}

impl From<Link> for LinkDto {
    fn from(link: Link) -> Self {
        Self {
            id: link.id,
            record_id: link.record_id,
            name: link.name,
            size: link.size,
            date: link.date,
            link: link.link,
            star: link.star,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema, Validate)]
pub struct CreateLinkDto {
    #[serde(default = "default_link_name")]
    #[validate(length(max = 255, message = "Name cannot exceed 255 characters"))]
    pub name: String,
    #[schema(value_type = String)]
    pub size: Option<Decimal>,
    #[serde(default, deserialize_with = "deserialize_option_date_or_none")]
    pub date: Option<Date>,
    #[validate(length(min = 1, message = "Link URL is required"))]
    pub link: String,
    pub star: Option<bool>,
}

/// Deserialize optional date string into Option<Date>.
/// - Missing field -> None
/// - Empty string -> None
/// - Valid YYYY-MM-DD -> Some(date)
/// - Invalid date -> None
fn deserialize_option_date_or_none<'de, D>(deserializer: D) -> Result<Option<Date>, D::Error>
where
    D: serde::Deserializer<'de>,
{
    let opt = Option::<String>::deserialize(deserializer)?;
    match opt {
        None => Ok(None),
        Some(s) => {
            let s_trim = s.trim();
            if s_trim.is_empty() {
                return Ok(None);
            }
            match NaiveDate::parse_from_str(s_trim, "%Y-%m-%d") {
                Ok(d) => Ok(Some(d)),
                Err(_) => Ok(None),
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::CreateLinkDto;

    #[test]
    fn create_link_dto_defaults_missing_name_to_none_sentinel() {
        // Manual link payloads may omit `name`; they should still deserialize
        // into the canonical placeholder value instead of failing early.
        let dto: CreateLinkDto = serde_json::from_str(
            r#"{
                "size": "1.5",
                "date": "2025-08-11",
                "link": "https://example.com/magnet",
                "star": true
            }"#,
        )
        .expect("CreateLinkDto should deserialize without name");

        assert_eq!(dto.name, "None");
    }

    #[test]
    fn create_link_dto_rejects_missing_link() {
        // `link` is the stable identity for dedup/update logic, so requests
        // without it must still fail at the DTO boundary.
        let result = serde_json::from_str::<CreateLinkDto>(
            r#"{
                "name": "test",
                "size": "1.5",
                "date": "2025-08-11"
            }"#,
        );
        assert!(result.is_err());
    }
}
