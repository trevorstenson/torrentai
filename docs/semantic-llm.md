# Semantic LLM Integration for TorrentAI

## Overview

This document outlines the implementation plan for integrating local LLM capabilities into TorrentAI to provide semantic understanding of natural language queries and intelligent torrent selection.

### Problem Statement

Current string-based search has limitations:
- Can't understand semantic variations ("season 2" vs "S02" vs "second season")
- Can't distinguish between single episodes and complete seasons
- No intelligent ranking based on user intent
- Poor handling of ambiguous queries

### Solution

Integrate Ollama-based local LLM to:
1. Parse natural language queries into structured search intents
2. Generate optimized search queries for each indexer
3. Evaluate and rank results based on relevance to original intent
4. Provide explanations for ranking decisions

## Architecture

### Core Components

```
User Query → LLM Service → Search Intent → Scrapers → Raw Results
                                    ↓                        ↓
                              Search Queries          LLM Evaluator
                                                            ↓
                                                    Ranked Results → User
```

### Module Structure

```
src/
├── llm_service.rs      # Core LLM interaction logic
├── models.rs           # Data structures for LLM operations
├── prompts.rs          # LLM prompt templates
├── smart_search.rs     # Smart search command implementation
└── main.rs             # CLI integration
```

## Data Structures

### Core Models

```rust
// src/models.rs

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ContentType {
    Movie,
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
```

## LLM Service Implementation

### Core Service

```rust
// src/llm_service.rs

use ollama_rs::{Ollama, generation::completion::request::GenerationRequest};
use anyhow::Result;
use serde_json;

pub struct LlmService {
    ollama: Ollama,
    model: String,
    temperature: f32,
}

impl LlmService {
    pub fn new(model: String) -> Result<Self> {
        let ollama = Ollama::default();
        Ok(Self {
            ollama,
            model,
            temperature: 0.3, // Low temperature for consistent parsing
        })
    }

    pub async fn parse_query(&self, query: &str) -> Result<SearchIntent> {
        let prompt = self.build_parse_prompt(query);
        let response = self.generate(&prompt).await?;
        self.parse_json_response(&response)
    }

    pub async fn evaluate_results(
        &self, 
        intent: &SearchIntent, 
        results: Vec<TorrentResult>
    ) -> Result<Vec<EvaluatedResult>> {
        let prompt = self.build_evaluation_prompt(intent, &results);
        let response = self.generate(&prompt).await?;
        self.parse_evaluation_response(&response, results)
    }

    pub async fn generate_search_queries(&self, intent: &SearchIntent) -> Result<SearchStrategy> {
        let prompt = self.build_query_generation_prompt(intent);
        let response = self.generate(&prompt).await?;
        self.parse_search_strategy(&response)
    }

    async fn generate(&self, prompt: &str) -> Result<String> {
        let request = GenerationRequest::new(self.model.clone(), prompt.to_string())
            .temperature(self.temperature);
        
        let response = self.ollama.generate(request).await?;
        Ok(response.response)
    }
}
```

## Prompt Templates

### Query Parsing Prompt

```rust
// src/prompts.rs

pub fn build_parse_prompt(query: &str) -> String {
    format!(r#"
You are a torrent search assistant. Parse the following natural language query into structured JSON.

Query: "{}"

Extract the following information:
1. Content type (movie, tv_show, music, software, book, game, other)
2. Title of the content
3. For TV shows: season number, episode number(s), whether they want complete season/series
4. Year (if mentioned)
5. Quality preferences (1080p, 4K, BluRay, etc.)
6. Language preferences
7. Any other relevant context

Respond with ONLY valid JSON in this format:
{{
    "content_type": "tv_show",
    "title": "Breaking Bad",
    "year": null,
    "tv_details": {{
        "season": 2,
        "episode": null,
        "episode_range": null,
        "complete_season": true,
        "complete_series": false
    }},
    "quality_preferences": [],
    "language": null,
    "additional_context": []
}}
"#, query)
}

pub fn build_evaluation_prompt(intent: &SearchIntent, results: &[TorrentResult]) -> String {
    format!(r#"
You are evaluating torrent search results for relevance.

User wants: {} - {}{}

Results to evaluate:
{}

For each result, provide:
1. Relevance score (0.0-1.0) - how well it matches the request
2. Confidence (0.0-1.0) - how sure you are about the match
3. Match reasons - why this is or isn't a good match
4. Warnings - any concerns (fake, wrong content, low quality)
5. Quality score (0.0-1.0) - based on resolution, encoding, source
6. Completeness score (0.0-1.0) - does it have everything requested?

Respond with a JSON array of evaluations in order.
"#,
        match &intent.content_type {
            ContentType::TVShow => "TV Show",
            ContentType::Movie => "Movie",
            _ => "Content",
        },
        intent.title,
        if let Some(tv) = &intent.tv_details {
            format!(" Season {}", tv.season.unwrap_or(0))
        } else {
            String::new()
        },
        results.iter().enumerate()
            .map(|(i, r)| format!("{}: {}", i + 1, r.title))
            .collect::<Vec<_>>()
            .join("\n")
    )
}
```

### Search Query Generation

```rust
pub fn build_query_generation_prompt(intent: &SearchIntent) -> String {
    format!(r#"
Generate optimized search queries for finding: {} - {}{}

Create multiple search query variations that torrent sites would understand:
1. Primary queries - most likely to find exact matches
2. Fallback queries - broader searches if primary fails
3. Scraper-specific hints - special formats for different sites

Consider variations like:
- "Breaking Bad S02" vs "Breaking Bad Season 2"
- With/without year
- Complete/Full/All episodes
- Different quality indicators

Respond with JSON:
{{
    "primary_queries": ["query1", "query2"],
    "fallback_queries": ["query3"],
    "scraper_hints": {{
        "piratebay": ["special format"],
        "yts": ["movie specific format"]
    }}
}}
"#,
        match &intent.content_type {
            ContentType::TVShow => "TV Show",
            ContentType::Movie => "Movie",
            _ => "Content",
        },
        intent.title,
        if let Some(tv) = &intent.tv_details {
            format!(" Season {}", tv.season.unwrap_or(0))
        } else {
            String::new()
        }
    )
}
```

## Smart Search Implementation

```rust
// src/smart_search.rs

use crate::{llm_service::LlmService, models::*, scraper::*};

pub struct SmartSearcher {
    llm: LlmService,
    min_confidence: f32,
}

impl SmartSearcher {
    pub async fn search(&self, query: &str) -> Result<Vec<EvaluatedResult>> {
        // 1. Parse query into intent
        println!("🤖 Understanding your request...");
        let intent = self.llm.parse_query(query).await?;
        self.display_intent(&intent);

        // 2. Generate search queries
        let strategy = self.llm.generate_search_queries(&intent).await?;
        
        // 3. Search across all scrapers
        println!("\n🔍 Searching across sources...");
        let mut all_results = Vec::new();
        
        for query in &strategy.primary_queries {
            let results = self.search_all_sources(query).await?;
            all_results.extend(results);
            
            if all_results.len() >= 20 {
                break; // Enough results to evaluate
            }
        }

        // 4. Deduplicate results
        let unique_results = self.deduplicate_results(all_results);

        // 5. Evaluate and rank results
        println!("\n📊 Evaluating {} results...", unique_results.len());
        let evaluated = self.llm.evaluate_results(&intent, unique_results).await?;
        
        // 6. Filter by confidence and sort by relevance
        let mut filtered: Vec<_> = evaluated.into_iter()
            .filter(|r| r.confidence >= self.min_confidence)
            .collect();
        
        filtered.sort_by(|a, b| {
            b.relevance_score.partial_cmp(&a.relevance_score).unwrap()
        });

        Ok(filtered)
    }

    async fn search_all_sources(&self, query: &str) -> Result<Vec<TorrentResult>> {
        let tpb = PirateBayScraper::new();
        let yts = YtsScraper::new();
        
        let (tpb_results, yts_results) = tokio::try_join!(
            tpb.search(query),
            yts.search(query)
        )?;

        let mut results = Vec::new();
        results.extend(tpb_results);
        results.extend(yts_results);
        
        Ok(results)
    }

    fn display_intent(&self, intent: &SearchIntent) {
        println!("   Content Type: {:?}", intent.content_type);
        println!("   Title: {}", intent.title);
        
        if let Some(tv) = &intent.tv_details {
            if let Some(season) = tv.season {
                println!("   Season: {} {}", 
                    season, 
                    if tv.complete_season { "(Complete)" } else { "" }
                );
            }
        }
        
        if !intent.quality_preferences.is_empty() {
            println!("   Quality: {}", intent.quality_preferences.join(", "));
        }
    }

    fn deduplicate_results(&self, results: Vec<TorrentResult>) -> Vec<TorrentResult> {
        let mut seen = HashSet::new();
        results.into_iter()
            .filter(|r| seen.insert(r.magnet_link.clone()))
            .collect()
    }
}
```

## CLI Integration

```rust
// Update Commands enum in main.rs

#[derive(Subcommand)]
enum Commands {
    // ... existing commands ...
    
    /// Smart search using natural language
    SmartSearch {
        /// Natural language search query
        query: String,
        
        /// Automatically download the best match
        #[arg(long)]
        auto_download: bool,
        
        /// Minimum confidence threshold (0.0-1.0)
        #[arg(long, default_value = "0.7")]
        min_confidence: f32,
        
        /// LLM model to use
        #[arg(long, default_value = "deepseek-r1:7b")]
        model: String,
        
        /// Show detailed evaluation reasoning
        #[arg(long)]
        verbose: bool,
    },
}

// In main.rs match statement
Commands::SmartSearch { query, auto_download, min_confidence, model, verbose } => {
    let llm = LlmService::new(model)?;
    let searcher = SmartSearcher::new(llm, min_confidence);
    
    let results = searcher.search(&query).await?;
    
    if results.is_empty() {
        println!("\n❌ No results found with confidence >= {}", min_confidence);
        return Ok(());
    }
    
    // Display results
    println!("\n📊 Top Results (ranked by relevance):");
    for (i, result) in results.iter().take(5).enumerate() {
        display_evaluated_result(i + 1, result, verbose);
    }
    
    // Auto-download logic
    if auto_download && !results.is_empty() {
        let best = &results[0];
        if best.relevance_score >= 0.9 {
            println!("\n✅ Auto-downloading best match...");
            downloader::download_torrent(&best.torrent.magnet_link, output).await?;
        } else {
            println!("\n⚠️  Best match has relevance {:.0}% - manual confirmation required", 
                     best.relevance_score * 100.0);
            // Interactive confirmation logic
        }
    }
}
```

## Error Handling

### Key Considerations

1. **LLM Availability**: Check if Ollama is running
```rust
impl LlmService {
    pub async fn health_check(&self) -> Result<bool> {
        match self.ollama.list_local_models().await {
            Ok(_) => Ok(true),
            Err(_) => Err(anyhow!("Ollama is not running. Start with: ollama serve"))
        }
    }
}
```

2. **Model Availability**: Verify requested model exists
```rust
pub async fn ensure_model(&self) -> Result<()> {
    let models = self.ollama.list_local_models().await?;
    if !models.iter().any(|m| m.name == self.model) {
        return Err(anyhow!("Model {} not found. Pull with: ollama pull {}", 
                           self.model, self.model));
    }
    Ok(())
}
```

3. **JSON Parsing**: Handle malformed LLM responses
```rust
fn parse_json_response<T: DeserializeOwned>(&self, response: &str) -> Result<T> {
    // Try to extract JSON from response (LLM might add explanation)
    let json_start = response.find('{').unwrap_or(0);
    let json_end = response.rfind('}').map(|i| i + 1).unwrap_or(response.len());
    let json_str = &response[json_start..json_end];
    
    serde_json::from_str(json_str)
        .map_err(|e| anyhow!("Failed to parse LLM response: {}", e))
}
```

## Configuration

### Config Structure

```toml
# ~/.torrentai/config.toml

[llm]
model = "deepseek-r1:7b"
temperature = 0.3
timeout = 30
max_retries = 3

[smart_search]
min_confidence = 0.7
auto_download_threshold = 0.9
max_results_to_evaluate = 50
verbose_by_default = false

[scrapers]
concurrent_searches = 3
deduplicate = true
```

## Testing Strategy

### Unit Tests

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_intent_parsing() {
        let test_cases = vec![
            ("breaking bad season 2", ContentType::TVShow, Some(2)),
            ("the matrix 1999", ContentType::Movie, Some(1999)),
            ("pink floyd dark side flac", ContentType::Music, None),
        ];
        
        // Test each case
    }

    #[test]
    fn test_result_deduplication() {
        // Test magnet link deduplication
    }
}
```

### Integration Tests

1. Mock Ollama responses for testing
2. Test full search flow with fixtures
3. Test error handling scenarios

## Performance Optimization

1. **Concurrent Processing**
   - Search multiple scrapers in parallel
   - Batch evaluate results with LLM

2. **Caching**
   - Cache parsed intents for similar queries
   - Cache LLM evaluations for identical results

3. **Early Termination**
   - Stop searching once enough high-confidence results found
   - Implement progressive result display

## Future Enhancements

1. **Multi-language Support**
   - Detect query language
   - Search in appropriate sites/languages

2. **Learning from Feedback**
   - Track which results users actually download
   - Fine-tune ranking based on user choices

3. **Advanced Filtering**
   - Exclude specific uploaders
   - Prefer verified torrents
   - Filter by file format preferences

4. **Batch Operations**
   - Process multiple queries at once
   - Download entire series progressively

5. **Integration Features**
   - Web UI with visual result comparison
   - Discord/Telegram bot interface
   - API endpoint for external tools

## Example Interactions

### TV Show Search
```bash
$ torrentai smart-search "I want to watch the second season of breaking bad"

🤖 Understanding your request...
   Content Type: TVShow
   Title: Breaking Bad
   Season: 2 (Complete)
   
🔍 Searching across sources...

📊 Top Results (ranked by relevance):
1. [95% match] Breaking.Bad.S02.Complete.1080p.BluRay.x264-SCENE
   ✓ Complete Season 2 (13 episodes)
   ✓ High quality (1080p BluRay)
   ✓ Trusted release group
   📦 15.2 GB | 👥 128/12 seeders/leechers
   
2. [82% match] Breaking Bad Season 2 720p HDTV
   ✓ Complete Season 2
   ⚠ Lower quality (720p HDTV)
   📦 8.7 GB | 👥 45/8 seeders/leechers

3. [45% match] Breaking.Bad.S02E01.Seven.Thirty-Seven.1080p
   ✗ Only episode 1, not complete season
   📦 1.2 GB | 👥 23/2 seeders/leechers
```

### Movie Search with Quality
```bash
$ torrentai smart-search "inception movie in 4k with atmos" --verbose

🤖 Understanding your request...
   Content Type: Movie
   Title: Inception
   Quality: ["4K", "Atmos"]
   
🔍 Searching across sources...
   Primary queries: ["Inception 4K Atmos", "Inception 2160p Atmos", "Inception UHD"]
   
📊 Evaluating 23 results...

Top Results:
1. [92% match] Inception.2010.2160p.4K.BluRay.x265.Atmos-GROUP
   ✓ Matches title exactly
   ✓ 4K resolution (2160p)
   ✓ Dolby Atmos audio
   ✓ Modern x265 encoding
   📦 28.5 GB | 👥 89/15 seeders/leechers
   
   Detailed reasoning:
   - Title match: "Inception" → exact match
   - Year verification: 2010 → correct
   - Quality match: 2160p → 4K confirmed
   - Audio match: "Atmos" in title → requested feature present
   - Encoding efficiency: x265 provides better quality/size ratio
```

## Dependencies

```toml
# Cargo.toml additions
[dependencies]
# ... existing dependencies ...
serde_json = "1.0"
tokio = { version = "1", features = ["full", "macros"] }
futures = "0.3"
lazy_static = "1.4"
regex = "1.10"
chrono = "0.4"
```

## Implementation Timeline

1. **Week 1**: Core LLM service and data models
2. **Week 2**: Query parsing and search query generation
3. **Week 3**: Result evaluation and ranking
4. **Week 4**: CLI integration and testing
5. **Week 5**: Performance optimization and error handling
6. **Week 6**: Documentation and examples

## Success Metrics

1. **Accuracy**: >90% correct intent parsing
2. **Relevance**: Top result matches intent >85% of the time
3. **Performance**: <3s for complete search and evaluation
4. **User Satisfaction**: Reduced need for manual result filtering