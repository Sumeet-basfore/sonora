pub mod coverartarchive;
pub mod fanart;
pub mod models;
pub mod pipeline;
pub mod traits;
pub mod wikidata;

pub use coverartarchive::CoverArtArchiveClient;
pub use fanart::FanartTvArtworkProvider;
pub use models::{
    ArtworkCandidate, ArtworkKind, ArtworkQuery, ArtworkSourceType, CachedArtworkAsset,
};
pub use pipeline::{
    ArtworkPipeline, FULL_ARTWORK_DIMENSION, MAX_IMAGE_DIMENSION, MAX_IMAGE_PAYLOAD_BYTES,
    THUMBNAIL_DIMENSION,
};
pub use traits::ArtworkProvider;
pub use wikidata::WikidataArtworkProvider;

#[cfg(test)]
mod tests;
