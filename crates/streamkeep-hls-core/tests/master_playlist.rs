use streamkeep_hls_core::{HlsError, parse_master_playlist};

const MASTER_PLAYLIST: &str = r#"#EXTM3U
#EXT-X-VERSION:3
#EXT-X-MEDIA:TYPE=AUDIO,GROUP-ID="audio-main",LANGUAGE="en",NAME="English",DEFAULT=YES,AUTOSELECT=YES,URI="audio/en/prog_index.m3u8"
#EXT-X-MEDIA:TYPE=AUDIO,GROUP-ID="audio-main",LANGUAGE="es",NAME="Spanish",DEFAULT=NO,AUTOSELECT=YES,URI="audio/es/prog_index.m3u8"
#EXT-X-STREAM-INF:BANDWIDTH=2400000,AVERAGE-BANDWIDTH=2100000,RESOLUTION=1280x720,CODECS="avc1.64001f,mp4a.40.2",AUDIO="audio-main"
low/prog_index.m3u8
#EXT-X-STREAM-INF:BANDWIDTH=5800000,AVERAGE-BANDWIDTH=5200000,RESOLUTION=1920x1080,CODECS="avc1.640028,mp4a.40.2",AUDIO="audio-main"
https://cdn.example.test/high/prog_index.m3u8
"#;

#[test]
fn parses_master_variants_and_resolves_playlist_urls() {
    let playlist = parse_master_playlist(
        "https://media.example.test/show/master.m3u8",
        MASTER_PLAYLIST,
    )
    .expect("master playlist should parse");

    assert_eq!(playlist.variants.len(), 2);
    assert_eq!(playlist.variants[0].label, "720p");
    assert_eq!(
        playlist.variants[0].media_playlist_url,
        "https://media.example.test/show/low/prog_index.m3u8"
    );
    assert_eq!(
        playlist.variants[1].media_playlist_url,
        "https://cdn.example.test/high/prog_index.m3u8"
    );
}

#[test]
fn extracts_audio_renditions_for_variant_group() {
    let playlist = parse_master_playlist(
        "https://media.example.test/show/master.m3u8",
        MASTER_PLAYLIST,
    )
    .expect("master playlist should parse");

    let first_variant = &playlist.variants[0];

    assert_eq!(first_variant.audio_group.as_deref(), Some("audio-main"));
    assert_eq!(first_variant.audio_renditions.len(), 2);
    assert_eq!(
        first_variant.audio_renditions[0].language.as_deref(),
        Some("en")
    );
    assert_eq!(
        first_variant.audio_renditions[0].playlist_url.as_deref(),
        Some("https://media.example.test/show/audio/en/prog_index.m3u8")
    );
}

#[test]
fn selects_best_variant_by_resolution_and_bandwidth() {
    let playlist = parse_master_playlist(
        "https://media.example.test/show/master.m3u8",
        MASTER_PLAYLIST,
    )
    .expect("master playlist should parse");

    let selected = playlist
        .best_variant()
        .expect("best variant should be selected");

    assert_eq!(selected.label, "1080p");
    assert_eq!(selected.bandwidth, 5_800_000);
}

#[test]
fn rejects_media_playlist_when_master_is_required() {
    let media_playlist = r#"#EXTM3U
#EXT-X-TARGETDURATION:8
#EXTINF:8.0,
segment-1.ts
"#;

    let error = parse_master_playlist(
        "https://media.example.test/show/master.m3u8",
        media_playlist,
    )
    .expect_err("media playlist should be rejected");

    assert!(matches!(error, HlsError::ExpectedMasterPlaylist));
}
