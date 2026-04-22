#![forbid(unsafe_code)]

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DownloadProgress {
    pub completed_segments: u32,
    pub total_segments: Option<u32>,
}

impl DownloadProgress {
    pub fn percent(&self) -> Option<u8> {
        let total_segments = self.total_segments?;

        if total_segments == 0 {
            return Some(0);
        }

        Some(((self.completed_segments.min(total_segments) * 100) / total_segments) as u8)
    }
}

#[cfg(test)]
mod tests {
    use super::DownloadProgress;

    #[test]
    fn percent_caps_completed_segments_at_total() {
        let progress = DownloadProgress {
            completed_segments: 12,
            total_segments: Some(10),
        };

        assert_eq!(progress.percent(), Some(100));
    }
}
