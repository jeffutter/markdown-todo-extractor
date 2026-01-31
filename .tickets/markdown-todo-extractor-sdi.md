---
id: markdown-todo-extractor-sdi
status: closed
deps: []
links: []
created: 2026-01-20T12:28:37.799886872-06:00
type: feature
priority: 2
---
# Change file list output to visual rather than json

Change the format of the file list output to an indented list.

Directory structure format:
- One entry per line
- Directories end with /
- Children are indented 2 spaces relative to their parent
- Files have no trailing slash

## Example:

```
project/
  src/
    main.py
    utils.py
  README.md
```

## Implementation Plan

### 1. Create Visual Tree Formatter Function
**Location**: `src/capabilities/files.rs`

Add a new function `format_tree_visual()` that converts a `FileTreeNode` to the indented visual format:

```rust
fn format_tree_visual(node: &FileTreeNode, indent_level: usize) -> String {
    let mut output = String::new();
    let indent = "  ".repeat(indent_level);
    
    // Add current node
    if node.is_directory {
        output.push_str(&format!("{}{}/\n", indent, node.name));
    } else {
        output.push_str(&format!("{}{}\n", indent, node.name));
    }
    
    // Recursively add children
    for child in &node.children {
        output.push_str(&format_tree_visual(child, indent_level + 1));
    }
    
    output
}
```

**Key implementation details**:
- Use 2 spaces per indent level (not tabs)
- Directories get a trailing `/`
- Files have no trailing slash
- Recursively process children with incremented indent level

### 2. Add Visual Output Option to Response
**Location**: `src/capabilities/files.rs`

Two approaches to consider:

**Option A: Replace JSON response entirely**
- Change `ListFilesResponse` to contain a `visual_tree: String` field
- Remove or make optional the `root: FileTreeNode` field
- CLI always outputs visual format
- HTTP/MCP also outputs visual format

**Option B: Add format parameter (more flexible)**
- Add `format: Option<String>` to `ListFilesRequest` (values: "json", "visual")
- Keep existing `FileTreeNode` structure
- Add optional `visual_tree: Option<String>` to `ListFilesResponse`
- Let caller choose format

**Recommendation**: Start with Option A for simplicity. Can add Option B later if needed.

### 3. Update CLI Output
**Location**: `src/capabilities/files.rs:341-362` (ListFilesOperation CLI impl)

Change the CLI operation to output the visual tree directly:

```rust
async fn execute_from_args(...) -> Result<String, Box<dyn std::error::Error>> {
    // ... existing request parsing ...
    
    let response = /* ... get response ... */;
    
    // Output visual tree instead of JSON
    Ok(response.visual_tree)
}
```

**Current code**: Line 361 serializes to JSON
**New code**: Return the visual tree string directly

### 4. Update list_files Method
**Location**: `src/capabilities/files.rs:121-177`

After building the file tree with `build_file_tree()`:

```rust
// Build the file tree (existing code)
let (root, total_files, total_directories) = build_file_tree(...)?;

// Generate visual representation
let visual_tree = format_tree_visual(&root, 0);

Ok(ListFilesResponse {
    visual_tree,
    total_files,
    total_directories,
})
```

### 5. Update Response Struct
**Location**: `src/capabilities/files.rs:56-62`

```rust
#[derive(Debug, Serialize, Deserialize, JsonSchema)]
pub struct ListFilesResponse {
    pub visual_tree: String,
    pub total_files: usize,
    pub total_directories: usize,
}
```

### 6. Handle Root Name Edge Case
The root directory name needs special handling:
- If listing vault root, show the vault's directory name
- If listing a subpath, show that subpath's name
- Ensure consistent behavior with how `build_file_tree()` names the root

### 7. Testing Strategy

Manual testing commands:
```bash
# Test basic listing
cargo run -- list-files /path/to/vault

# Test subpath
cargo run -- list-files /path/to/vault --path "subfolder"

# Test max depth
cargo run -- list-files /path/to/vault --max-depth 2
```

Expected output format:
```
vault/
  folder1/
    file1.md
    file2.md
  folder2/
    nested/
      deep.md
  root-file.md
```

### 8. Files to Modify

1. `src/capabilities/files.rs`:
   - Add `format_tree_visual()` function (~20 lines)
   - Update `ListFilesResponse` struct (1 line change)
   - Update `list_files()` method (add 2 lines)
   - Update CLI `execute_from_args()` (change line 361)

### 9. Backward Compatibility Considerations

**Breaking changes**:
- HTTP/MCP clients expecting JSON structure will break
- This is acceptable if no external clients exist yet

**If backward compatibility needed**:
- Use Option B (format parameter) instead
- Default to "visual" for CLI
- Keep "json" as default for HTTP/MCP initially

### 10. Edge Cases to Handle

1. **Empty directories**: Still show with `/` suffix
2. **Single file**: Should not have indent (at root level)
3. **Deep nesting**: Verify indent math is correct
4. **Special characters in names**: Ensure proper display
5. **Excluded paths**: Already handled by `build_file_tree()`

### Estimated Complexity
- Low complexity change
- ~30 lines of new code
- ~5 lines of modifications to existing code
- Main work is the formatter function and testing


