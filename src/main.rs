use anyhow::Result;
use clap::{Parser, Subcommand};
use std::path::PathBuf;
use tracing::info;
use tracing_subscriber;

mod downloader;
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
                println!("\nSearch results for: {}\n", query);
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
        Commands::Status => {
            info!("Status command not yet implemented");
        }
        Commands::List => {
            info!("List command not yet implemented");
        }
    }
    
    Ok(())
}