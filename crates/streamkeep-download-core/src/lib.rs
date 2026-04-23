#![forbid(unsafe_code)]

use std::fs::File;
use std::io::{BufWriter, Write};
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

use futures_util::StreamExt;
use m3u8_rs::{KeyMethod, Playlist, parse_playlist_res};
use reqwest::Client;
use reqwest::header::{COOKIE, HeaderMap, HeaderValue, REFERER, USER_AGENT};
use serde::{Deserialize, Serialize};
use streamkeep_hls_core::{HlsError, parse_master_playlist};
use thiserror::Error;
use tracing::{debug, info};
use url::Url;

const OUTPUT_BUFFER_SIZE: usize = 1024 * 1024;
const PROGRESS_EMIT_INTERVAL: Duration = Duration::from_millis(180);
const PROGRESS_EMIT_BYTES: u64 = 128 * 1024;

#[derive(Debug, Error)]
pub enum DownloadError {
    #[error(transparent)]
    Hls(#[from] HlsError),
    #[error("failed to parse media playlist: {reason}")]
    InvalidMediaPlaylist { reason: String },
    #[error("expected a media playlist, got a master playlist")]
    ExpectedMediaPlaylist,
    #[error("media playlist did not contain segments")]
    EmptyMediaPlaylist,
    #[error("unsupported media playlist: {reason}")]
    UnsupportedMediaPlaylist { reason: String },
    #[error("invalid media playlist URL `{value}`")]
    InvalidMediaPlaylistUrl {
        value: String,
        source: url::ParseError,
    },
    #[error("failed to resolve segment URL `{value}` against `{base_url}`")]
    InvalidSegmentUrl {
        base_url: String,
        value: String,
        source: url::ParseError,
    },
    #[error("invalid request header `{name}`")]
    InvalidHeader {
        name: &'static str,
        source: reqwest::header::InvalidHeaderValue,
    },
    #[error("http request failed for `{url}`")]
    Http { url: String, source: reqwest::Error },
    #[error("file operation failed for `{path}`")]
    Io {
        path: PathBuf,
        source: std::io::Error,
    },
    #[error("no variant was available in the master playlist")]
    NoVariant,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DownloadRequest {
    pub master_url: String,
    pub media_playlist_url: Option<String>,
    pub referer: Option<String>,
    pub user_agent: Option<String>,
    pub cookies: Option<String>,
    pub output_name: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum DownloadStatus {
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
pub struct DownloadProgress {
    pub status: DownloadStatus,
    pub completed_segments: u32,
    pub total_segments: Option<u32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub current_segment_index: Option<u32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub current_segment_downloaded_bytes: Option<u64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub current_segment_total_bytes: Option<u64>,
    pub downloaded_bytes: u64,
    pub total_bytes: Option<u64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub message: Option<String>,
}

impl DownloadProgress {
    pub fn queued() -> Self {
        Self {
            status: DownloadStatus::Queued,
            completed_segments: 0,
            total_segments: None,
            current_segment_index: None,
            current_segment_downloaded_bytes: None,
            current_segment_total_bytes: None,
            downloaded_bytes: 0,
            total_bytes: None,
            message: None,
        }
    }

    pub fn with_message(mut self, message: impl Into<String>) -> Self {
        self.message = Some(message.into());
        self
    }

    pub fn percent(&self) -> Option<u8> {
        if let Some(total_bytes) = self.total_bytes
            && total_bytes > 0
        {
            return Some(((self.downloaded_bytes.min(total_bytes) * 100) / total_bytes) as u8);
        }

        let total_segments = self.total_segments?;
        if total_segments == 0 {
            return Some(0);
        }

        let current_segment_progress = match (
            self.current_segment_downloaded_bytes,
            self.current_segment_total_bytes,
        ) {
            (Some(downloaded), Some(total)) if total > 0 => {
                downloaded.min(total) as f64 / total as f64
            }
            _ => 0.0,
        };
        let completed = self.completed_segments.min(total_segments) as f64;
        Some((((completed + current_segment_progress) * 100.0) / total_segments as f64) as u8)
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MediaPlaylistPlan {
    pub media_playlist_url: String,
    pub segments: Vec<MediaSegmentPlan>,
    pub target_duration_seconds: u64,
    pub end_list: bool,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MediaSegmentPlan {
    pub index: u32,
    pub url: String,
    pub duration_seconds: f32,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SegmentDownloadResult {
    pub media_playlist_url: String,
    pub transport_stream_path: String,
    pub completed_segments: u32,
    pub downloaded_bytes: u64,
}

pub async fn download_segments_to_transport_stream(
    request: &DownloadRequest,
    output_path: impl AsRef<Path>,
    mut on_progress: impl FnMut(DownloadProgress),
) -> Result<SegmentDownloadResult, DownloadError> {
    info!(
        master_url = %request.master_url,
        media_playlist_url = request.media_playlist_url.as_deref().unwrap_or(""),
        output_path = %output_path.as_ref().display(),
        "starting HLS segment download"
    );
    on_progress(
        DownloadProgress {
            status: DownloadStatus::Preparing,
            ..DownloadProgress::queued()
        }
        .with_message("building http client"),
    );

    let client = Client::builder()
        .default_headers(headers_for_request(request)?)
        .connect_timeout(Duration::from_secs(15))
        .timeout(Duration::from_secs(45))
        .build()
        .map_err(|source| DownloadError::Http {
            url: request.master_url.clone(),
            source,
        })?;

    let media_playlist_url = resolve_media_playlist_url(&client, request, &mut on_progress).await?;
    debug!(%media_playlist_url, "resolved media playlist");
    on_progress(
        DownloadProgress {
            status: DownloadStatus::Preparing,
            ..DownloadProgress::queued()
        }
        .with_message("fetching media playlist"),
    );
    let media_playlist_body = fetch_text(&client, &media_playlist_url).await?;
    debug!(
        %media_playlist_url,
        bytes = media_playlist_body.len(),
        "fetched media playlist"
    );
    on_progress(
        DownloadProgress {
            status: DownloadStatus::Preparing,
            ..DownloadProgress::queued()
        }
        .with_message("parsing media playlist"),
    );
    let plan = parse_media_playlist(&media_playlist_url, media_playlist_body.as_bytes())?;
    info!(
        %media_playlist_url,
        segments = plan.segments.len(),
        target_duration_seconds = plan.target_duration_seconds,
        end_list = plan.end_list,
        "planned media playlist download"
    );
    let total_segments = Some(plan.segments.len() as u32);
    let output_path = output_path.as_ref();
    let mut output = BufWriter::with_capacity(
        OUTPUT_BUFFER_SIZE,
        File::create(output_path).map_err(|source| DownloadError::Io {
            path: output_path.to_path_buf(),
            source,
        })?,
    );
    let mut downloaded_bytes = 0_u64;
    let mut completed_segments = 0_u32;

    on_progress(DownloadProgress {
        status: DownloadStatus::Downloading,
        completed_segments,
        total_segments,
        current_segment_index: None,
        current_segment_downloaded_bytes: None,
        current_segment_total_bytes: None,
        downloaded_bytes,
        total_bytes: None,
        message: Some("writing segments".to_owned()),
    });

    for segment in &plan.segments {
        debug!(
            index = segment.index,
            url = %segment.url,
            "fetching media segment"
        );
        let segment_download = fetch_segment_to_writer(
            &client,
            segment,
            output_path,
            &mut output,
            SegmentProgressContext {
                completed_segments,
                total_segments,
                initial_downloaded_bytes: downloaded_bytes,
            },
            &mut on_progress,
        )
        .await?;
        downloaded_bytes += segment_download.bytes;
        completed_segments += 1;
        debug!(
            index = segment.index,
            segment_bytes = segment_download.bytes,
            completed_segments,
            downloaded_bytes,
            "wrote media segment"
        );
        on_progress(DownloadProgress {
            status: DownloadStatus::Downloading,
            completed_segments,
            total_segments,
            current_segment_index: Some(segment.index),
            current_segment_downloaded_bytes: Some(segment_download.bytes),
            current_segment_total_bytes: segment_download.total_bytes,
            downloaded_bytes,
            total_bytes: None,
            message: Some(format!("downloaded segment {}", segment.index + 1)),
        });
    }

    output.flush().map_err(|source| DownloadError::Io {
        path: output_path.to_path_buf(),
        source,
    })?;
    info!(
        completed_segments,
        downloaded_bytes,
        output_path = %output_path.display(),
        "finished transport stream download"
    );

    on_progress(DownloadProgress {
        status: DownloadStatus::Remuxing,
        completed_segments,
        total_segments,
        current_segment_index: None,
        current_segment_downloaded_bytes: None,
        current_segment_total_bytes: None,
        downloaded_bytes,
        total_bytes: None,
        message: Some("remuxing transport stream".to_owned()),
    });

    Ok(SegmentDownloadResult {
        media_playlist_url,
        transport_stream_path: output_path.to_string_lossy().to_string(),
        completed_segments,
        downloaded_bytes,
    })
}

pub fn parse_media_playlist(
    media_playlist_url: impl AsRef<str>,
    playlist_body: impl AsRef<[u8]>,
) -> Result<MediaPlaylistPlan, DownloadError> {
    let media_playlist_url = media_playlist_url.as_ref();
    let base_url = Url::parse(media_playlist_url).map_err(|source| {
        DownloadError::InvalidMediaPlaylistUrl {
            value: media_playlist_url.to_owned(),
            source,
        }
    })?;
    let parsed = parse_playlist_res(playlist_body.as_ref()).map_err(|error| {
        DownloadError::InvalidMediaPlaylist {
            reason: error.to_string(),
        }
    })?;
    let Playlist::MediaPlaylist(playlist) = parsed else {
        return Err(DownloadError::ExpectedMediaPlaylist);
    };

    if playlist.segments.is_empty() {
        return Err(DownloadError::EmptyMediaPlaylist);
    }

    let mut segments = Vec::with_capacity(playlist.segments.len());
    for (index, segment) in playlist.segments.iter().enumerate() {
        if segment.map.is_some() {
            return Err(DownloadError::UnsupportedMediaPlaylist {
                reason: "fMP4 EXT-X-MAP segments need the MP4 remux path".to_owned(),
            });
        }
        if segment.byte_range.is_some() {
            return Err(DownloadError::UnsupportedMediaPlaylist {
                reason: "byte-range segments are not supported in the MVP".to_owned(),
            });
        }
        if let Some(key) = &segment.key
            && key.method != KeyMethod::None
        {
            return Err(DownloadError::UnsupportedMediaPlaylist {
                reason: "encrypted HLS segments are not supported in the MVP".to_owned(),
            });
        }

        let url = base_url
            .join(&segment.uri)
            .map(|url| url.to_string())
            .map_err(|source| DownloadError::InvalidSegmentUrl {
                base_url: media_playlist_url.to_owned(),
                value: segment.uri.clone(),
                source,
            })?;
        segments.push(MediaSegmentPlan {
            index: u32::try_from(index).unwrap_or(u32::MAX),
            url,
            duration_seconds: segment.duration,
        });
    }

    Ok(MediaPlaylistPlan {
        media_playlist_url: media_playlist_url.to_owned(),
        segments,
        target_duration_seconds: playlist.target_duration,
        end_list: playlist.end_list,
    })
}

async fn resolve_media_playlist_url(
    client: &Client,
    request: &DownloadRequest,
    on_progress: &mut impl FnMut(DownloadProgress),
) -> Result<String, DownloadError> {
    if let Some(media_playlist_url) = &request.media_playlist_url {
        on_progress(
            DownloadProgress {
                status: DownloadStatus::Preparing,
                ..DownloadProgress::queued()
            }
            .with_message("using detected media playlist"),
        );
        return Ok(media_playlist_url.clone());
    }

    on_progress(
        DownloadProgress {
            status: DownloadStatus::Preparing,
            ..DownloadProgress::queued()
        }
        .with_message("fetching master playlist"),
    );
    let master_playlist_body = fetch_text(client, &request.master_url).await?;
    debug!(
        master_url = %request.master_url,
        bytes = master_playlist_body.len(),
        "fetched master playlist"
    );
    on_progress(
        DownloadProgress {
            status: DownloadStatus::Preparing,
            ..DownloadProgress::queued()
        }
        .with_message("selecting best variant"),
    );
    let master = parse_master_playlist(&request.master_url, master_playlist_body.as_bytes())?;
    let variant = master.best_variant().ok_or(DownloadError::NoVariant)?;
    info!(
        master_url = %request.master_url,
        media_playlist_url = %variant.media_playlist_url,
        bandwidth = variant.bandwidth,
        width = variant.width.unwrap_or_default(),
        height = variant.height.unwrap_or_default(),
        "selected HLS variant"
    );
    Ok(variant.media_playlist_url.clone())
}

async fn fetch_text(client: &Client, url: &str) -> Result<String, DownloadError> {
    debug!(%url, "fetching text resource");
    client
        .get(url)
        .send()
        .await
        .map_err(|source| DownloadError::Http {
            url: url.to_owned(),
            source,
        })?
        .error_for_status()
        .map_err(|source| DownloadError::Http {
            url: url.to_owned(),
            source,
        })?
        .text()
        .await
        .map_err(|source| DownloadError::Http {
            url: url.to_owned(),
            source,
        })
}

#[derive(Debug, Clone, Copy)]
struct SegmentDownload {
    bytes: u64,
    total_bytes: Option<u64>,
}

#[derive(Debug, Clone, Copy)]
struct SegmentProgressContext {
    completed_segments: u32,
    total_segments: Option<u32>,
    initial_downloaded_bytes: u64,
}

async fn fetch_segment_to_writer(
    client: &Client,
    segment: &MediaSegmentPlan,
    output_path: &Path,
    output: &mut BufWriter<File>,
    progress_context: SegmentProgressContext,
    on_progress: &mut impl FnMut(DownloadProgress),
) -> Result<SegmentDownload, DownloadError> {
    debug!(url = %segment.url, "fetching byte resource");
    let response = client
        .get(&segment.url)
        .send()
        .await
        .map_err(|source| DownloadError::Http {
            url: segment.url.clone(),
            source,
        })?
        .error_for_status()
        .map_err(|source| DownloadError::Http {
            url: segment.url.clone(),
            source,
        })?;
    let total_bytes = response.content_length();
    let mut stream = response.bytes_stream();
    let mut segment_downloaded_bytes = 0_u64;
    let mut downloaded_bytes = progress_context.initial_downloaded_bytes;
    let mut last_progress_at = Instant::now();
    let mut last_progress_bytes = 0_u64;

    while let Some(chunk) = stream.next().await {
        let chunk = chunk.map_err(|source| DownloadError::Http {
            url: segment.url.clone(),
            source,
        })?;
        output
            .write_all(&chunk)
            .map_err(|source| DownloadError::Io {
                path: output_path.to_path_buf(),
                source,
            })?;
        let chunk_len = chunk.len() as u64;
        segment_downloaded_bytes += chunk_len;
        downloaded_bytes += chunk_len;

        let now = Instant::now();
        if now.duration_since(last_progress_at) >= PROGRESS_EMIT_INTERVAL
            || segment_downloaded_bytes.saturating_sub(last_progress_bytes) >= PROGRESS_EMIT_BYTES
        {
            last_progress_at = now;
            last_progress_bytes = segment_downloaded_bytes;
            on_progress(DownloadProgress {
                status: DownloadStatus::Downloading,
                completed_segments: progress_context.completed_segments,
                total_segments: progress_context.total_segments,
                current_segment_index: Some(segment.index),
                current_segment_downloaded_bytes: Some(segment_downloaded_bytes),
                current_segment_total_bytes: total_bytes,
                downloaded_bytes,
                total_bytes: None,
                message: Some(format!("downloading segment {}", segment.index + 1)),
            });
        }
    }

    Ok(SegmentDownload {
        bytes: segment_downloaded_bytes,
        total_bytes,
    })
}

fn headers_for_request(request: &DownloadRequest) -> Result<HeaderMap, DownloadError> {
    let mut headers = HeaderMap::new();
    insert_header(
        &mut headers,
        USER_AGENT,
        "user-agent",
        request.user_agent.as_deref(),
    )?;
    insert_header(&mut headers, REFERER, "referer", request.referer.as_deref())?;
    insert_header(&mut headers, COOKIE, "cookie", request.cookies.as_deref())?;
    Ok(headers)
}

fn insert_header(
    headers: &mut HeaderMap,
    header: reqwest::header::HeaderName,
    name: &'static str,
    value: Option<&str>,
) -> Result<(), DownloadError> {
    let Some(value) = value else {
        return Ok(());
    };
    let value = value.trim();
    if value.is_empty() {
        return Ok(());
    }
    let header_value = HeaderValue::from_str(value)
        .map_err(|source| DownloadError::InvalidHeader { name, source })?;
    headers.insert(header, header_value);
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::{DownloadProgress, DownloadStatus, parse_media_playlist};

    #[test]
    fn percent_caps_completed_segments_at_total() {
        let progress = DownloadProgress {
            status: DownloadStatus::Downloading,
            completed_segments: 12,
            total_segments: Some(10),
            current_segment_index: None,
            current_segment_downloaded_bytes: None,
            current_segment_total_bytes: None,
            downloaded_bytes: 0,
            total_bytes: None,
            message: None,
        };

        assert_eq!(progress.percent(), Some(100));
    }

    #[test]
    fn percent_prefers_bytes_when_available() {
        let progress = DownloadProgress {
            status: DownloadStatus::Downloading,
            completed_segments: 1,
            total_segments: Some(10),
            current_segment_index: None,
            current_segment_downloaded_bytes: None,
            current_segment_total_bytes: None,
            downloaded_bytes: 75,
            total_bytes: Some(100),
            message: None,
        };

        assert_eq!(progress.percent(), Some(75));
    }

    #[test]
    fn parses_media_playlist_segments() {
        let playlist = r#"#EXTM3U
#EXT-X-TARGETDURATION:8
#EXTINF:8.0,
segment-1.ts
#EXTINF:7.5,
https://cdn.example.test/segment-2.ts
#EXT-X-ENDLIST
"#;

        let plan =
            parse_media_playlist("https://media.example.test/path/prog_index.m3u8", playlist)
                .expect("media playlist should parse");

        assert_eq!(plan.segments.len(), 2);
        assert_eq!(
            plan.segments[0].url,
            "https://media.example.test/path/segment-1.ts"
        );
        assert_eq!(
            plan.segments[1].url,
            "https://cdn.example.test/segment-2.ts"
        );
        assert!(plan.end_list);
    }
}
