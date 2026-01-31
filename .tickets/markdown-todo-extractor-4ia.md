---
id: markdown-todo-extractor-4ia
status: closed
deps: []
links: []
created: 2026-01-20T22:48:18.912714052-06:00
type: task
priority: 4
tags: ["dependencies","simplification-opportunity"]
---
# Optimize tokio features to reduce binary size

## Problem

In `Cargo.toml`, tokio is included with `features = ["full"]`:

```toml
tokio = { version = "1.44.2", features = ["full"] }
```

This includes many features not needed by this CLI/server application:
- `signal` - Unix signal handling
- `process` - Child process spawning
- `fs` - File system operations (we use std::fs)
- `tracing` - Tracing integration
- Various test utilities

## Proposed Solution

Replace `full` with only the features actually needed:

```toml
tokio = { version = "1.44.2", features = ["rt-multi-thread", "macros", "net", "io-util", "sync"] }
```

Features breakdown:
- `rt-multi-thread` - Multi-threaded runtime for async execution
- `macros` - `#[tokio::main]` attribute macro
- `net` - TCP/UDP for HTTP server
- `io-util` - Async I/O utilities
- `sync` - Synchronization primitives (channels, mutexes)

## Investigation Required

1. Check which tokio features are actually used in the code
2. May need `time` feature if any timeouts are used
3. May need `signal` if graceful shutdown is implemented

## Estimated Impact

- Reduced binary size (~1-3 MB smaller)
- Faster compile times
- Clearer declaration of actual dependencies


