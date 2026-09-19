# Sonora: Deterministic Metadata Matching & Ranking Model

## 1. Problem Statement

Automated metadata matching engines in media software often suffer from two extremes:
1. **Over-Strict Naive String Matching**: Rejects valid matches due to trivial string differences (e.g. `"Track Title (Remastered 2011)"` vs `"Track Title"`, or `"Artist feat. Someone"` vs `"Artist"`).
2. **Opaque "Black-Box" AI Matching**: Automatically overwrites user library metadata based on fuzzy probabilistic models or AI embeddings without explaining why a match was made, leading to corrupted library tags and unrecoverable user frustration.

Sonora requires a **deterministic, explainable metadata matching and scoring engine** that ranks online MusicBrainz candidates using transparent scoring rules, enforces strict confidence thresholds, never automatically overwrites metadata on low-confidence matches, and provides clear mathematical rationale for candidate scores.

---

## 2. User Goals

1. **High Match Accuracy**: Automatically identify candidate MusicBrainz recordings and release groups despite minor punctuation differences, feature credits, or remaster suffixes.
2. **100% Explainable Scores**: Inspect exact score breakdowns (e.g., *Title Match: 0.40/0.40, Duration Match: 0.25/0.25, Artist Similarity: 0.20/0.20, Total Confidence: 0.95*) inside the Match Inspector.
3. **Safety Guardrails**: Never suffer silent metadata corruptions caused by low-confidence fuzzy mismatches.
4. **Instant Match Shortcuts**: Benefit from 1.0 confidence shortcuts when exact ISRCs or MBIDs exist in local track tags.
5. **Clear Confidence Tiers**: Immediately understand whether a match is a Safe Suggestion (High Confidence), requires User Confirmation (Medium Confidence), or demands Manual Selection (Low Confidence).

---

## 3. Operational Principles & Ranking Workflow

```
┌─────────────────────────────────────────────────────────────────────────────┐
│                    DETERMINISTIC MATCHING PIPELINE                         │
│                                                                             │
│  [Local Audio Track Metadata] ──► Extract (Title, Artist, Album, ISRC, etc.)│
│                                          │                                  │
│                                          ▼                                  │
│  [Query Candidates from MusicBrainz] ──► (Recordings / Releases)            │
│                                          │                                  │
│                                          ▼                                  │
│  [String Normalization & Feature Extraction Pipeline]                       │
│  • Strip Punctuation, Diacritics, Case Folding (NFKD Unicode)               │
│  • Separate Primary Artist & Featured Artist Credits ("feat.", "ft.")       │
│  • Extract Version/Remix Tokens ("Remastered", "Live", "Radio Edit")        │
│                                          │                                  │
│                                          ▼                                  │
│  [Weighted Confidence Scoring Engine]                                       │
│  • Exact ISRC / MBID Shortcut? ──► Score = 1.00                             │
│  • Otherwise: Composite Weighted Score Formula (Title + Artist + Duration...)│
│                                          │                                  │
│                                          ▼                                  │
│  [Confidence Threshold Classification]                                      │
│  • High (>= 0.90)  ──► Safe Suggestion (1-Click Auto-Apply)                 │
│  • Medium (0.65-0.89) ──► User Confirmation Required (Side-by-Side Diff)    │
│  • Low (< 0.65)   ──► Show Alternatives / Manual Search                     │
└─────────────────────────────────────────────────────────────────────────────┘
```

---

## 4. Normalization & Pre-Processing Rules

Before candidate evaluation, raw strings from local files and MusicBrainz APIs undergo deterministic normalization:

### 4.1 String Normalization Pipeline
1. **Unicode NFKD Decomposition**: Converts diacritics and accented characters to ASCII equivalents (e.g. `Björk` $\to$ `Bjork`, `Motörhead` $\to$ `Motorhead`).
2. **Case Folding**: Converts all text to lowercase.
3. **Punctuation Stripping**: Replaces punctuation (`.`, `,`, `-`, `_`, `:`, `/`, `'`, `"`) with single spaces.
4. **Whitespace Collapse**: Trims leading/trailing whitespace and collapses multiple spaces to a single space.

### 4.2 Feature Artist Extraction (`feat.` / `featuring` / `ft.`)
Strings containing feature phrases (`feat.`, `featuring`, `ft.`, `with`, `vs.`) are split into:
- `primary_artist`: The main performing artist name.
- `featured_artists`: List of guest performers.

*Example*: `"Gorillaz feat. Del The Funky Homosapien"` $\to$ Primary: `"Gorillaz"`, Featured: `["Del The Funky Homosapien"]`.

### 4.3 Version & Remix Suffix Extraction
Parenthetical and trailing brackets containing version noise are extracted into separate comparison fields:
- Noise Terms: `"Remastered"`, `"Deluxe Edition"`, `"Live at ..."`, `"Radio Edit"`, `"Bonus Track"`, `"Original Mix"`.

---

## 5. Composite Confidence Scoring Algorithm

When no exact MBID or ISRC shortcut matches, candidate score $S \in [0.0, 1.0]$ is calculated as a weighted sum of normalized sub-scores:

$$S = w_{title} \cdot S_{title} + w_{artist} \cdot S_{artist} + w_{album} \cdot S_{album} + w_{duration} \cdot S_{duration} + w_{track} \cdot S_{track}$$

### 5.1 Sub-Score Weights & Formulas

| Feature | Weight ($w_i$) | Metric / Calculation | Description |
| :--- | :--- | :--- | :--- |
| **Track Title** | $0.35$ | Jaro-Winkler Similarity on normalized titles | Evaluates title similarity after stripping remix/version noise. |
| **Artist Name** | $0.25$ | Token-Set Similarity on primary artist | Evaluates primary artist match; bonus points for featured match. |
| **Album Title** | $0.20$ | Levenshtein / Token-Sort Ratio | Evaluates album title match against Release / Release Group. |
| **Duration Delta** | $0.15$ | Continuous Gaussian Decay Penalty | $S_{duration} = \exp\left(-\frac{(\Delta t)^2}{2 \sigma^2}\right)$ with $\sigma = 3.0\text{ sec}$. |
| **Track / Disc #** | $0.05$ | Exact Integer Match ($1.0$ or $0.0$) | Matches track position on album media. |

### 5.2 Duration Penalty Curve
- $\Delta t \le 1.0\text{ sec} \implies S_{duration} = 1.00$
- $\Delta t = 3.0\text{ sec} \implies S_{duration} = 0.60$
- $\Delta t = 5.0\text{ sec} \implies S_{duration} = 0.25$
- $\Delta t > 10.0\text{ sec} \implies S_{duration} = 0.00$

### 5.3 Exact Match Shortcuts (Confidence = 1.0)
1. **MBID Exact Match**: If local track tag contains a valid `MUSICBRAINZ_TRACKID` or `MUSICBRAINZ_RELEASEGROUPID` matching the candidate entity, $S = 1.00$ immediately.
2. **ISRC Exact Match**: If local track tag contains a valid ISRC matching the candidate recording, $S = 1.00$ immediately.

---

## 6. Confidence Thresholds & Action Matrix

```
┌─────────────────────────────────────────────────────────────────────────────┐
│                    CONFIDENCE THRESHOLD CLASSIFICATION                      │
│                                                                             │
│   Score S >= 0.90  ──► [ HIGH CONFIDENCE ]  ──► Safe Suggestion             │
│                        • One-Click Auto-Apply Option in Batch Tagger       │
│                        • Green Confidence Badge in Match Inspector         │
│                                                                             │
│   0.65 <= S < 0.90 ──► [ MEDIUM CONFIDENCE ] ──► Confirmation Required      │
│                        • Displays Side-by-Side Diff Inspector Modal        │
│                        • Yellow Confidence Badge with Breakdown Metrics    │
│                                                                             │
│   Score S < 0.65   ──► [ LOW CONFIDENCE ]   ──► Manual Selection            │
│                        • Exposes Alternative Candidates in Candidate List   │
│                        • Red Warning Badge; No Auto-Apply                   │
└─────────────────────────────────────────────────────────────────────────────┘
```

| Confidence Tier | Score Range | Default Action | User Verification Required? |
| :--- | :--- | :--- | :--- |
| **High Confidence** | $0.90 \le S \le 1.00$ | Safe Suggestion / Batch Auto-Match Candidate | No (Optional 1-Click Apply) |
| **Medium Confidence** | $0.65 \le S < 0.90$ | Requires User Confirmation Diff View | Yes (User clicks Confirm) |
| **Low Confidence** | $0.00 \le S < 0.65$ | Exposes Alternatives; Requires Manual Select | Yes (Strict Manual Selection) |

---

## 7. Data Model: Matching Breakdown & Candidate Score AST

```rust
// Matching Engine Data Schema in Sonora Core

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct MatchScoreBreakdown {
    pub total_score: f32, // 0.0 to 1.0
    pub confidence_tier: ConfidenceTier,
    pub title_score: f32,
    pub artist_score: f32,
    pub album_score: f32,
    pub duration_score: f32,
    pub track_number_score: f32,
    pub is_exact_shortcut: bool,
    pub shortcut_reason: Option<String>, // "Exact ISRC Match", "Exact MBID Match"
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum ConfidenceTier {
    High,   // >= 0.90
    Medium, // 0.65 - 0.89
    Low,    // < 0.65
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RankedCandidateMatch {
    pub candidate_track: OnlineTrack,
    pub candidate_release: OnlineRelease,
    pub candidate_release_group: OnlineReleaseGroup,
    pub score_breakdown: MatchScoreBreakdown,
}
```

---

## 8. Failure Cases & Edge Behavior

| Edge Case | Root Cause | System Resolution |
| :--- | :--- | :--- |
| **Classical Track Matching** | Long titles with Opus/Movement details | Normalization retains movement numbers (`Op. 27 No. 2`); title weight increased. |
| **Live vs Studio Track** | Same title & artist, different duration | Duration penalty curve ($S_{duration} \to 0$) prevents matching studio track to live recording. |
| **Various Artists Compilations** | Track artist differs from Album artist | Scorer compares track artist against `OnlineArtistCredit`, not album artist alone. |
| **Multi-Disc Indexing** | Disc 1 vs Disc 2 track numbering | Disc number exact match multiplier prevents cross-disc track offset misalignment. |

---

## 9. Security & Privacy Considerations

1. **Deterministic & Explainable**: Matches use open, mathematical scoring logic without nondeterministic external LLMs or vector databases.
2. **Local Evaluation**: Candidate ranking occurs 100% locally within `sonorad`. Only raw search query strings hit MusicBrainz servers.

---

## 10. Open Decisions

1. **Batch Auto-Matching Threshold**:
   - *Status*: Open user preference.
   - *Decision*: When running bulk library enrichment, should high-confidence matches ($S \ge 0.95$) auto-apply without popping up a diff inspector for every track?
   - *Recommendation*: Allow a "Batch Enrich High-Confidence Matches" button in Settings, with a summary log displayed after completion.

---

## 11. References

- Jaro-Winkler String Similarity Metric: `https://en.wikipedia.org/wiki/Jaro%E2%80%93Winkler_distance`
- Levenshtein Distance & Token Sort Algorithms: `https://en.wikipedia.org/wiki/Levenshtein_distance`
- MusicBrainz Data Model & Recording Relationships: `https://musicbrainz.org/doc/Recording`
- Sonora Data Model & Storage Spec: `docs/10-data-and-library-model.md`
