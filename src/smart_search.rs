use crate::{llm_service::LlmService, models::*, scraper::*};
use crate::pirate_bay_scraper::TorrentResult;
use anyhow::Result;
use std::collections::HashSet;

pub struct SmartSearcher {
    llm: LlmService,
    min_confidence: f32,
}

impl SmartSearcher {
    pub fn new(llm: LlmService, min_confidence: f32) -> Self {
        Self {
            llm,
            min_confidence,
        }
    }

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

pub fn display_evaluated_result(index: usize, result: &EvaluatedResult, verbose: bool) {
    println!("\n{}. [{}% match] {}", 
        index, 
        (result.relevance_score * 100.0) as u8,
        result.torrent.title
    );
    
    // Display match reasons
    for reason in &result.match_reasons {
        println!("   ✓ {}", reason);
    }
    
    // Display warnings
    for warning in &result.warnings {
        println!("   ⚠ {}", warning);
    }
    
    // Display torrent info
    let size_str = result.torrent.size.as_ref()
        .map(|s| s.as_str())
        .unwrap_or("Unknown");
    let seeders = result.torrent.seeders.unwrap_or(0);
    let leechers = result.torrent.leechers.unwrap_or(0);
    println!("   📦 {} | 👥 {}/{} seeders/leechers", size_str, seeders, leechers);
    
    if verbose {
        println!("   Confidence: {:.0}%", result.confidence * 100.0);
        println!("   Quality Score: {:.0}%", result.quality_score * 100.0);
        println!("   Completeness: {:.0}%", result.completeness_score * 100.0);
    }
}