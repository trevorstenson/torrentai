# TorrentAI

A natural language BitTorrent client that uses local LLMs to understand and fulfill content requests automatically.

## Overview

TorrentAI is an intelligent torrent client built in Rust that accepts natural language requests and automatically finds, evaluates, and downloads content from multiple torrent indexers. Simply tell it what you want, and it handles the entire process from search to download.

## Features

- **Multi-Source Search**: Search across multiple torrent indexers (ThePirateBay, YTS.mx)
- **Movie-Focused YTS Integration**: High-quality movie torrents from YTS with multiple resolutions
- **Concurrent Search**: Search multiple sources simultaneously for faster results
- **Clean Output**: Well-formatted results with clear source identification
- **Direct Download**: Download torrents via magnet links
- **Rust Performance**: Fast and reliable implementation

## Search Commands

### Search ThePirateBay
```bash
torrentai search "iron man"
```

### Search YTS Movies
```bash
torrentai search-yts "avengers"
```

### Search All Sources
```bash
torrentai search-all "iron man"
```

## Example Usage

```bash
# Search for movies on YTS (high quality, small file sizes)
torrentai search-yts "the matrix"

# Search across all sources for maximum results
torrentai search-all "breaking bad"

# Search ThePirateBay for general torrents
torrentai search "ubuntu iso"

# Download torrents directly
torrentai download "magnet:?xt=urn:btih:..."

# Management commands
torrentai status    # Show active downloads
torrentai list      # Show downloaded content
```

## Quick Start

### Development Usage

Currently in early development. To test the search and download functionality:

```bash
# Test YTS movie search
cargo run -- search-yts "iron man"

# Test combined search
cargo run -- search-all "avengers"

# Test direct download
cargo run -- download "magnet:?xt=urn:btih:cab507494d02ebb1178b38f2e9d7be299c86b862"
```

## Supported Sources

- **ThePirateBay**: General torrent search via HTML scraping
- **YTS.mx**: High-quality movie torrents via official API
- More sources planned for future releases