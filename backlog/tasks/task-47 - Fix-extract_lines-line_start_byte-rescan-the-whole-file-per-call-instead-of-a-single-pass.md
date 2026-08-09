---
id: TASK-47
title: >-
  Fix: extract_lines/line_start_byte rescan the whole file per call instead of a
  single pass
status: Done
assignee:
  - '@ralph'
created_date: '2026-08-09 16:38'
updated_date: '2026-08-09 17:32'
labels:
  - review-followup
  - planned
dependencies:
  - TASK-45
  - TASK-46
priority: high
type: chore
ordinal: 105
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Found while reviewing TASK-45 (notectl-outline/src/outline_extractor.rs, commit e1a4551). The CRLF-preservation fix added extract_lines()/line_start_byte() (around lines 321-351), where line_start_byte() does a fresh linear scan of content.bytes() from offset 0 to find a single line's start. extract_lines() calls it twice (once for the start line, once for the end line), and both extract_sections_from_content and get_section call extract_lines() once per heading in their loops. For a file with H matching headings, this is O(H*n) work, versus O(n) before this diff (which built a 'lines: Vec<&str>' once via content.lines().collect() and then did O(1) indexed slicing per section). TASK-45's own implementation plan explicitly instructed mirroring the sibling fix in notectl-files/src/capability.rs::filter_excluded_sections (commit cd1f34d), which computes line boundaries in a single O(n) pass via content.split_inclusive("\n").enumerate() rather than rescanning per lookup — the implementer used a different, per-call-rescanning technique instead. This is an Organized-axis defect (the same 'extract content preserving line terminators' knowledge is now implemented two different ways in sibling capabilities) and a real algorithmic regression versus the pre-TASK-45 code, not just a style deviation. TASK-46 (already filed, To Do) fixes a related but narrower issue (hoisting a redundant total_lines computation) in the same function and does not touch line_start_byte's per-call rescanning — this ticket is additive to it, hence the dependency on TASK-46 as well as TASK-45.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 line_start_byte-style per-lookup full-content rescanning is eliminated: line-start byte offsets needed to extract a section's content are derived from a single O(n) pass over the file content (e.g. a precomputed newline-offset table, or a split_inclusive("\n")-based walk analogous to filter_excluded_sections), not from an independent scan-from-zero per call
- [ ] #2 extract_sections_from_content and get_section both use the updated single-pass approach (no duplicated divergent implementations between them)
- [ ] #3 All existing outline_extractor tests continue to pass unmodified in their assertions, including test_get_section_crlf_preserved and extract_sections_from_content::test_crlf_preserved
- [ ] #4 nix develop -c cargo test -p notectl-outline passes
- [ ] #5 nix develop -c cargo clippy -p notectl-outline --all-features -- -D warnings passes
- [ ] #6 nix develop -c cargo test --workspace passes
<!-- AC:END -->

## Implementation Plan

<!-- SECTION:PLAN:BEGIN -->
SETUP: All commands inside Nix dev shell (direnv allow or 'nix develop -c'). Work from repo root. Only notectl-outline/src/outline_extractor.rs is modified. No new dependencies.

PROBLEM: line_start_byte() walks content.bytes() from offset 0 every call. extract_lines() calls it twice per invocation. extract_sections_from_content and get_section each call extract_lines() once per heading → O(H×n) total.

APPROACH: Precompute line-start byte offsets once per content string, then do O(1) index lookups. Two private helpers replace the rescan-per-call design:

1. **Add fn build_line_offsets(content: &str) -> Vec<usize>** — walks content's bytes once, recording the byte offset after each '\n'. Returns vec where index i holds the start offset of 0-indexed line i. Index 0 is always 0. Construction is O(n). Example:
   

2. **Add fn extract_lines_with_offsets(content: &str, offsets: &[usize], start_1: usize, end_1: usize) -> String** — looks up byte offsets by direct index instead of rescanning. Preserves all edge-case behavior:
   - start_1 >= end_1 or start_1 < 1 → empty String
   - Out-of-range start → empty String (sb >= content.len())
   - Out-of-range end → clamped to content.len()
   - CRLF preserved via byte slicing content[sb..eb]

3. **Update extract_sections_from_content**: hoist  call before the heading loop (same position as the already-hoisted ). Pass offsets to extract_lines_with_offsets for every section.

4. **Update get_section**: same pattern — hoist  before the matching_indices loop. Pass offsets to extract_lines_with_offsets for every section.

5. **Remove line_start_byte and extract_lines** — they are replaced entirely by the new helpers. No callers remain.

6. **Run validation gates:**
   - 
running 33 tests
test capability::remote_command_tests::outline_remote_command_hierarchical_bare_flag_fails ... ok
test capability::remote_command_tests::outline_remote_command_args_to_json_no_vault_path_panic ... ok
test outline_extractor::tests::extract_headings::test_nested_code_blocks ... ok
test capability::remote_command_tests::outline_remote_command_hierarchical_accepts_bool_value ... ok
test capability::remote_command_tests::search_headings_remote_command_with_all_options ... ok
test outline_extractor::tests::extract_sections_from_content::test_no_headings_returns_single_section ... ok
test outline_extractor::tests::build_hierarchy::test_level_skipping ... ok
test capability::remote_command_tests::section_remote_command_include_subsections_bare_flag_fails ... ok
test outline_extractor::tests::extract_sections_from_content::test_crlf_preserved ... ok
test outline_extractor::tests::extract_headings::test_headings_in_code_blocks_ignored ... ok
test capability::remote_command_tests::outline_remote_command_with_all_options ... ok
test capability::remote_command_tests::search_headings_remote_command_args_to_json_no_vault_path_panic ... ok
test capability::remote_command_tests::section_remote_command_args_to_json_no_vault_path_panic ... ok
test capability::remote_command_tests::section_remote_command_include_subsections_accepts_bool_value ... ok
test outline_extractor::tests::get_section::test_get_section_crlf_preserved ... ok
test outline_extractor::tests::get_section::test_get_section_without_subsections ... ok
test outline_extractor::tests::get_section::test_get_section_with_subsections ... ok
test outline_extractor::tests::parse_heading::test_h6_heading ... ok
test outline_extractor::tests::get_section::test_multiple_matching_sections ... ok
test outline_extractor::tests::parse_heading::test_h1_heading ... ok
test outline_extractor::tests::get_section::test_get_section_basic ... ok
test outline_extractor::tests::extract_sections_from_content::test_basic_extraction ... ok
test outline_extractor::tests::parse_heading::test_regular_text ... ok
test outline_extractor::tests::extract_headings::test_simple_document ... ok
test outline_extractor::tests::build_hierarchy::test_simple_hierarchy ... ok
test outline_extractor::tests::parse_heading::test_too_many_hashes ... ok
test outline_extractor::tests::parse_heading::test_heading_with_unicode ... ok
test outline_extractor::tests::search_headings::test_search_limit ... ok
test outline_extractor::tests::parse_heading::test_not_a_heading_no_space ... ok
test outline_extractor::tests::parse_heading::test_heading_with_obsidian_id ... ok
test outline_extractor::tests::search_headings::test_search_with_level_filter ... ok
test outline_extractor::tests::search_headings::test_case_insensitive_search ... ok
test outline_extractor::tests::search_headings::test_search_across_files ... ok

test result: ok. 33 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.01s

running 0 tests

test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s — all existing tests pass including test_get_section_crlf_preserved and test_crlf_preserved
   - 
   - 
running 0 tests

test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s

running 2 tests
test mcp::search_tools::tests::mcp_search_params_accepts_query ... ok
test mcp::search_tools::tests::mcp_search_params_rejects_missing_query ... ok

test result: ok. 2 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s

running 0 tests

test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s

running 1 test
test mcp_stdio_exposes_and_dispatches_all_registered_tools ... ok

test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.11s

running 17 tests
test config::tests::test_default_config ... ok
test config::tests::test_apply_partial_only_sets_provided_fields ... ok
test config::tests::test_load_partial_none_for_missing_file ... ok
test config::tests::test_search_config_default ... ok
test config::tests::test_search_config_toml_new_fields ... ok
test config::tests::test_env_overrides_vault_and_global ... ok
test config::tests::test_should_exclude_glob_pattern ... ok
test config::tests::test_should_exclude_heading ... ok
test config::tests::test_should_exclude_substring ... ok
test config::tests::test_env_with_empty_patterns ... ok
test config::tests::test_exclude_headings_env_var ... ok
test config::tests::test_global_config_path_xdg_set ... ok
test config::tests::test_global_config_path_fallback_to_home ... ok
test config::tests::test_search_config_from_toml ... ok
test config::tests::test_load_from_base_path_layering ... ok
test config::tests::test_merge_from_env ... ok
test config::tests::test_search_config_all_env_vars ... ok

test result: ok. 17 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s

running 29 tests
test capability::tests::test_get_daily_note_request_validation ... ok
test capability::tests::test_search_daily_notes_request_validation ... ok
test capability::tests::test_validate_date ... ok
test date_utils::tests::test_date_range_invalid_end ... ok
test date_utils::tests::test_date_range_single_day ... ok
test date_utils::tests::test_date_range_cross_year ... ok
test date_utils::tests::test_date_range_start_after_end ... ok
test date_utils::tests::test_date_range_invalid_start ... ok
test date_utils::tests::test_leap_year ... ok
test date_utils::tests::test_date_range_cross_month ... ok
test date_utils::tests::test_days_in_month ... ok
test date_utils::tests::test_date_range ... ok
test capability::tests::test_search_daily_notes_invalid_date_range ... ok
test date_utils::tests::test_validate_date_invalid ... ok
test date_utils::tests::test_parse_date ... ok
test date_utils::tests::test_validate_date_valid ... ok
test capability::tests::test_search_daily_notes ... ok
test capability::tests::test_get_daily_note_not_found ... ok
test pattern::tests::test_apply_pattern ... ok
test capability::tests::test_search_daily_notes_with_content ... ok
test pattern::tests::test_find_daily_note ... ok
test capability::tests::test_get_daily_note_found ... ok
test pattern::tests::test_find_daily_note_multiple_matches_error ... ok
test pattern::tests::test_find_daily_note_with_exclusion ... ok
test capability::tests::test_search_daily_notes_limit ... ok
test pattern::tests::test_get_daily_note_relative_path ... ok
test capability::tests::test_search_daily_notes_descending_sort ... ok
test pattern::tests::test_find_daily_note_multiple_patterns ... ok
test capability::tests::test_search_daily_notes_date_range_too_large ... ok

test result: ok. 29 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s

running 26 tests
test capability::filter_excluded_sections_tests::empty_exclude_headings_returns_unchanged ... ok
test recent_files::tests::test_extract_updated_no_frontmatter ... ok
test recent_files::tests::test_parse_iso8601_with_offset ... ok
test capability::remote_command_tests::read_files_remote_command_args_to_json_no_vault_path_panic ... ok
test recent_files::tests::test_unix_roundtrip ... ok
test capability::remote_command_tests::read_files_remote_command_minimal_args ... ok
test recent_files::tests::test_extract_updated_quoted ... ok
test recent_files::tests::test_parse_iso8601_positive_offset ... ok
test recent_files::tests::test_parse_iso8601_z ... ok
test recent_files::tests::test_extract_updated_missing ... ok
test recent_files::tests::test_extract_updated_from_frontmatter ... ok
test capability::filter_excluded_sections_tests::nested_subheadings_under_excluded_parent ... ok
test capability::filter_excluded_sections_tests::whitespace_preserved_on_retained_sections ... ok
test capability::filter_excluded_sections_tests::trailing_newline_preserved_with_remaining_content ... ok
test capability::filter_excluded_sections_tests::preamble_before_first_heading_preserved ... ok
test capability::filter_excluded_sections_tests::all_lines_excluded_returns_empty ... ok
test capability::filter_excluded_sections_tests::case_insensitive_matching ... ok
test capability::filter_excluded_sections_tests::excluded_section_removed_rest_preserved_exactly ... ok
test capability::filter_excluded_sections_tests::excluded_all_sections_returns_empty ... ok
test capability::filter_excluded_sections_tests::no_headings_returns_unchanged ... ok
test capability::filter_excluded_sections_tests::crlf_preserved_when_nothing_excluded ... ok
test capability::filter_excluded_sections_tests::frontmatter_preserved_when_no_heading_matches ... ok
test capability::filter_excluded_sections_tests::excluded_section_at_eof ... ok
test capability::filter_excluded_sections_tests::crlf_preserved_when_section_excluded ... ok
test capability::filter_excluded_sections_tests::one_excluded_section_among_several ... ok
test capability::filter_excluded_sections_tests::no_heading_matches_returns_unchanged ... ok

test result: ok. 26 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s

running 33 tests
test capability::remote_command_tests::section_remote_command_include_subsections_accepts_bool_value ... ok
test capability::remote_command_tests::outline_remote_command_hierarchical_bare_flag_fails ... ok
test capability::remote_command_tests::outline_remote_command_with_all_options ... ok
test capability::remote_command_tests::search_headings_remote_command_args_to_json_no_vault_path_panic ... ok
test capability::remote_command_tests::outline_remote_command_args_to_json_no_vault_path_panic ... ok
test capability::remote_command_tests::section_remote_command_args_to_json_no_vault_path_panic ... ok
test capability::remote_command_tests::search_headings_remote_command_with_all_options ... ok
test outline_extractor::tests::extract_headings::test_simple_document ... ok
test outline_extractor::tests::extract_headings::test_nested_code_blocks ... ok
test outline_extractor::tests::extract_sections_from_content::test_crlf_preserved ... ok
test capability::remote_command_tests::outline_remote_command_hierarchical_accepts_bool_value ... ok
test outline_extractor::tests::build_hierarchy::test_simple_hierarchy ... ok
test outline_extractor::tests::get_section::test_multiple_matching_sections ... ok
test outline_extractor::tests::get_section::test_get_section_with_subsections ... ok
test capability::remote_command_tests::section_remote_command_include_subsections_bare_flag_fails ... ok
test outline_extractor::tests::extract_sections_from_content::test_no_headings_returns_single_section ... ok
test outline_extractor::tests::get_section::test_get_section_without_subsections ... ok
test outline_extractor::tests::extract_headings::test_headings_in_code_blocks_ignored ... ok
test outline_extractor::tests::extract_sections_from_content::test_basic_extraction ... ok
test outline_extractor::tests::build_hierarchy::test_level_skipping ... ok
test outline_extractor::tests::get_section::test_get_section_basic ... ok
test outline_extractor::tests::parse_heading::test_h6_heading ... ok
test outline_extractor::tests::parse_heading::test_h1_heading ... ok
test outline_extractor::tests::parse_heading::test_regular_text ... ok
test outline_extractor::tests::get_section::test_get_section_crlf_preserved ... ok
test outline_extractor::tests::parse_heading::test_too_many_hashes ... ok
test outline_extractor::tests::parse_heading::test_heading_with_obsidian_id ... ok
test outline_extractor::tests::search_headings::test_search_limit ... ok
test outline_extractor::tests::search_headings::test_search_across_files ... ok
test outline_extractor::tests::search_headings::test_case_insensitive_search ... ok
test outline_extractor::tests::parse_heading::test_not_a_heading_no_space ... ok
test outline_extractor::tests::search_headings::test_search_with_level_filter ... ok
test outline_extractor::tests::parse_heading::test_heading_with_unicode ... ok

test result: ok. 33 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s

running 181 tests
test bm25::tests::test_unrelated_query_returns_empty ... ok
test bm25::tests::test_tokenize ... ok
test bm25::tests::test_single_document_corpus ... ok
test bm25::tests::test_identical_documents ... ok
test bm25::tests::test_long_document_vs_short ... ok
test bm25::tests::test_basic_scoring ... ok
test bm25::tests::test_extreme_params ... ok
test capability::remote_command_tests::index_remote_command_reindex_bare_flag_fails ... ok
test capability::remote_command_tests::index_remote_command_args_to_json_no_vault_path_panic ... ok
test capability::remote_command_tests::search_remote_command_args_to_json_no_vault_path_panic ... ok
test capability::remote_command_tests::search_remote_command_no_reindex_bare_flag_fails ... ok
test capability::remote_command_tests::index_remote_command_reindex_accepts_bool_value ... ok
test capability::remote_command_tests::search_remote_command_enable_recency_accepts_bool_value ... ok
test capability::remote_command_tests::search_remote_command_no_reindex_accepts_bool_value ... ok
test chunker::tests::test_chunker_config_from_search_config ... ok
test capability::remote_command_tests::search_remote_command_with_all_options ... ok
test chunker::tests::test_extract_frontmatter_date_created_fallback ... ok
test chunker::tests::test_extract_frontmatter_date_iso8601_with_timezone ... ok
test chunker::tests::test_extract_frontmatter_date_quoted_value ... ok
test chunker::tests::test_extract_frontmatter_date_updated_priority ... ok
test chunker::tests::test_extract_frontmatter_date_obsidian_default ... ok
test chunker::tests::test_extract_frontmatter_tags_no_frontmatter ... ok
test chunker::tests::test_extract_frontmatter_tags_comma_separated ... ok
test chunker::tests::test_extract_frontmatter_tags_no_tags_key ... ok
test chunker::tests::test_extract_frontmatter_tags_quoted ... ok
test chunker::tests::test_extract_frontmatter_date_iso8601_zulu ... ok
test chunker::tests::test_extract_frontmatter_tags_array ... ok
test chunker::tests::test_extract_frontmatter_date_no_date_field ... ok
test chunker::tests::test_extract_frontmatter_tags_single ... ok
test chunker::tests::test_extract_frontmatter_date_no_frontmatter ... ok
test chunker::tests::test_exclude_headings_case_insensitive ... ok
test chunker::tests::test_extract_frontmatter_date_malformed ... ok
test chunker::tests::test_chunk_file_preserves_tags ... ok
test chunker::tests::test_chunk_by_size_fallback_multi_line_line_numbers ... ok
test chunker::tests::test_chunk_by_size_fallback ... ok
test chunker::tests::test_chunk_file_propagates_date ... ok
test chunker::tests::test_chunk_file_basic ... ok
test civil_date::tests::test_century_non_leap ... ok
test civil_date::tests::test_day_difference_across_years ... ok
test chunker::tests::test_long_section_splitting ... ok
test chunker::tests::test_empty_content ... ok
test civil_date::tests::test_epoch_boundary ... ok
test civil_date::tests::test_february_round_trips ... ok
test civil_date::tests::test_known_dates ... ok
test civil_date::tests::test_leap_year_feb29 ... ok
test civil_date::tests::test_march_round_trips ... ok
test civil_date::tests::test_negative_days_for_pre_epoch ... ok
test civil_date::tests::test_pre_epoch_dates ... ok
test civil_date::tests::test_reciprocal ... ok
test civil_date::tests::test_y2k_boundary ... ok
test embeddings::tests::test_default_embedding_config ... ok
test chunker::tests::test_exclude_headings_substring_match ... ok
test embeddings::tests::test_embedding_config_from_search_config_none_without_api_base ... ok
test embeddings::tests::test_embedding_config_from_search_config_some_with_api_base ... ok
test embeddings::tests::test_normalize_embedding_unit_vector ... ok
test embeddings::tests::test_normalize_embedding_zero_vector ... ok
test embeddings::tests::test_task_prefix_document_without_title ... ok
test embeddings::tests::test_task_prefix_document_with_title ... ok
test embeddings::tests::test_task_prefix_query ... ok
test embeddings::tests::test_truncate_shorter_than_target ... ok
test embeddings::tests::test_truncate_longer_than_target ... ok
test fusion::tests::test_cosine_top_k_empty_vectors ... ok
test chunker::tests::test_chunk_file_with_sections ... ok
test fusion::tests::test_cosine_top_k_k_zero ... ok
test fusion::tests::test_cosine_top_k_exact_match ... ok
test fusion::tests::test_cosine_top_k_orthogonal ... ok
test chunker::tests::test_exclude_headings_skips_matching_sections ... ok
test fusion::tests::test_cosine_top_k_truncation ... ok
test fusion::tests::test_rrf_defaults_match_config ... ok
test fusion::tests::test_rrf_fuse_empty_both ... ok
test fusion::tests::test_rrf_fuse_empty_dense ... ok
test fusion::tests::test_rrf_fuse_non_overlapping ... ok
test chunker::tests::test_heading_path_tracking ... ok
test fusion::tests::test_rrf_fuse_preserves_order_for_ties ... ok
test fusion::tests::test_rrf_fuse_overlapping ... ok
test chunker::tests::test_long_section_split_distinct_line_spans ... ok
test fusion::tests::test_rrf_fuse_weighted_dense_heavy ... ok
test chunker::tests::test_merged_section_split_overlap_nonzero_line_spans ... ok
test chunker::tests::test_heading_path_sibling_sections ... ok
test chunker::tests::test_long_section_split_overlap_nonzero_line_spans ... ok
test chunker::tests::test_tiny_section_merging ... ok
test search::tests::test_extract_preview_exact_length ... ok
test search::tests::test_extract_preview_long_text ... ok
test search::tests::test_extract_preview_short_text ... ok
test chunker::tests::test_chunk_by_size_bounds_mixed_prose_and_huge_unbroken_word ... ok
test search::tests::test_recency_boost_all_same_date ... ok
test search::tests::test_recency_boost_falls_back_to_mtime ... ok
test search::tests::test_recency_boost_newest_ranks_higher ... ok
test search::tests::test_recency_boost_weight_zero_returns_identical ... ok
test chunker::tests::test_chunk_by_size_excludes_pure_blob_content ... ok
test chunker::tests::test_long_section_splitting_excludes_pure_blob_section ... ok
test search::tests::test_search_mode_default ... ok
test search::tests::test_search_mode_json_uses_lowercase ... ok
test search::tests::test_search_mode_needs_dense ... ok
test search::tests::test_search_mode_needs_sparse ... ok
test index::tests::test_metadata_mtime_secs ... ok
test index::tests::test_build_empty_vault ... ok
test search::tests::test_search_options_default ... ok
test search::tests::test_search_options_from_config ... ok
test search::tests::test_search_empty_vault ... ok
test capability::build_index_tests::build_index_reindex_when_no_existing_index_succeeds ... ok
test sparse::tests::test_empty_corpus ... ok
test sparse::tests::test_empty_query ... ok
test sparse::tests::test_index_and_score_basic ... ok
test sparse::tests::test_multi_term_ranking ... ok
test sparse::tests::test_single_chunk ... ok
test index::tests::test_build_exclusion_patterns ... ok
test index::tests::test_build_initial_index ... ok
test storage::tests::test_atomic_write_json ... ok
test storage::tests::test_compute_overall_content_hash_deterministic ... ok
test storage::tests::test_manifest_new_empty ... ok
test storage::tests::test_atomic_write_no_temp_leak ... ok
test storage::tests::test_manifest_serialization_round_trip ... ok
test index::tests::test_manifest_persists_after_build ... ok
test search::tests::test_ranked_chunk_fields ... ok
test index::tests::test_content_hash_changes_on_modification ... ok
test search::tests::test_search_no_reindex_uses_existing ... ok
test index::tests::test_chunk_source_file_is_relative_not_absolute ... ok
test capability::build_index_tests::build_index_reindex_removes_and_rebuilds_artifacts_preserves_models ... ok
test index::tests::test_build_full_rebuild_model_changed ... ok
test storage::tests::test_remove_manifest_noop_when_absent ... ok
test index::tests::test_build_up_to_date ... ok
test storage::tests::test_open_or_create_v2_manifest_cleans_up_orphaned_chunk_files ... ok
test index::tests::test_build_incremental_added_file ... ok
test index::tests::test_build_processes_files_in_sorted_relative_path_order ... ok
test storage::tests::test_rfc3339_formatting ... ok
test storage::tests::test_remove_vectors_noop_when_absent ... ok
test search::tests::test_search_max_results_limit ... ok
test storage::tests::test_remove_vectors_removes_existing_file ... ok
test storage::tests::test_remove_manifest_removes_existing_file ... ok
test storage::tests::test_staleness_diff_empty_index ... ok
test storage::tests::test_staleness_diff_full_rebuild_dimension_changed ... ok
test index::tests::test_full_rebuild_clears_chunks ... ok
test index::tests::test_touch_without_content_change_no_reindex ... ok
test index::tests::test_build_incremental_modified_file ... ok
test storage::tests::test_staleness_diff_modified_file ... ok
test storage::tests::test_open_or_create_new ... ok
test tests::test_config_default ... ok
test tests::test_config_resolve_absolute ... ok
test storage::tests::test_staleness_diff_removed_file ... ok
test tests::test_config_resolve_relative ... ok
test storage::tests::test_staleness_diff_up_to_date ... ok
test tokenize::tests::test_count_tokens_simple ... ok
test tokenize::tests::test_count_tokens_with_extra_whitespace ... ok
test storage::tests::test_vector_writer_empty_batches_produce_zero_count ... ok
test storage::tests::test_open_or_create_version_mismatch ... ok
test tokenize::tests::test_overlap_ge_max_tokens ... ok
test tokenize::tests::test_overlap_max_one ... ok
test tokenize::tests::test_split_long_word_ranges_never_splits_mid_utf8_char ... ok
test tokenize::tests::test_split_long_word_ranges_preserves_all_bytes_and_char_boundaries ... ok
test tokenize::tests::test_count_tokens_long_unbroken_word_is_not_counted_as_one_token ... ok
test tokenize::tests::test_tokenize_empty ... ok
test tokenize::tests::test_tokenize_fixed_remainder ... ok
test tokenize::tests::test_tokenize_fixed_simple ... ok
test tokenize::tests::test_tokenize_with_overlap_indexed_basic ... ok
test tokenize::tests::test_tokenize_with_overlap ... ok
test tokenize::tests::test_tokenize_with_overlap_indexed_empty ... ok
test tokenize::tests::test_tokenize_with_overlap_indexed_consistency ... ok
test tokenize::tests::test_tokenize_with_overlap_indexed_no_overlap ... ok
test tokenize::tests::test_tokenize_with_overlap_indexed_zero_max ... ok
test tokenize::tests::test_tokenize_zero_max ... ok
test tokenize::tests::test_tokenize_with_overlap_indexed_remainder ... ok
test tokenize::tests::test_tokenize_with_overlap_no_remainder ... ok
test storage::tests::test_staleness_diff_full_rebuild_chunk_config_changed ... ok
test storage::tests::test_open_or_create_v2_manifest_with_absolute_paths_is_rebuilt ... ok
test embeddings::tests::test_embed_single_without_api_base_returns_init_error ... ok
test storage::tests::test_staleness_diff_full_rebuild_model_changed ... ok
test storage::tests::test_remove_chunks ... ok
test storage::tests::test_vector_writer_streams_batches_matching_write_vectors ... ok
test storage::tests::test_staleness_diff_added_file ... ok
test storage::tests::test_staleness_diff_exclusion_filtering ... ok
test storage::tests::test_write_and_read_chunks ... ok
test storage::tests::test_open_or_create_existing ... ok
test storage::tests::test_reindex_cleanup_preserves_models_dir ... ok
test tokenize::tests::test_tokenize_with_overlap_indexed_bounds_a_single_giant_word ... ok
test chunker::tests::test_long_section_splitting_bounds_embedded_huge_unbroken_word ... ok
test embeddings::tests::test_embedder_creation_and_display ... ok
test search::tests::test_search_results_sorted_by_score ... ok
test search::tests::test_search_mode_used_reflects_degradation ... ok
test search::tests::test_dense_mode_degrades_to_sparse_when_embedding_unavailable ... ok
test search::tests::test_search_sparse_only ... ok

test result: ok. 181 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.04s

running 16 tests
test tag_extractor::tests::test_extract_frontmatter ... ok
test tag_extractor::tests::test_no_frontmatter ... ok
test tag_extractor::tests::test_empty_string_tag_filtered ... ok
test tag_extractor::tests::test_extract_tags_from_content ... ok
test tag_extractor::tests::test_empty_tags_filtered ... ok
test tag_extractor::tests::test_parse_tags_array ... ok
test tag_extractor::tests::test_parse_tags_single_string ... ok
test tag_extractor::tests::test_extract_tags_with_counts_duplicate_in_same_file ... ok
test tag_extractor::tests::test_tagged_file_contains_all_tags ... ok
test tag_extractor::tests::test_extract_tags_with_counts_multiple_files ... ok
test tag_extractor::tests::test_search_by_tags_and_logic ... ok
test tag_extractor::tests::test_search_by_tags_or_logic ... ok
test tag_extractor::tests::test_extract_tags_with_counts_single_file ... ok
test tag_extractor::tests::test_search_by_tags_empty_result ... ok
test tag_extractor::tests::test_search_by_tags_case_insensitive ... ok
test tag_extractor::tests::test_search_by_tags_respects_exclusions ... ok

test result: ok. 16 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s

running 60 tests
test extractor::tests::clean_content::test_removes_multiple_metadata ... ok
test extractor::tests::clean_content::test_removes_timestamp ... ok
test extractor::tests::clean_content::test_removes_created_date ... ok
test extractor::tests::metadata_extraction::test_extract_due_date_text ... ok
test extractor::tests::metadata_extraction::test_extract_due_date_function ... ok
test extractor::tests::clean_content::test_removes_tags_preserved ... ok
test extractor::tests::integration::test_completed_task_with_completion_date ... ok
test extractor::tests::clean_content::test_cleans_extra_whitespace ... ok
test capability::tests::args_to_json_minimal_args ... ok
test capability::tests::args_to_json_strips_path_and_preserves_filters ... ok
test extractor::tests::clean_content::test_preserves_task_text ... ok
test extractor::tests::clean_content::test_removes_due_date_text ... ok
test extractor::tests::metadata_extraction::test_extract_completed_date_text ... ok
test extractor::tests::clean_content::test_removes_completed_date ... ok
test extractor::tests::clean_content::test_removes_due_date_emoji ... ok
test extractor::tests::metadata_extraction::test_extract_created_date_emoji ... ok
test extractor::tests::metadata_extraction::test_extract_completed_date_emoji ... ok
test extractor::tests::integration::test_file_path_and_name ... ok
test extractor::tests::clean_content::test_removes_priority_emoji ... ok
test extractor::tests::clean_content::test_removes_priority_text ... ok
test extractor::tests::metadata_extraction::test_extract_due_date_emoji ... ok
test extractor::tests::integration::test_task_preserves_raw_line ... ok
test extractor::tests::integration::test_full_task_with_all_metadata ... ok
test extractor::tests::metadata_extraction::test_extract_created_date_text ... ok
test extractor::tests::metadata_extraction::test_extract_multiple_tags ... ok
test extractor::tests::metadata_extraction::test_extract_priority_high_emoji ... ok
test extractor::tests::metadata_extraction::test_extract_priority_low_emoji ... ok
test extractor::tests::metadata_extraction::test_extract_priority_text_low ... ok
test extractor::tests::metadata_extraction::test_extract_priority_lowest_emoji ... ok
test extractor::tests::metadata_extraction::test_extract_priority_text_high ... ok
test extractor::tests::metadata_extraction::test_extract_priority_text_medium ... ok
test extractor::tests::metadata_extraction::test_extract_priority_urgent_emoji ... ok
test extractor::tests::metadata_extraction::test_no_due_date ... ok
test filter::tests::test_empty_task_list ... ok
test filter::tests::test_no_filters_returns_all_tasks ... ok
test filter::tests::test_single_tag_filter ... ok
test filter::tests::test_status_filter_incomplete ... ok
test extractor::tests::sub_items::test_is_sub_item_with_asterisk ... ok
test extractor::tests::sub_items::test_is_sub_item_with_indent ... ok
test extractor::tests::sub_items::test_is_sub_item_with_checkbox ... ok
test extractor::tests::sub_items::test_parse_sub_item_not_list ... ok
test extractor::tests::sub_items::test_parse_sub_item_checkbox ... ok
test extractor::tests::sub_items::test_parse_sub_item_completed_checkbox ... ok
test extractor::tests::sub_items::test_parse_sub_item_regular_list ... ok
test extractor::tests::metadata_extraction::test_no_priority ... ok
test extractor::tests::parse_task_line::test_not_a_task ... ok
test extractor::tests::metadata_extraction::test_hashtag_alone_no_match ... ok
test extractor::tests::metadata_extraction::test_extract_single_tag ... ok
test extractor::tests::metadata_extraction::test_extract_tags_with_numbers ... ok
test extractor::tests::sub_items::test_is_not_sub_item_empty_line ... ok
test extractor::tests::parse_task_line::test_regular_list_item ... ok
test extractor::tests::parse_task_line::test_completed_task_uppercase ... ok
test extractor::tests::parse_task_line::test_cancelled_task ... ok
test extractor::tests::parse_task_line::test_other_status_task ... ok
test extractor::tests::sub_items::test_parse_sub_item_with_asterisk ... ok
test extractor::tests::parse_task_line::test_unchecked_task ... ok
test extractor::tests::sub_items::test_is_not_sub_item_same_indent ... ok
test extractor::tests::metadata_extraction::test_no_tags ... ok
test extractor::tests::parse_task_line::test_task_with_leading_whitespace ... ok
test extractor::tests::parse_task_line::test_completed_task ... ok

test result: ok. 60 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.13s

running 0 tests

test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s

running 0 tests

test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s

running 0 tests

test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s

running 0 tests

test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s

running 0 tests

test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s

running 5 tests
test notectl-search/src/fusion.rs - fusion::cosine_top_k (line 25) ... ok
test notectl-search/src/bm25.rs - bm25::Bm25Indexer::tokenize (line 167) ... ok
test notectl-search/src/fusion.rs - fusion::rrf_fuse (line 71) ... ok
test notectl-search/src/bm25.rs - bm25::Bm25Indexer (line 31) ... ok
test notectl-search/src/sparse.rs - sparse::SparseIndexer (line 10) ... ok

test result: ok. 5 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s

all doctests ran in 0.41s; merged doctests compilation took 0.41s

running 0 tests

test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s

running 0 tests

test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s — no downstream regressions

EDGE CASES TO PRESERVE (verify with existing tests):
- Empty content / no headings → handled by early return in extract_sections_from_content
- start_1 < 1 or start_1 >= end_1 → empty string
- Out-of-range line numbers → clamp to content.len()
- CRLF → byte-offset slicing preserves \r\n verbatim
- Unicode multi-byte chars → char-based iteration ensures correct byte offsets
<!-- SECTION:PLAN:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
Implementation complete. Replaced extract_lines()/line_start_byte() with build_line_offsets() + extract_lines_with_offsets(). Precomputes byte-offset table once per content string (O(n)) instead of rescanning from offset 0 for each heading lookup (was O(H×n)). Updated both extract_sections_from_content and get_section. All 318 workspace tests pass, clippy clean.
<!-- SECTION:NOTES:END -->

## Final Summary

<!-- SECTION:FINAL_SUMMARY:BEGIN -->
Replaced O(H×n) per-heading file rescanning with single-pass byte-offset table in outline extractor. Added build_line_offsets() and extract_lines_with_offsets(), removed extract_lines/line_start_byte. Updated extract_sections_from_content and get_section. All 318 tests pass, clippy clean. Push blocked by upstream RustSec advisory DB corruption (duplicate RUSTSEC-2026-0244).
<!-- SECTION:FINAL_SUMMARY:END -->
