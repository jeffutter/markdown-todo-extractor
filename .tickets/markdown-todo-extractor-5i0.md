---
id: markdown-todo-extractor-5i0
status: closed
deps: [markdown-todo-extractor-b68]
links: []
created: 2026-01-20T19:35:13.508211743-06:00
type: task
priority: 2
---
# Implement CLI automatic registration for files operations

Apply CLI automatic registration pattern to 2 file operations (list_files, read_file). Follow the same pattern used for tasks: add Parser derives to request structs, implement CliOperation for operation structs, update create_cli_operations() in capabilities/mod.rs.


