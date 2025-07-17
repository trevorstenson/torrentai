use anyhow::Result;
use reqwest;
use serde::Deserialize;
use tracing::info;

use crate::pirate_bay_scraper::TorrentResult;

#[derive(Debug, Deserialize)]
struct YtsResponse {
    status: String,
    data: YtsData,
}

#[derive(Debug, Deserialize)]
struct YtsData {
    movies: Option<Vec<YtsMovie>>,
}

#[derive(Debug, Deserialize)]
struct YtsMovie {
    title: String,
    year: u32,
    rating: f32,
    runtime: Option<u32>,
    genres: Option<Vec<String>>,
    torrents: Option<Vec<YtsTorrent>>,
    date_uploaded: Option<String>,
}

#[derive(Debug, Deserialize)]
struct YtsTorrent {
    hash: String,
    quality: String,
    #[serde(rename = "type")]
    torrent_type: Option<String>,
    seeds: Option<u32>,
    peers: Option<u32>,
    size: Option<String>,
    date_uploaded: Option<String>,
}

pub struct YtsScraper {
    client: reqwest::Client,
    base_url: String,
}

impl YtsScraper {
    pub fn new() -> Self {
        let client = reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(30))
            .user_agent("Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36")
            .build()
            .expect("Failed to create HTTP client");
        
        Self {
            client,
            base_url: "https://yts.mx/api/v2".to_string(),
        }
    }
    
    pub async fn search(&self, query: &str) -> Result<Vec<TorrentResult>> {
        let search_url = format!("{}/list_movies.json", self.base_url);
        info!("Searching YTS: {}", search_url);
        
        let response = self.client
            .get(&search_url)
            .query(&[
                ("query_term", query),
                ("limit", "50"),
                ("sort_by", "date_added"),
                ("order_by", "desc"),
            ])
            .send()
            .await?;
        
        if !response.status().is_success() {
            return Err(anyhow::anyhow!("HTTP error: {}", response.status()));
        }
        
        let json_content = response.text().await?;
        
        // Debug: Save JSON to file for inspection
        if std::env::var("DEBUG_JSON").is_ok() {
            std::fs::write("debug_yts_results.json", &json_content)?;
            info!("Saved JSON to debug_yts_results.json");
        }
        
        self.parse_api_response(&json_content)
    }
    
    fn parse_api_response(&self, json: &str) -> Result<Vec<TorrentResult>> {
        let response: YtsResponse = serde_json::from_str(json)?;
        let mut results = Vec::new();
        
        if response.status != "ok" {
            return Err(anyhow::anyhow!("YTS API returned error status: {}", response.status));
        }
        
        let movies = response.data.movies.unwrap_or_default();
        
        for movie in movies {
            let torrents = movie.torrents.unwrap_or_default();
            
            for torrent in torrents {
                // Generate magnet link from hash
                let magnet_link = format!(
                    "magnet:?xt=urn:btih:{}&dn={}&tr=udp://open.demonii.com:1337&tr=udp://tracker.openbittorrent.com:80&tr=udp://tracker.coppersurfer.tk:6969&tr=udp://glotorrents.pw:6969/announce&tr=udp://tracker.opentrackr.org:1337/announce&tr=udp://torrent.gresille.org:80/announce&tr=udp://p4p.arenabg.com:1337&tr=udp://tracker.leechers-paradise.org:6969",
                    torrent.hash,
                    urlencoding::encode(&format!("{} ({}) [{}]", movie.title, movie.year, torrent.quality))
                );
                
                let title = format!("{} ({}) [{}]", movie.title, movie.year, torrent.quality);
                
                results.push(TorrentResult {
                    title,
                    magnet_link,
                    size: torrent.size,
                    seeders: torrent.seeds,
                    leechers: torrent.peers,
                    uploaded: torrent.date_uploaded.or(movie.date_uploaded.clone()),
                });
            }
        }
        
        info!("Found {} YTS results", results.len());
        Ok(results)
    }
}

// Helper module for URL encoding
mod urlencoding {
    pub fn encode(s: &str) -> String {
        s.chars()
            .map(|c| match c {
                ' ' => "%20".to_string(),
                '!' => "%21".to_string(),
                '"' => "%22".to_string(),
                '#' => "%23".to_string(),
                '$' => "%24".to_string(),
                '%' => "%25".to_string(),
                '&' => "%26".to_string(),
                '\'' => "%27".to_string(),
                '(' => "%28".to_string(),
                ')' => "%29".to_string(),
                '*' => "%2A".to_string(),
                '+' => "%2B".to_string(),
                ',' => "%2C".to_string(),
                '/' => "%2F".to_string(),
                ':' => "%3A".to_string(),
                ';' => "%3B".to_string(),
                '<' => "%3C".to_string(),
                '=' => "%3D".to_string(),
                '>' => "%3E".to_string(),
                '?' => "%3F".to_string(),
                '@' => "%40".to_string(),
                '[' => "%5B".to_string(),
                ']' => "%5D".to_string(),
                _ if c.is_alphanumeric() || c == '-' || c == '_' || c == '.' || c == '~' => c.to_string(),
                _ => format!("%{:02X}", c as u8),
            })
            .collect()
    }
} 