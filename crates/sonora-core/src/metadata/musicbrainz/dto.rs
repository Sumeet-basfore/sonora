//! MusicBrainz Web Service v2 Serde Data Transfer Objects.

use crate::metadata::models::{
    ArtistType, ExternalLink, OnlineArtist, OnlineArtistCredit, OnlineMedia, OnlineRelease,
    OnlineReleaseGroup, OnlineTrack,
};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MbArtistSearchResponse {
    #[serde(default)]
    pub count: usize,
    #[serde(default)]
    pub artists: Vec<MbArtist>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MbArtist {
    pub id: String,
    pub name: String,
    #[serde(rename = "sort-name")]
    pub sort_name: Option<String>,
    #[serde(rename = "type")]
    pub artist_type: Option<String>,
    pub country: Option<String>,
    pub disambiguation: Option<String>,
    #[serde(default)]
    pub relations: Vec<MbRelation>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MbRelation {
    #[serde(rename = "type")]
    pub relation_type: Option<String>,
    pub url: Option<MbUrl>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MbUrl {
    pub resource: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MbReleaseGroupSearchResponse {
    #[serde(default)]
    pub count: usize,
    #[serde(default, rename = "release-groups")]
    pub release_groups: Vec<MbReleaseGroup>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MbReleaseGroup {
    pub id: String,
    pub title: String,
    #[serde(rename = "primary-type")]
    pub primary_type: Option<String>,
    #[serde(default, rename = "secondary-types")]
    pub secondary_types: Vec<String>,
    #[serde(rename = "first-release-date")]
    pub first_release_date: Option<String>,
    #[serde(default, rename = "artist-credit")]
    pub artist_credit: Vec<MbArtistCredit>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MbArtistCredit {
    pub name: Option<String>,
    pub joinphrase: Option<String>,
    pub artist: Option<MbArtistSummary>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MbArtistSummary {
    pub id: String,
    pub name: String,
    #[serde(rename = "sort-name")]
    pub sort_name: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MbReleaseSearchResponse {
    #[serde(default)]
    pub count: usize,
    #[serde(default)]
    pub releases: Vec<MbRelease>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MbRelease {
    pub id: String,
    pub title: String,
    pub status: Option<String>,
    pub date: Option<String>,
    pub country: Option<String>,
    pub barcode: Option<String>,
    #[serde(rename = "release-group")]
    pub release_group: Option<MbReleaseGroupSummary>,
    #[serde(default, rename = "artist-credit")]
    pub artist_credit: Vec<MbArtistCredit>,
    #[serde(default)]
    pub media: Vec<MbMedium>,
    #[serde(default, rename = "label-info")]
    pub label_info: Vec<MbLabelInfo>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MbLabelInfo {
    pub label: Option<MbLabelSummary>,
    #[serde(rename = "catalog-number")]
    pub catalog_number: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MbLabelSummary {
    pub id: Option<String>,
    pub name: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MbReleaseGroupSummary {
    pub id: String,
    pub title: Option<String>,
    #[serde(rename = "primary-type")]
    pub primary_type: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MbMedium {
    pub position: Option<u32>,
    pub format: Option<String>,
    pub title: Option<String>,
    #[serde(rename = "track-count")]
    pub track_count: Option<u32>,
    #[serde(default)]
    pub tracks: Vec<MbTrack>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MbTrack {
    pub id: Option<String>,
    pub position: Option<u32>,
    pub number: Option<String>,
    pub title: Option<String>,
    pub length: Option<u64>,
    pub recording: Option<MbRecordingSummary>,
    #[serde(default, rename = "artist-credit")]
    pub artist_credit: Vec<MbArtistCredit>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MbRecordingSummary {
    pub id: String,
    pub title: Option<String>,
    pub length: Option<u64>,
    #[serde(default)]
    pub isrcs: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MbRecordingSearchResponse {
    #[serde(default)]
    pub count: usize,
    #[serde(default)]
    pub recordings: Vec<MbRecording>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MbRecording {
    pub id: String,
    pub title: String,
    pub length: Option<u64>,
    #[serde(default)]
    pub isrcs: Vec<String>,
    #[serde(default, rename = "artist-credit")]
    pub artist_credit: Vec<MbArtistCredit>,
    #[serde(default)]
    pub releases: Vec<MbReleaseSummary>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MbReleaseSummary {
    pub id: String,
    pub title: String,
    pub date: Option<String>,
    pub country: Option<String>,
    #[serde(rename = "release-group")]
    pub release_group: Option<MbReleaseGroupSummary>,
    #[serde(default)]
    pub media: Vec<MbMedium>,
}

// Convert DTOs to Domain Models

impl From<MbArtistCredit> for OnlineArtistCredit {
    fn from(dto: MbArtistCredit) -> Self {
        let (artist_mbid, name) = if let Some(summary) = dto.artist {
            (summary.id, dto.name.unwrap_or(summary.name))
        } else {
            (String::new(), dto.name.unwrap_or_default())
        };

        Self {
            artist_mbid,
            name,
            join_phrase: dto.joinphrase.filter(|s| !s.is_empty()),
        }
    }
}

impl From<MbArtist> for OnlineArtist {
    fn from(dto: MbArtist) -> Self {
        let artist_type = match dto.artist_type.as_deref() {
            Some("Person") => Some(ArtistType::Person),
            Some("Group") => Some(ArtistType::Group),
            Some("Orchestra") => Some(ArtistType::Orchestra),
            Some("Choir") => Some(ArtistType::Choir),
            Some("Character") => Some(ArtistType::Character),
            Some("Other") => Some(ArtistType::Other),
            _ => None,
        };

        let external_links = dto
            .relations
            .into_iter()
            .filter_map(|r| {
                let link_type = r.relation_type?;
                let target_url = r.url?.resource?;
                Some(ExternalLink {
                    link_type,
                    target_url,
                })
            })
            .collect();

        Self {
            mbid: dto.id,
            name: dto.name,
            sort_name: dto.sort_name,
            artist_type,
            country: dto.country,
            disambiguation: dto.disambiguation.filter(|s| !s.is_empty()),
            biography: None,
            external_links,
        }
    }
}

impl From<MbReleaseGroup> for OnlineReleaseGroup {
    fn from(dto: MbReleaseGroup) -> Self {
        Self {
            mbid: dto.id,
            title: dto.title,
            primary_type: dto.primary_type,
            secondary_types: dto.secondary_types,
            first_release_date: dto.first_release_date,
            artist_credits: dto.artist_credit.into_iter().map(Into::into).collect(),
        }
    }
}

impl From<MbRelease> for OnlineRelease {
    fn from(dto: MbRelease) -> Self {
        let label_info = dto.label_info.into_iter().next();
        let (label, catalog_number) = if let Some(info) = label_info {
            (info.label.and_then(|l| l.name), info.catalog_number)
        } else {
            (None, None)
        };

        let media_format = dto.media.first().and_then(|m| m.format.clone());
        let track_count = dto
            .media
            .iter()
            .map(|m| m.track_count.unwrap_or(m.tracks.len() as u32))
            .sum();

        let release_mbid = dto.id.clone();
        let release_group_mbid = dto.release_group.as_ref().map(|rg| rg.id.clone());

        let media = dto
            .media
            .into_iter()
            .map(|m| {
                let position = m.position.unwrap_or(1);
                let tracks = m
                    .tracks
                    .into_iter()
                    .map(|t| {
                        let (recording_mbid, isrcs) = if let Some(r) = t.recording {
                            (r.id, r.isrcs)
                        } else {
                            (String::new(), Vec::new())
                        };

                        let artist_credits = if t.artist_credit.is_empty() {
                            dto.artist_credit.iter().cloned().map(Into::into).collect()
                        } else {
                            t.artist_credit.into_iter().map(Into::into).collect()
                        };

                        OnlineTrack {
                            recording_mbid,
                            release_mbid: Some(release_mbid.clone()),
                            release_group_mbid: release_group_mbid.clone(),
                            position: t.position,
                            number: t.number,
                            title: t.title.unwrap_or_default(),
                            duration_ms: t.length,
                            artist_credits,
                            isrcs,
                        }
                    })
                    .collect();

                OnlineMedia {
                    position,
                    format: m.format,
                    title: m.title,
                    track_count: m.track_count.unwrap_or(0),
                    tracks,
                }
            })
            .collect();

        Self {
            mbid: dto.id,
            release_group_mbid: dto.release_group.map(|rg| rg.id),
            title: dto.title,
            status: dto.status,
            date: dto.date,
            country: dto.country,
            barcode: dto.barcode,
            media_format,
            track_count,
            media,
            artist_credits: dto.artist_credit.into_iter().map(Into::into).collect(),
            label,
            catalog_number,
        }
    }
}

impl From<MbRecording> for OnlineTrack {
    fn from(dto: MbRecording) -> Self {
        let first_release = dto.releases.first();
        let release_mbid = first_release.map(|r| r.id.clone());
        let release_group_mbid =
            first_release.and_then(|r| r.release_group.as_ref().map(|rg| rg.id.clone()));

        Self {
            recording_mbid: dto.id,
            release_mbid,
            release_group_mbid,
            position: None,
            number: None,
            title: dto.title,
            duration_ms: dto.length,
            artist_credits: dto.artist_credit.into_iter().map(Into::into).collect(),
            isrcs: dto.isrcs,
        }
    }
}
