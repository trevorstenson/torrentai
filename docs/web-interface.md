# Leptos Web Interface Implementation Plan

## Overview

This document outlines the implementation plan for adding a local-first web interface to TorrentAI using the Leptos framework. The interface will provide a modern, reactive UI for natural language torrent searches while maintaining the security and simplicity of the CLI tool.

### Goals
- **Local-first**: Runs on localhost by default, no cloud dependencies
- **Natural Language**: Full semantic search capabilities from the web
- **Real-time Updates**: Live progress tracking and status updates
- **Feature Parity**: Support all important CLI functionality
- **Single Binary**: Web UI embedded in the main executable

## Tech Stack

### Core Dependencies
```toml
# Cargo.toml additions
[dependencies]
# Web framework
leptos = { version = "0.6", features = ["ssr"] }
leptos_actix = { version = "0.6", features = ["ssr"] }
leptos_router = { version = "0.6", features = ["ssr"] }
actix-web = "4"
actix-ws = "0.2"

# Serialization
serde = { version = "1.0", features = ["derive"] }
serde_json = "1.0"

# Asset embedding
rust-embed = { version = "8", features = ["compression"] }

# Authentication
jsonwebtoken = "9"
uuid = { version = "1", features = ["v4", "serde"] }

# Development
[build-dependencies]
leptos_codegen = "0.6"
```

### Frontend Stack
- **Leptos**: Full-stack Rust web framework
- **Tailwind CSS**: Utility-first styling
- **DaisyUI**: Component library for rapid development
- **Chart.js**: Download progress visualization

## Architecture Overview

```
┌─────────────────────────────────────────────────┐
│                   Browser                        │
│  ┌─────────────────────────────────────────┐   │
│  │          Leptos Frontend (WASM)          │   │
│  │  ┌─────────┐  ┌──────────┐  ┌────────┐ │   │
│  │  │ Search  │  │ Results  │  │Progress│ │   │
│  │  │Component│  │   View   │  │Tracker │ │   │
│  │  └────┬────┘  └─────┬────┘  └────┬───┘ │   │
│  │       └─────────────┼────────────┘      │   │
│  │                     │                    │   │
│  │              Signal State Store          │   │
│  └─────────────────────┬────────────────────┘   │
│                        │                         │
└────────────────────────┼─────────────────────────┘
                         │ WebSocket + REST
┌────────────────────────┼─────────────────────────┐
│                 Actix Web Server                  │
│  ┌──────────────┐     │      ┌──────────────┐   │
│  │   REST API   │←────┼─────→│  WebSocket   │   │
│  │   Handlers   │     │      │   Handler    │   │
│  └──────┬───────┘     │      └──────┬───────┘   │
│         │             │             │            │
│  ┌──────┴───────────────────────────┴────────┐  │
│  │          Shared Business Logic             │  │
│  │  (LLM Service, Scrapers, Downloader)      │  │
│  └────────────────────────────────────────────┘  │
└───────────────────────────────────────────────────┘
```

## Step-by-Step Implementation Plan

### Phase 1: Basic Web Server Setup (Week 1)

#### 1.1 Create Web Server Module
```rust
// src/web/mod.rs
pub mod server;
pub mod api;
pub mod ws;
pub mod auth;

// src/web/server.rs
use actix_web::{web, App, HttpServer, middleware};
use leptos::*;
use leptos_actix::{generate_route_list, LeptosRoutes};

pub async fn start_server(config: WebConfig) -> Result<()> {
    let conf = get_configuration(None).await.unwrap();
    let addr = conf.leptos_options.site_addr;
    let routes = generate_route_list(App);

    HttpServer::new(move || {
        let leptos_options = &conf.leptos_options;
        let site_root = &leptos_options.site_root;

        App::new()
            .service(Files::new("/pkg", format!("{site_root}/pkg")))
            .service(Files::new("/assets", site_root))
            .service(favicon)
            .leptos_routes(leptos_options.to_owned(), routes.to_owned(), App)
            .app_data(web::Data::new(leptos_options.to_owned()))
            .wrap(middleware::Compress::default())
    })
    .bind(&addr)?
    .run()
    .await
}
```

#### 1.2 Update Main CLI
```rust
// Add to Commands enum in main.rs
#[derive(Subcommand)]
enum Commands {
    // ... existing commands ...
    
    /// Start the web interface
    Serve {
        /// Host to bind to
        #[arg(long, default_value = "127.0.0.1")]
        host: String,
        
        /// Port to bind to
        #[arg(long, default_value = "3000")]
        port: u16,
        
        /// Open browser automatically
        #[arg(long, default_value = "true")]
        open: bool,
    },
}
```

### Phase 2: Core API Implementation (Week 1-2)

#### 2.1 RESTful API Endpoints
```rust
// src/web/api/search.rs
#[derive(Serialize, Deserialize)]
pub struct SearchRequest {
    pub query: String,
    pub auto_download: bool,
    pub min_confidence: f32,
    pub model: Option<String>,
}

#[derive(Serialize, Deserialize)]
pub struct SearchResponse {
    pub intent: SearchIntent,
    pub results: Vec<EvaluatedResult>,
    pub search_id: String,
}

#[post("/api/search")]
pub async fn search_endpoint(
    req: web::Json<SearchRequest>,
    llm: web::Data<LlmService>,
    searcher: web::Data<SmartSearcher>,
) -> Result<HttpResponse, Error> {
    let search_id = Uuid::new_v4().to_string();
    
    // Send immediate response with search ID
    let intent = llm.parse_query(&req.query).await?;
    
    // Start async search
    tokio::spawn(async move {
        let results = searcher.search(&req.query).await;
        // Store results for later retrieval
        // Emit WebSocket event with results
    });
    
    Ok(HttpResponse::Ok().json(SearchResponse {
        intent,
        results: vec![],
        search_id,
    }))
}
```

#### 2.2 WebSocket for Real-time Updates
```rust
// src/web/ws/handler.rs
pub async fn websocket_handler(
    req: HttpRequest,
    stream: web::Payload,
) -> Result<HttpResponse, Error> {
    let (res, session, msg_stream) = actix_ws::handle(&req, stream)?;
    
    // Spawn WebSocket handler
    rt::spawn(websocket_loop(session, msg_stream));
    
    Ok(res)
}

async fn websocket_loop(mut session: Session, mut msg_stream: MessageStream) {
    while let Some(Ok(msg)) = msg_stream.next().await {
        match msg {
            Message::Text(text) => {
                let command: WsCommand = serde_json::from_str(&text).unwrap();
                handle_command(command, &mut session).await;
            }
            Message::Close(_) => break,
            _ => {}
        }
    }
}
```

### Phase 3: Leptos Frontend Components (Week 2-3)

#### 3.1 App Structure
```rust
// src/app.rs
use leptos::*;
use leptos_router::*;

#[component]
pub fn App() -> impl IntoView {
    provide_meta_context();
    
    view! {
        <Stylesheet id="leptos" href="/pkg/tailwind.css"/>
        <Router>
            <Routes>
                <Route path="/" view=HomePage/>
                <Route path="/search" view=SearchPage/>
                <Route path="/downloads" view=DownloadsPage/>
                <Route path="/settings" view=SettingsPage/>
            </Routes>
        </Router>
    }
}
```

#### 3.2 Search Component
```rust
// src/components/search.rs
#[component]
pub fn SearchBox() -> impl IntoView {
    let (query, set_query) = create_signal(String::new());
    let (loading, set_loading) = create_signal(false);
    let search_action = create_server_action::<SearchAction>();
    
    let on_submit = move |ev: SubmitEvent| {
        ev.prevent_default();
        set_loading.set(true);
        search_action.dispatch(SearchAction {
            query: query.get(),
        });
    };
    
    view! {
        <form on:submit=on_submit class="w-full max-w-3xl mx-auto">
            <div class="relative">
                <input
                    type="text"
                    placeholder="Find me the entire Lord of the Rings trilogy in 4K..."
                    class="input input-bordered input-lg w-full pr-24"
                    prop:value=query
                    on:input=move |ev| set_query.set(event_target_value(&ev))
                    disabled=loading
                />
                <button
                    type="submit"
                    class="btn btn-primary btn-lg absolute right-0 top-0"
                    disabled=loading
                >
                    {move || if loading.get() {
                        view! { <span class="loading loading-spinner"></span> }
                    } else {
                        view! { <span>"Search"</span> }
                    }}
                </button>
            </div>
            
            // Advanced options dropdown
            <details class="mt-4">
                <summary class="cursor-pointer text-sm">Advanced Options</summary>
                <div class="grid grid-cols-2 gap-4 mt-4">
                    <label class="form-control">
                        <div class="label">
                            <span class="label-text">Min Confidence</span>
                        </div>
                        <input type="range" min="0" max="1" step="0.1" class="range" />
                    </label>
                    
                    <label class="form-control">
                        <div class="label">
                            <span class="label-text">LLM Model</span>
                        </div>
                        <select class="select select-bordered">
                            <option>deepseek-r1:7b</option>
                            <option>llama3:8b</option>
                            <option>mistral:7b</option>
                        </select>
                    </label>
                </div>
            </details>
        </form>
    }
}
```

#### 3.3 Results Display Component
```rust
// src/components/results.rs
#[component]
pub fn ResultsList(results: ReadSignal<Vec<EvaluatedResult>>) -> impl IntoView {
    view! {
        <div class="space-y-4">
            <For
                each=move || results.get()
                key=|result| result.torrent.magnet_link.clone()
                children=move |result| {
                    view! { <ResultCard result=result/> }
                }
            />
        </div>
    }
}

#[component]
pub fn ResultCard(result: EvaluatedResult) -> impl IntoView {
    let relevance_percent = (result.relevance_score * 100.0) as u8;
    let confidence_class = if result.confidence > 0.8 {
        "text-success"
    } else if result.confidence > 0.5 {
        "text-warning"
    } else {
        "text-error"
    };
    
    view! {
        <div class="card bg-base-200 shadow-xl">
            <div class="card-body">
                <div class="flex justify-between items-start">
                    <h3 class="card-title">{&result.torrent.title}</h3>
                    <div class="badge badge-lg badge-primary">
                        {relevance_percent}"% match"
                    </div>
                </div>
                
                <div class="grid grid-cols-2 md:grid-cols-4 gap-2 mt-4">
                    <div class="stat">
                        <div class="stat-title">Size</div>
                        <div class="stat-value text-sm">{&result.torrent.size}</div>
                    </div>
                    <div class="stat">
                        <div class="stat-title">Seeders</div>
                        <div class="stat-value text-sm">{result.torrent.seeders}</div>
                    </div>
                    <div class="stat">
                        <div class="stat-title">Quality</div>
                        <div class="stat-value text-sm">
                            {format!("{:.0}%", result.quality_score * 100.0)}
                        </div>
                    </div>
                    <div class="stat">
                        <div class="stat-title">Confidence</div>
                        <div class=format!("stat-value text-sm {}", confidence_class)>
                            {format!("{:.0}%", result.confidence * 100.0)}
                        </div>
                    </div>
                </div>
                
                // Match reasons
                <div class="mt-4">
                    <p class="text-sm font-semibold">Why this matches:</p>
                    <ul class="list-disc list-inside text-sm">
                        <For
                            each=move || result.match_reasons.clone()
                            key=|reason| reason.clone()
                            children=move |reason| {
                                view! { <li>{reason}</li> }
                            }
                        />
                    </ul>
                </div>
                
                // Warnings if any
                {if !result.warnings.is_empty() {
                    view! {
                        <div class="alert alert-warning mt-4">
                            <svg xmlns="http://www.w3.org/2000/svg" class="stroke-current shrink-0 h-6 w-6" fill="none" viewBox="0 0 24 24">
                                <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M12 9v2m0 4h.01m-6.938 4h13.856c1.54 0 2.502-1.667 1.732-3L13.732 4c-.77-1.333-2.694-1.333-3.464 0L3.34 16c-.77 1.333.192 3 1.732 3z" />
                            </svg>
                            <div>
                                <For
                                    each=move || result.warnings.clone()
                                    key=|warning| warning.clone()
                                    children=move |warning| {
                                        view! { <p>{warning}</p> }
                                    }
                                />
                            </div>
                        </div>
                    }.into_view()
                } else {
                    view! {}.into_view()
                }}
                
                <div class="card-actions justify-end mt-4">
                    <button class="btn btn-primary">
                        Download
                    </button>
                    <button class="btn btn-ghost">
                        Copy Magnet
                    </button>
                </div>
            </div>
        </div>
    }
}
```

### Phase 4: State Management (Week 3)

#### 4.1 Global App State
```rust
// src/state/mod.rs
use leptos::*;

#[derive(Clone, Debug, Default)]
pub struct AppState {
    pub current_search: Option<SearchState>,
    pub downloads: Vec<DownloadState>,
    pub settings: Settings,
}

#[derive(Clone, Debug)]
pub struct SearchState {
    pub id: String,
    pub query: String,
    pub intent: SearchIntent,
    pub results: Vec<EvaluatedResult>,
    pub status: SearchStatus,
}

#[derive(Clone, Debug)]
pub enum SearchStatus {
    Parsing,
    Searching,
    Evaluating,
    Complete,
    Failed(String),
}

pub fn provide_app_state() {
    provide_context(create_rw_signal(AppState::default()));
}
```

#### 4.2 WebSocket Integration for Live Updates
```rust
// src/state/websocket.rs
use leptos::*;
use leptos_use::*;

#[component]
pub fn WebSocketProvider(children: Children) -> impl IntoView {
    let state = use_context::<RwSignal<AppState>>().unwrap();
    
    let UseWebsocketReturn {
        ready_state,
        message,
        send,
        ..
    } = use_websocket("ws://localhost:3000/ws");
    
    // Handle incoming messages
    create_effect(move |_| {
        if let Some(msg) = message.get() {
            let update: StateUpdate = serde_json::from_str(&msg).unwrap();
            match update {
                StateUpdate::SearchProgress { id, status } => {
                    state.update(|s| {
                        if let Some(search) = &mut s.current_search {
                            if search.id == id {
                                search.status = status;
                            }
                        }
                    });
                }
                StateUpdate::SearchResults { id, results } => {
                    state.update(|s| {
                        if let Some(search) = &mut s.current_search {
                            if search.id == id {
                                search.results = results;
                                search.status = SearchStatus::Complete;
                            }
                        }
                    });
                }
                StateUpdate::DownloadProgress { id, progress } => {
                    state.update(|s| {
                        if let Some(download) = s.downloads.iter_mut().find(|d| d.id == id) {
                            download.progress = progress;
                        }
                    });
                }
            }
        }
    });
    
    children()
}
```

### Phase 5: Download Management UI (Week 4)

#### 5.1 Downloads Page
```rust
// src/pages/downloads.rs
#[component]
pub fn DownloadsPage() -> impl IntoView {
    let state = use_context::<RwSignal<AppState>>().unwrap();
    let downloads = create_memo(move |_| state.get().downloads);
    
    view! {
        <div class="container mx-auto p-4">
            <h1 class="text-3xl font-bold mb-6">Downloads</h1>
            
            <div class="grid gap-4">
                <For
                    each=move || downloads.get()
                    key=|download| download.id.clone()
                    children=move |download| {
                        view! { <DownloadCard download=download/> }
                    }
                />
            </div>
            
            {move || if downloads.get().is_empty() {
                view! {
                    <div class="text-center py-12">
                        <p class="text-lg text-base-content/60">
                            "No active downloads"
                        </p>
                    </div>
                }.into_view()
            } else {
                view! {}.into_view()
            }}
        </div>
    }
}

#[component]
pub fn DownloadCard(download: DownloadState) -> impl IntoView {
    let progress_percent = (download.progress * 100.0) as u8;
    
    view! {
        <div class="card bg-base-200">
            <div class="card-body">
                <h3 class="card-title">{&download.title}</h3>
                
                <div class="flex justify-between text-sm">
                    <span>{format_bytes(download.downloaded)} " / " {format_bytes(download.total)}</span>
                    <span>{format_speed(download.speed)}</span>
                    <span>"ETA: " {format_duration(download.eta)}</span>
                </div>
                
                <progress 
                    class="progress progress-primary w-full"
                    value=progress_percent.to_string()
                    max="100"
                ></progress>
                
                <div class="card-actions justify-end">
                    <button class="btn btn-sm btn-ghost">Pause</button>
                    <button class="btn btn-sm btn-error">Cancel</button>
                </div>
            </div>
        </div>
    }
}
```

### Phase 6: Settings & Configuration (Week 4)

#### 6.1 Settings Page
```rust
// src/pages/settings.rs
#[component]
pub fn SettingsPage() -> impl IntoView {
    let state = use_context::<RwSignal<AppState>>().unwrap();
    let settings = create_memo(move |_| state.get().settings);
    
    view! {
        <div class="container mx-auto p-4 max-w-2xl">
            <h1 class="text-3xl font-bold mb-6">Settings</h1>
            
            <div class="space-y-6">
                // Download Settings
                <div class="card bg-base-200">
                    <div class="card-body">
                        <h2 class="card-title">Download Settings</h2>
                        
                        <div class="form-control">
                            <label class="label">
                                <span class="label-text">Download Path</span>
                            </label>
                            <input 
                                type="text" 
                                class="input input-bordered"
                                value=move || settings.get().download_path
                            />
                        </div>
                        
                        <div class="form-control">
                            <label class="label">
                                <span class="label-text">Max Concurrent Downloads</span>
                            </label>
                            <input 
                                type="number" 
                                class="input input-bordered"
                                value=move || settings.get().max_concurrent
                            />
                        </div>
                    </div>
                </div>
                
                // LLM Settings
                <div class="card bg-base-200">
                    <div class="card-body">
                        <h2 class="card-title">LLM Settings</h2>
                        
                        <div class="form-control">
                            <label class="label">
                                <span class="label-text">Default Model</span>
                            </label>
                            <select class="select select-bordered">
                                <option>deepseek-r1:7b</option>
                                <option>llama3:8b</option>
                                <option>mistral:7b</option>
                            </select>
                        </div>
                        
                        <div class="form-control">
                            <label class="label">
                                <span class="label-text">Temperature</span>
                            </label>
                            <input 
                                type="range" 
                                min="0" 
                                max="1" 
                                step="0.1" 
                                class="range"
                            />
                        </div>
                    </div>
                </div>
            </div>
        </div>
    }
}
```

## UI/UX Design Guidelines

### Visual Design
1. **Dark Mode First**: Use DaisyUI's dark themes for a modern torrent client feel
2. **Information Hierarchy**: Most important info (title, match %) prominent
3. **Status Colors**: 
   - Green: High confidence matches
   - Yellow: Medium confidence
   - Red: Low confidence or warnings
4. **Progress Visualization**: Clear download progress with speed/ETA

### Interaction Patterns
1. **Instant Feedback**: Show loading states immediately
2. **Progressive Disclosure**: Hide advanced options by default
3. **Keyboard Shortcuts**: 
   - `/` to focus search
   - `Enter` to download best match
   - `Esc` to cancel operations

### Responsive Design
```css
/* Mobile First Approach */
.result-grid {
    @apply grid grid-cols-1 gap-4;
    @apply md:grid-cols-2;
    @apply lg:grid-cols-3;
}
```

## Security Considerations

### 1. Authentication Token
```rust
// Generate token on server start
let token = Uuid::new_v4().to_string();
println!("🔐 Access token: {}", token);

// Validate on each request
fn validate_token(req: &HttpRequest) -> Result<(), Error> {
    let token = req.headers()
        .get("Authorization")
        .and_then(|h| h.to_str().ok())
        .and_then(|h| h.strip_prefix("Bearer "));
    
    if token != Some(&CONFIG.access_token) {
        return Err(ErrorUnauthorized("Invalid token"));
    }
    
    Ok(())
}
```

### 2. CORS Configuration
```rust
// Only allow localhost by default
let cors = Cors::default()
    .allowed_origin("http://localhost:3000")
    .allowed_origin("http://127.0.0.1:3000")
    .allowed_methods(vec!["GET", "POST"])
    .max_age(3600);
```

## Testing Strategy

### 1. Component Tests
```rust
#[cfg(test)]
mod tests {
    use leptos::*;
    use leptos_test::*;
    
    #[test]
    fn test_search_component() {
        let runtime = create_runtime();
        
        mount_to_body(|| view! { <SearchBox/> });
        
        // Test search input
        let input = document().query_selector("input[type='text']").unwrap();
        simulate_input(&input, "breaking bad season 2");
        
        // Test form submission
        let form = document().query_selector("form").unwrap();
        simulate_submit(&form);
        
        // Verify loading state
        assert!(document().query_selector(".loading").is_some());
    }
}
```

### 2. E2E Tests with Playwright
```javascript
// tests/e2e/search.spec.ts
test('search flow', async ({ page }) => {
  await page.goto('http://localhost:3000');
  
  // Perform search
  await page.fill('input[placeholder*="Find me"]', 'the matrix trilogy');
  await page.click('button:has-text("Search")');
  
  // Wait for results
  await page.waitForSelector('.card', { timeout: 30000 });
  
  // Verify results displayed
  const results = await page.$$('.card');
  expect(results.length).toBeGreaterThan(0);
});
```

## Development Workflow

### 1. Setup Commands
```bash
# Install dependencies
cargo install cargo-leptos
npm install -D tailwindcss daisyui

# Development
cargo leptos watch

# Build for production
cargo leptos build --release
```

### 2. Project Structure
```
torrentai/
├── src/
│   ├── app.rs              # Root Leptos app
│   ├── main.rs             # CLI entry point
│   ├── web/
│   │   ├── mod.rs
│   │   ├── server.rs       # Actix server setup
│   │   ├── api/            # REST endpoints
│   │   └── ws/             # WebSocket handlers
│   ├── components/         # Leptos components
│   ├── pages/              # Page components
│   └── state/              # State management
├── style/
│   └── tailwind.css        # Tailwind styles
├── Cargo.toml
└── leptos.toml             # Leptos config
```

### 3. Leptos Configuration
```toml
# leptos.toml
[package]
name = "torrentai"
version = "0.1.0"

[build]
output-name = "torrentai"
site-root = "target/site"
site-pkg-dir = "pkg"
style-file = "style/tailwind.css"
assets-dir = "assets"

[watch]
watch-additional-files = ["style"]

[[proxy]]
backend = "http://localhost:8080"
frontend = "http://localhost:3000"
```

## Deployment Considerations

### 1. Single Binary Distribution
```rust
// Embed all assets in binary
#[derive(RustEmbed)]
#[folder = "target/site"]
struct Assets;

// Serve embedded assets
fn serve_embedded_assets() -> impl Responder {
    // Serve from embedded files
}
```

### 2. Systemd Service (Linux)
```ini
[Unit]
Description=TorrentAI Web Interface
After=network.target

[Service]
Type=simple
User=torrentai
ExecStart=/usr/local/bin/torrentai serve
Restart=on-failure

[Install]
WantedBy=multi-user.target
```

### 3. Docker Support
```dockerfile
FROM rust:1.75 as builder
WORKDIR /app
COPY . .
RUN cargo leptos build --release

FROM debian:bookworm-slim
COPY --from=builder /app/target/release/torrentai /usr/local/bin/
EXPOSE 3000
CMD ["torrentai", "serve", "--host", "0.0.0.0"]
```

## Future Enhancements

1. **Mobile App**: Use Tauri for native mobile apps
2. **Browser Extension**: Quick-add torrents from any page
3. **Remote Access**: Secure tunnel for accessing from anywhere
4. **Integrations**: Plex/Jellyfin/Kodi library updates
5. **Scheduled Searches**: Cron-like automatic searches
6. **RSS/Calendar**: Monitor for new releases

## Implementation Priority

1. **Week 1**: Basic server + search API
2. **Week 2**: Core UI components + search flow
3. **Week 3**: WebSocket updates + download management
4. **Week 4**: Settings + polish + testing
5. **Week 5**: Documentation + deployment tools

This plan provides a solid foundation for building a modern, reactive web interface for TorrentAI while maintaining the simplicity and security of a local-first approach.