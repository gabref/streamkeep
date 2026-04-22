#![forbid(unsafe_code)]

const FALLBACK_TITLE: &str = "streamkeep-video";

pub fn sanitize_file_stem(input: &str) -> String {
    let normalized = input
        .trim()
        .chars()
        .map(|character| match character {
            '<' | '>' | ':' | '"' | '/' | '\\' | '|' | '?' | '*' => '-',
            character if character.is_control() => '-',
            character => character,
        })
        .collect::<String>();

    let collapsed = normalized
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
        .trim_matches(['.', ' ', '-'])
        .to_string();

    if collapsed.is_empty() {
        FALLBACK_TITLE.to_string()
    } else {
        collapsed
    }
}

pub fn ensure_mp4_extension(file_stem: &str) -> String {
    let sanitized = sanitize_file_stem(file_stem);

    if sanitized.to_ascii_lowercase().ends_with(".mp4") {
        sanitized
    } else {
        format!("{sanitized}.mp4")
    }
}

#[cfg(test)]
mod tests {
    use super::{ensure_mp4_extension, sanitize_file_stem};

    #[test]
    fn sanitize_file_stem_replaces_invalid_path_characters() {
        assert_eq!(
            sanitize_file_stem("Episode: 1 / Intro?"),
            "Episode- 1 - Intro"
        );
    }

    #[test]
    fn ensure_mp4_extension_adds_missing_extension() {
        assert_eq!(ensure_mp4_extension("Sample"), "Sample.mp4");
    }
}
