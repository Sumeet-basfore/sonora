use crate::model::{LyricLine, LyricsDocument, LyricsFormat};
use sonora_common::Result;

/// Trait implemented by lyrics format parsers.
pub trait LyricsParser {
    fn format(&self) -> LyricsFormat;
    fn parse(&self, raw: &str) -> Result<LyricsDocument>;
}

/// Fallback plain text lyrics parser.
pub struct PlainTextLyricsParser;

impl LyricsParser for PlainTextLyricsParser {
    fn format(&self) -> LyricsFormat {
        LyricsFormat::Plain
    }

    fn parse(&self, raw: &str) -> Result<LyricsDocument> {
        let lines = raw
            .lines()
            .map(|line| LyricLine {
                start_time_ms: 0,
                end_time_ms: None,
                text: line.trim().to_string(),
                syllables: Vec::new(),
            })
            .collect();

        Ok(LyricsDocument {
            format: LyricsFormat::Plain,
            lines,
            ..Default::default()
        })
    }
}

/// Standard Synchronized LRC Parser.
///
/// Handles:
/// - Standard line timestamps: `[mm:ss.xx]` and `[mm:ss.xxx]`
/// - Multiple timestamps per line: `[00:12.34][01:05.67]Chorus line`
/// - Metadata tags: `[ti:]`, `[ar:]`, `[al:]`, `[offset:]`
/// - Timestamp chronological sorting
/// - End timestamp calculation based on subsequent line
/// - Graceful handling of corrupted/malformed tags
pub struct LrcLyricsParser;

impl LyricsParser for LrcLyricsParser {
    fn format(&self) -> LyricsFormat {
        LyricsFormat::Lrc
    }

    fn parse(&self, raw: &str) -> Result<LyricsDocument> {
        let mut title = None;
        let mut artist = None;
        let mut album = None;
        let mut doc_offset_ms: i64 = 0;
        let mut raw_lines: Vec<(i64, String)> = Vec::new();
        let mut has_synced_timestamps = false;

        for line in raw.lines() {
            let trimmed = line.trim();
            if trimmed.is_empty() {
                continue;
            }

            // Check for metadata tag [tag:value]
            if let Some(meta) = parse_metadata_tag(trimmed) {
                match meta.0.to_lowercase().as_str() {
                    "ti" | "title" => title = Some(meta.1.to_string()),
                    "ar" | "artist" => artist = Some(meta.1.to_string()),
                    "al" | "album" => album = Some(meta.1.to_string()),
                    "offset" => {
                        if let Ok(off) = meta.1.trim().parse::<i64>() {
                            doc_offset_ms = off;
                        }
                    }
                    _ => {} // Ignore creator, length, etc.
                }
                continue;
            }

            // Check for timestamps: [mm:ss.xx]
            let (timestamps, text) = extract_timestamps_and_text(trimmed);
            if !timestamps.is_empty() {
                has_synced_timestamps = true;
                for ts in timestamps {
                    raw_lines.push((ts, text.clone()));
                }
            } else {
                // Unsynced line within file
                raw_lines.push((0, trimmed.to_string()));
            }
        }

        // If no timestamps were found at all, treat as plain text
        if !has_synced_timestamps {
            let plain_lines = raw_lines
                .into_iter()
                .map(|(_, text)| LyricLine {
                    start_time_ms: 0,
                    end_time_ms: None,
                    text,
                    syllables: Vec::new(),
                })
                .collect();

            return Ok(LyricsDocument {
                title,
                artist,
                album,
                offset_ms: doc_offset_ms,
                format: LyricsFormat::Plain,
                lines: plain_lines,
            });
        }

        // Apply document offset and sort chronologically
        let mut timed_lines: Vec<LyricLine> = raw_lines
            .into_iter()
            .map(|(raw_ms, text)| {
                let adjusted_ms = (raw_ms + doc_offset_ms).max(0) as u64;
                LyricLine {
                    start_time_ms: adjusted_ms,
                    end_time_ms: None,
                    text,
                    syllables: Vec::new(),
                }
            })
            .collect();

        timed_lines.sort_by_key(|l| l.start_time_ms);

        // Calculate end_time_ms for each line
        let len = timed_lines.len();
        for i in 0..len {
            if i + 1 < len {
                let next_start = timed_lines[i + 1].start_time_ms;
                timed_lines[i].end_time_ms = Some(next_start.max(timed_lines[i].start_time_ms));
            } else {
                // Last line: estimate default 4-second display duration
                timed_lines[i].end_time_ms = Some(timed_lines[i].start_time_ms + 4000);
            }
        }

        Ok(LyricsDocument {
            title,
            artist,
            album,
            offset_ms: doc_offset_ms,
            format: LyricsFormat::Lrc,
            lines: timed_lines,
        })
    }
}

/// Extract metadata tag like [ti:Song Title] or [offset:+500]
fn parse_metadata_tag(line: &str) -> Option<(&str, &str)> {
    if !line.starts_with('[') || !line.ends_with(']') {
        return None;
    }
    let inner = &line[1..line.len() - 1];
    let colon_idx = inner.find(':')?;
    let key = inner[..colon_idx].trim();
    let val = inner[colon_idx + 1..].trim();

    // Ensure key does not start with digits (which would indicate a timestamp like [00:12.34])
    if key.chars().all(|c| c.is_ascii_alphabetic()) && !key.is_empty() {
        Some((key, val))
    } else {
        None
    }
}

/// Extracts all timestamps at the start of a line and returns the remaining text.
/// E.g. "[00:12.50][01:05.20] Hello world" -> (vec![12500, 65200], "Hello world")
fn extract_timestamps_and_text(line: &str) -> (Vec<i64>, String) {
    let mut timestamps = Vec::new();
    let mut remainder = line;

    while remainder.starts_with('[') {
        if let Some(close_bracket) = remainder.find(']') {
            let candidate = &remainder[1..close_bracket];
            if let Some(ms) = parse_timestamp_ms(candidate) {
                timestamps.push(ms);
                remainder = remainder[close_bracket + 1..].trim_start();
            } else {
                // Not a timestamp (e.g. unknown tag or inline note); break out
                break;
            }
        } else {
            break;
        }
    }

    (timestamps, remainder.trim().to_string())
}

/// Parses a timestamp string like "01:23.45", "01:23.456", or "01:23" into milliseconds.
fn parse_timestamp_ms(s: &str) -> Option<i64> {
    let parts: Vec<&str> = s.split(':').collect();
    if parts.len() != 2 {
        return None;
    }

    let minutes: i64 = parts[0].trim().parse().ok()?;
    let seconds_str = parts[1].trim();

    if let Some(dot_idx) = seconds_str.find('.') {
        let seconds: i64 = seconds_str[..dot_idx].parse().ok()?;
        let fraction_str = &seconds_str[dot_idx + 1..];

        let ms = match fraction_str.len() {
            0 => 0,
            1 => fraction_str.parse::<i64>().ok()? * 100,
            2 => fraction_str.parse::<i64>().ok()? * 10,
            3 => fraction_str.parse::<i64>().ok()?,
            _ => fraction_str[..3].parse::<i64>().ok()?,
        };

        Some(minutes * 60_000 + seconds * 1_000 + ms)
    } else {
        let seconds: i64 = seconds_str.parse().ok()?;
        Some(minutes * 60_000 + seconds * 1_000)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_standard_lrc_parsing() {
        let raw = r#"
[ti:Midnight City]
[ar:M83]
[al:Hurry Up, We're Dreaming]
[offset:100]
[00:10.50] Waiting in a car
[00:15.80] Waiting for a ride in the dark
[00:22.100] The city is my church
"#;

        let parser = LrcLyricsParser;
        let doc = parser.parse(raw).unwrap();

        assert_eq!(doc.format, LyricsFormat::Lrc);
        assert_eq!(doc.title.as_deref(), Some("Midnight City"));
        assert_eq!(doc.artist.as_deref(), Some("M83"));
        assert_eq!(doc.album.as_deref(), Some("Hurry Up, We're Dreaming"));
        assert_eq!(doc.offset_ms, 100);

        assert_eq!(doc.lines.len(), 3);
        // 10500 + 100 offset = 10600
        assert_eq!(doc.lines[0].start_time_ms, 10600);
        assert_eq!(doc.lines[0].text, "Waiting in a car");
        assert_eq!(doc.lines[0].end_time_ms, Some(15900));

        // 15800 + 100 offset = 15900
        assert_eq!(doc.lines[1].start_time_ms, 15900);
        assert_eq!(doc.lines[1].text, "Waiting for a ride in the dark");

        // 22100 + 100 offset = 22200
        assert_eq!(doc.lines[2].start_time_ms, 22200);
        assert_eq!(doc.lines[2].text, "The city is my church");
        assert_eq!(doc.lines[2].end_time_ms, Some(26200));
    }

    #[test]
    fn test_multi_timestamp_and_out_of_order_lines() {
        let raw = r#"
[00:30.00] Second verse
[00:10.00][00:50.00] Repeated chorus
[00:05.00] First verse
"#;

        let parser = LrcLyricsParser;
        let doc = parser.parse(raw).unwrap();

        assert_eq!(doc.lines.len(), 4);
        assert_eq!(doc.lines[0].start_time_ms, 5000);
        assert_eq!(doc.lines[0].text, "First verse");

        assert_eq!(doc.lines[1].start_time_ms, 10000);
        assert_eq!(doc.lines[1].text, "Repeated chorus");

        assert_eq!(doc.lines[2].start_time_ms, 30000);
        assert_eq!(doc.lines[2].text, "Second verse");

        assert_eq!(doc.lines[3].start_time_ms, 50000);
        assert_eq!(doc.lines[3].text, "Repeated chorus");
    }

    #[test]
    fn test_malformed_lyrics_resilience() {
        let raw = r#"
[invalid tag]
[00:invalid] Corrupted timestamp line
[00:04.50] Valid line 1
Some random untimed commentary
[00:10.00] Valid line 2
[9999:99.99]
"#;

        let parser = LrcLyricsParser;
        let doc = parser.parse(raw).unwrap();

        assert_eq!(doc.format, LyricsFormat::Lrc);
        assert!(doc.lines.iter().any(|l| l.text == "Valid line 1"));
        assert!(doc.lines.iter().any(|l| l.text == "Valid line 2"));
    }

    #[test]
    fn test_plain_text_fallback() {
        let raw = "Just plain lyrics without timestamps\nLine two\nLine three";
        let parser = LrcLyricsParser;
        let doc = parser.parse(raw).unwrap();

        assert_eq!(doc.format, LyricsFormat::Plain);
        assert_eq!(doc.lines.len(), 3);
        assert_eq!(doc.lines[0].text, "Just plain lyrics without timestamps");
    }
}
