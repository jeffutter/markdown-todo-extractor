---
id: markdown-todo-extractor-84a
status: closed
deps: []
links: []
created: 2026-01-20T22:48:50.12793518-06:00
type: task
priority: 4
tags: ["dependencies","simplification-opportunity"]
---
# Consider replacing config crate with simpler toml crate

## Problem

The `config` crate (26KB) provides a comprehensive configuration framework with:
- Multiple file format support (TOML, JSON, YAML, INI)
- Environment variable binding
- Layered configuration sources
- Type coercion

However, this project only uses:
- TOML file loading from `.markdown-todo-extractor.toml`
- Manual environment variable handling (done separately)

The `config` crate may be overkill for this use case.

## Current Usage

From `src/config.rs`:
```rust
use config::{Config as ConfigBuilder, File};

let settings = ConfigBuilder::builder()
    .add_source(File::with_name(&config_path.to_string_lossy()).required(false))
    .build()
    .ok()?;

settings.try_deserialize::<Self>().ok()
```

## Proposed Alternative

Use the `toml` crate directly (~12KB):

```rust
use std::fs;
use toml;

let content = fs::read_to_string(&config_path).ok()?;
let config: Config = toml::from_str(&content).ok()?;
```

Or with error handling:
```rust
pub fn load_from_base_path(base_path: &Path) -> Self {
    let config_path = base_path.join(".markdown-todo-extractor.toml");
    
    match fs::read_to_string(&config_path) {
        Ok(content) => toml::from_str(&content).unwrap_or_default(),
        Err(_) => Self::default(),
    }
}
```

## Trade-offs

**Current (`config` crate):**
- More flexible if we add JSON/YAML support later
- Handles missing files gracefully
- More complex API

**Proposed (`toml` crate):**
- Simpler, more direct
- Smaller dependency
- Explicit error handling
- Limited to TOML format

## Investigation Required

1. Are there plans to support multiple config formats?
2. Are other `config` crate features being used that I missed?

## Estimated Impact

- Smaller binary size
- Simpler code
- One less complex dependency


