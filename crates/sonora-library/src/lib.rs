pub mod db;
pub mod metadata;
pub mod models;
pub mod repository;
pub mod schema;

pub use db::Database;
pub use metadata::{ExtractedMetadata, MetadataExtractor};
pub use models::{Album, Artist, SearchResult, Track};
pub use repository::LibraryRepository;
