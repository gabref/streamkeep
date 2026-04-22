#![forbid(unsafe_code)]

use m3u8_rs::{
    AlternativeMedia, AlternativeMediaType, MasterPlaylist, Playlist, VariantStream,
    parse_playlist_res,
};
use serde::{Deserialize, Serialize};
use thiserror::Error;
use url::Url;

#[derive(Debug, Error)]
pub enum HlsError {
    #[error("invalid master playlist URL `{value}`")]
    InvalidMasterUrl {
        value: String,
        source: url::ParseError,
    },
    #[error("failed to parse HLS playlist: {reason}")]
    InvalidPlaylist { reason: String },
    #[error("expected a master playlist, got a media playlist")]
    ExpectedMasterPlaylist,
    #[error("failed to resolve playlist URL `{value}` against `{base_url}`")]
    InvalidPlaylistUrl {
        base_url: String,
        value: String,
        source: url::ParseError,
    },
    #[error("master playlist did not contain variant streams")]
    EmptyMasterPlaylist,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ParsedMasterPlaylist {
    pub master_url: String,
    pub variants: Vec<HlsVariant>,
}

impl ParsedMasterPlaylist {
    pub fn quality_options(&self) -> Vec<QualityOption> {
        self.variants
            .iter()
            .map(|variant| QualityOption {
                id: variant.id.clone(),
                label: variant.label.clone(),
                bandwidth: Some(variant.bandwidth),
                width: variant.width,
                height: variant.height,
                media_playlist_url: variant.media_playlist_url.clone(),
            })
            .collect()
    }

    pub fn best_variant(&self) -> Option<&HlsVariant> {
        select_best_variant(&self.variants)
    }

    pub fn best_quality(&self) -> Option<QualityOption> {
        self.best_variant().map(|variant| QualityOption {
            id: variant.id.clone(),
            label: variant.label.clone(),
            bandwidth: Some(variant.bandwidth),
            width: variant.width,
            height: variant.height,
            media_playlist_url: variant.media_playlist_url.clone(),
        })
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct HlsVariant {
    pub id: String,
    pub label: String,
    pub bandwidth: u64,
    pub average_bandwidth: Option<u64>,
    pub width: Option<u32>,
    pub height: Option<u32>,
    pub codecs: Option<String>,
    pub audio_group: Option<String>,
    pub audio_renditions: Vec<AudioRendition>,
    pub media_playlist_url: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AudioRendition {
    pub group_id: String,
    pub name: String,
    pub language: Option<String>,
    pub default: bool,
    pub autoselect: bool,
    pub playlist_url: Option<String>,
}

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

pub fn parse_master_playlist(
    master_url: impl AsRef<str>,
    playlist_body: impl AsRef<[u8]>,
) -> Result<ParsedMasterPlaylist, HlsError> {
    let master_url = master_url.as_ref();
    let base_url = Url::parse(master_url).map_err(|source| HlsError::InvalidMasterUrl {
        value: master_url.to_owned(),
        source,
    })?;

    let parsed =
        parse_playlist_res(playlist_body.as_ref()).map_err(|error| HlsError::InvalidPlaylist {
            reason: error.to_string(),
        })?;

    let Playlist::MasterPlaylist(master_playlist) = parsed else {
        return Err(HlsError::ExpectedMasterPlaylist);
    };

    master_playlist_from_parsed(master_url, &base_url, &master_playlist)
}

pub fn select_best_variant(variants: &[HlsVariant]) -> Option<&HlsVariant> {
    variants.iter().max_by_key(|variant| {
        (
            variant.height.unwrap_or_default(),
            variant.width.unwrap_or_default(),
            variant.average_bandwidth.unwrap_or(variant.bandwidth),
            variant.bandwidth,
        )
    })
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

fn master_playlist_from_parsed(
    master_url: &str,
    base_url: &Url,
    playlist: &MasterPlaylist,
) -> Result<ParsedMasterPlaylist, HlsError> {
    if playlist.variants.is_empty() {
        return Err(HlsError::EmptyMasterPlaylist);
    }

    let variants = playlist
        .variants
        .iter()
        .enumerate()
        .filter(|(_, variant)| !variant.is_i_frame)
        .map(|(index, variant)| variant_from_parsed(master_url, base_url, playlist, index, variant))
        .collect::<Result<Vec<_>, _>>()?;

    if variants.is_empty() {
        return Err(HlsError::EmptyMasterPlaylist);
    }

    Ok(ParsedMasterPlaylist {
        master_url: master_url.to_owned(),
        variants,
    })
}

fn variant_from_parsed(
    master_url: &str,
    base_url: &Url,
    playlist: &MasterPlaylist,
    index: usize,
    variant: &VariantStream,
) -> Result<HlsVariant, HlsError> {
    let width = variant
        .resolution
        .and_then(|resolution| u32::try_from(resolution.width).ok());
    let height = variant
        .resolution
        .and_then(|resolution| u32::try_from(resolution.height).ok());
    let label = quality_label(index, height, variant.bandwidth);
    let media_playlist_url = resolve_url(master_url, base_url, &variant.uri)?;

    Ok(HlsVariant {
        id: format!("variant-{}", index + 1),
        label,
        bandwidth: variant.bandwidth,
        average_bandwidth: variant.average_bandwidth,
        width,
        height,
        codecs: variant.codecs.clone(),
        audio_group: variant.audio.clone(),
        audio_renditions: audio_renditions_for_variant(master_url, base_url, playlist, variant)?,
        media_playlist_url,
    })
}

fn audio_renditions_for_variant(
    master_url: &str,
    base_url: &Url,
    playlist: &MasterPlaylist,
    variant: &VariantStream,
) -> Result<Vec<AudioRendition>, HlsError> {
    let Some(audio_group) = &variant.audio else {
        return Ok(Vec::new());
    };

    playlist
        .alternatives
        .iter()
        .filter(|alternative| {
            alternative.media_type == AlternativeMediaType::Audio
                && alternative.group_id == *audio_group
        })
        .map(|alternative| audio_rendition_from_parsed(master_url, base_url, alternative))
        .collect()
}

fn audio_rendition_from_parsed(
    master_url: &str,
    base_url: &Url,
    alternative: &AlternativeMedia,
) -> Result<AudioRendition, HlsError> {
    Ok(AudioRendition {
        group_id: alternative.group_id.clone(),
        name: alternative.name.clone(),
        language: alternative.language.clone(),
        default: alternative.default,
        autoselect: alternative.autoselect,
        playlist_url: alternative
            .uri
            .as_deref()
            .map(|uri| resolve_url(master_url, base_url, uri))
            .transpose()?,
    })
}

fn resolve_url(master_url: &str, base_url: &Url, value: &str) -> Result<String, HlsError> {
    base_url
        .join(value)
        .map(|url| url.to_string())
        .map_err(|source| HlsError::InvalidPlaylistUrl {
            base_url: master_url.to_owned(),
            value: value.to_owned(),
            source,
        })
}

fn quality_label(index: usize, height: Option<u32>, bandwidth: u64) -> String {
    if let Some(height) = height {
        return format!("{height}p");
    }

    if bandwidth > 0 {
        let mbps = bandwidth as f64 / 1_000_000.0;
        return format!("{mbps:.1} Mbps");
    }

    format!("Variant {}", index + 1)
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
