# Validation Run

- Timestamp: 2026-01-28T15:51:09+0100
- Workspace: /home/tfo01/code/tools/h3xy

## Test Suite

### cargo test
```

running 168 tests
test hexfile::tests::test_normalized_errors_on_overlap ... ok
test hexfile::tests::test_as_contiguous_fills_gaps ... ok
test hexfile::tests::test_normalized_lossy_last_wins ... ok
test hexfile::tests::test_normalized_preserves_gaps ... ok
test hexfile::tests::test_normalized_merges_contiguous ... ok
test hexfile::tests::test_read_byte ... ok
test hexfile::tests::test_read_bytes_contiguous ... ok
test hexfile::tests::test_read_bytes_with_gaps ... ok
test hexfile::tests::test_sorted_order ... ok
test hexfile::tests::test_write_bytes ... ok
test io::binary::tests::test_parse_binary_base_address ... ok
test io::binary::tests::test_parse_binary_overflow ... ok
test io::binary::tests::test_write_binary_fill_gaps ... ok
test io::binary::tests::test_write_binary_order_of_appearance ... ok
test io::c_code::tests::test_write_c_code_basic ... ok
test io::hex_ascii::tests::test_hex_ascii_odd_digits_error ... ok
test io::hex_ascii::tests::test_hex_ascii_roundtrip ... ok
test io::hex_ascii::tests::test_hex_ascii_single_digit_tokens ... ok
test hexfile::tests::test_span_start_end ... ok
test io::intel_hex::tests::test_missing_eof ... ok
test io::hex_ascii::tests::test_hex_ascii_accepts_0x_prefix ... ok
test io::intel_hex::tests::test_checksum_error ... ok
test io::intel_hex::tests::test_parse_16bit_addresses_scaled ... ok
test io::hex_ascii::tests::test_hex_ascii_contiguous_pairs ... ok
test io::intel_hex::tests::test_parse_16bit_overflow ... ok
test io::intel_hex::tests::test_parse_extended_linear ... ok
test io::intel_hex::tests::test_parse_extended_segment ... ok
test io::intel_hex::tests::test_parse_simple ... ok
test io::intel_hex::tests::test_roundtrip ... ok
test io::intel_hex::tests::test_write_auto_mixed_modes ... ok
test io::srec::tests::test_parse_lowercase_prefix ... ok
test io::intel_hex::tests::test_write_simple ... ok
test io::srec::tests::test_srec_auto_type_s2 ... ok
test io::srec::tests::test_srec_bad_checksum ... ok
test ops::checksum::tests::test_algorithm_from_index ... ok
test io::srec::tests::test_srec_roundtrip_s1 ... ok
test ops::checksum::tests::test_byte_sum ... ok
test ops::checksum::tests::test_byte_sum_overflow ... ok
test ops::checksum::tests::test_crc16_arc_empty ... ok
test ops::checksum::tests::test_algorithm_result_size ... ok
test ops::checksum::tests::test_crc16_ibm_sdlc ... ok
test ops::checksum::tests::test_crc16_xmodem ... ok
test ops::checksum::tests::test_crc16_xmodem_empty ... ok
test ops::checksum::tests::test_crc32_iso_hdlc ... ok
test ops::checksum::tests::test_crc32_iso_hdlc_empty ... ok
test ops::checksum::tests::test_hexfile_checksum_append ... ok
test ops::checksum::tests::test_hexfile_checksum_crc16 ... ok
test ops::checksum::tests::test_hexfile_checksum_crc16_ccitt_be_init_ffff ... ok
test ops::checksum::tests::test_hexfile_checksum_append_overflow ... ok
test ops::checksum::tests::test_hexfile_checksum_crc16_ccitt_le_init_ffff ... ok
test ops::checksum::tests::test_hexfile_checksum_crc16_with_range ... ok
test ops::checksum::tests::test_hexfile_checksum_byte_sum ... ok
test ops::checksum::tests::test_hexfile_checksum_crc_empty_data ... ok
test ops::checksum::tests::test_hexfile_checksum_crc16_ccitt_be_init_0 ... ok
test ops::checksum::tests::test_hexfile_checksum_crc16_ccitt_le_init_0 ... ok
test ops::checksum::tests::test_hexfile_checksum_crc32 ... ok
test ops::checksum::tests::test_hexfile_checksum_crc32_le ... ok
test ops::checksum::tests::test_crc16_arc ... ok
test ops::checksum::tests::test_hexfile_checksum_forced_range_fill ... ok
test ops::checksum::tests::test_hexfile_checksum_overwrite_end_crc32 ... ok
test ops::checksum::tests::test_hexfile_checksum_crc16_le ... ok
test ops::checksum::tests::test_hexfile_checksum_overwrite_underflow ... ok
test ops::checksum::tests::test_hexfile_checksum_exclude_ranges ... ok
test ops::checksum::tests::test_hexfile_checksum_prepend_underflow ... ok
test ops::checksum::tests::test_hexfile_checksum_overwrite_end ... ok
test ops::checksum::tests::test_hexfile_checksum_with_range ... ok
test ops::checksum::tests::test_word_sum_be ... ok
test ops::checksum::tests::test_twos_complement ... ok
test ops::checksum::tests::test_word_sum_le ... ok
test ops::checksum::tests::test_word_sum_odd_length ... ok
test ops::filter::tests::test_cut_head_only ... ok
test ops::filter::tests::test_cut_multiple_ranges_on_single_segment ... ok
test ops::filter::tests::test_cut_removes_entire_segment ... ok
test ops::filter::tests::test_cut_tail_only ... ok
test ops::filter::tests::test_cut_splits_segment ... ok
test ops::filter::tests::test_fill_gaps ... ok
test ops::filter::tests::test_cut_spanning_multiple_segments ... ok
test ops::filter::tests::test_fill_creates_segment ... ok
test ops::filter::tests::test_fill_gaps_with_overlapping_segments ... ok
test ops::filter::tests::test_fill_with_pattern ... ok
test ops::filter::tests::test_fill_gaps_single_segment ... ok
test ops::filter::tests::test_fill_overwrite_partial ... ok
test ops::filter::tests::test_filter_range_all_removed ... ok
test ops::filter::tests::test_filter_multiple_ranges ... ok
test ops::filter::tests::test_merge_preserve ... ok
test ops::filter::tests::test_filter_range_clips_segment ... ok
test ops::filter::tests::test_merge_with_offset ... ok
test ops::filter::tests::test_filter_ranges_empty_clears_all ... ok
test ops::filter::tests::test_filter_ranges_overlapping ... ok
test ops::filter::tests::test_offset_i64_min_errors ... ok
test ops::filter::tests::test_merge_overwrite ... ok
test ops::filter::tests::test_merge_range_applies_before_offset ... ok
test ops::filter::tests::test_merge_with_range_filter ... ok
test ops::log::tests::test_parse_log_commands_basic ... ok
test ops::transform::tests::test_align_already_aligned ... ok
test ops::transform::tests::test_align_causes_overlap ... ok
test ops::pipeline::tests::test_pipeline_fill_cut_align ... ok
test ops::transform::tests::test_align_invalid_alignment ... ok
test ops::transform::tests::test_align_prepends_fill ... ok
test ops::transform::tests::test_align_non_power_of_two ... ok
test ops::transform::tests::test_align_then_split ... ok
test ops::filter::tests::test_offset_negative ... ok
test ops::log::tests::test_parse_log_commands_missing_filename ... ok
test ops::transform::tests::test_dspic_clear_ghost ... ok
test ops::transform::tests::test_map_star12_basic ... ok
test ops::transform::tests::test_remap_basic_banked ... ok
test ops::transform::tests::test_remap_invalid_params ... ok
test ops::filter::tests::test_filter_range_removes_outside ... ok
test ops::filter::tests::test_merge_with_negative_offset ... ok
test ops::transform::tests::test_map_star12x_basic ... ok
test ops::log::tests::test_execute_log_commands_load_error ... ok
test ops::transform::tests::test_scale_unscale_roundtrip ... ok
test ops::transform::tests::test_scale_addresses ... ok
test ops::filter::tests::test_offset_overflow_errors ... ok
test ops::transform::tests::test_remap_skips_oversize_block ... ok
test ops::transform::tests::test_split_segments ... ok
test ops::filter::tests::test_offset_underflow_errors ... ok
test ops::transform::tests::test_split_zero_size_noop ... ok
test ops::transform::tests::test_split_larger_than_segment ... ok
test ops::filter::tests::test_offset_positive ... ok
test ops::transform::tests::test_align_with_length ... ok
test ops::flags::tests::test_random_fill_bytes_deterministic ... ok
test ops::filter::tests::test_offset_large_negative_errors ... ok
test ops::transform::tests::test_align_with_alignment_1 ... ok
test ops::transform::tests::test_swap_dword ... ok
test ops::transform::tests::test_map_star08_examples ... ok
test ops::transform::tests::test_swap_multiple_segments ... ok
test ops::transform::tests::test_swap_dword_larger_buffer ... ok
test ops::transform::tests::test_swap_odd_length_leaves_trailing ... ok
test ops::transform::tests::test_swap_word ... ok
test ops::log::tests::test_execute_log_commands_fileopen ... ok
test range::tests::test_intersection ... ok
test range::tests::test_full_4gib_range_rejected ... ok
test range::tests::test_near_max_range_allowed ... ok
test ops::transform::tests::test_scale_overflow_errors ... ok
test range::tests::test_overlaps_single_byte_boundary ... ok
test ops::transform::tests::test_dspic_expand_appends_zeros ... ok
test range::tests::test_overlaps ... ok
test range::tests::test_parse_decimal ... ok
test range::tests::test_parse_binary ... ok
test range::tests::test_parse_empty_string ... ok
test range::tests::test_parse_hexview_ranges_quotes ... ok
test range::tests::test_parse_invalid_binary ... ok
test range::tests::test_address_overflow_in_start_length ... ok
test range::tests::test_parse_malformed_comma ... ok
test range::tests::test_contains ... ok
test range::tests::test_parse_malformed_dash ... ok
test ops::transform::tests::test_unscale_addresses ... ok
test ops::transform::tests::test_unscale_by_zero ... ok
test range::tests::test_from_start_end ... ok
test ops::transform::tests::test_dspic_shrink_keeps_low_bytes ... ok
test ops::transform::tests::test_unscale_transactional ... ok
test range::tests::test_from_start_length ... ok
test ops::transform::tests::test_unscale_not_divisible ... ok
test range::tests::test_parse_overflow_number ... ok
test range::tests::test_parse_range_with_c_suffixes ... ok
test range::tests::test_parse_range_with_dots ... ok
test range::tests::test_parse_range_with_hex_suffix ... ok
test range::tests::test_parse_ranges_multiple ... ok
test range::tests::test_parse_ranges_single ... ok
test range::tests::test_parse_start_length_hex ... ok
test range::tests::test_parse_start_end_hex ... ok
test range::tests::test_parse_u32_max ... ok
test range::tests::test_zero_length_error ... ok
test range::tests::test_single_byte_range ... ok
test segment::tests::test_end_address_saturates_on_overflow ... ok
test range::tests::test_start_exceeds_end_error ... ok
test segment::tests::test_is_contiguous_with_overflow_false ... ok

test result: ok. 168 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.01s


running 21 tests
test args::parse::tests::test_parse_ad_no_separator_hex ... ok
test args::parse_util::tests::test_parse_checksum_forced_range_with_pattern ... ok
test args::parse_util::tests::test_parse_merge_params_invalid_range ... ok
test args::parse_util::tests::test_parse_number_with_c_suffixes ... ok
test args::parse_util::tests::test_parse_signed_number_negative_hex ... ok
test args::parse_util::tests::test_parse_number_with_hex_suffix ... ok
test args::parse_util::tests::test_parse_import_param_with_offset ... ok
test args::io::tests::test_write_ford_ihex_missing_required ... ok
test args::parse::tests::test_parse_af_no_separator_hex ... ok
test args::parse_util::tests::test_parse_checksum_exclude_ranges ... ok
test args::parse_util::tests::test_parse_checksum_forced_invalid_pattern ... ok
test args::parse_util::tests::test_parse_number_with_dots ... ok
test args::parse_util::tests::test_parse_import_param_invalid_offset ... ok
test args::parse_util::tests::test_parse_output_params_hex ... ok
test args::parse_util::tests::test_parse_merge_params_with_range ... ok
test args::io::tests::test_write_porsche_output_appends_checksum ... ok
test args::io::tests::test_write_ford_ihex_happy_path ... ok
test args::io::tests::test_write_separate_binary_outputs_segments ... ok
test args::parse::tests::test_output_record_type_requires_length ... ok
test args::types::tests::test_parse_absolute_path_existing_file ... ok
test args::types::tests::test_parse_double_dash_forces_positional ... ok

test result: ok. 21 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s


running 11 tests
test test_cli_checksum_append ... ok
test test_cli_checksum_invalid_forced_pattern ... ok
test test_cli_checksum_address ... ok
test test_cli_checksum_file_output ... ok
test test_cli_checksum_overwrite_end ... ok
test test_cli_checksum_little_endian_output ... ok
test test_cli_checksum_upfront ... ok
test test_cli_checksum_exclude_range ... ok
test test_cli_checksum_begin ... ok
test test_cli_checksum_forced_range_fill ... ok
test test_cli_checksum_limited_range ... ok

test result: ok. 11 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.01s


running 1 test
test test_cli_multistage_bin_merge_align_output ... ok

test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s


running 16 tests
test test_cli_address_range_multiple_ranges ... ok
test test_cli_address_range_reduction_start_end ... ok
test test_cli_address_range_reduction ... ok
test test_cli_error_log_on_failure ... ok
test test_cli_cut_splits_block ... ok
test test_cli_align_address_binary_separator ... ok
test test_cli_align_fill_separator_and_hex_forms ... ok
test test_cli_align_length_fill ... ok
test test_cli_nested_chain_checksum ... ok
test test_cli_align_fill_no_separator_equivalence ... ok
test test_cli_fill_region_without_pattern_random ... ok
test test_cli_fill_region_pattern_preserves_data ... ok
test test_cli_threshold_options_noop ... ok
test test_cli_align_address_no_separator_semantics ... ok
test test_cli_error_log_created_on_success ... ok
test test_cli_align_fill_equal_separator ... ok

test result: ok. 16 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.02s


running 26 tests
test test_cli_auto_detect_binary_on_ascii_without_markers ... ok
test test_cli_auto_detect_binary_on_non_ascii ... ok
test test_cli_auto_detect_ignores_ihex_after_25_lines ... ok
test test_cli_auto_detect_srec_after_blank_lines ... ok
test test_cli_auto_detect_header_then_ihex_is_binary ... ok
test test_cli_auto_detect_ihex_after_blank_lines ... ok
test test_cli_auto_detect_srec_lowercase ... ok
test test_cli_hex_ascii_overlap_ignores_input ... ok
test test_cli_log_file_invalid_command ... ok
test test_cli_cut_multiple_ranges_one_arg ... ok
test test_cli_import_i16_scales_addresses ... ok
test test_cli_merge_transparent_range_offset ... ok
test test_cli_dspic_clear_ghost ... ok
test test_cli_hex_ascii_single_digit_tokens ... ok
test test_cli_log_file_open_without_input ... ok
test test_cli_remap_basic ... ok
test test_cli_merge_opaque_range_offset ... ok
test test_cli_s12xmap_basic ... ok
test test_cli_dspic_expand_default_target ... ok
test test_cli_dspic_shrink_default_target ... ok
test test_cli_s12map_basic ... ok
test test_cli_version_string_written_to_error_log ... ok
test test_cli_s08map_examples ... ok
test test_cli_mt_mo_conflict ... ok
test test_cli_remap_invalid_size ... ok
test test_cli_log_file_new_clears_data ... ok

test result: ok. 26 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.01s


running 15 tests
test test_cli_binary_order_of_appearance ... ok
test test_cli_hex_ascii_basic ... ok
test test_cli_hex_ascii_long_line ... ok
test test_cli_intel_hex_reclen_zero_defaults ... ok
test test_cli_output_option_exclusive ... ok
test test_cli_intel_hex_auto_modes ... ok
test test_cli_intel_hex_reclen ... ok
test test_cli_intel_hex_rectype_requires_reclen ... ok
test test_cli_intel_hex_forced_modes ... ok
test test_cli_binary_and_separate_binary ... ok
test test_cli_srec_rectype_requires_reclen ... ok
test test_cli_hex_ascii_separator ... ok
test test_cli_srec_reclen ... ok
test test_cli_srec_reclen_zero_defaults ... ok
test test_cli_srec_auto_and_forced ... ok

test result: ok. 15 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.01s


running 6 tests
test test_cli_checksum_parity_begin ... ok
test test_cli_pipeline_parity_binary_order ... ok
test test_cli_pipeline_parity_basic_chain ... ok
test test_cli_pipeline_parity_fa_fill_binary ... ok
test test_cli_checksum_parity_little_endian_file ... ok
test test_cli_pipeline_parity_xsb_split ... ok

test result: ok. 6 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s


running 4 tests
test test_merge_range_then_offset_transparent ... ok
test test_fill_region_preserves_existing_data ... ok
test test_merge_range_then_offset_opaque ... ok
test test_hexview_multistage_order ... ok

test result: ok. 4 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s


running 4 tests
test test_hexview_ranges_accept_formats ... ok
test test_hexview_ranges_binary_and_suffix ... ok
test test_hexview_ranges_reject_full_space ... ok
test test_hexview_ranges_multiple_and_quotes ... ok

test result: ok. 4 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s


running 12 tests
test test_align_then_split ... ok
test test_fill_gaps_then_merge_overwrite ... ok
test test_cut_fill_normalize ... ok
test test_fill_gaps_then_merge_preserve ... ok
test test_normalized_strict_vs_lossy ... ok
test test_merge_align_normalize_preserve ... ok
test test_operations_on_empty_file ... ok
test test_scale_unscale_roundtrip ... ok
test test_swap_and_read ... ok
test test_three_overlapping_segments_last_wins ... ok
test test_merge_align_normalize_overwrite ... ok
test test_filter_cut_fill_align_split ... ok

test result: ok. 12 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s


running 1 test
test test_pipeline_matches_cli_basic ... ok

test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s


running 0 tests

test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s

```

- Exit: 0

## Validation Suite

### /home/tfo01/code/tools/h3xy/scripts/validation_suite.sh
```

############################################################
#  h3xy Validation Run
#  Seed: 42
############################################################

============================================================
Generating input files...
============================================================

  Standard suite...
    Generated 22 files
  Random files (10)...
    Generated 10 files
  Merge pairs...
    Generated 6 merge pair files

  Total: 38 input files generated


============================================================
Generating test cases...
============================================================

  Standard test suite...
    Generated 2318 tests
  Fuzz tests (5 per file)...
    Generated 190 tests
  Merge tests...
    Generated 21 tests

  Total: 2529 test cases generated


============================================================
Running 2529 tests...
============================================================

.................................................. [50/2529]
.................................................. [100/2529]
.................................................. [150/2529]
.................................................. [200/2529]
.................................................. [250/2529]
........................FF..F..F..F............... [300/2529]
..F............................................... [350/2529]
.................................................. [400/2529]
F..F..F.................F......................... [450/2529]
.................................................. [500/2529]
.................................................. [550/2529]
.............................FF..F..F..F.......... [600/2529]
.......F................................FF........ [650/2529]
..................F............................... [700/2529]
.................................................. [750/2529]
.................................................. [800/2529]
.................................................. [850/2529]
..........F....................................... [900/2529]
.....................F............................ [950/2529]
.........................................FF....... [1000/2529]
.............................FF...F............... [1050/2529]
.................................................. [1100/2529]
.................................................. [1150/2529]
.................................................. [1200/2529]
.................................................. [1250/2529]
.................................................. [1300/2529]
.................................................. [1350/2529]
.......................F.......................... [1400/2529]
..................................F............... [1450/2529]
................................................F. [1500/2529]
.F..F............................................. [1550/2529]
.....FF..F..F..F.................F................ [1600/2529]
.................................................. [1650/2529]
.................................................. [1700/2529]
.................................................. [1750/2529]
.................................................. [1800/2529]
F................................................. [1850/2529]
...........F...................................... [1900/2529]
......................F........................... [1950/2529]
.................................................. [2000/2529]
.................................................. [2050/2529]
.................................................. [2100/2529]
.................................................. [2150/2529]
.................................................. [2200/2529]
.................................................. [2250/2529]
.................................................. [2300/2529]
...................F..F......F.F.....F...F.F.F..F. [2350/2529]
.F......F..F.FF.....F....F....F.....F.........FF.F [2400/2529]
..F..F.....F..FF..F.F..F.....F..FF.F.F.....F....F. [2450/2529]
...F..F..F..F.....F...F.F....F....F......F..F..... [2500/2529]
..............F..............

============================================================
RESULTS
============================================================

  Seed:     42
  Time:     1359.87s
  Tests:    2529
  Passed:   2441
  Failed:   88
  Skipped:  0

────────────────────────────────────────────────────────────
FAILURES:
────────────────────────────────────────────────────────────

  1. swapword_scattered
     Input: scattered.hex
     Args:  /SWAPWORD

  2. swaplong_scattered
     Input: scattered.hex
     Args:  /SWAPLONG

  3. bytesum_scattered_fc_fc
     Input: scattered.hex
     Args:  /CS2:@0xFC

  4. bytesum_scattered_1fc_1fc
     Input: scattered.hex
     Args:  /CS2:@0x1FC

  5. bytesum_scattered_ffc_ffc
     Input: scattered.hex
     Args:  /CS2:@0xFFC

  6. fa_swap_crc_scattered_ffc
     Input: scattered.hex
     Args:  /FA /FP:FF /SWAPWORD /CS0:@0xFFC

  7. bytesum_single_bytes_fc_fc
     Input: single_bytes.hex
     Args:  /CS2:@0xFC

  8. bytesum_single_bytes_1fc_1fc
     Input: single_bytes.hex
     Args:  /CS2:@0x1FC

  9. bytesum_single_bytes_ffc_ffc
     Input: single_bytes.hex
     Args:  /CS2:@0xFFC

  10. fa_swap_crc_single_bytes_ffc
     Input: single_bytes.hex
     Args:  /FA /FP:FF /SWAPWORD /CS0:@0xFFC

  11. swapword_misaligned
     Input: misaligned.hex
     Args:  /SWAPWORD

  12. swaplong_misaligned
     Input: misaligned.hex
     Args:  /SWAPLONG

  13. bytesum_misaligned_fc_fc
     Input: misaligned.hex
     Args:  /CS2:@0xFC

  14. bytesum_misaligned_1fc_1fc
     Input: misaligned.hex
     Args:  /CS2:@0x1FC

  15. bytesum_misaligned_ffc_ffc
     Input: misaligned.hex
     Args:  /CS2:@0xFFC

  16. fa_swap_crc_misaligned_ffc
     Input: misaligned.hex
     Args:  /FA /FP:FF /SWAPWORD /CS0:@0xFFC

  17. swapword_odd_length
     Input: odd_length.hex
     Args:  /SWAPWORD

  18. swaplong_odd_length
     Input: odd_length.hex
     Args:  /SWAPLONG

  19. fa_swap_crc_odd_length_ffc
     Input: odd_length.hex
     Args:  /FA /FP:FF /SWAPWORD /CS0:@0xFFC

  20. cr_large_256k_0_f_start
     Input: large_256k.hex
     Args:  /CR:0x0-0xF

  ... and 68 more failures

============================================================

Failures written to: /home/tfo01/code/tools/h3xy/validation/failures.json

```

- Exit: 1
