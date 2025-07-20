use anyhow::Result;
use clap::{Parser, Subcommand};
use std::path::PathBuf;
use tracing::info;
use tracing_subscriber;

mod downloader;
mod pirate_bay_scraper;
mod yts_scraper;
mod scraper;
mod models;
mod prompts;
mod llm_service;
mod smart_search;

#[derive(Parser)]
#[command(name = "torrentai")]
#[command(about = "Natural Language BitTorrent Client", long_about = None)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Download a torrent from a magnet link or .torrent file
    Download {
        /// The magnet link or path to .torrent file
        torrent: String,
        
        /// Download directory
        #[arg(short, long, default_value = "./downloads")]
        output: PathBuf,
    },
    
    /// Search for torrents on ThePirateBay
    Search {
        /// Search query
        query: String,
    },
    
    /// Search for movies on YTS
    SearchYts {
        /// Search query
        query: String,
    },
    
    /// Search both ThePirateBay and YTS
    SearchAll {
        /// Search query
        query: String,
    },
    
    /// Show status of active downloads
    Status,
    
    /// List downloaded content
    List,
    
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
        
        /// Download directory (if auto-download is enabled)
        #[arg(short, long, default_value = "./downloads")]
        output: PathBuf,
    },
}

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt::init();
    
    let cli = Cli::parse();
    
    match cli.command {
        Commands::Download { torrent, output } => {
            downloader::download_torrent(&torrent, output).await?;
        }
        Commands::Search { query } => {
            use crate::scraper::PirateBayScraper;
            
            let scraper = PirateBayScraper::new();
            let results = scraper.search(&query).await?;
            
            if results.is_empty() {
                println!("No results found for: {}", query);
            } else {
                println!("\nThePirateBay search results for: {}\n", query);
                println!("{:-<120}", "");
                
                for (i, result) in results.iter().enumerate() {
                    println!("{}. {}", i + 1, result.title);
                    
                    if let Some(size) = &result.size {
                        print!("   Size: {}", size);
                    }
                    if let Some(seeders) = result.seeders {
                        print!(" | Seeders: {}", seeders);
                    }
                    if let Some(leechers) = result.leechers {
                        print!(" | Leechers: {}", leechers);
                    }
                    if let Some(uploaded) = &result.uploaded {
                        print!(" | Uploaded: {}", uploaded);
                    }
                    println!();
                    
                    println!("   Magnet: {}", result.magnet_link);
                    println!("{:-<120}", "");
                }
                
                println!("\nTotal results: {}", results.len());
            }
        }
        Commands::SearchYts { query } => {
            use crate::scraper::YtsScraper;
            
            let scraper = YtsScraper::new();
            let results = scraper.search(&query).await?;
            
            if results.is_empty() {
                println!("No results found for: {}", query);
            } else {
                println!("\nYTS search results for: {}\n", query);
                println!("{:-<120}", "");
                
                for (i, result) in results.iter().enumerate() {
                    println!("{}. {}", i + 1, result.title);
                    
                    if let Some(size) = &result.size {
                        print!("   Size: {}", size);
                    }
                    if let Some(seeders) = result.seeders {
                        print!(" | Seeders: {}", seeders);
                    }
                    if let Some(leechers) = result.leechers {
                        print!(" | Leechers: {}", leechers);
                    }
                    if let Some(uploaded) = &result.uploaded {
                        print!(" | Uploaded: {}", uploaded);
                    }
                    println!();
                    
                    println!("   Magnet: {}", result.magnet_link);
                    println!("{:-<120}", "");
                }
                
                println!("\nTotal results: {}", results.len());
            }
        }
        Commands::SearchAll { query } => {
            use crate::scraper::{PirateBayScraper, YtsScraper};
            
            println!("\nSearching both ThePirateBay and YTS for: {}\n", query);
            
            let tpb_scraper = PirateBayScraper::new();
            let yts_scraper = YtsScraper::new();
            
            // Search both sources concurrently
            let (tpb_results, yts_results) = tokio::try_join!(
                tpb_scraper.search(&query),
                yts_scraper.search(&query)
            )?;
            
            // Display ThePirateBay results
            if !tpb_results.is_empty() {
                println!("📦 ThePirateBay Results ({}):", tpb_results.len());
                println!("{:-<120}", "");
                
                for (i, result) in tpb_results.iter().take(10).enumerate() {
                    println!("{}. {}", i + 1, result.title);
                    
                    if let Some(size) = &result.size {
                        print!("   Size: {}", size);
                    }
                    if let Some(seeders) = result.seeders {
                        print!(" | Seeders: {}", seeders);
                    }
                    if let Some(leechers) = result.leechers {
                        print!(" | Leechers: {}", leechers);
                    }
                    if let Some(uploaded) = &result.uploaded {
                        print!(" | Uploaded: {}", uploaded);
                    }
                    println!();
                    
                    println!("   Magnet: {}", result.magnet_link);
                    println!("{:-<120}", "");
                }
                
                if tpb_results.len() > 10 {
                    println!("... and {} more results", tpb_results.len() - 10);
                }
            } else {
                println!("📦 ThePirateBay: No results found");
            }
            
            println!();
            
            // Display YTS results
            if !yts_results.is_empty() {
                println!("🎬 YTS Results ({}):", yts_results.len());
                println!("{:-<120}", "");
                
                for (i, result) in yts_results.iter().take(10).enumerate() {
                    println!("{}. {}", i + 1, result.title);
                    
                    if let Some(size) = &result.size {
                        print!("   Size: {}", size);
                    }
                    if let Some(seeders) = result.seeders {
                        print!(" | Seeders: {}", seeders);
                    }
                    if let Some(leechers) = result.leechers {
                        print!(" | Leechers: {}", leechers);
                    }
                    if let Some(uploaded) = &result.uploaded {
                        print!(" | Uploaded: {}", uploaded);
                    }
                    println!();
                    
                    println!("   Magnet: {}", result.magnet_link);
                    println!("{:-<120}", "");
                }
                
                if yts_results.len() > 10 {
                    println!("... and {} more results", yts_results.len() - 10);
                }
            } else {
                println!("🎬 YTS: No results found");
            }
            
            println!("\nTotal results: {} (TPB: {}, YTS: {})", 
                     tpb_results.len() + yts_results.len(), 
                     tpb_results.len(), 
                     yts_results.len());
        }
        Commands::Status => {
            info!("Status command not yet implemented");
        }
        Commands::List => {
            info!("List command not yet implemented");
        }
        Commands::SmartSearch { query, auto_download, min_confidence, model, verbose, output } => {
            use crate::llm_service::LlmService;
            use crate::smart_search::{SmartSearcher, display_evaluated_result};
            
            // Initialize LLM service
            let llm = LlmService::new(model)?;
            
            // Check LLM availability
            println!("🔍 Checking LLM service...");
            llm.health_check().await?;
            llm.ensure_model().await?;
            
            // Create searcher
            let searcher = SmartSearcher::new(llm, min_confidence);
            
            // Perform search
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
                    println!("To download, run: torrentai download \"{}\"", best.torrent.magnet_link);
                }
            }
        }
    }
    
    Ok(())
}