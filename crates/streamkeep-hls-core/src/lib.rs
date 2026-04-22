#![forbid(unsafe_code)]

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct QualityOption {
    pub id: String,
    pub label: String,
    pub bandwidth: Option<u64>,
    pub width: Option<u32>,
    pub height: Option<u32>,
    pub media_playlist_url: String,
}

pub fn select_best_quality(options: &[QualityOption]) -> Option<&QualityOption> {
    options.iter().max_by_key(|option| {
        (
            option.height.unwrap_or_default(),
            option.width.unwrap_or_default(),
            option.bandwidth.unwrap_or_default(),
        )
    })
}

#[cfg(test)]
mod tests {
    use super::{QualityOption, select_best_quality};

    #[test]
    fn select_best_quality_prefers_highest_resolution() {
        let options = vec![
            QualityOption {
                id: "low".into(),
                label: "720p".into(),
                bandwidth: Some(2_000_000),
                width: Some(1280),
                height: Some(720),
                media_playlist_url: "low.m3u8".into(),
            },
            QualityOption {
                id: "high".into(),
                label: "1080p".into(),
                bandwidth: Some(4_000_000),
                width: Some(1920),
                height: Some(1080),
                media_playlist_url: "high.m3u8".into(),
            },
        ];

        let selected = select_best_quality(&options).expect("quality should be selected");

        assert_eq!(selected.id, "high");
    }
}
