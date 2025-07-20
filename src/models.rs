use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use crate::pirate_bay_scraper::TorrentResult;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ContentType {
    Movie,
    #[serde(rename = "tv_show")]
    TVShow,
    Music,
    Software,
    Book,
    Game,
    Other(String),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchIntent {
    pub content_type: ContentType,
    pub title: String,
    pub year: Option<u16>,
    pub tv_details: Option<TvDetails>,
    pub quality_preferences: Vec<String>,
    pub language: Option<String>,
    pub additional_context: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TvDetails {
    pub season: Option<u8>,
    pub episode: Option<u8>,
    pub episode_range: Option<(u8, u8)>,
    pub complete_season: bool,
    pub complete_series: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EvaluatedResult {
    pub torrent: TorrentResult,
    pub relevance_score: f32,  // 0.0 to 1.0
    pub confidence: f32,       // 0.0 to 1.0
    pub match_reasons: Vec<String>,
    pub warnings: Vec<String>,
    pub quality_score: f32,
    pub completeness_score: f32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchStrategy {
    pub primary_queries: Vec<String>,
    pub fallback_queries: Vec<String>,
    pub scraper_hints: HashMap<String, Vec<String>>,
}