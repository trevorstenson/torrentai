# TorrentAI - Natural Language BitTorrent Client

This file provides guidance to Claude Code when working with the TorrentAI project - a Rust-based BitTorrent client enhanced with local LLM capabilities for natural language content discovery and management.

## Project Overview

TorrentAI is an intelligent torrent client that accepts natural language requests and automatically finds, evaluates, and downloads content from multiple torrent indexers. Users can make requests like "find and download all the Jason Bourne movies" and the system will handle the entire process from search to download.

### Core Features
- **Natural Language Processing**: Parse user requests using local LLMs
- **Multi-Indexer Search**: Scrape and search across multiple torrent sites (ThePirateBay, 1337x.to, etc.)
- **Intelligent Selection**: Rank torrents by quality, seeders, file size, and authenticity
- **Automated Downloads**: Programmatically manage torrent downloads
- **Content Organization**: Automatically organize downloaded files with proper naming

## Technical Requirements

### Core Technology Stack
- **Backend**: Rust for performance and safety
- **Interface**: Terminal/CLI initially (future: web or desktop GUI)
- **LLM Integration**: Local LLM for request parsing and content understanding
- **Torrent Engine**: Rust torrent library for download management
- **Web Scraping**: HTTP client and HTML parsing for indexer integration

### Key Dependencies (Rust Crates)
TBD

## Architecture Design

### Core Components

TBD

### Message Flow
1. User input → Request Parser (LLM) → Structured query
2. Structured query → Indexer Manager → Raw search results
3. Raw results → Torrent Evaluator → Ranked candidates
4. Top candidates → Download Manager → Active downloads
5. Completed downloads → Content Organizer → Organized library

## Implementation Phases

### Phase 1: Basic Bittorrent downloading from code

## Example Usage

```bash
# Basic movie search
torrentai "download the matrix trilogy"

# TV series with specific quality
torrentai "get breaking bad season 1 in 1080p"

# Music with format preference
torrentai "find pink floyd dark side of the moon flac"

# Interactive mode for ambiguous requests
torrentai "download john wick"
# → "Found multiple John Wick movies. Which would you like?"
# → "1. John Wick (2014), 2. John Wick 2 (2017), 3. John Wick 3 (2019), 4. All of them"

# Status and management
torrentai status                    # Show active downloads
torrentai list                      # Show downloaded content
torrentai search "blade runner"     # Search without downloading
```

## Configuration

### Config File (`~/.torrentai/config.toml`)
```toml
[download]
path = "~/Downloads/TorrentAI"
max_concurrent = 3
bandwidth_limit = "10MB/s"

[indexers]
enabled = ["piratebay", "1337x"]
timeout = 30
retry_attempts = 3

[llm]
model_path = "~/.torrentai/models/llama-7b.gguf"
context_length = 2048

[quality]
min_seeders = 5
prefer_verified = true
max_file_age_days = 365
```

## Future Roadmap

### Content Quality Intelligence
- Video quality analysis using ML
- Audio quality detection for music
- Malware scanning integration
- Community-driven reputation system

### Proactive Features
- Release calendar monitoring
- Automatic upgrading to better versions
- Smart storage management
- Recommendation engine

### Advanced Interfaces
- Discord/Telegram bot integration
- Mobile app companion
- Home media server integration (Plex/Jellyfin)
- Voice control capabilities

## Development Guidelines

### Code Organization
- Use module-based architecture for easy testing
- Implement proper error handling with custom error types
- Add comprehensive logging for debugging
- Write unit tests for core logic components

### Security Considerations
- Never log or store user queries permanently
- Implement secure credential storage for indexers
- Use VPN/proxy support for privacy
- Validate all downloaded content before execution

### Performance Requirements
- Concurrent indexer searches for speed
- Efficient HTML parsing to minimize memory usage
- Smart caching to reduce redundant requests
- Async/await throughout for responsiveness

## Getting Started

1. **Setup Development Environment**
   ```bash
   cargo new torrentai
   cd torrentai
   # Copy this file to CLAUDE.md
   # Add dependencies to Cargo.toml
   ```

2. **Initial Implementation Order**
   - Start with CLI argument parsing
   - Implement single indexer scraping
   - Add basic LLM integration
   - Connect torrent downloading
   - Test with simple queries

3. **Testing Strategy**
   - Unit tests for each component
   - Integration tests with mock indexers
   - End-to-end tests with real (legal) content
   - Performance benchmarks for search speed

This project combines practical utility with learning opportunities in AI integration, web scraping, P2P protocols, and systems programming in Rust.