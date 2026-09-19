use serde::Deserialize;
use std::collections::HashMap;

#[derive(Debug, Deserialize)]
pub struct CaaResponse {
    #[serde(default)]
    pub images: Vec<CaaImage>,
    pub release: Option<String>,
    #[serde(rename = "releaseGroup")]
    pub release_group: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct CaaImage {
    pub id: serde_json::Value, // could be string or integer
    pub image: String,
    #[serde(default)]
    pub thumbnails: HashMap<String, String>,
    #[serde(default)]
    pub front: bool,
    #[serde(default)]
    pub back: bool,
    #[serde(default)]
    pub types: Vec<String>,
    #[serde(default)]
    pub approved: Option<bool>,
    pub comment: Option<String>,
}
