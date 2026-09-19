//! Deterministic string normalizer for music metadata matching.

use unicode_normalization::UnicodeNormalization;

/// String normalizer utility for diacritics, case-folding, punctuation,
/// featured artist extraction, and version noise parsing.
pub struct StringNormalizer;

impl StringNormalizer {
    /// Normalize a raw string:
    /// 1. NFKD decomposition
    /// 2. Remove combining diacritical marks (\u{0300}..\u{036F})
    /// 3. Special Latin ligatures (æ->ae, œ->oe, ß->ss, etc.)
    /// 4. Case folding to lowercase
    /// 5. Replace punctuation with space
    /// 6. Collapse whitespace and trim
    pub fn normalize(input: &str) -> String {
        let mut decomposed = String::with_capacity(input.len());

        for c in input.nfkd() {
            // Check combining diacritical mark ranges
            if ('\u{0300}'..='\u{036F}').contains(&c)
                || ('\u{1AB0}'..='\u{1AFF}').contains(&c)
                || ('\u{1DC0}'..='\u{1DFF}').contains(&c)
                || ('\u{20D0}'..='\u{20FF}').contains(&c)
                || ('\u{FE20}'..='\u{FE2F}').contains(&c)
            {
                continue;
            }

            match c {
                'Æ' | 'æ' => decomposed.push_str("ae"),
                'Œ' | 'œ' => decomposed.push_str("oe"),
                'ß' => decomposed.push_str("ss"),
                'Ø' | 'ø' => decomposed.push('o'),
                'Đ' | 'đ' => decomposed.push('d'),
                'Ł' | 'ł' => decomposed.push('l'),
                'Þ' | 'þ' => decomposed.push_str("th"),
                _ => decomposed.push(c),
            }
        }

        let lower = decomposed.to_lowercase();
        let mut result = String::with_capacity(lower.len());

        for c in lower.chars() {
            if c.is_alphanumeric() {
                result.push(c);
            } else {
                result.push(' ');
            }
        }

        Self::collapse_whitespace(&result)
    }

    /// Collapse multiple consecutive whitespace characters into a single space and trim.
    pub fn collapse_whitespace(input: &str) -> String {
        input.split_whitespace().collect::<Vec<&str>>().join(" ")
    }

    /// Extract primary artist and featured artist list from a raw artist string.
    ///
    /// Recognizes patterns like:
    /// - `"Artist feat. Featured"`
    /// - `"Artist featuring Featured"`
    /// - `"Artist ft. Featured"`
    /// - `"Artist with Featured"`
    /// - `"Artist vs. Artist 2"`
    pub fn extract_featured_artists(raw_artist: &str) -> (String, Vec<String>) {
        let lower = raw_artist.to_lowercase();
        let feat_keywords = [
            " feat. ",
            " featuring ",
            " ft. ",
            " feat ",
            " ft ",
            " with ",
            " vs. ",
            " vs ",
        ];

        for kw in &feat_keywords {
            if let Some(pos) = lower.find(kw) {
                let primary_raw = &raw_artist[..pos];
                let featured_raw = &raw_artist[pos + kw.len()..];

                let primary = Self::normalize(primary_raw);
                let featured_list: Vec<String> = featured_raw
                    .split(['&', ',', '+'])
                    .flat_map(|s| s.split(" and "))
                    .map(Self::normalize)
                    .filter(|s| !s.is_empty())
                    .collect();

                return (primary, featured_list);
            }
        }

        (Self::normalize(raw_artist), Vec::new())
    }

    /// Extract version/remix tokens from a track title.
    ///
    /// Recognizes and strips noise tokens such as:
    /// - `"Remastered"`, `"2011 Remaster"`, `"Deluxe Edition"`, `"Live"`
    /// - `"Radio Edit"`, `"Bonus Track"`, `"Original Mix"`, `"Acoustic"`
    ///
    /// Returns `(clean_title, Option<version_info>)`.
    pub fn extract_version_info(raw_title: &str) -> (String, Option<String>) {
        let version_keywords = [
            "remastered",
            "remaster",
            "deluxe edition",
            "deluxe",
            "live at",
            "live",
            "radio edit",
            "bonus track",
            "original mix",
            "extended mix",
            "club mix",
            "mono version",
            "stereo version",
            "mono",
            "stereo",
            "acoustic version",
            "acoustic",
            "instrumental",
            "anniversary edition",
            "explicit",
            "clean version",
            "edit",
            "mix",
            "version",
        ];

        let norm = Self::normalize(raw_title);

        for kw in &version_keywords {
            if norm.contains(kw) {
                let cleaned = norm.replace(kw, "");
                let clean_title = Self::collapse_whitespace(&cleaned);
                if !clean_title.is_empty() {
                    return (clean_title, Some(kw.to_string()));
                }
            }
        }

        (norm, None)
    }
}
