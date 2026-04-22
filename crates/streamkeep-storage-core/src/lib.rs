#![forbid(unsafe_code)]

use serde::{Deserialize, Serialize};
use time::OffsetDateTime;
use time::format_description::well_known::Rfc3339;
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
    pub title: String,
    pub output_name: String,
    pub page_url: String,
    pub master_url: String,
    pub media_playlist_url: Option<String>,
    pub quality: String,
    pub status: DownloadJobStatus,
    pub progress: u8,
    pub created_at: String,
    pub updated_at: String,
    pub output_path: Option<String>,
    pub output_bytes: Option<u64>,
    pub error_message: Option<String>,
}

impl DownloadJobRecord {
    pub fn queued(
        title: impl Into<String>,
        output_name: impl Into<String>,
        page_url: impl Into<String>,
        master_url: impl Into<String>,
        media_playlist_url: Option<String>,
        quality: impl Into<String>,
    ) -> Self {
        let now = now_timestamp();

        Self {
            id: Uuid::new_v4(),
            title: title.into(),
            output_name: output_name.into(),
            page_url: page_url.into(),
            master_url: master_url.into(),
            media_playlist_url,
            quality: quality.into(),
            status: DownloadJobStatus::Queued,
            progress: 0,
            created_at: now.clone(),
            updated_at: now,
            output_path: None,
            output_bytes: None,
            error_message: None,
        }
    }

    pub fn apply_progress(&mut self, status: DownloadJobStatus, progress: Option<u8>) {
        self.status = status;
        if let Some(progress) = progress {
            self.progress = progress.min(100);
        }
        self.updated_at = now_timestamp();
    }

    pub fn mark_done(&mut self, output_path: impl Into<String>, output_bytes: u64) {
        self.status = DownloadJobStatus::Done;
        self.progress = 100;
        self.output_path = Some(output_path.into());
        self.output_bytes = Some(output_bytes);
        self.error_message = None;
        self.updated_at = now_timestamp();
    }

    pub fn mark_failed(&mut self, error_message: impl Into<String>) {
        self.status = DownloadJobStatus::Failed;
        self.error_message = Some(error_message.into());
        self.updated_at = now_timestamp();
    }
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DownloadHistory {
    pub jobs: Vec<DownloadJobRecord>,
}

impl DownloadHistory {
    pub fn upsert(&mut self, record: DownloadJobRecord) {
        if let Some(existing) = self.jobs.iter_mut().find(|job| job.id == record.id) {
            *existing = record;
        } else {
            self.jobs.push(record);
        }

        self.jobs
            .sort_by(|left, right| right.updated_at.cmp(&left.updated_at));
    }
}

fn now_timestamp() -> String {
    OffsetDateTime::now_utc()
        .format(&Rfc3339)
        .unwrap_or_else(|_| "1970-01-01T00:00:00Z".to_owned())
}

#[cfg(test)]
mod tests {
    use super::{DownloadHistory, DownloadJobRecord, DownloadJobStatus};

    #[test]
    fn upsert_replaces_existing_record() {
        let mut history = DownloadHistory::default();
        let mut record = DownloadJobRecord::queued(
            "Title",
            "Title.mp4",
            "https://example.test/watch",
            "https://example.test/master.m3u8",
            None,
            "Best available",
        );
        let id = record.id;

        history.upsert(record.clone());
        record.mark_failed("network error");
        history.upsert(record);

        assert_eq!(history.jobs.len(), 1);
        assert_eq!(history.jobs[0].id, id);
        assert_eq!(history.jobs[0].status, DownloadJobStatus::Failed);
        assert_eq!(
            history.jobs[0].error_message.as_deref(),
            Some("network error")
        );
    }
}
