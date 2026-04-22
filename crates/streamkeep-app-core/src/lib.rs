#![forbid(unsafe_code)]

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct HealthSnapshot {
    pub app_name: String,
    pub app_version: String,
    pub target_platform: String,
}

impl HealthSnapshot {
    pub fn new(
        app_name: impl Into<String>,
        app_version: impl Into<String>,
        target_platform: impl Into<String>,
    ) -> Self {
        Self {
            app_name: app_name.into(),
            app_version: app_version.into(),
            target_platform: target_platform.into(),
        }
    }
}
