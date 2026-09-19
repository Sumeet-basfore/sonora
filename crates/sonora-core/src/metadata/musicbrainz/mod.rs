//! MusicBrainz metadata provider implementation.

pub mod client;
pub mod dto;

pub use client::{MusicBrainzClient, DEFAULT_MUSICBRAINZ_BASE_URL, DEFAULT_USER_AGENT};
