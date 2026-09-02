# CONFIGS.md — the CONFIGURATION-SURFACE TABLE (Phase A / Phase B)

The mirror of `ERRORS.md` for **valid** inputs. Rows are the meaningful
combinations of the axes the C code actually branches on, derived from the public
header and the `if` / `switch` / `#ifdef` branches the C takes on those values —
not from a guess about which options "matter".

## Axes, and where the C branches on them

### A1 — compile options (`pcre2_compile`, 3rd argument)

All 32 bits of the options word are probed individually and in the combinations
below. From `c_src/include/pcre2.h` and the `options` tests in
`pcre2_compile.c` / `pcre2_match.c` / `pcre2_dfa_match.c`:

`PCRE2_ANCHORED`, `PCRE2_NO_UTF_CHECK`, `PCRE2_ENDANCHORED`,
`PCRE2_ALLOW_EMPTY_CLASS`, `PCRE2_ALT_BSUX`, `PCRE2_AUTO_CALLOUT`,
`PCRE2_CASELESS`, `PCRE2_DOLLAR_ENDONLY`, `PCRE2_DOTALL`, `PCRE2_DUPNAMES`,
`PCRE2_EXTENDED`, `PCRE2_FIRSTLINE`, `PCRE2_MATCH_UNSET_BACKREF`,
`PCRE2_MULTILINE`, `PCRE2_NEVER_UCP`, `PCRE2_NEVER_UTF`,
`PCRE2_NO_AUTO_CAPTURE`, `PCRE2_NO_AUTO_POSSESS`, `PCRE2_NO_DOTSTAR_ANCHOR`,
`PCRE2_NO_START_OPTIMIZE`, `PCRE2_UCP`, `PCRE2_UNGREEDY`, `PCRE2_UTF`,
`PCRE2_NEVER_BACKSLASH_C`, `PCRE2_ALT_CIRCUMFLEX`, `PCRE2_ALT_VERBNAMES`,
`PCRE2_USE_OFFSET_LIMIT`, `PCRE2_EXTENDED_MORE`, `PCRE2_LITERAL`,
`PCRE2_MATCH_INVALID_UTF`, `PCRE2_ALT_EXTENDED_CLASS`, plus the 3 undefined bits.

### A2 — compile *extra* options (`pcre2_set_compile_extra_options`)

All 32 bits probed individually; the 17 defined ones are
`ALLOW_SURROGATE_ESCAPES`, `BAD_ESCAPE_IS_LITERAL`, `MATCH_WORD`, `MATCH_LINE`,
`ESCAPED_CR_IS_LF`, `ALT_BSUX`, `ALLOW_LOOKAROUND_BSK`, `CASELESS_RESTRICT`,
`ASCII_BSD`, `ASCII_BSS`, `ASCII_BSW`, `ASCII_POSIX`, `ASCII_DIGIT`,
`PYTHON_OCTAL`, `NO_BS0`, `NEVER_CALLOUT`, `TURKISH_CASING`.

### A3 — compile-context state

`pcre2_set_newline` (6 values), `pcre2_set_bsr` (2 values),
`pcre2_set_character_tables` (built-in vs `pcre2_maketables`),
`pcre2_set_max_varlookbehind`, `pcre2_set_parens_nest_limit`,
`pcre2_set_max_pattern_length`, `pcre2_set_max_pattern_compiled_length`,
`pcre2_set_optimize` (8 directives), `pcre2_set_compile_recursion_guard`,
and the same state reached through `pcre2_compile_context_copy`.

### A4 — in-pattern option setters (a separate C code path from A1/A3)

`(*UTF)`, `(*UCP)`, `(*CR)`, `(*LF)`, `(*CRLF)`, `(*ANY)`, `(*ANYCRLF)`,
`(*NUL)`, `(*BSR_ANYCRLF)`, `(*BSR_UNICODE)`, `(*LIMIT_MATCH=)`,
`(*LIMIT_DEPTH=)`, `(*LIMIT_HEAP=)`, `(*NO_AUTO_POSSESS)`,
`(*NO_DOTSTAR_ANCHOR)`, `(*NO_START_OPT)`, `(*NOTEMPTY)`,
`(*NOTEMPTY_ATSTART)`, `(*NO_JIT)`, `(*CASELESS)`, and the inline `(?imsxxnJU)`
/ `(?-...)` / `(?^...)` forms.

### A5 — match options (`pcre2_match` / `pcre2_dfa_match` / `pcre2_jit_match` / `pcre2_substitute`)

All 32 bits probed individually; the meaningful ones are `NOTBOL`, `NOTEOL`,
`NOTEMPTY`, `NOTEMPTY_ATSTART`, `PARTIAL_SOFT`, `PARTIAL_HARD`, `DFA_RESTART`,
`DFA_SHORTEST`, `ANCHORED`, `ENDANCHORED`, `NO_UTF_CHECK`,
`COPY_MATCHED_SUBJECT`, `NO_JIT`, `DISABLE_RECURSELOOP_CHECK`, and the 6
`SUBSTITUTE_*` bits.

### A6 — match-context state

`pcre2_set_match_limit`, `pcre2_set_depth_limit`, `pcre2_set_heap_limit`,
`pcre2_set_offset_limit`, `pcre2_set_recursion_limit`,
`pcre2_set_recursion_memory_management`, `pcre2_set_callout`,
`pcre2_set_substitute_callout`, `pcre2_set_substitute_case_callout`,
`pcre2_jit_stack_assign`, and the state reached through
`pcre2_match_context_copy`.

### A7 — convert-context state and convert options

`pcre2_set_glob_escape`, `pcre2_set_glob_separator`,
`pcre2_convert_context_copy`, and `PCRE2_CONVERT_{UTF,NO_UTF_CHECK,POSIX_BASIC,
POSIX_EXTENDED,GLOB,GLOB_NO_WILD_SEPARATOR,GLOB_NO_STARSTAR}`.

### A8 — entry points (the FULL public surface, lowest level included)

Low level: `_pcre2_strlen`, `_pcre2_strcmp`, `_pcre2_strcmp_c8`,
`_pcre2_strncmp`, `_pcre2_strncmp_c8`, `_pcre2_strcpy_c8`, `_pcre2_ord2utf`,
`_pcre2_valid_utf`, `_pcre2_is_newline`, `_pcre2_was_newline`, `_pcre2_extuni`,
`_pcre2_script_run`, `_pcre2_ckd_smul`, `_pcre2_find_bracket`,
`_pcre2_update_classbits`, `_pcre2_compile_get_hash_from_name`,
`_pcre2_memctl_malloc`, `_pcre2_jit_get_target`, `_pcre2_jit_get_size`,
plus all 27 exported data tables.
Mid level: `pcre2_compile`, `pcre2_code_copy`, `pcre2_code_copy_with_tables`,
`pcre2_pattern_info`, `pcre2_callout_enumerate`, `pcre2_match_data_create`,
`pcre2_match_data_create_from_pattern`, `pcre2_match`, `pcre2_dfa_match`,
`pcre2_jit_match`, `pcre2_jit_compile`, `pcre2_get_ovector_pointer`,
`pcre2_get_ovector_count`, `pcre2_get_startchar`, `pcre2_get_mark`,
`pcre2_get_match_data_size`, `pcre2_get_match_data_heapframes_size`,
`pcre2_next_match`, all 11 `pcre2_substring_*`, all 4 `pcre2_serialize_*`,
`pcre2_maketables`, `pcre2_get_error_message`, `pcre2_config`.
Convenience: `pcre2_substitute`, `pcre2_pattern_convert`.

`_pcre2_study`, `_pcre2_auto_possessify`, `_pcre2_xclass`, `_pcre2_eclass`,
`_pcre2_check_escape`, `_pcre2_compile_class_{nested,not_nested}`,
`_pcre2_compile_{find_named_group,find_dupname_details,add_name_to_table,
parse_scan_substr_args,parse_recurse_args}` take internal `compile_block *` /
byte-code pointers that cannot be synthesised from outside the library; they are
driven through `pcre2_compile` / `pcre2_match` and their results are verified by
comparing the *entire* compiled byte code (see the "how each row is checked"
note below) and the match results.

### A9 — input shapes

Pattern: empty, 1 unit, many units, 40 000 units, embedded NUL, explicit length
vs `PCRE2_ZERO_TERMINATED`, ASCII vs multi-byte UTF-8 vs invalid UTF-8,
250/251 nesting depth, 10 001 named groups, 6 000-range class.
Subject: empty (`NULL`+0 and pointer+0), 1, 2, 255, 256, 1 000, 5 000 units;
explicit length vs `PCRE2_ZERO_TERMINATED`; embedded NUL; every newline sequence
(`\n`, `\r`, `\r\n`, `\x85`, U+2028/9); valid and invalid UTF-8; every start
offset from 0 to `len`.
Ovector: `create(0)`, `create(1)`, `create(8)`, `create_from_pattern`,
`create(65535)`, `create(65536)`, `create(UINT32_MAX)`.
Output buffers (substitute / substring / convert): capacities 0, 1, 2, 3, 4, 8,
16, 48, 256.

## How each row is checked

For every row, both `.so`s are driven through their exported symbols with **many
randomized inputs** (fixed seeds, see the `Rng::new(...)` calls) and the
following are compared:

1. `pcre2_compile`: NULL-ness, `*errorcode`, `*erroroffset`.
2. The **entire compiled byte code, byte for byte** — obtained with
   `pcre2_serialize_encode`, which dumps the whole `pcre2_real_code` block plus
   the character tables. This is what makes the untestable internal helpers
   (`_pcre2_study`, `_pcre2_auto_possessify`, the class compilers, the name-table
   builder) verifiable: any difference in what they produce changes these bytes.
3. Every `pcre2_pattern_info` item, including `FIRSTBITMAP` (32 bytes) and
   `NAMETABLE` (`namecount * nameentrysize` bytes).
4. `pcre2_code_copy` and `pcre2_code_copy_with_tables` byte code.
5. `pcre2_match`, `pcre2_dfa_match` (workspace 20 / 64 / 1000, with and without
   `DFA_SHORTEST`, plus `DFA_RESTART` after a partial match) and
   `pcre2_jit_match`: return code, the defined part of the ovector,
   `pcre2_get_startchar`, `pcre2_get_mark`, `pcre2_get_match_data_size`,
   `pcre2_next_match`.
6. All substring accessors for groups 0..5 (`length_bynumber`,
   `copy_bynumber` at several capacities, `get_bynumber`, `list_get`).
7. Callout blocks: every field of `pcre2_callout_block` and
   `pcre2_callout_enumerate_block`, in order, for accepting / skipping /
   aborting callbacks.
8. The allocation *sequence*: a counting allocator records every requested size,
   and the two sequences must be identical.

## Feature combinations

`translation/Cargo.toml` declares **no `[features]` table**, so the crate has
exactly one feature combination: `--no-default-features` and the default build
are the same thing. `tools/check_features.sh` enumerates the features from
`Cargo.toml` and runs `cargo check` / `cargo test` for each combination; it
reports the single (empty) combination.

## Configuration rows

| # | entry point(s) | configuration (options set + input shape) | test | [x] |
|---|----------------|--------------------------------------------|------|-----|
| 1 | `_pcre2_strlen`, `_pcre2_strcmp`, `_pcre2_strcmp_c8`, `_pcre2_strncmp`, `_pcre2_strncmp_c8`, `_pcre2_strcpy_c8` | 10 strings x 10 strings x n in {0,1,2,3,4,8,64}; empty, 1-unit, 32-unit, high-bit and control bytes | `priv_string_functions` | [x] |
| 2 | `_pcre2_ord2utf` | every code point 0..0x2FF, every boundary of the 1/2/3/4/5/6-byte encodings ±1, the surrogate range, 0x10FFFF±1, 0x7FFFFFFF, 0xFFFFFFFF, and 4 000 random `uint32_t` | `priv_ord2utf_over_full_range` | [x] |
| 3 | `_pcre2_valid_utf` | 25 hand-built malformations + 80 000 random / lead-byte-biased buffers of length 0..8 | `priv_valid_utf_agrees_on_every_malformation`, `priv_valid_utf_randomized` | [x] |
| 4 | `_pcre2_is_newline`, `_pcre2_was_newline` | 11 `nltype` values (1..6 valid, plus 0, 7, 8, 100, `UINT32_MAX` out-of-range enum) x utf in {0,1} x all 256 lead bytes x 4 window positions, plus the U+0085 / U+2028 / U+2029 / CRLF / LFCR / NUL / VT / FF sequences | `priv_newline_functions` | [x] |
| 5 | `_pcre2_script_run` | 12 hand-built script mixes + 4 000 random sequences over Latin / Greek / Hebrew / Arabic-Indic / Hiragana / ASCII digits, utf in {0,1} | `priv_script_run` | [x] |
| 6 | `_pcre2_extuni` | 6 000 random grapheme-cluster sequences over a pool of 19 code points (combining marks, Hangul jamo, ZWJ, regional indicators, viramas, spacing marks), utf in {0,1} | `priv_extuni` | [x] |
| 7 | `_pcre2_ckd_smul` | 20 x 20 boundary pairs (0, ±1, ±2, 32767/8, 65535/6, 46340/1, `INT32_MIN/MAX`) + 20 000 random pairs | `priv_ckd_smul` | [x] |
| 8 | `_pcre2_compile_get_hash_from_name` | 20 000 random names of length 1..24 over printable ASCII, plus uniform names of length 1/2/3/128 filled with 0x00, 0x01, 0x7f, 0x80, 0xff | `priv_get_hash_from_name` | [x] |
| 9 | `_pcre2_update_classbits` | `ptype` 0..20 x `pdata` 0..40 x `negated` in {0,1}, on a zeroed and on a pre-seeded (0xA5) bitmap, plus out-of-range `ptype` 100/255/1000/`UINT32_MAX` | `priv_update_classbits` | [x] |
| 10 | `_pcre2_find_bracket` | 6 patterns x `n` in {0,1,2,3,4,-1,100} x `capturing` in {0,1}, on real compiled byte code | `priv_find_bracket_on_real_code` | [x] |
| 11 | `_pcre2_memctl_malloc` | sizes 1, 8, 100, 4096 with a NULL memctl (default allocator) | `priv_memctl_malloc` | [x] |
| 12 | all 27 exported data tables | byte-for-byte, using the ELF symbol sizes from the C `.so` | `all_exported_data_tables_are_byte_identical` | [x] |
| 13 | `_pcre2_unicode_version_8` | pointed-to C string | `unicode_version_string_matches` | [x] |
| 14 | `pcre2_maketables`, `pcre2_maketables_free` | default general context (3 repetitions), and a context whose allocator always fails; result also compared against `_pcre2_default_tables_8` | `maketables_output_is_byte_identical` | [x] |
| 15 | `pcre2_config` | all 17 request codes, value and string forms, plus `where == NULL` | `config_agrees`, `config_with_null_where_returns_size` | [x] |
| 16 | `pcre2_get_error_message` | every code in -80..225 x buffer capacity in {0,1,2,5,512}, plus `INT32_MIN/MAX` | `get_error_message_every_code_and_boundary` | [x] |
| 17 | `pcre2_compile` + full match matrix | **default configuration**: no options, no context; 176 patterns x 17 match-option sets x 6 randomized subjects each | `cfg_default` | [x] |
| 18 | `pcre2_compile` + match | **each compile-option bit alone** (all 32, including the 3 undefined ones) x 176 patterns x 4 match-option sets | `cfg_each_compile_option_bit_alone` | [x] |
| 19 | `pcre2_compile` + match | **each extra-option bit alone** (all 32) x 176 patterns x 3 match-option sets | `cfg_each_extra_option_bit_alone` | [x] |
| 20 | `pcre2_compile` + match | `PCRE2_UTF` | `cfg_utf_and_ucp_matrix` | [x] |
| 21 | `pcre2_compile` + match | `PCRE2_UCP` | `cfg_utf_and_ucp_matrix` | [x] |
| 22 | `pcre2_compile` + match | `PCRE2_UTF|PCRE2_UCP` | `cfg_utf_and_ucp_matrix` | [x] |
| 23 | `pcre2_compile` + match | `PCRE2_UTF|PCRE2_CASELESS` | `cfg_utf_and_ucp_matrix` | [x] |
| 24 | `pcre2_compile` + match | `PCRE2_UCP|PCRE2_CASELESS` | `cfg_utf_and_ucp_matrix` | [x] |
| 25 | `pcre2_compile` + match | `PCRE2_UTF|PCRE2_UCP|PCRE2_CASELESS` | `cfg_utf_and_ucp_matrix` | [x] |
| 26 | `pcre2_compile` + match | `PCRE2_UTF|PCRE2_MATCH_INVALID_UTF` (invalid-UTF subjects included) | `cfg_utf_and_ucp_matrix`, `match_utf_subject_errors` | [x] |
| 27 | `pcre2_compile` + match | `PCRE2_UTF|PCRE2_MATCH_INVALID_UTF|PCRE2_UCP` | `cfg_utf_and_ucp_matrix` | [x] |
| 28 | `pcre2_compile` + match | 8 UTF/UCP/CASELESS combinations x each of `EXTRA_CASELESS_RESTRICT`, `ASCII_BSD`, `ASCII_BSS`, `ASCII_BSW`, `ASCII_POSIX`, `ASCII_DIGIT`, all five together, `TURKISH_CASING` (40 rows) | `cfg_utf_and_ucp_matrix` | [x] |
| 29 | `pcre2_compile` + match | 6 newline conventions x 2 BSR conventions x 6 anchoring modes (plain, `MULTILINE`, `DOLLAR_ENDONLY`, both, `FIRSTLINE`, `ALT_CIRCUMFLEX|MULTILINE`) = 72 rows, with CR / LF / CRLF / NEL subjects and `NOTBOL`/`NOTEOL` | `cfg_newline_and_bsr_matrix` | [x] |
| 30 | `pcre2_compile` + match | `pcre2_set_optimize` with each of `NONE`, `FULL`, `AUTO_POSSESS`, `AUTO_POSSESS_OFF`, `DOTSTAR_ANCHOR`, `DOTSTAR_ANCHOR_OFF`, `START_OPTIMIZE`, `START_OPTIMIZE_OFF`, and 3 multi-directive sequences | `cfg_optimize_directives` | [x] |
| 31 | `pcre2_compile` + match | the equivalent compile options `NO_AUTO_POSSESS`, `NO_DOTSTAR_ANCHOR`, `NO_START_OPTIMIZE` and all three together | `cfg_optimize_directives` | [x] |
| 32 | `pcre2_compile` + match | custom character tables from `pcre2_maketables`, alone and with `CASELESS|UCP` | `cfg_custom_character_tables` | [x] |
| 33 | `pcre2_compile` + match | `EXTENDED`, `EXTENDED_MORE`, both | `cfg_extended_and_literal_modes` | [x] |
| 34 | `pcre2_compile` + match | `LITERAL`, `LITERAL|CASELESS`, `LITERAL|NO_START_OPTIMIZE`, `LITERAL|ANCHORED|ENDANCHORED` | `cfg_extended_and_literal_modes` | [x] |
| 35 | `pcre2_compile` + match | `ALT_BSUX`, `ALT_VERBNAMES`, `ALT_CIRCUMFLEX`, `ALT_EXTENDED_CLASS`, `ALLOW_EMPTY_CLASS`, `AUTO_CALLOUT`, `NO_AUTO_CAPTURE`, `DUPNAMES`, `UNGREEDY`, `MATCH_UNSET_BACKREF`, `MATCH_UNSET_BACKREF|DUPNAMES`, `USE_OFFSET_LIMIT` | `cfg_extended_and_literal_modes` | [x] |
| 36 | `pcre2_compile` + match | `ALT_BSUX` together with `EXTRA_ALT_BSUX` | `cfg_extended_and_literal_modes` | [x] |
| 37 | `pcre2_compile` + match | `EXTRA_MATCH_WORD`, `EXTRA_MATCH_LINE`, both x {plain, `CASELESS`, `MULTILINE`, `UTF`} | `cfg_match_word_and_match_line_extra_options` | [x] |
| 38 | `pcre2_compile` | `set_max_varlookbehind` in {0,1,2,3,255,65535} x variable-length lookbehind patterns | `cfg_max_varlookbehind_values`, `err100_max_varlookbehind_exceeded` | [x] |
| 39 | `pcre2_compile` | `set_parens_nest_limit` in {0,1,2,5} and the default 250, at depths 249/250/251/400 | `err19_parentheses_nest_too_deep` | [x] |
| 40 | `pcre2_compile` | `set_max_pattern_length` in {0,1,2,4,5,6} on a 5-unit pattern | `err88_pattern_string_too_long` | [x] |
| 41 | `pcre2_compile` | `set_max_pattern_compiled_length` in {0,1,8,32,1<<20} | `err101_pattern_compiled_size_too_big` | [x] |
| 42 | `pcre2_compile` | `set_compile_recursion_guard` with always-ok / always-fail / depth>3 callbacks, at nesting depths 0/1/4/6 | `err33_recursion_guard_rejects` | [x] |
| 43 | `pcre2_compile` | `pcre2_compile_context_copy` of a context with non-default newline, BSR, extra options, max_varlookbehind, parens limit and optimize directive | `cfg_compile_context_copy_is_faithful` | [x] |
| 44 | `pcre2_match` / `pcre2_dfa_match` | `set_offset_limit` in {0,1,2,3,5,10,`PCRE2_UNSET`} x `USE_OFFSET_LIMIT` present/absent x 6 patterns x 4 subjects | `cfg_offset_limit_and_match_context_limits`, `match_bad_offset_limit` | [x] |
| 45 | `pcre2_match` / `pcre2_dfa_match` | `set_match_limit`, `set_depth_limit`, `set_heap_limit` sweeps on a catastrophic-backtracking pattern | `match_limits_produce_same_error` | [x] |
| 46 | `pcre2_match` | `pcre2_match_context_copy`, and `set_recursion_memory_management` with real and NULL callbacks | `cfg_recursion_memory_management_setter` | [x] |
| 47 | `pcre2_match` / `pcre2_dfa_match` | every match-option bit alone (all 32) plus `UINT32_MAX`, `PARTIAL_SOFT|PARTIAL_HARD` | `match_rejects_unknown_option_bits` | [x] |
| 48 | `pcre2_dfa_match` | workspace counts 20 / 64 / 1000 x `DFA_SHORTEST` on/off x `DFA_RESTART` after a partial match, for every pattern in the pool | `run_subject` (used by all `cfg_*` tests), `dfa_match_workspace_size_errors` | [x] |
| 49 | `pcre2_match_data_create` | `oveccount` 0, 1, 2, 3, 65535, 65536, `UINT32_MAX`; and `create_from_pattern` | `match_data_create_edge_cases`, `run_subject` | [x] |
| 50 | `pcre2_match` + substring accessors | ovector too small for the pattern (sizes 0..6 on a 4-group pattern) | `ovector_too_small_is_reported_identically` | [x] |
| 51 | `pcre2_match` + `pcre2_next_match` | iterating empty matches, `\K` non-progressing matches, exhaustion | `next_match_error_paths`, `cmp_match_state` | [x] |
| 52 | `pcre2_set_callout` + `pcre2_match` / `pcre2_dfa_match` | 8 callout patterns x `AUTO_CALLOUT` on/off x accepting / skipping / aborting callbacks x 6 subjects, every callout-block field compared | `cfg_callout_and_enumerate` | [x] |
| 53 | `pcre2_callout_enumerate` | the same 8 patterns x `AUTO_CALLOUT` on/off, every enumerate-block field compared | `cfg_callout_and_enumerate` | [x] |
| 54 | `pcre2_serialize_encode` / `_decode` / `_get_number_of_codes` | 176 patterns x {plain, `UTF`, `CASELESS`, `MULTILINE|DOTALL`}, round-trip, **cross**-decode (C blob into Rust and vice versa), decode counts 0/1/2/-1, and every one of the first 32 header bytes flipped | `cfg_serialize_roundtrip_then_match`, `serialize_roundtrip_and_truncation` | [x] |
| 55 | `pcre2_match` / `pcre2_dfa_match` | long subjects: lengths 0, 1, 2, 255, 256, 1000, 5000 x 6 fillers (`a`, `ab`, `a\n`, `a\r\n`, U+00E9, `x`) x 10 patterns, random start offsets | `cfg_long_and_pathological_subjects` | [x] |
| 56 | `pcre2_compile` + `pcre2_match` with a custom allocator | counting allocator; the **sequence of allocation sizes** must match for 80 patterns | `cfg_custom_allocator_paths` | [x] |
| 57 | `pcre2_substitute` | 18 patterns x 19 option sets x 18 replacements x 8 randomized subjects, output capacities 0/1/4/16/256, explicit length vs `PCRE2_ZERO_TERMINATED` for both subject and replacement | `substitute_randomized_matrix` | [x] |
| 58 | `pcre2_substitute` | `SUBSTITUTE_GLOBAL`, `_EXTENDED`, `_LITERAL`, `_UNSET_EMPTY`, `_UNKNOWN_UNSET`, `_OVERFLOW_LENGTH`, `_REPLACEMENT_ONLY`, `_MATCHED` and their combinations | `substitute_randomized_matrix`, `substitute_unset_group_handling`, `substitute_output_overflow`, `substitute_matched_mode_consistency_errors` | [x] |
| 59 | `pcre2_set_substitute_callout` + `pcre2_substitute` | accept / skip / abort / alternating callbacks x `GLOBAL` on/off | `substitute_callout_return_values` | [x] |
| 60 | `pcre2_set_substitute_case_callout` + `pcre2_substitute` | failing / identity / growing callbacks x `\U`, `\L`, `\u`, `\l`, plain `$1` x capacities 0/4/128 | `substitute_case_callout_errors` | [x] |
| 61 | `pcre2_pattern_convert` | `CONVERT_GLOB` x {plain, `CONVERT_UTF`, `CONVERT_UTF|NO_UTF_CHECK`} x 40 glob patterns x explicit length and `PCRE2_ZERO_TERMINATED`; the converted pattern is then compiled and its byte code compared | `convert_glob_matrix` | [x] |
| 62 | `pcre2_pattern_convert` | `CONVERT_GLOB_NO_WILD_SEPARATOR`, `CONVERT_GLOB_NO_STARSTAR` (same matrix) | `convert_glob_matrix` | [x] |
| 63 | `pcre2_pattern_convert` | `set_glob_escape` in {0, `\`, `!`, `^`, `/`, 0x100} x `set_glob_separator` in {`/`, `.`, `\`, `:`, 0, 0x100} x 3 glob modes x 40 patterns, plus a copied convert context | `convert_glob_with_custom_escape_and_separator` | [x] |
| 64 | `pcre2_pattern_convert` | `CONVERT_POSIX_BASIC` and `CONVERT_POSIX_EXTENDED` x {plain, `CONVERT_UTF`, `+NO_UTF_CHECK`} x 30 POSIX patterns x both length forms | `convert_posix_matrix` | [x] |
| 65 | `pcre2_pattern_convert` | 40 000 randomized patterns x 8 option sets | `convert_randomized` | [x] |
| 66 | `pcre2_pattern_convert` | long inputs: 100 / 1000 / 5000 units x 6 repeating units x 4 conversion modes | `convert_long_inputs` | [x] |
| 67 | `pcre2_compile` + `pcre2_match` | 60 000 randomized pattern strings x 14 compile-option sets x 11 extra-option sets, each successful compile fully compared and matched against 2 subjects (swept over 4 further seeds at 3 000 000 iterations each → ~12 M patterns, ~8 M of which compiled) | `cfg_random_patterns_fuzz` | [x] |
| 67b | `pcre2_compile` + `pcre2_match` + `pcre2_dfa_match` + `pcre2_substitute` | **grammar-driven** pattern generation (depth 2–4 over 26 leaf and 14 composite productions: groups, named groups, all 7 lookaround forms, alternation, all 14 quantifier forms, inline option groups, conditionals, subroutine calls, extended classes, verbs, callouts, backreferences) x 30 compile-option sets x 12 extra-option sets x 7 newline settings x 3 BSR settings x 6 optimize settings x 12 match-option sets; ~90 % of generated patterns compile. Swept over 40 seeds at 120 000–400 000 iterations each (~14 M generated patterns, ~12 M compiled) | `structured_pattern_fuzz` | [x] |
| 68 | `pcre2_compile` | in-pattern option setters `(*UTF)`, `(*UCP)`, `(*CR)`, `(*LF)`, `(*CRLF)`, `(*ANY)`, `(*ANYCRLF)`, `(*NUL)`, `(*BSR_ANYCRLF)`, `(*BSR_UNICODE)`, `(*LIMIT_MATCH=)`, `(*LIMIT_DEPTH=)`, `(*LIMIT_HEAP=)`, `(*NO_AUTO_POSSESS)`, `(*NO_DOTSTAR_ANCHOR)`, `(*NO_START_OPT)`, `(*NOTEMPTY)`, `(*NOTEMPTY_ATSTART)`, `(*NO_JIT)`, `(*CASELESS)` — present in the pattern pool, therefore crossed with every row 17–37 | `cfg_default` and all `cfg_*` rows | [x] |
| 69 | `pcre2_compile` | inline options `(?i)`, `(?i:...)`, `(?-i)`, `(?x)`, `(?xx)`, `(?s)`, `(?m)`, `(?U)`, `(?J)`, `(?n)` — in the pattern pool, crossed with every row 17–37 | `cfg_default` and all `cfg_*` rows | [x] |
| 70 | `pcre2_compile` | verbs `(*FAIL)`, `(*ACCEPT)`, `(*COMMIT)`, `(*PRUNE)`, `(*SKIP)`, `(*THEN)`, `(*MARK:)`, `(*:)` and their argument forms — in the pattern pool; `pcre2_get_mark` compared after every match | `cfg_default` and all `cfg_*` rows | [x] |
| 71 | `pcre2_compile` | Unicode property classes `\p{L}`, `\P{L}`, `\p{Lu}`, `\p{^Lu}`, `\pL`, `\p{Greek}`, `\p{Any}`, `\p{Xan}`, `\p{Xps}`, `\p{Xsp}`, `\p{Xuc}`, `\p{Xwd}`, `\p{Bidi_Control}`, `\p{ASCII}`, `\p{Cased}` — in the pattern pool | `cfg_default`, `cfg_utf_and_ucp_matrix` | [x] |
| 72 | `pcre2_match` (`_pcre2_xclass`, `_pcre2_eclass`) | 11 wide / property / extended-class patterns x 3 000 randomized code points each (ASCII, Latin-1, BMP, full 21-bit range) | `xclass_and_eclass_via_matching` | [x] |
| 73 | `pcre2_compile` | extended classes `(?[...])` with `&&`, `--`, `||`, `~~`, `!`, and the `PCRE2_ALT_EXTENDED_CLASS` forms | `cfg_default`, `cfg_extended_and_literal_modes`, `err107_eclass_nest_too_deep` | [x] |
| 74 | `pcre2_jit_compile`, `pcre2_jit_match`, `pcre2_jit_stack_create/assign/free`, `pcre2_jit_free_unused_memory`, `_pcre2_jit_get_target`, `_pcre2_jit_get_size`, `_pcre2_jit_free`, `_pcre2_jit_free_rodata` | every JIT option bit, NULL arguments, stack create/free; `PCRE2_INFO_JITSIZE` | `jit_functions_agree` | [x] |
| 75 | `pcre2_substring_nametable_scan`, `pcre2_substring_number_from_name` | unique names, duplicate names (`PCRE2_DUPNAMES`), unknown names, empty name, wrong case, `firstptr == NULL` form | `substring_error_paths`, `substring_duplicate_names` | [x] |
| 76 | `pcre2_code_copy`, `pcre2_code_copy_with_tables` | every pattern in the pool under every row 17–37, byte code compared | `drive()` in `match_diff.rs` | [x] |
