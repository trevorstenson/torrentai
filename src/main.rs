use anyhow::Result;
use clap::{Parser, Subcommand};
use std::path::PathBuf;
use tracing::info;
use tracing_subscriber;

mod downloader;
mod pirate_bay_scraper;
mod yts_scraper;
mod scraper;

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
    }
    
    Ok(())
}