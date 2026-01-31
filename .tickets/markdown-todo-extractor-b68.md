---
id: markdown-todo-extractor-b68
status: closed
deps: []
links: []
created: 2026-01-20T19:35:09.492684048-06:00
type: task
priority: 2
---
# Implement CLI automatic registration for tags operations

Apply CLI automatic registration pattern to 3 tag operations (extract_tags, list_tags, search_by_tags). Follow the same pattern used for tasks: add Parser derives to request structs, implement CliOperation for operation structs, update create_cli_operations() in capabilities/mod.rs.


