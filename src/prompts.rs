use crate::models::{ContentType, SearchIntent};
use crate::pirate_bay_scraper::TorrentResult;

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

Respond with a JSON array of evaluations in order. Each evaluation should have this structure:
{{
    "relevance_score": 0.95,
    "confidence": 0.9,
    "match_reasons": ["Complete season 2", "High quality BluRay"],
    "warnings": [],
    "quality_score": 0.9,
    "completeness_score": 1.0
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
        },
        results.iter().enumerate()
            .map(|(i, r)| format!("{}: {}", i + 1, r.title))
            .collect::<Vec<_>>()
            .join("\n")
    )
}

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