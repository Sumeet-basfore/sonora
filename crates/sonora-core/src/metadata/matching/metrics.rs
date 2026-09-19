//! String similarity and duration penalty metrics.

use std::collections::BTreeSet;

/// Compute Jaro-Winkler string similarity in range [0.0, 1.0].
pub fn jaro_winkler_similarity(s1: &str, s2: &str) -> f32 {
    let s1_chars: Vec<char> = s1.chars().collect();
    let s2_chars: Vec<char> = s2.chars().collect();

    let len1 = s1_chars.len();
    let len2 = s2_chars.len();

    if len1 == 0 && len2 == 0 {
        return 1.0;
    }
    if len1 == 0 || len2 == 0 {
        return 0.0;
    }

    if s1_chars == s2_chars {
        return 1.0;
    }

    let match_distance = (len1.max(len2) / 2).saturating_sub(1);

    let mut s1_matches = vec![false; len1];
    let mut s2_matches = vec![false; len2];

    let mut matches = 0;
    for i in 0..len1 {
        let start = i.saturating_sub(match_distance);
        let end = (i + match_distance + 1).min(len2);

        for j in start..end {
            if s2_matches[j] || s1_chars[i] != s2_chars[j] {
                continue;
            }
            s1_matches[i] = true;
            s2_matches[j] = true;
            matches += 1;
            break;
        }
    }

    if matches == 0 {
        return 0.0;
    }

    let mut k = 0;
    let mut transpositions = 0;
    for i in 0..len1 {
        if !s1_matches[i] {
            continue;
        }
        while !s2_matches[k] {
            k += 1;
        }
        if s1_chars[i] != s2_chars[k] {
            transpositions += 1;
        }
        k += 1;
    }

    let jaro = ((matches as f32 / len1 as f32)
        + (matches as f32 / len2 as f32)
        + ((matches as f32 - (transpositions as f32 / 2.0)) / matches as f32))
        / 3.0;

    // Winkler prefix scaling (up to 4 characters with standard factor 0.1)
    let mut prefix = 0;
    for i in 0..len1.min(len2).min(4) {
        if s1_chars[i] == s2_chars[i] {
            prefix += 1;
        } else {
            break;
        }
    }

    let p = 0.1;
    jaro + (prefix as f32 * p * (1.0 - jaro))
}

/// Compute Levenshtein edit distance between two strings.
pub fn levenshtein_distance(s1: &str, s2: &str) -> usize {
    let s1_chars: Vec<char> = s1.chars().collect();
    let s2_chars: Vec<char> = s2.chars().collect();

    let len1 = s1_chars.len();
    let len2 = s2_chars.len();

    if len1 == 0 {
        return len2;
    }
    if len2 == 0 {
        return len1;
    }

    let mut prev_row: Vec<usize> = (0..=len2).collect();
    let mut curr_row: Vec<usize> = vec![0; len2 + 1];

    for (i, &c1) in s1_chars.iter().enumerate() {
        curr_row[0] = i + 1;
        for (j, &c2) in s2_chars.iter().enumerate() {
            let cost = if c1 == c2 { 0 } else { 1 };
            curr_row[j + 1] = (curr_row[j] + 1)
                .min(prev_row[j + 1] + 1)
                .min(prev_row[j] + cost);
        }
        prev_row.copy_from_slice(&curr_row);
    }

    prev_row[len2]
}

/// Levenshtein similarity ratio in range [0.0, 1.0].
pub fn levenshtein_similarity(s1: &str, s2: &str) -> f32 {
    let max_len = s1.chars().count().max(s2.chars().count());
    if max_len == 0 {
        return 1.0;
    }
    let dist = levenshtein_distance(s1, s2);
    1.0 - (dist as f32 / max_len as f32)
}

/// Token Sort Similarity: sorts tokens alphabetically before computing similarity.
pub fn token_sort_similarity(s1: &str, s2: &str) -> f32 {
    let mut t1: Vec<&str> = s1.split_whitespace().collect();
    let mut t2: Vec<&str> = s2.split_whitespace().collect();

    t1.sort_unstable();
    t2.sort_unstable();

    let sorted1 = t1.join(" ");
    let sorted2 = t2.join(" ");

    jaro_winkler_similarity(&sorted1, &sorted2)
}

/// Token Set Similarity: compares intersection and differences of token sets.
pub fn token_set_similarity(s1: &str, s2: &str) -> f32 {
    let set1: BTreeSet<&str> = s1.split_whitespace().collect();
    let set2: BTreeSet<&str> = s2.split_whitespace().collect();

    if set1.is_empty() && set2.is_empty() {
        return 1.0;
    }
    if set1.is_empty() || set2.is_empty() {
        return 0.0;
    }

    let intersection: Vec<&str> = set1.intersection(&set2).copied().collect();
    let diff1: Vec<&str> = set1.difference(&set2).copied().collect();
    let diff2: Vec<&str> = set2.difference(&set1).copied().collect();

    let sorted_inter = intersection.join(" ");
    let sorted1 = format!("{} {}", sorted_inter, diff1.join(" "));
    let sorted2 = format!("{} {}", sorted_inter, diff2.join(" "));

    let score_inter_1 = jaro_winkler_similarity(sorted_inter.trim(), sorted1.trim());
    let score_inter_2 = jaro_winkler_similarity(sorted_inter.trim(), sorted2.trim());
    let score_1_2 = jaro_winkler_similarity(sorted1.trim(), sorted2.trim());

    score_inter_1.max(score_inter_2).max(score_1_2)
}

/// Calculate duration match score in range [0.0, 1.0] from duration delta.
///
/// Specification duration curve:
/// - Delta <= 1.0s -> 1.00
/// - Delta = 3.0s  -> 0.60
/// - Delta = 5.0s  -> 0.25
/// - Delta > 10.0s -> 0.00
pub fn duration_score(local_duration_ms: u64, candidate_duration_ms: u64) -> f32 {
    if local_duration_ms == 0 || candidate_duration_ms == 0 {
        // If one duration is missing, assign neutral penalty
        return 0.50;
    }

    let delta_secs = (local_duration_ms as f64 - candidate_duration_ms as f64).abs() / 1000.0;

    if delta_secs <= 1.0 {
        1.00
    } else if delta_secs <= 3.0 {
        // Linear interpolation from 1.00 down to 0.60 across [1.0, 3.0]
        let t = (delta_secs - 1.0) / 2.0;
        (1.00 - t * (1.00 - 0.60)) as f32
    } else if delta_secs <= 5.0 {
        // Linear interpolation from 0.60 down to 0.25 across [3.0, 5.0]
        let t = (delta_secs - 3.0) / 2.0;
        (0.60 - t * (0.60 - 0.25)) as f32
    } else if delta_secs <= 10.0 {
        // Linear interpolation from 0.25 down to 0.00 across [5.0, 10.0]
        let t = (delta_secs - 5.0) / 5.0;
        (0.25 - t * 0.25) as f32
    } else {
        0.00
    }
}
