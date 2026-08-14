---
name: release
description: Use when releasing a new version of notectl — runs cargo-release to bump the version, commit, tag, and push.
---

# release

Release a new version of notectl. Accepts one argument: `major`, `minor`, or `patch`.

Version bumping, tagging, and pushing are handled by [cargo-release](https://github.com/crate-ci/cargo-release), configured via `release.toml` at the repo root:

- `shared-version = true` — the workspace shares one version (`[workspace.package] version`, inherited by every member via `version.workspace = true`); cargo-release bumps it once for the whole workspace.
- `publish = false` — this project ships prebuilt binaries via GitHub Releases / Nix, not crates.io, so `cargo publish` is skipped.
- `pre-release-hook = ["cargo", "build", "--workspace"]` — build must succeed before anything is committed/tagged.
- `tag-name = "v{{version}}"` and `pre-release-commit-message = "chore: bump version to {{version}}"` — match this repo's existing tag/commit conventions.

## Steps

1. **Ensure a clean working tree.** cargo-release refuses to run with uncommitted changes — commit or stash first.

2. **Dry run** (default; no `--execute` flag) so the version bump, commit, tag, and push can be reviewed before anything happens:
   ```bash
   cargo release <major|minor|patch>
   ```

3. **Execute for real** once the dry run looks right:
   ```bash
   cargo release <major|minor|patch> --execute --no-confirm
   ```
   This bumps the version across the workspace, runs `cargo build --workspace`, commits (`chore: bump version to X.Y.Z`), creates an annotated tag `vX.Y.Z`, and pushes both the commit and tag to `origin`.

## Preconditions

- Working tree must be clean before starting.
- `cargo build --workspace` (run automatically as the pre-release hook) must succeed before tagging.

## Example

```
/release patch   # 0.12.1 -> 0.12.2, tag v0.12.2
/release minor   # 0.12.1 -> 0.13.0, tag v0.13.0
/release major   # 0.12.1 -> 1.0.0,  tag v1.0.0
```
