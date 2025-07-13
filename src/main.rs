use anyhow::Result;
use clap::{Parser, Subcommand};
use std::path::PathBuf;
use tracing::info;
use tracing_subscriber;

mod downloader;

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
        Commands::Status => {
            info!("Status command not yet implemented");
        }
        Commands::List => {
            info!("List command not yet implemented");
        }
    }
    
    Ok(())
}