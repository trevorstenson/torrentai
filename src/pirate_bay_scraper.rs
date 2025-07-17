use anyhow::Result;
use reqwest;
use scraper::{Html, Selector};
use serde::{Deserialize, Serialize};
use tracing::info;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TorrentResult {
    pub title: String,
    pub magnet_link: String,
    pub size: Option<String>,
    pub seeders: Option<u32>,
    pub leechers: Option<u32>,
    pub uploaded: Option<String>,
}

pub struct PirateBayScraper {
    client: reqwest::Client,
    base_url: String,
}

impl PirateBayScraper {
    pub fn new() -> Self {
        let client = reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(30))
            .user_agent("Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36")
            .build()
            .expect("Failed to create HTTP client");
        
        Self {
            client,
            base_url: "https://thepiratebay10.info".to_string(),
        }
    }
    
    pub async fn search(&self, query: &str) -> Result<Vec<TorrentResult>> {
        let search_url = format!("{}/search/{}/1/99/0", self.base_url, urlencoding::encode(query));
        info!("Searching: {}", search_url);
        
        let response = self.client.get(&search_url).send().await?;
        
        if !response.status().is_success() {
            return Err(anyhow::anyhow!("HTTP error: {}", response.status()));
        }
        
        let html_content = response.text().await?;
        
        // Debug: Save HTML to file for inspection
        if std::env::var("DEBUG_HTML").is_ok() {
            std::fs::write("debug_search_results.html", &html_content)?;
            info!("Saved HTML to debug_search_results.html");
        }
        
        self.parse_search_results(&html_content)
    }
    
    fn parse_search_results(&self, html: &str) -> Result<Vec<TorrentResult>> {
        let document = Html::parse_document(html);
        let mut results = Vec::new();
        
        // Find the search results table
        let table_selector = Selector::parse("table#searchResult").unwrap();
        let row_selector = Selector::parse("tr").unwrap();
        let td_selector = Selector::parse("td").unwrap();
        let link_selector = Selector::parse("a").unwrap();
        let magnet_selector = Selector::parse("a[href^='magnet:']").unwrap();
        
        if let Some(table) = document.select(&table_selector).next() {
            for row in table.select(&row_selector) {
                let td_elements: Vec<_> = row.select(&td_selector).collect();
                
                // Skip rows that don't have enough columns (header row, etc.)
                if td_elements.len() < 4 {
                    continue;
                }
                
                // Extract title from second td element
                let title = if let Some(title_td) = td_elements.get(1) {
                    if let Some(link) = title_td.select(&link_selector).next() {
                        let title_text = link.text().collect::<String>().trim().to_string();
                        if !title_text.is_empty() && title_text.contains("Details for") == false {
                            Some(title_text)
                        } else {
                            None
                        }
                    } else {
                        None
                    }
                } else {
                    None
                };
                
                if title.is_none() {
                    continue;
                }
                
                // Extract magnet link
                let magnet_link = if let Some(elem) = row.select(&magnet_selector).next() {
                    Some(elem.value().attr("href").unwrap_or("").to_string())
                } else {
                    None
                };
                
                if magnet_link.is_none() || !magnet_link.as_ref().unwrap().starts_with("magnet:") {
                    continue;
                }
                
                // Extract size from 5th column (index 4)
                let size = td_elements.get(4)
                    .map(|td| td.text().collect::<String>().trim().to_string())
                    .filter(|s| !s.is_empty());
                
                // Extract seeders from 6th column (index 5)
                let seeders = td_elements.get(5)
                    .and_then(|td| td.text().collect::<String>().trim().parse::<u32>().ok());
                
                // Extract leechers from 7th column (index 6) 
                let leechers = td_elements.get(6)
                    .and_then(|td| td.text().collect::<String>().trim().parse::<u32>().ok());
                
                // Extract upload date from 3rd column (index 2)
                let uploaded = td_elements.get(2)
                    .map(|td| td.text().collect::<String>().trim().to_string())
                    .filter(|s| !s.is_empty());
                
                results.push(TorrentResult {
                    title: title.unwrap(),
                    magnet_link: magnet_link.unwrap(),
                    size,
                    seeders,
                    leechers,
                    uploaded,
                });
            }
        }
        
        info!("Found {} results", results.len());
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