---
id: TASK-39
title: >-
  Fix: document rrf_recency_weight / --recency-weight (TASK-38 left prime.rs,
  README, AGENTS.md stale)
status: Needs Plan
assignee: []
created_date: '2026-08-05 14:38'
updated_date: '2026-08-07 18:05'
labels:
  - review-followup
dependencies:
  - TASK-38
priority: high
type: docs
ordinal: 100
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Found while reviewing TASK-38 (commit 49387e1, notectl-search/src/{capability.rs,search.rs}, notectl-core/src/config.rs). TASK-38 added a new CLI flag (--recency-weight), a new SearchConfig default (rrf_recency_weight = 0.5), a new env var (NOTECTL_SEARCH_RRF_RECENCY_WEIGHT), and a new output field (RankedChunk.date), but did not update the three hand-maintained docs that describe the search command/config surface: src/prime.rs, README.md, and AGENTS.md. This is a direct violation of the explicit checklist in this repo's CLAUDE.md ('Reminder — src/prime.rs ... If your change touches a command, option, default value, or config default ... check whether src/prime.rs text needs updating too'). Since src/prime.rs is the LLM-facing skill file returned by 'notectl prime' / 'notectl-remote prime', an LLM agent driving notectl today has no way to discover --recency-weight exists. Correct axis: violates an explicit, documented project invariant.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 src/prime.rs's Search section (around the 'search{vp} <query>' block, currently lines 184-200) lists a --recency-weight line alongside --limit/--mode/--no-reindex, with the default (0.5) and that 0 disables it
- [ ] #2 src/prime.rs's 'Output fields per result' line (currently line 194) includes 'date (optional)' alongside id, source_file, score, heading, preview
- [ ] #3 README.md's [search] TOML example (currently lines 380-394) includes an rrf_recency_weight = 0.5 line with a comment, and its env var table (currently lines 398-414) includes a NOTECTL_SEARCH_RRF_RECENCY_WEIGHT row
- [ ] #4 AGENTS.md's [search] TOML example (currently lines 149-162) includes the same rrf_recency_weight = 0.5 line for consistency with every other SearchConfig field already listed there
- [ ] #5 grep -rn rrf_recency_weight src/prime.rs README.md AGENTS.md returns a match in all three files
<!-- AC:END -->

## Implementation Plan

<!-- SECTION:PLAN:BEGIN -->
SETUP (read first): This is a Rust workspace (notectl-core, notectl-search, notectl-outline, and the notectl/notectl-remote binaries). ALL commands must run inside the Nix dev shell: either run 'direnv allow' once, or prefix every command with 'nix develop -c'. Work from the repository root unless told otherwise. Do not change pinned dependency versions.

1. Open src/prime.rs. Find the '### Search' section (search for 'search{vp} <query>').
2. Under the '{bin} search{vp} <query>' block, after the existing '--no-reindex true|false' line, add:
   '  --recency-weight <f64>     recency boost weight (0 disables, default 0.5)'
3. Update the 'Output fields per result:' line to read:
   'Output fields per result: id, source_file, score, heading (optional), date (optional), preview'
4. Open README.md, find the '### Search Configuration' section's fenced toml code block (search for 'merge_threshold = 30'). Add a new line after 'merge_threshold = 30 ...':
   'rrf_recency_weight = 0.5                     # Recency boost weight post-RRF fusion (0 disables)'
5. In the same README.md section, find the environment variable table (search for 'NOTECTL_SEARCH_MERGE_THRESHOLD'). Add a new row:
   '| NOTECTL_SEARCH_RRF_RECENCY_WEIGHT | search.rrf_recency_weight |' (backtick-quote both cells to match the existing table style)
6. Open AGENTS.md, find the '[search]' toml block (search for 'merge_threshold = 30'). Add the same rrf_recency_weight line as step 4, matching the existing comment style in that file (no aligned '#' column padding needed, match neighboring lines).
7. Read notectl-search/src/capability.rs's SearchRequest struct and SearchOptions default in notectl-search/src/search.rs to confirm the flag name (--recency-weight), the config key (rrf_recency_weight), and the env var (NOTECTL_SEARCH_RRF_RECENCY_WEIGHT) exactly match what you wrote in docs -- these must not drift from the actual CLI/config surface.
8. Run: grep -rn rrf_recency_weight src/prime.rs README.md AGENTS.md -- confirm all three files now match.
9. Run: nix develop -c cargo build (docs-only change, but confirms nothing else broke).
10. No tests exist for prime.rs's generated text content (it's asserted only by manual doc review) -- do not add any; this is a documentation-only fix.
<!-- SECTION:PLAN:END -->
