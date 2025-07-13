# TorrentAI

A natural language BitTorrent client that uses local LLMs to understand and fulfill content requests automatically.

## Overview

TorrentAI is an intelligent torrent client built in Rust that accepts natural language requests and automatically finds, evaluates, and downloads content from multiple torrent indexers. Simply tell it what you want, and it handles the entire process from search to download.

## Features
TBD

## Example Usage

```bash
# Download movies
torrentai "download the matrix trilogy"

# TV series with quality preference
torrentai "get breaking bad season 1 in 1080p"

# Music with format preference
torrentai "find pink floyd dark side of the moon flac"

# Management commands
torrentai status    # Show active downloads
torrentai list      # Show downloaded content
```

## Quick Start

### Development Usage

Currently in early development. To test basic torrent downloading:

```bash
cargo run -- download "magnet:?xt=urn:btih:cab507494d02ebb1178b38f2e9d7be299c86b862"
```