---
id: TASK-41
title: >-
  Fix: prime.rs doesn't document rrf_recency_weight config key, fails TASK-39's
  own AC
status: Done
assignee:
  - '@ralph'
created_date: '2026-08-07 18:14'
updated_date: '2026-08-07 21:10'
labels:
  - review-followup
dependencies:
  - TASK-39
priority: high
type: docs
ordinal: 100
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Found while reviewing TASK-39 (commit e0615cc, src/prime.rs). TASK-39's own acceptance criterion #5 requires 'grep -rn rrf_recency_weight src/prime.rs README.md AGENTS.md' to return a match in all three files. It does not: prime.rs's Search section only mentions the CLI flag name (--recency-weight), never the config key name (rrf_recency_weight), so the grep matches README.md and AGENTS.md but not src/prime.rs. The commit was pushed to origin/main, so this needs a follow-up ticket rather than a fixup. Correct axis: an explicitly stated, checkable acceptance criterion in the ticket's own plan was not satisfied, yet the ticket was marked Done. Since prime.rs is the LLM-facing skill file returned by 'notectl prime'/'notectl-remote prime', an agent grepping that file for the config key name (as opposed to the CLI flag) currently finds nothing.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 src/prime.rs's Search section mentions the config key name rrf_recency_weight (e.g. a parenthetical noting the TOML/env config key alongside the --recency-weight flag description), not only the CLI flag name
- [ ] #2 grep -rn rrf_recency_weight src/prime.rs README.md AGENTS.md returns a match in all three files
- [ ] #3 nix develop -c cargo build succeeds
<!-- AC:END -->

## Implementation Plan

<!-- SECTION:PLAN:BEGIN -->
SETUP (read first): This is a Rust workspace (notectl-core, notectl-search, notectl-outline, and the notectl/notectl-remote binaries). ALL commands must run inside the Nix dev shell: either run 'direnv allow' once, or prefix every command with 'nix develop -c'. Work from the repository root unless told otherwise. Do not change pinned dependency versions.

1. Open src/prime.rs, find the '--recency-weight <f64>' line (currently around line 192, in the '{bin} search{vp} <query>' block).
2. Append a parenthetical config-key cross-reference so an agent grepping for the TOML/env key name can find it, e.g. change the line to:
   '  --recency-weight <f64>     recency boost weight (0 disables, default 0.5; config: rrf_recency_weight)'
   Match the existing style/alignment of the surrounding option lines in that block.
3. Run: grep -rn rrf_recency_weight src/prime.rs README.md AGENTS.md -- confirm it now matches in all three files.
4. Run: nix develop -c cargo build -- confirm nothing else broke (docs-only change inside a Rust string constant).
5. No tests exist for prime.rs's generated text content (asserted only by manual doc review) -- do not add any; this is a documentation-only fix.
<!-- SECTION:PLAN:END -->
