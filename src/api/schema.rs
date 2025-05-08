use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

/// A wrapper for DateTime<Utc> that implements ToSchema
/// Represents a timestamp in ISO 8601 format
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
#[schema(example = "2023-01-01T12:00:00Z")]
pub struct DateTimeUtc(pub DateTime<Utc>);

impl From<DateTime<Utc>> for DateTimeUtc {
    fn from(dt: DateTime<Utc>) -> Self {
        DateTimeUtc(dt)
    }
}

impl From<DateTimeUtc> for DateTime<Utc> {
    fn from(dt: DateTimeUtc) -> Self {
        dt.0
    }
}
