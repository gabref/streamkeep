#![forbid(unsafe_code)]

use std::fs::File;
use std::io::Write;
use std::path::{Path, PathBuf};

use futures_util::StreamExt;
use m3u8_rs::{KeyMethod, Playlist, parse_playlist_res};
use reqwest::Client;
use reqwest::header::{COOKIE, HeaderMap, HeaderValue, REFERER, USER_AGENT};
use serde::{Deserialize, Serialize};
use streamkeep_hls_core::{HlsError, parse_master_playlist};
use thiserror::Error;
use url::Url;

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
    pub downloaded_bytes: u64,
    pub total_bytes: Option<u64>,
}

impl DownloadProgress {
    pub fn queued() -> Self {
        Self {
            status: DownloadStatus::Queued,
            completed_segments: 0,
            total_segments: None,
            downloaded_bytes: 0,
            total_bytes: None,
        }
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

        Some(((self.completed_segments.min(total_segments) * 100) / total_segments) as u8)
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
    on_progress(DownloadProgress {
        status: DownloadStatus::Preparing,
        ..DownloadProgress::queued()
    });

    let client = Client::builder()
        .default_headers(headers_for_request(request)?)
        .build()
        .map_err(|source| DownloadError::Http {
            url: request.master_url.clone(),
            source,
        })?;

    let media_playlist_url = resolve_media_playlist_url(&client, request).await?;
    let media_playlist_body = fetch_text(&client, &media_playlist_url).await?;
    let plan = parse_media_playlist(&media_playlist_url, media_playlist_body.as_bytes())?;
    let total_segments = Some(plan.segments.len() as u32);
    let output_path = output_path.as_ref();
    let mut output = File::create(output_path).map_err(|source| DownloadError::Io {
        path: output_path.to_path_buf(),
        source,
    })?;
    let mut downloaded_bytes = 0_u64;
    let mut completed_segments = 0_u32;

    on_progress(DownloadProgress {
        status: DownloadStatus::Downloading,
        completed_segments,
        total_segments,
        downloaded_bytes,
        total_bytes: None,
    });

    for segment in &plan.segments {
        let bytes = fetch_bytes(&client, &segment.url).await?;
        output
            .write_all(&bytes)
            .map_err(|source| DownloadError::Io {
                path: output_path.to_path_buf(),
                source,
            })?;
        downloaded_bytes += bytes.len() as u64;
        completed_segments += 1;
        on_progress(DownloadProgress {
            status: DownloadStatus::Downloading,
            completed_segments,
            total_segments,
            downloaded_bytes,
            total_bytes: None,
        });
    }

    output.flush().map_err(|source| DownloadError::Io {
        path: output_path.to_path_buf(),
        source,
    })?;

    on_progress(DownloadProgress {
        status: DownloadStatus::Remuxing,
        completed_segments,
        total_segments,
        downloaded_bytes,
        total_bytes: None,
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
) -> Result<String, DownloadError> {
    if let Some(media_playlist_url) = &request.media_playlist_url {
        return Ok(media_playlist_url.clone());
    }

    let master_playlist_body = fetch_text(client, &request.master_url).await?;
    let master = parse_master_playlist(&request.master_url, master_playlist_body.as_bytes())?;
    let variant = master.best_variant().ok_or(DownloadError::NoVariant)?;
    Ok(variant.media_playlist_url.clone())
}

async fn fetch_text(client: &Client, url: &str) -> Result<String, DownloadError> {
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

async fn fetch_bytes(client: &Client, url: &str) -> Result<bytes::Bytes, DownloadError> {
    let response = client
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
        })?;
    let mut stream = response.bytes_stream();
    let mut bytes = bytes::BytesMut::new();

    while let Some(chunk) = stream.next().await {
        let chunk = chunk.map_err(|source| DownloadError::Http {
            url: url.to_owned(),
            source,
        })?;
        bytes.extend_from_slice(&chunk);
    }

    Ok(bytes.freeze())
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
            downloaded_bytes: 0,
            total_bytes: None,
        };

        assert_eq!(progress.percent(), Some(100));
    }

    #[test]
    fn percent_prefers_bytes_when_available() {
        let progress = DownloadProgress {
            status: DownloadStatus::Downloading,
            completed_segments: 1,
            total_segments: Some(10),
            downloaded_bytes: 75,
            total_bytes: Some(100),
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
