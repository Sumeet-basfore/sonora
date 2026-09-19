use serde::Deserialize;

#[derive(Debug, Clone, Deserialize)]
pub struct LrclibItemDto {
    pub id: u64,
    #[serde(alias = "name")]
    pub track_name: Option<String>,
    #[serde(alias = "artistName")]
    pub artist_name: Option<String>,
    #[serde(alias = "albumName")]
    pub album_name: Option<String>,
    pub duration: Option<f64>,
    pub instrumental: Option<bool>,
    #[serde(alias = "plainLyrics")]
    pub plain_lyrics: Option<String>,
    #[serde(alias = "syncedLyrics")]
    pub synced_lyrics: Option<String>,
}
