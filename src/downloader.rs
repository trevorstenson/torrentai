use anyhow::Result;
use librqbit::{AddTorrent, AddTorrentOptions, Session};
use std::path::PathBuf;
use tracing::info;

pub async fn download_torrent(torrent: &str, output_dir: PathBuf) -> Result<()> {
    info!("Starting download: {}", torrent);
    
    // Create the session
    let session = Session::new(output_dir).await?;
    
    // Prepare torrent addition
    let add_torrent = if torrent.starts_with("magnet:") {
        AddTorrent::from_url(torrent)
    } else if torrent.starts_with("http://") || torrent.starts_with("https://") {
        AddTorrent::from_url(torrent)
    } else {
        // Assume it's a local file path
        AddTorrent::from_local_filename(torrent)?
    };
    
    // Add the torrent with options
    let handle_result = session.add_torrent(add_torrent, Some(AddTorrentOptions::default())).await?;
    
    match handle_result {
        librqbit::AddTorrentResponse::Added(id, managed_handle) => {
            info!("Torrent added successfully with ID: {}", id);
            
            // Wait for metadata if needed
            if torrent.starts_with("magnet:") {
                info!("Waiting for metadata...");
                if let Err(e) = managed_handle.wait_until_initialized().await {
                    return Err(anyhow::anyhow!("Failed to get metadata: {}", e));
                }
                info!("Metadata received");
            }
            
            // Get torrent info
            let mut name = String::new();
            let mut total_size = 0u64;
            
            managed_handle.with_metadata(|meta| {
                name = meta.info.name
                    .as_ref()
                    .map(|n| n.to_string())
                    .unwrap_or_else(|| "Unknown".to_string());
                total_size = meta.info.iter_file_lengths()
                    .map(|iter| iter.sum::<u64>())
                    .unwrap_or(0);
            })?;
            
            info!("Torrent name: {}", name);
            info!("Total size: {} bytes", total_size);
            
            // Note: start() is private, torrents start automatically when added
            info!("Download in progress...");
            
            // Monitor progress
            loop {
                let stats = managed_handle.stats();
                info!("{}", stats);
                
                // Check if download is complete by checking if all pieces are finished
                if stats.finished {
                    info!("Download completed!");
                    break;
                }
                
                tokio::time::sleep(tokio::time::Duration::from_secs(1)).await;
            }
        }
        librqbit::AddTorrentResponse::AlreadyManaged(id, managed_handle) => {
            info!("Torrent already exists with ID: {}", id);
            
            let stats = managed_handle.stats();
            if stats.finished {
                info!("This torrent is already downloaded");
            } else {
                info!("This torrent is already being downloaded");
                // Note: Can't restart it as start() is private
            }
        }
        librqbit::AddTorrentResponse::ListOnly(_list_response) => {
            return Err(anyhow::anyhow!("Torrent was added in list-only mode. Session might be read-only."));
        }
    }
    
    Ok(())
}