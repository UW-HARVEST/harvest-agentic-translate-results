# Configuration surface

Mechanically derived from the width-expanded declarations in `include/pcre2.h`,
the public option masks, and public-function branches in `src/*.c`. This build
has one compile-time configuration: 8-bit code units, Unicode enabled, JIT
disabled. `cargo metadata` exposes no Cargo features, so the default build is
the complete feature matrix.

The parenthesized name in each row is the differential integration test that
owns it. Randomized rows use a fixed seed and at least 64 generated cases.

| # | entry point(s) | configuration (options set + input shape) | [ ] |
|---:|----------------|-------------------------------------------|:---:|
| 1 | `pcre2_config_8` | Each numeric selector 0, 1, 3-9, and 12-16; value output (`config_and_messages`) | [x] |
| 2 | `pcre2_config_8` | Each numeric selector with `where = NULL`; size query (`config_and_messages`) | [x] |
| 3 | `pcre2_config_8` | Unicode-version and library-version string value and length queries (`config_and_messages`) | [x] |
| 4 | `pcre2_general_context_create_8`, `pcre2_general_context_copy_8`, `pcre2_general_context_free_8` | Default callbacks and null `memory_data` (`context_lifecycle_and_setters`) | [x] |
| 5 | `pcre2_general_context_create_8`, `pcre2_general_context_copy_8`, `pcre2_general_context_free_8` | Custom malloc/free callbacks and non-null `memory_data` (`context_lifecycle_and_setters`) | [x] |
| 6 | `pcre2_compile_context_create_8`, `pcre2_compile_context_copy_8`, `pcre2_compile_context_free_8` | Default general context and custom general context (`context_lifecycle_and_setters`) | [x] |
| 7 | `pcre2_match_context_create_8`, `pcre2_match_context_copy_8`, `pcre2_match_context_free_8` | Default general context and custom general context (`context_lifecycle_and_setters`) | [x] |
| 8 | `pcre2_convert_context_create_8`, `pcre2_convert_context_copy_8`, `pcre2_convert_context_free_8` | Default general context and custom general context (`context_lifecycle_and_setters`) | [x] |
| 9 | All context/free functions | Null pointer free is a no-op (`context_lifecycle_and_setters`) | [x] |
| 10 | `pcre2_set_bsr_8` | `PCRE2_BSR_UNICODE` and `PCRE2_BSR_ANYCRLF` (`context_lifecycle_and_setters`) | [x] |
| 11 | `pcre2_set_newline_8` | CR, LF, CRLF, ANY, ANYCRLF, and NUL conventions (`context_lifecycle_and_setters`) | [x] |
| 12 | `pcre2_set_character_tables_8`, `pcre2_maketables_8`, `pcre2_maketables_free_8` | Default and locale-generated tables (`tables_and_allocators`) | [x] |
| 13 | `pcre2_set_compile_extra_options_8` | Every individual public extra-option bit (`context_lifecycle_and_setters`) | [x] |
| 14 | `pcre2_set_max_pattern_length_8` | Zero, one, finite many, and `SIZE_MAX` (`context_lifecycle_and_setters`) | [x] |
| 15 | `pcre2_set_max_pattern_compiled_length_8` | Zero, finite boundary, and `SIZE_MAX` (`context_lifecycle_and_setters`) | [x] |
| 16 | `pcre2_set_max_varlookbehind_8`, `pcre2_set_parens_nest_limit_8` | Zero, one, and large limits (`context_lifecycle_and_setters`) | [x] |
| 17 | `pcre2_set_compile_recursion_guard_8` | Null callback and rejecting callback (`context_lifecycle_and_setters`) | [x] |
| 18 | `pcre2_set_optimize_8` | NONE, FULL, and each on/off directive 64-69 (`context_lifecycle_and_setters`) | [x] |
| 19 | `pcre2_set_callout_8` | Null and active callbacks with user data (`context_lifecycle_and_setters`) | [x] |
| 20 | `pcre2_set_substitute_callout_8`, `pcre2_set_substitute_case_callout_8` | Null and active callbacks with user data (`context_lifecycle_and_setters`) | [x] |
| 21 | `pcre2_set_depth_limit_8`, `pcre2_set_recursion_limit_8` | Zero, one, and large synonymous limits (`context_lifecycle_and_setters`) | [x] |
| 22 | `pcre2_set_heap_limit_8`, `pcre2_set_match_limit_8` | Zero, one, and large limits (`context_lifecycle_and_setters`) | [x] |
| 23 | `pcre2_set_offset_limit_8` | Zero, in-subject offset, and `SIZE_MAX` (`context_lifecycle_and_setters`) | [x] |
| 24 | `pcre2_set_recursion_memory_management_8` | Null and non-null callbacks; obsolete no-op (`context_lifecycle_and_setters`) | [x] |
| 25 | `pcre2_set_glob_separator_8` | Slash, backslash, and dot (`context_lifecycle_and_setters`) | [x] |
| 26 | `pcre2_set_glob_escape_8` | Disabled zero and every accepted ASCII punctuation (`context_lifecycle_and_setters`) | [x] |
| 27 | `pcre2_compile_8` | Null pointer plus zero length, explicit empty slice, and zero-terminated empty string (`compile_info_copy_callouts`) | [x] |
| 28 | `pcre2_compile_8` | Explicit-length and zero-terminated randomized ASCII patterns (`compile_info_copy_callouts`) | [x] |
| 29 | `pcre2_compile_8` | Literal patterns, including regex metacharacters as data (`compile_info_copy_callouts`) | [x] |
| 30 | `pcre2_compile_8` | UTF and UTF+UCP patterns with one-, two-, three-, and four-byte code points (`compile_info_copy_callouts`) | [x] |
| 31 | `pcre2_compile_8` | Caseless, multiline, dotall, ungreedy, extended, duplicate-name, no-auto-capture, and anchored compile options (`compile_info_copy_callouts`) | [x] |
| 32 | `pcre2_compile_8` | Optimization flags enabled/disabled through options and compile context (`compile_info_copy_callouts`) | [x] |
| 33 | `pcre2_compile_8` | Each newline and BSR context mode with newline-sensitive patterns (`compile_info_copy_callouts`) | [x] |
| 34 | `pcre2_compile_8` | Character tables, pattern-size limits, lookbehind limit, nesting limit, and recursion guard contexts (`compile_info_copy_callouts`) | [x] |
| 35 | `pcre2_code_copy_8`, `pcre2_code_copy_with_tables_8`, `pcre2_code_free_8` | Compiled code with default and custom tables (`compile_info_copy_callouts`) | [x] |
| 36 | `pcre2_pattern_info_8` | All selectors 0-26 as size queries and value queries (`compile_info_copy_callouts`) | [x] |
| 37 | `pcre2_callout_enumerate_8` | No callout, numbered callout, string callout, auto-callout, and callback early return (`compile_info_copy_callouts`) | [x] |
| 38 | `pcre2_match_data_create_8`, `pcre2_match_data_free_8` | Requested ovector count 0, 1, many, and above `UINT16_MAX` (`match_data_and_getters`) | [x] |
| 39 | `pcre2_match_data_create_from_pattern_8`, `pcre2_match_data_free_8` | Pattern with zero, one, and many captures (`match_data_and_getters`) | [x] |
| 40 | `pcre2_get_ovector_count_8`, `pcre2_get_ovector_pointer_8`, `pcre2_get_match_data_size_8` | Before and after successful match (`match_data_and_getters`) | [x] |
| 41 | `pcre2_get_mark_8`, `pcre2_get_startchar_8`, `pcre2_get_match_data_heapframes_size_8` | Marked and unmarked successful matches (`match_data_and_getters`) | [x] |
| 42 | `pcre2_match_8` | Randomized literal pattern/subject pairs: match and no-match (`randomized_match`) | [x] |
| 43 | `pcre2_match_8` | Explicit length, zero-terminated length, null+zero empty subject, and embedded NUL (`randomized_match`) | [x] |
| 44 | `pcre2_match_8` | Start offsets at zero, interior code-unit boundaries, and end (`randomized_match`) | [x] |
| 45 | `pcre2_match_8` | ANCHORED, ENDANCHORED, NOTBOL, NOTEOL, NOTEMPTY, and NOTEMPTY_ATSTART (`randomized_match`) | [x] |
| 46 | `pcre2_match_8` | PARTIAL_SOFT, PARTIAL_HARD, and full matching (`randomized_match`) | [x] |
| 47 | `pcre2_match_8` | UTF subjects across all UTF-8 widths, with and without `NO_UTF_CHECK` (`randomized_match`) | [x] |
| 48 | `pcre2_match_8` | Match, heap, depth, and offset context limits (`randomized_match`) | [x] |
| 49 | `pcre2_match_8` | Callout success, positive abort, and negative abort (`randomized_match`) | [x] |
| 50 | `pcre2_match_8` | `COPY_MATCHED_SUBJECT` and `NO_JIT` paths (`randomized_match`) | [x] |
| 51 | `pcre2_dfa_match_8` | Randomized full match/no-match with workspace sizes 20 and 256 (`dfa_and_iteration`) | [x] |
| 52 | `pcre2_dfa_match_8` | SHORTEST, ANCHORED, partial soft/hard, and zero-terminated subject (`dfa_and_iteration`) | [x] |
| 53 | `pcre2_dfa_match_8` | Valid restart after partial state (`dfa_and_iteration`) | [x] |
| 54 | `pcre2_next_match_8` | Prior error/no-match, non-empty match, empty interior match, and empty end match (`dfa_and_iteration`) | [x] |
| 55 | `pcre2_substring_length_bynumber_8`, `pcre2_substring_copy_bynumber_8`, `pcre2_substring_get_bynumber_8`, `pcre2_substring_free_8` | Capture 0, set capture, unset optional capture, and empty capture (`substring_api`) | [x] |
| 56 | `pcre2_substring_length_byname_8`, `pcre2_substring_copy_byname_8`, `pcre2_substring_get_byname_8` | Unique name and duplicate names with first-set selection (`substring_api`) | [x] |
| 57 | `pcre2_substring_nametable_scan_8`, `pcre2_substring_number_from_name_8` | First/middle/last names and duplicate-name range (`substring_api`) | [x] |
| 58 | `pcre2_substring_list_get_8`, `pcre2_substring_list_free_8` | With lengths, without lengths, unset capture, and embedded NUL (`substring_api`) | [x] |
| 59 | `pcre2_serialize_encode_8`, `pcre2_serialize_free_8` | One code and several codes using the same tables (`serialize_roundtrip`) | [x] |
| 60 | `pcre2_serialize_get_number_of_codes_8`, `pcre2_serialize_decode_8` | Decode fewer than and exactly the encoded count; rematch decoded code (`serialize_roundtrip`) | [x] |
| 61 | `pcre2_substitute_8` | First-only and GLOBAL substitution over randomized subjects (`substitute_modes`) | [x] |
| 62 | `pcre2_substitute_8` | Literal, numeric capture, named capture, unset-empty, unknown-unset, and extended replacement syntax (`substitute_modes`) | [x] |
| 63 | `pcre2_substitute_8` | Caller buffer fits, is too small, and overflow-length query (`substitute_modes`) | [x] |
| 64 | `pcre2_substitute_8` | Null+zero subject/replacement, explicit and zero-terminated lengths (`substitute_modes`) | [x] |
| 65 | `pcre2_substitute_8` | Existing match, replacement-only, copied subject, and substitute callout (`substitute_modes`) | [x] |
| 66 | `pcre2_pattern_convert_8`, `pcre2_converted_pattern_free_8` | POSIX basic, POSIX extended, and glob conversions (`convert_modes`) | [x] |
| 67 | `pcre2_pattern_convert_8` | UTF and no-UTF-check; explicit, zero-terminated, and null+zero input (`convert_modes`) | [x] |
| 68 | `pcre2_pattern_convert_8` | Length-only, caller buffer, and library-allocated buffer (`convert_modes`) | [x] |
| 69 | `pcre2_pattern_convert_8` | Glob separator slash/backslash/dot; disabled and punctuation escapes (`convert_modes`) | [x] |
| 70 | `pcre2_jit_compile_8` | COMPLETE, partial modes, INVALID_UTF, TEST_ALLOC, and zero options with JIT disabled (`jit_disabled`) | [x] |
| 71 | `pcre2_jit_match_8` | Direct call against non-JIT code with each match mode (`jit_disabled`) | [x] |
| 72 | `pcre2_jit_stack_create_8`, `pcre2_jit_stack_assign_8`, `pcre2_jit_stack_free_8`, `pcre2_jit_free_unused_memory_8` | Zero and nonzero stack sizes with JIT disabled; null frees/assignment (`jit_disabled`) | [x] |
| 73 | `pcre2_get_error_message_8` | Every defined compile and runtime error number with exact-fit and oversized buffers (`config_and_messages`) | [x] |
