#![forbid(unsafe_code)]

use serde::{Deserialize, Serialize};
use time::OffsetDateTime;
use uuid::Uuid;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum DownloadJobStatus {
    Queued,
    Preparing,
    Downloading,
    Remuxing,
    Done,
    Failed,
    Cancelled,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DownloadJobRecord {
    pub id: Uuid,
    pub output_name: String,
    pub status: DownloadJobStatus,
    pub created_at: OffsetDateTime,
    pub updated_at: OffsetDateTime,
}

impl DownloadJobRecord {
    pub fn queued(output_name: impl Into<String>) -> Self {
        let now = OffsetDateTime::now_utc();

        Self {
            id: Uuid::new_v4(),
            output_name: output_name.into(),
            status: DownloadJobStatus::Queued,
            created_at: now,
            updated_at: now,
        }
    }
}
