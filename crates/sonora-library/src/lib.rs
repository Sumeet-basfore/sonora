pub mod artwork;
pub mod db;
pub mod metadata;
pub mod models;
pub mod repository;
pub mod schema;

pub use artwork::ArtworkCache;
pub use db::Database;
pub use metadata::{ExtractedMetadata, MetadataExtractor};
pub use models::{Album, AlbumDto, Artist, ArtistDto, LibrarySummary, SearchResult, Track};
pub use repository::{LibraryRepository, ScanStats, SUPPORTED_EXTENSIONS};
