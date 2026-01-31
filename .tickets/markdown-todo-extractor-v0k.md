---
id: markdown-todo-extractor-v0k
status: closed
deps: []
links: []
created: 2026-01-19T23:21:12.250198134-06:00
type: feature
priority: 2
---
# Refactor into capability-based modules with unified trait interface

Refactor the project architecture to support multiple interfaces (MCP, HTTP, CLI) through a common trait system.

## Architecture Overview

Split capabilities into focused modules:
- Tag listing/retrieval
- Task search  
- File list/fetch

## Module Design

Each module should:
1. Handle logic for its specific capability area
2. Expose a type implementing a common trait
3. Support all three interface types through the trait

## Trait Requirements

Create a common trait that provides:
- MCP server integration
- HTTP REST-style endpoint support
- CLI command interface

When implemented, the trait should enable near-automatic exposure via all three interfaces.

## Benefits

- Consistent interface across all capability areas
- Single implementation supports multiple access patterns
- Easier to add new capabilities in the future
- Cleaner separation of concerns


