---
name: rust
description: Rust coding best practices for idiomatic, efficient, and maintainable code. Use when writing Rust code, reviewing code, or learning Rust patterns.
allowed-tools: Read, Edit, Write, Bash, Grep, Glob, Task
---

# Rust Best Practices

Guidelines for writing idiomatic, efficient, and maintainable Rust code.

## Core Principles

1. **Leverage the type system** - Make invalid states unrepresentable
2. **Prefer compile-time checks** - Catch errors before runtime
3. **Be explicit about ownership** - Don't fight the borrow checker
4. **Write code that passes fmt/clippy first** - Not after fixing

## Code health

1. Prefer smaller files, refactor large files into multi-file modules
2. Refactor large modules into several
3. Prefer re-usable functions. Don't create separate functions if re-use is not planned.

## Error Handling

### Library code — use `thiserror`

```rust
use thiserror::Error;

#[derive(Error, Debug)]
pub enum ConfigError {
    #[error("Failed to read config: {0}")]
    Io(#[from] std::io::Error),

    #[error("Failed to parse config: {0}")]
    Parse(#[from] toml::de::Error),

    #[error("Invalid configuration: {message}")]
    Invalid { message: String },
}
```

### Application/binary code — use `anyhow`

Use `anyhow` in binaries and CLI tools where you want ergonomic error propagation without defining custom error types.

```rust
use anyhow::{Context, Result, bail, anyhow};

fn load_config(path: &Path) -> Result<Config> {
    let text = std::fs::read_to_string(path)
        .with_context(|| format!("failed to read config at {}", path.display()))?;

    let config: Config = toml::from_str(&text)
        .context("failed to parse config")?;

    if config.timeout == 0 {
        bail!("timeout must be greater than zero");
    }

    Ok(config)
}

// Return a one-off error without a custom type
fn validate(n: u32) -> Result<()> {
    if n > 100 {
        return Err(anyhow!("value {} exceeds maximum of 100", n));
    }
    Ok(())
}
```

**Rule of thumb**: `thiserror` for libraries (callers need to match on variants), `anyhow` for binaries and CLI tools (only humans read the error).

### Never Use `.unwrap()` in production code

```rust
// BAD
let value = map.get("key").unwrap();

// GOOD
let value = map.get("key").ok_or_else(|| Error::MissingKey("key"))?;

// GOOD (when None is truly impossible)
let value = map.get("key").expect("key always present after init");
```

## Ownership & Borrowing

### Prefer Borrowing Over Cloning

```rust
// BAD - unnecessary clone
fn process(data: String) { ... }
process(my_string.clone());

// GOOD - borrow when possible
fn process(data: &str) { ... }
process(&my_string);
```

### Use `Cow` for Flexible Ownership

```rust
use std::borrow::Cow;

fn process(data: Cow<'_, str>) -> Cow<'_, str> {
    if data.contains("bad") {
        Cow::Owned(data.replace("bad", "good"))
    } else {
        data  // No allocation if unchanged
    }
}
```

### Return Owned Data from Constructors

```rust
// GOOD - clear ownership
impl User {
    pub fn new(name: impl Into<String>) -> Self {
        Self { name: name.into() }
    }
}
```

## CLI Argument Parsing with `clap`

Use the derive macro for clean, self-documenting CLIs.

```rust
use clap::{Parser, Subcommand, Args, ValueEnum};

#[derive(Parser)]
#[command(name = "mytool", version, about = "Does useful things")]
struct Cli {
    /// Increase verbosity (-v, -vv, -vvv)
    #[arg(short, long, action = clap::ArgAction::Count)]
    verbose: u8,

    /// Path to config file
    #[arg(short, long, default_value = "config.toml")]
    config: PathBuf,

    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Run the processor
    Run(RunArgs),
    /// Show status
    Status,
}

#[derive(Args)]
struct RunArgs {
    /// Output format
    #[arg(short, long, value_enum, default_value_t = OutputFormat::Text)]
    format: OutputFormat,

    /// Input file(s)
    #[arg(required = true)]
    files: Vec<PathBuf>,
}

#[derive(ValueEnum, Clone)]
enum OutputFormat {
    Text,
    Json,
    Csv,
}

fn main() -> anyhow::Result<()> {
    let cli = Cli::parse();
    match cli.command {
        Commands::Run(args) => run(args, cli.verbose)?,
        Commands::Status => status()?,
    }
    Ok(())
}
```

**Tips:**
- Use `#[arg(env = "MY_VAR")]` to allow env var fallback
- `#[command(flatten)]` lets you share arg groups across subcommands
- `clap::builder::styling` for colored help output
- Combine with `anyhow` for clean `main() -> anyhow::Result<()>`

## Serialization with `serde`

`serde` is the standard for serialization/deserialization. Pair with format crates (`serde_json`, `toml`, `serde_yaml`, etc.).

```rust
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
struct Config {
    host: String,
    port: u16,
    #[serde(default = "default_timeout")]
    timeout_secs: u64,
    #[serde(skip_serializing_if = "Option::is_none")]
    api_key: Option<String>,
    #[serde(rename = "log_level")]
    log: String,
}

fn default_timeout() -> u64 { 30 }

// Deserialize from TOML
let config: Config = toml::from_str(raw_toml)?;

// Serialize to JSON
let json = serde_json::to_string_pretty(&config)?;

// Parse from JSON string
let config: Config = serde_json::from_str(&json)?;
```

**Useful serde attributes:**

| Attribute | Purpose |
|-----------|---------|
| `#[serde(default)]` | Use `Default::default()` if field missing |
| `#[serde(default = "fn")]` | Call a function for the default value |
| `#[serde(rename = "name")]` | Use a different key name |
| `#[serde(skip_serializing_if = "Option::is_none")]` | Omit `None` fields |
| `#[serde(flatten)]` | Inline a nested struct's fields |
| `#[serde(tag = "type")]` | Enum as internally tagged `{"type": "Variant"}` |
| `#[serde(alias = "old_name")]` | Accept multiple key names |

## Observability with `tracing`

Prefer `tracing` over `log` — it supports structured fields, async contexts, and spans.

```rust
use tracing::{debug, error, info, instrument, warn};

// Initialize in main (pair with tracing-subscriber)
fn main() {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::from_default_env()
                .add_directive("myapp=debug".parse().unwrap()),
        )
        .init();

    run();
}

// Instrument a function — creates a span automatically
#[instrument(skip(password), fields(user_id = %user.id))]
async fn authenticate(user: &User, password: &str) -> Result<Token> {
    debug!("checking credentials");
    let token = db_lookup(user).await
        .context("db lookup failed")?;
    info!(token_expiry = %token.expiry, "authenticated");
    Ok(token)
}

// Structured event fields
info!(
    file = %path.display(),
    bytes = metadata.len(),
    "file processed"
);

warn!(attempt = retries, max = MAX_RETRIES, "retrying request");
error!(err = ?e, "unexpected failure");
```

**Cargo.toml:**
```toml
[dependencies]
tracing = "0.1"
tracing-subscriber = { version = "0.3", features = ["env-filter"] }
```

## API Design

### Builder Pattern for Complex Configuration

```rust
#[derive(Default)]
pub struct ServerBuilder {
    host: Option<String>,
    port: Option<u16>,
    timeout: Option<Duration>,
}

impl ServerBuilder {
    pub fn host(mut self, host: impl Into<String>) -> Self {
        self.host = Some(host.into());
        self
    }

    pub fn port(mut self, port: u16) -> Self {
        self.port = Some(port);
        self
    }

    pub fn build(self) -> Result<Server, ConfigError> {
        Ok(Server {
            host: self.host.unwrap_or_else(|| "localhost".into()),
            port: self.port.ok_or(ConfigError::MissingPort)?,
            timeout: self.timeout.unwrap_or(Duration::from_secs(30)),
        })
    }
}
```

### Newtype Pattern for Type Safety

```rust
// BAD - easy to mix up
fn transfer(from: i64, to: i64, amount: i64) { ... }

// GOOD - compile-time safety
pub struct AccountId(i64);
pub struct Amount(i64);

fn transfer(from: AccountId, to: AccountId, amount: Amount) { ... }
```

### Use `#[must_use]` for Important Returns

```rust
#[must_use]
pub fn validate(&self) -> Result<(), ValidationError> {
    // ...
}
```

## Collections & Iterators

### Prefer Iterators Over Loops

```rust
// BAD
let mut results = Vec::new();
for item in items {
    if item.is_valid() {
        results.push(item.transform());
    }
}

// GOOD
let results: Vec<_> = items
    .into_iter()
    .filter(|item| item.is_valid())
    .map(|item| item.transform())
    .collect();
```

### Use `collect()` Type Inference

```rust
// Collect into Vec
let vec: Vec<_> = iter.collect();

// Collect into HashMap
let map: HashMap<_, _> = iter.collect();

// Collect Results — short-circuits on first Err
let results: Result<Vec<_>, _> = iter.collect();
```

## Async Patterns

### Use `tokio` for Async Runtime

```rust
#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let result = fetch_data().await?;
    Ok(())
}
```

### Avoid Blocking in Async Code

```rust
// BAD - blocks the runtime
async fn bad() {
    std::thread::sleep(Duration::from_secs(1));
}

// GOOD - async sleep
async fn good() {
    tokio::time::sleep(Duration::from_secs(1)).await;
}

// GOOD - spawn blocking for CPU-intensive work
async fn compute() -> i32 {
    tokio::task::spawn_blocking(|| expensive_computation()).await.unwrap()
}
```

### Channel Patterns for Task Communication

Use `tokio::sync` channels instead of `std::sync` inside async code.

```rust
use tokio::sync::mpsc;

// Producer/consumer pipeline
async fn run_pipeline() -> anyhow::Result<()> {
    let (tx, mut rx) = mpsc::channel::<String>(32);

    // Spawn producer
    tokio::spawn(async move {
        for item in source_data() {
            if tx.send(item).await.is_err() {
                break; // receiver dropped
            }
        }
    });

    // Consume
    while let Some(item) = rx.recv().await {
        process(item).await?;
    }
    Ok(())
}
```

**Channel quick-reference:**

| Channel | Use case |
|---------|---------|
| `mpsc::channel` | Multiple producers, one consumer (most common) |
| `oneshot::channel` | Single value, one shot (request/response) |
| `broadcast::channel` | One producer, many consumers (fan-out) |
| `watch::channel` | Share latest state, multiple readers |
| `std::sync::mpsc` | Non-async code only |

### Shared State in Async Code

```rust
use std::sync::Arc;
use tokio::sync::{Mutex, RwLock};

// RwLock when reads >> writes
let state: Arc<RwLock<State>> = Arc::new(RwLock::new(State::default()));

// Read (multiple concurrent readers allowed)
let val = state.read().await.some_field;

// Write (exclusive)
state.write().await.some_field = new_val;

// Mutex when you always need exclusive access or writes are common
let counter: Arc<Mutex<u64>> = Arc::new(Mutex::new(0));
*counter.lock().await += 1;
```

## Feature Flags

Use Cargo features to enable optional functionality without bloating the default build.

```toml
# Cargo.toml
[features]
default = []
json = ["dep:serde_json"]
cli = ["dep:clap"]
full = ["json", "cli"]

[dependencies]
serde = { version = "1", features = ["derive"] }
serde_json = { version = "1", optional = true }
clap = { version = "4", features = ["derive"], optional = true }
```

```rust
// Guard optional code with cfg
#[cfg(feature = "json")]
pub fn to_json(&self) -> String {
    serde_json::to_string(self).unwrap()
}

// Conditional imports
#[cfg(feature = "cli")]
use clap::Parser;

// In tests, enable features explicitly
// cargo test --features json
```

**Best practices:**
- Keep `default = []` minimal — let users opt in
- Gate heavy deps (tokio, clap, serde_json) as optional where it makes sense
- Use `dep:crate_name` syntax (Rust 1.60+) to avoid implicit feature names
- Document which features unlock which APIs

## Workspace Organization (Multi-Crate Projects)

For larger projects, split into a Cargo workspace.

```
my-project/
├── Cargo.toml          # workspace root
├── crates/
│   ├── core/           # shared types and logic
│   ├── cli/            # binary, depends on core
│   └── server/         # binary, depends on core
```

```toml
# Cargo.toml (workspace root)
[workspace]
members = ["crates/*"]
resolver = "2"

[workspace.dependencies]
anyhow = "1"
serde = { version = "1", features = ["derive"] }
tokio = { version = "1", features = ["full"] }
tracing = "0.1"
```

```toml
# crates/cli/Cargo.toml
[dependencies]
core = { path = "../core" }
anyhow.workspace = true     # inherit version from workspace
clap = { version = "4", features = ["derive"] }
```

**Benefits:**
- Single `cargo build` / `cargo test` at the root builds everything
- Shared dependency versions via `[workspace.dependencies]`
- Independent versioning per crate when needed
- `cargo build -p cli` to build just one crate

## Testing

### Unit Tests in Same File

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_basic() {
        assert_eq!(add(1, 2), 3);
    }

    #[test]
    fn test_edge_case() {
        assert!(validate("").is_err());
    }
}
```

### Integration Tests in `tests/`

```rust
// tests/integration_test.rs
use my_crate::public_api;

#[test]
fn test_full_workflow() {
    let result = public_api::process("input");
    assert!(result.is_ok());
}
```

### Use `assert!` Macros Effectively

```rust
assert!(condition);
assert_eq!(left, right);
assert_ne!(left, right);
assert!(result.is_ok());
assert!(result.is_err());
assert_matches!(value, Pattern::Variant { .. });
```

## Performance

### Avoid Premature Allocation

```rust
// BAD - allocates even if not needed
fn maybe_string() -> String {
    String::from("default")
}

// GOOD - return static str when possible
fn maybe_string() -> &'static str {
    "default"
}
```

### Use `Vec::with_capacity` for Known Sizes

```rust
// BAD - multiple reallocations
let mut vec = Vec::new();
for i in 0..1000 {
    vec.push(i);
}

// GOOD - single allocation
let mut vec = Vec::with_capacity(1000);
for i in 0..1000 {
    vec.push(i);
}
```

### Import instead of using absolute paths

```rust
// BAD
let v = tokio::net::TcpStream::connect("localhost:8080");

// GOOD
use tokio::net::TcpStream;
let v = TcpStream::connect("localhost:8080");
```

### Profile Before Optimizing

```bash
cargo build --release
cargo flamegraph  # requires cargo-flamegraph
```

## Module Organization

### Keep Modules Focused

```rust
// src/lib.rs
pub mod config;
pub mod client;
pub mod error;

// Re-export public API
pub use config::Config;
pub use client::Client;
pub use error::Error;
```

### Use `pub(crate)` for Internal APIs

```rust
// Public to crate, not external users
pub(crate) fn internal_helper() { ... }
```

## Documentation

### Document Public APIs

```rust
/// Creates a new client with the given configuration.
///
/// # Arguments
///
/// * `config` - The client configuration
///
/// # Errors
///
/// Returns an error if the configuration is invalid.
///
/// # Examples
///
/// ```
/// let client = Client::new(Config::default())?;
/// ```
pub fn new(config: Config) -> Result<Self> {
    // ...
}
```

## Anti-Patterns to Avoid

| Anti-Pattern | Better Approach |
|--------------|-----------------|
| `.unwrap()` everywhere | Use `?` operator |
| `clone()` to satisfy borrow checker | Restructure ownership |
| `String` parameters | Use `&str` or `impl Into<String>` |
| Boolean parameters | Use enums |
| Long function bodies | Extract to smaller functions |
| Deep nesting | Use early returns |
| Magic numbers | Use named constants |
| `anyhow` in library code | Use `thiserror` |
| `std::sync::Mutex` in async | Use `tokio::sync::Mutex` |
| Blocking I/O in async fn | `tokio::task::spawn_blocking` |
| Hardcoded feature-gated code | `#[cfg(feature = "...")]` |

## Quick Reference

```bash
# Quality gates
cargo fmt -- --check && cargo clippy -- -D warnings && cargo test

# Common cargo commands
cargo check                    # Fast syntax/type check
cargo build                    # Debug build
cargo build --release          # Release build
cargo nextest run              # Run tests
cargo doc --open               # Generate and view docs
cargo clippy --fix             # Auto-fix lint issues
cargo test --features json     # Test with a specific feature
cargo build -p mycrate         # Build one crate in a workspace

# Useful one-liners
cargo tree                     # Show dependency tree
cargo outdated                 # Check for outdated deps (cargo-outdated)
cargo audit                    # Security audit (cargo-audit)
```

## Common Dependency Stack

```toml
[dependencies]
# Error handling
thiserror = "1"         # library errors
anyhow = "1"            # binary/CLI errors

# CLI
clap = { version = "4", features = ["derive"] }

# Serialization
serde = { version = "1", features = ["derive"] }
serde_json = "1"
toml = "0.8"

# Async
tokio = { version = "1", features = ["full"] }

# Observability
tracing = "0.1"
tracing-subscriber = { version = "0.3", features = ["env-filter"] }
```
