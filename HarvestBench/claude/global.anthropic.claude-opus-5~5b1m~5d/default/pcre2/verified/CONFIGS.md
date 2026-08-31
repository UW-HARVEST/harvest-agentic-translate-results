# CONFIGS.md — Configuration-surface table (valid inputs)

Derived mechanically from `c_src/include/pcre2.h` (public option/mode constants)
and the `if` / `switch` branches the C sources take on those flags.
Build config is fixed by `c_src/CMakeLists.txt`: **8-bit code units,
`SUPPORT_UNICODE` on, `SUPPORT_JIT` off**. The Rust crate has **no cargo
features**, so there is exactly one feature combination (the default) — see
§ "Feature combinations" at the bottom.

Every row is exercised with **many randomized inputs** (fixed seed) in
`translation/tests/`. `[x]` = row passes.

## Axis inventory (what the C actually branches on)

* **Compile options** (`PUBLIC_COMPILE_OPTIONS`): `ANCHORED`, `NO_UTF_CHECK`,
  `ENDANCHORED`, `ALLOW_EMPTY_CLASS`, `ALT_BSUX`, `AUTO_CALLOUT`, `CASELESS`,
  `DOLLAR_ENDONLY`, `DOTALL`, `DUPNAMES`, `EXTENDED`, `FIRSTLINE`,
  `MATCH_UNSET_BACKREF`, `MULTILINE`, `NEVER_UCP`, `NEVER_UTF`,
  `NO_AUTO_CAPTURE`, `NO_AUTO_POSSESS`, `NO_DOTSTAR_ANCHOR`,
  `NO_START_OPTIMIZE`, `UCP`, `UNGREEDY`, `UTF`, `NEVER_BACKSLASH_C`,
  `ALT_CIRCUMFLEX`, `ALT_VERBNAMES`, `USE_OFFSET_LIMIT`, `EXTENDED_MORE`,
  `LITERAL`, `MATCH_INVALID_UTF`, `ALT_EXTENDED_CLASS`.
* **Compile extra options**: `ALLOW_SURROGATE_ESCAPES`, `BAD_ESCAPE_IS_LITERAL`,
  `MATCH_WORD`, `MATCH_LINE`, `ESCAPED_CR_IS_LF`, `ALT_BSUX`,
  `ALLOW_LOOKAROUND_BSK`, `CASELESS_RESTRICT`, `ASCII_BSD`, `ASCII_BSS`,
  `ASCII_BSW`, `ASCII_POSIX`, `ASCII_DIGIT`, `PYTHON_OCTAL`, `NEVER_CALLOUT`,
  `TURKISH_CASING`.
* **Match options** (`PUBLIC_MATCH_OPTIONS`): `ANCHORED`, `ENDANCHORED`,
  `NO_UTF_CHECK`, `NOTBOL`, `NOTEOL`, `NOTEMPTY`, `NOTEMPTY_ATSTART`,
  `PARTIAL_SOFT`, `PARTIAL_HARD`, `NO_JIT`, `COPY_MATCHED_SUBJECT`,
  `DISABLE_RECURSELOOP_CHECK`.
* **DFA-only match options**: `DFA_RESTART`, `DFA_SHORTEST`.
* **Substitute options**: `SUBSTITUTE_GLOBAL`, `EXTENDED`, `UNSET_EMPTY`,
  `UNKNOWN_UNSET`, `OVERFLOW_LENGTH`, `LITERAL`, `MATCHED`,
  `REPLACEMENT_ONLY`.
* **Convert options**: `CONVERT_UTF`, `CONVERT_NO_UTF_CHECK`,
  `CONVERT_POSIX_BASIC`, `CONVERT_POSIX_EXTENDED`, `CONVERT_GLOB`,
  `CONVERT_GLOB_NO_WILD_SEPARATOR`, `CONVERT_GLOB_NO_STARSTAR`.
* **Newline conventions** (`pcre2_set_newline`): `CR`(1), `LF`(2), `CRLF`(3),
  `ANY`(4), `ANYCRLF`(5), `NUL`(6).
* **BSR conventions** (`pcre2_set_bsr`): `UNICODE`(1), `ANYCRLF`(2).
* **Optimization directives** (`pcre2_set_optimize`): `NONE`(0), `FULL`(1),
  `AUTO_POSSESS`(64)/`_OFF`(65), `DOTSTAR_ANCHOR`(66)/`_OFF`(67),
  `START_OPTIMIZE`(68)/`_OFF`(69).
* **Limits**: `match_limit`, `depth_limit`, `heap_limit`, `offset_limit`,
  `max_pattern_length`, `max_pattern_compiled_length`, `parens_nest_limit`,
  `max_varlookbehind`.
* **Character tables**: built-in `_pcre2_default_tables_8` vs.
  `pcre2_maketables()`-generated tables (via `pcre2_set_character_tables`).
* **Input shapes**: empty subject / 1 code unit / many; ASCII vs. UTF-8
  multi-byte (2/3/4-byte) vs. invalid UTF-8; start offset 0 / mid / == length;
  ovector size 0 / 1 / exact / oversized; zero-terminated
  (`PCRE2_ZERO_TERMINATED`) vs. explicit length; `NULL`+len 0.
* **Engines**: `pcre2_match` (interpreter), `pcre2_dfa_match`,
  `pcre2_jit_match` (stub in this build).
* **All 27 `PCRE2_INFO_*` requests** and **all 17 `PCRE2_CONFIG_*` requests**.

## Rows

### A. `pcre2_config` — every request

| # | entry point(s) | configuration (options set + input shape) | [ ] |
|---|----------------|--------------------------------------------|-----|
| 1 | `pcre2_config` | each of `BSR`,`JIT`,`LINKSIZE`,`MATCHLIMIT`,`NEWLINE`,`PARENSLIMIT`,`DEPTHLIMIT`,`STACKRECURSE`,`UNICODE`,`HEAPLIMIT`,`NEVER_BACKSLASH_C`,`COMPILED_WIDTHS`,`TABLES_LENGTH`,`EFFECTIVE_LINKSIZE` with a `uint32_t*` sink | [x] |
| 2 | `pcre2_config` | `UNICODE_VERSION`, `VERSION` with a char buffer — compare returned length AND bytes | [x] |
| 3 | `pcre2_config` | `UNICODE_VERSION`, `VERSION`, `JITTARGET` with `where == NULL` (length query) | [x] |

### B. `pcre2_maketables` / `pcre2_set_character_tables` / default tables

| # | entry point(s) | configuration | [ ] |
|---|----------------|---------------|-----|
| 4 | `pcre2_maketables` | `NULL` gcontext — compare all `TABLES_LENGTH` bytes | [x] |
| 5 | `pcre2_maketables` | custom gcontext with custom malloc/free | [x] |
| 6 | `_pcre2_default_tables_8` | exported table data — byte-identical | [x] |
| 7 | `pcre2_compile` + `pcre2_set_character_tables(maketables())` | compile & match with generated tables | [x] |

### C. Low-level exported helpers (`_pcre2_*`)

| # | entry point(s) | configuration | [ ] |
|---|----------------|---------------|-----|
| 8 | `_pcre2_strlen_8` | empty / 1 / random lengths up to 4 KiB | [x] |
| 9 | `_pcre2_strcmp_8` | equal, prefix, differing at each position, empty | [x] |
| 10 | `_pcre2_strncmp_8` | `n` = 0, < min len, > max len, exact | [x] |
| 11 | `_pcre2_strcmp_c8_8` / `_pcre2_strncmp_c8_8` | vs. C `char*`, all above shapes | [x] |
| 12 | `_pcre2_strcpy_c8_8` | copies of length 0..64, returns length | [x] |
| 13 | `_pcre2_ord2utf_8` | every code point 0..0x10FFFF (sampled + all boundaries) | [x] |
| 14 | `_pcre2_valid_utf_8` | random valid UTF-8; every one of the 21 error classes; `PCRE2_ZERO_TERMINATED` and explicit length | [x] |
| 15 | `_pcre2_ckd_smul_8` | random operands incl. overflow boundaries and `INT64_MIN/MAX` | [x] |
| 16 | `_pcre2_is_newline_8` | 6 newline types × {CR, LF, CRLF, NEL(0x85), LS(0x2028), PS(0x2029), other} × utf on/off | [x] |
| 17 | `_pcre2_was_newline_8` | same cross-product, plus at start of subject | [x] |
| 18 | `_pcre2_extuni_8` | random code points × grapheme-break table | [x] |
| 19 | `_pcre2_script_run_8` | random UTF-8 strings, utf on/off | [x] |
| 20 | `_pcre2_xclass_8` | classes produced from real compiled patterns × random code points | [x] |
| 21 | `_pcre2_find_bracket_8` | compiled code of random patterns, each bracket number, utf on/off | [x] |
| 22 | `_pcre2_study_8` | via `pcre2_compile` (start bitmap / minlength observable through `PCRE2_INFO_*`) | [x] |
| 23 | `_pcre2_auto_possessify_8` | via compile with/without `PCRE2_NO_AUTO_POSSESS` (observable via `PCRE2_INFO_SIZE`) | [x] |
| 24 | `_pcre2_memctl_malloc_8` | with default and custom allocators | [x] |
| 25 | exported data tables | `_pcre2_OP_lengths_8`, `_pcre2_utf8_table1..4` + `_size`, `_pcre2_hspace_list_8`, `_pcre2_vspace_list_8`, `_pcre2_posix_class_maps8`, `_pcre2_callout_start_delims_8`, `_pcre2_callout_end_delims_8`, `_pcre2_unicode_version_8`, `_pcre2_ucp_gbtable_8`, `_pcre2_ucp_gentype_8`, `_pcre2_utt_8`/`_names_8`/`_size_8` — byte-identical | [x] |
| 26 | UCD tables | `_pcre2_ucd_records_8`, `_ucd_stage1_8`, `_ucd_stage2_8`, `_ucd_caseless_sets_8`, `_ucd_digit_sets_8`, `_ucd_script_sets_8`, `_ucd_boolprop_sets_8`, `_ucd_nocase_ranges_8` + `_size_8`, `_ucd_turkish_dotted_i_caseset_8` — byte-identical | [x] |
| 27 | `_pcre2_update_classbits_8`, `_pcre2_eclass_8`, `_pcre2_compile_class_nested_8`, `_pcre2_compile_class_not_nested_8`, `_pcre2_check_escape_8`, `_pcre2_compile_add_name_to_table8`, `_pcre2_compile_find_dupname_details8`, `_pcre2_compile_find_named_group8`, `_pcre2_compile_get_hash_from_name8`, `_pcre2_compile_parse_recurse_args8`, `_pcre2_compile_parse_scan_substr_args8` | exercised indirectly through `pcre2_compile` of patterns covering classes, extended classes, named groups, recursion and escapes; compiled bytecode compared byte-for-byte via `PCRE2_INFO_SIZE` + code copy | [x] |

### D. Context create / copy / free / set

| # | entry point(s) | configuration | [ ] |
|---|----------------|---------------|-----|
| 28 | `pcre2_general_context_create` + `_copy` + `_free` | default and custom malloc/free with user data | [x] |
| 29 | `pcre2_compile_context_create` + `_copy` + `_free` | `NULL` gcontext and custom gcontext | [x] |
| 30 | `pcre2_match_context_create` + `_copy` + `_free` | `NULL` and custom gcontext | [x] |
| 31 | `pcre2_convert_context_create` + `_copy` + `_free` | `NULL` and custom gcontext | [x] |
| 32 | `_pcre2_default_compile_context_8` / `_default_match_context_8` / `_default_convert_context_8` | exported default context contents — field-by-field | [x] |
| 33 | `pcre2_set_bsr` | both valid values, observed through `PCRE2_INFO_BSR` after compile | [x] |
| 34 | `pcre2_set_newline` | all 6 valid values, observed through `PCRE2_INFO_NEWLINE` and matching behaviour of `.`/`$`/`\R` | [x] |
| 35 | `pcre2_set_max_pattern_length` | 0, 1, exact patlen, huge | [x] |
| 36 | `pcre2_set_max_pattern_compiled_length` | 0, small, exact, huge | [x] |
| 37 | `pcre2_set_max_varlookbehind` | 0, 1, 255, default | [x] |
| 38 | `pcre2_set_parens_nest_limit` | 0, 1, 5, default 250 | [x] |
| 39 | `pcre2_set_compile_extra_options` | each of the 16 extra options individually + random combinations | [x] |
| 40 | `pcre2_set_optimize` | `NONE`, `FULL`, and each of 64..69, each verified via compiled-size / match behaviour | [x] |
| 41 | `pcre2_set_heap_limit` / `set_match_limit` / `set_depth_limit` | 0, 1, 10, default, `UINT32_MAX`, observed via `PCRE2_INFO_*LIMIT` and match rc | [x] |
| 42 | `pcre2_set_offset_limit` + `PCRE2_USE_OFFSET_LIMIT` | `PCRE2_UNSET`, 0, mid-subject, == length | [x] |
| 43 | `pcre2_set_glob_separator` | `/`, `\`, `.` × glob conversion | [x] |
| 44 | `pcre2_set_glob_escape` | 0, `\`, and each ASCII punctuation char × glob conversion | [x] |
| 45 | `pcre2_set_compile_recursion_guard` | guard returning 0 always, and returning nonzero at depth N | [x] |
| 46 | `pcre2_set_recursion_limit` / `set_recursion_memory_management` | obsolete APIs, verify rc and effect | [x] |

### E. `pcre2_compile` × option cross-product (compiled bytecode compared)

Compiled output is compared via `PCRE2_INFO_SIZE`, `PCRE2_INFO_FRAMESIZE`, all
other `PCRE2_INFO_*`, and — where reachable — the serialized byte stream from
`pcre2_serialize_encode`, which contains the full bytecode.

| # | entry point(s) | configuration | [ ] |
|---|----------------|---------------|-----|
| 47 | `pcre2_compile` | random patterns, no options, `NULL` ccontext | [x] |
| 48 | `pcre2_compile` | `PCRE2_ZERO_TERMINATED` vs. explicit `patlen`; embedded NUL in pattern | [x] |
| 49 | `pcre2_compile` | `PCRE2_CASELESS` alone; + `PCRE2_UTF`; + `PCRE2_UCP`; + `EXTRA_CASELESS_RESTRICT` | [x] |
| 50 | `pcre2_compile` | `PCRE2_MULTILINE` × all 6 newline conventions | [x] |
| 51 | `pcre2_compile` | `PCRE2_DOTALL` × `PCRE2_MULTILINE` × `DOLLAR_ENDONLY` | [x] |
| 52 | `pcre2_compile` | `PCRE2_EXTENDED` and `PCRE2_EXTENDED_MORE` on whitespace-laden patterns | [x] |
| 53 | `pcre2_compile` | `PCRE2_UNGREEDY` on quantifier-heavy patterns | [x] |
| 54 | `pcre2_compile` | `PCRE2_NO_AUTO_CAPTURE` on patterns with groups | [x] |
| 55 | `pcre2_compile` | `PCRE2_DUPNAMES` with duplicate group names | [x] |
| 56 | `pcre2_compile` | `PCRE2_UTF` on ASCII and multi-byte patterns | [x] |
| 57 | `pcre2_compile` | `PCRE2_UTF` + `PCRE2_NO_UTF_CHECK` on invalid-UTF patterns | [x] |
| 58 | `pcre2_compile` | `PCRE2_UCP` with `\d \w \s \b` and `\p{...}` | [x] |
| 59 | `pcre2_compile` | `PCRE2_UCP` + each of `EXTRA_ASCII_BSD/BSS/BSW/POSIX/DIGIT` | [x] |
| 60 | `pcre2_compile` | `PCRE2_MATCH_INVALID_UTF` (implies UTF) | [x] |
| 61 | `pcre2_compile` | `PCRE2_LITERAL` alone and with the allowed literal options only | [x] |
| 62 | `pcre2_compile` | `PCRE2_ANCHORED` / `PCRE2_ENDANCHORED` / both | [x] |
| 63 | `pcre2_compile` | `PCRE2_FIRSTLINE` × newline conventions | [x] |
| 64 | `pcre2_compile` | `PCRE2_ALT_CIRCUMFLEX` + `MULTILINE` | [x] |
| 65 | `pcre2_compile` | `PCRE2_ALT_BSUX` and `EXTRA_ALT_BSUX` with `\u`,`\x`,`\U` | [x] |
| 66 | `pcre2_compile` | `PCRE2_ALT_VERBNAMES` with `(*MARK:...)` verbs | [x] |
| 67 | `pcre2_compile` | `PCRE2_ALLOW_EMPTY_CLASS` with `"[]"` and `"[^]"` | [x] |
| 68 | `pcre2_compile` | `PCRE2_ALT_EXTENDED_CLASS` with `&&`, `--`, `\|\|`, `~~`, nesting | [x] |
| 69 | `pcre2_compile` | `PCRE2_AUTO_CALLOUT` on all pattern shapes | [x] |
| 70 | `pcre2_compile` | `PCRE2_NO_AUTO_POSSESS` / `NO_DOTSTAR_ANCHOR` / `NO_START_OPTIMIZE` each on/off | [x] |
| 71 | `pcre2_compile` | `PCRE2_MATCH_UNSET_BACKREF` with back references | [x] |
| 72 | `pcre2_compile` | `EXTRA_MATCH_WORD` and `EXTRA_MATCH_LINE` (mutually and with LITERAL) | [x] |
| 73 | `pcre2_compile` | `EXTRA_BAD_ESCAPE_IS_LITERAL` with unknown escapes | [x] |
| 74 | `pcre2_compile` | `EXTRA_ESCAPED_CR_IS_LF` with `\r` | [x] |
| 75 | `pcre2_compile` | `EXTRA_ALLOW_SURROGATE_ESCAPES` with `\x{d800}` (UTF and non-UTF) | [x] |
| 76 | `pcre2_compile` | `EXTRA_ALLOW_LOOKAROUND_BSK` with `(?<=\K)` | [x] |
| 77 | `pcre2_compile` | `EXTRA_PYTHON_OCTAL` with `\0`, `\12`, `\377` | [x] |
| 78 | `pcre2_compile` | `EXTRA_TURKISH_CASING` + `PCRE2_UTF` (+`CASELESS`) | [x] |
| 79 | `pcre2_compile` | `PCRE2_NEVER_UTF` / `NEVER_UCP` / `NEVER_BACKSLASH_C` with benign patterns | [x] |
| 80 | `pcre2_compile` | `EXTRA_NEVER_CALLOUT` with a callout-free pattern | [x] |
| 81 | `pcre2_compile` | inline start-of-pattern directives `(*UTF)`, `(*UCP)`, `(*CR)`,`(*LF)`,`(*CRLF)`,`(*ANY)`,`(*ANYCRLF)`,`(*NUL)`,`(*BSR_UNICODE)`,`(*BSR_ANYCRLF)`,`(*LIMIT_MATCH=n)`,`(*LIMIT_DEPTH=n)`,`(*LIMIT_HEAP=n)`,`(*NO_AUTO_POSSESS)`,`(*NO_START_OPT)`,`(*NOTEMPTY)`,`(*NOTEMPTY_ATSTART)`,`(*NO_DOTSTAR_ANCHOR)`,`(*NO_JIT)` | [x] |
| 82 | `pcre2_compile` | randomized *valid* pattern generator: literals, classes, POSIX classes, quantifiers (greedy/lazy/possessive/interval), alternation, groups (capturing, non-capturing, atomic, named, dup-named, conditional, `(?\|`), back references, recursion `(?R)`/`(?1)`/`(?&name)`, lookaheads/lookbehinds (fixed & variable), `\b \B \A \Z \z \G \K \R \X \C \h \H \v \V \N`, `\p{...}`/`\P{...}` across all script/category names in `_pcre2_utt_8`, callouts, verbs — **500+ patterns per option combo** | [x] |
| 83 | `pcre2_code_copy` / `pcre2_code_copy_with_tables` | copy then match/query; copies of deserialized codes | [x] |

### F. `pcre2_pattern_info` — every request × pattern shape

| # | entry point(s) | configuration | [ ] |
|---|----------------|---------------|-----|
| 84 | `pcre2_pattern_info` | all 27 `PCRE2_INFO_*` requests × each randomized compiled pattern | [x] |
| 85 | `pcre2_pattern_info` | `PCRE2_INFO_FIRSTBITMAP` — compare the 32-byte bitmap when non-NULL | [x] |
| 86 | `pcre2_pattern_info` | `PCRE2_INFO_NAMETABLE` + `NAMECOUNT` + `NAMEENTRYSIZE` — compare whole table bytes | [x] |
| 87 | `pcre2_pattern_info` | `PCRE2_INFO_ALLOPTIONS`/`ARGOPTIONS`/`EXTRAOPTIONS` after inline directives | [x] |
| 88 | `pcre2_pattern_info` | `MATCHLIMIT`/`DEPTHLIMIT`/`HEAPLIMIT` set by `(*LIMIT_*)` in pattern | [x] |
| 89 | `pcre2_callout_enumerate` | patterns with numeric callouts, string callouts, `AUTO_CALLOUT` — compare each callout block field | [x] |

### G. `pcre2_match_data_*`

| # | entry point(s) | configuration | [ ] |
|---|----------------|---------------|-----|
| 90 | `pcre2_match_data_create` | `ovecsize` 0,1,2,16,1000 × `NULL`/custom gcontext | [x] |
| 91 | `pcre2_match_data_create_from_pattern` | each compiled pattern × `NULL`/custom gcontext | [x] |
| 92 | `pcre2_get_ovector_count` / `pcre2_get_ovector_pointer` | after success, after `NOMATCH`, after `PARTIAL` — full ovector contents | [x] |
| 93 | `pcre2_get_startchar` / `pcre2_get_mark` | after success/NOMATCH/PARTIAL, patterns with `(*MARK)` | [x] |
| 94 | `pcre2_get_match_data_size` / `pcre2_get_match_data_heapframes_size` | before and after matching | [x] |

### H. `pcre2_match` (interpreter) × option cross-product

| # | entry point(s) | configuration | [ ] |
|---|----------------|---------------|-----|
| 95 | `pcre2_match` | random pattern × random subject, no options — compare rc + full ovector + startchar + mark | [x] |
| 96 | `pcre2_match` | subject shapes: empty, 1 unit, long (4 KiB), `NULL`+len 0, `PCRE2_ZERO_TERMINATED` | [x] |
| 97 | `pcre2_match` | `start_offset` = 0, 1, mid, `length` | [x] |
| 98 | `pcre2_match` | `PCRE2_NOTBOL` / `NOTEOL` / both × `MULTILINE` patterns | [x] |
| 99 | `pcre2_match` | `PCRE2_NOTEMPTY` / `NOTEMPTY_ATSTART` on empty-matching patterns | [x] |
| 100 | `pcre2_match` | `PCRE2_ANCHORED` / `ENDANCHORED` / both at match time | [x] |
| 101 | `pcre2_match` | `PCRE2_PARTIAL_SOFT` and `PARTIAL_HARD` × truncated subjects | [x] |
| 102 | `pcre2_match` | `PCRE2_COPY_MATCHED_SUBJECT` | [x] |
| 103 | `pcre2_match` | `PCRE2_DISABLE_RECURSELOOP_CHECK` on recursive patterns | [x] |
| 104 | `pcre2_match` | `PCRE2_NO_JIT` (no-op in this build) | [x] |
| 105 | `pcre2_match` | `PCRE2_UTF` pattern × valid multi-byte subjects (2/3/4-byte sequences) | [x] |
| 106 | `pcre2_match` | `PCRE2_UTF` + `PCRE2_NO_UTF_CHECK` × invalid subjects | [x] |
| 107 | `pcre2_match` | `PCRE2_MATCH_INVALID_UTF` × subjects with invalid sequences at various offsets | [x] |
| 108 | `pcre2_match` | ovector too small (size 1) so groups spill — compare rc == 0 path | [x] |
| 109 | `pcre2_match` | match context with callout that returns 0 / 1 / -1 / `PCRE2_ERROR_NOMATCH`; compare every callout block field and the call sequence | [x] |
| 110 | `pcre2_match` | `USE_OFFSET_LIMIT` + `offset_limit` 0..length | [x] |
| 111 | `pcre2_match` | patterns with `\K`, `(*ACCEPT)`, `(*FAIL)`, `(*COMMIT)`, `(*PRUNE)`, `(*SKIP)`, `(*THEN)` (with and without names) | [x] |
| 112 | `pcre2_match` | patterns with recursion / subroutine calls / conditional groups / possessive quantifiers / atomic groups | [x] |
| 113 | `pcre2_match` | back references, incl. caseless and `MATCH_UNSET_BACKREF` | [x] |
| 114 | `pcre2_match` | `\R` × both BSR conventions × all 6 newline conventions × subjects containing CR/LF/CRLF/NEL/LS/PS | [x] |
| 115 | `pcre2_match` | `\X` (extended grapheme cluster) on combining/emoji/regional-indicator sequences | [x] |
| 116 | `pcre2_match` | `(*script_run:...)` / `(*sr:...)` patterns on mixed-script subjects | [x] |
| 117 | `pcre2_match` | repeated calls advancing over a subject with `pcre2_next_match` (global-match loop) | [x] |
| 118 | `pcre2_match` | limits set just at the boundary so some runs succeed and some fail | [x] |

### I. `pcre2_dfa_match` × option cross-product

| # | entry point(s) | configuration | [ ] |
|---|----------------|---------------|-----|
| 119 | `pcre2_dfa_match` | random pattern × subject, `wscount` = 20, 100, 1000 — compare rc + ovector | [x] |
| 120 | `pcre2_dfa_match` | `PCRE2_DFA_SHORTEST` | [x] |
| 121 | `pcre2_dfa_match` | `PCRE2_DFA_RESTART` continuing a `PARTIAL` result with the same workspace | [x] |
| 122 | `pcre2_dfa_match` | `PARTIAL_SOFT` / `PARTIAL_HARD` | [x] |
| 123 | `pcre2_dfa_match` | `NOTBOL`/`NOTEOL`/`NOTEMPTY`/`NOTEMPTY_ATSTART`/`ANCHORED`/`ENDANCHORED` | [x] |
| 124 | `pcre2_dfa_match` | UTF patterns + valid/invalid subjects, `NO_UTF_CHECK` on/off | [x] |
| 125 | `pcre2_dfa_match` | ovector sizes 0,1,2,16 — DFA returns multiple match lengths | [x] |
| 126 | `pcre2_dfa_match` | callouts, `(*MARK)` retrieval, `USE_OFFSET_LIMIT` | [x] |
| 127 | `pcre2_dfa_match` | all 6 newline conventions × `\R` × BSR | [x] |
| 128 | `pcre2_dfa_match` | `COPY_MATCHED_SUBJECT` | [x] |

### J. `pcre2_jit_*` (stubs in this non-JIT build)

| # | entry point(s) | configuration | [ ] |
|---|----------------|---------------|-----|
| 129 | `pcre2_jit_compile` | `PCRE2_JIT_COMPLETE`, `PARTIAL_SOFT`, `PARTIAL_HARD`, `INVALID_UTF`, `TEST_ALLOC`, combinations, 0 | [x] |
| 130 | `pcre2_jit_match` | after a successful/failed `pcre2_jit_compile` | [x] |
| 131 | `pcre2_jit_stack_create` / `_assign` / `_free` | various sizes; assign with and without a callback | [x] |
| 132 | `pcre2_jit_free_unused_memory` | `NULL` and real gcontext | [x] |
| 133 | `_pcre2_jit_get_size_8` / `_pcre2_jit_get_target_8` / `_pcre2_jit_free_8` / `_pcre2_jit_free_rodata_8` | on codes with/without JIT data | [x] |
| 134 | `pcre2_pattern_info` | `PCRE2_INFO_JITSIZE` after `pcre2_jit_compile` | [x] |

### K. `pcre2_substring_*`

| # | entry point(s) | configuration | [ ] |
|---|----------------|---------------|-----|
| 135 | `pcre2_substring_length_bynumber` | group 0..count, ovector exact/small, `sizeptr` NULL and non-NULL | [x] |
| 136 | `pcre2_substring_copy_bynumber` | exact-size, oversized and undersized buffers; compare buffer bytes + `*sizeptr` | [x] |
| 137 | `pcre2_substring_get_bynumber` | compare returned bytes and length, then `pcre2_substring_free` | [x] |
| 138 | `pcre2_substring_number_from_name` | existing, non-existing, duplicate names | [x] |
| 139 | `pcre2_substring_length_byname` / `copy_byname` / `get_byname` | unique and duplicate names, set and unset groups | [x] |
| 140 | `pcre2_substring_nametable_scan` | with and without `first`/`last` out params; dup names | [x] |
| 141 | `pcre2_substring_list_get` | compare the whole list (all strings + lengths), with and without `lengthsptr` | [x] |
| 142 | `pcre2_substring_*` | after a DFA match (where allowed) and after a partial match | [x] |

### L. `pcre2_substitute`

| # | entry point(s) | configuration | [ ] |
|---|----------------|---------------|-----|
| 143 | `pcre2_substitute` | no options: random pattern/subject/replacement — compare rc, output bytes, `*blength` | [x] |
| 144 | `pcre2_substitute` | `SUBSTITUTE_GLOBAL` | [x] |
| 145 | `pcre2_substitute` | `SUBSTITUTE_LITERAL` | [x] |
| 146 | `pcre2_substitute` | `SUBSTITUTE_EXTENDED` with `\U \L \u \l \E`, `${n:-default}`, `${n:+yes:no}`, `\a \e \f \n \r \t \0 \x{}` | [x] |
| 147 | `pcre2_substitute` | `SUBSTITUTE_UNSET_EMPTY` × unset groups | [x] |
| 148 | `pcre2_substitute` | `SUBSTITUTE_UNKNOWN_UNSET` × unknown names | [x] |
| 149 | `pcre2_substitute` | `SUBSTITUTE_OVERFLOW_LENGTH` × undersized buffer (verify required length) | [x] |
| 150 | `pcre2_substitute` | `SUBSTITUTE_REPLACEMENT_ONLY` (alone and with GLOBAL) | [x] |
| 151 | `pcre2_substitute` | `SUBSTITUTE_MATCHED` with a valid pre-existing match | [x] |
| 152 | `pcre2_substitute` | all valid combinations of GLOBAL × EXTENDED × REPLACEMENT_ONLY × OVERFLOW_LENGTH × UNSET_EMPTY | [x] |
| 153 | `pcre2_substitute` | `$1`, `${1}`, `$name`, `${name}`, `$0`, `$$`, `$` forms | [x] |
| 154 | `pcre2_substitute` | UTF patterns/subjects/replacements, `NO_UTF_CHECK` on/off | [x] |
| 155 | `pcre2_substitute` | `pcre2_set_substitute_callout` — verify every callout block field and the rc effect (0 / 1 / negative) | [x] |
| 156 | `pcre2_substitute` | `pcre2_set_substitute_case_callout` with `CASE_LOWER`/`UPPER`/`TITLE_FIRST` | [x] |
| 157 | `pcre2_substitute` | `start_offset` 0/mid/length; `PCRE2_ZERO_TERMINATED` subject and replacement | [x] |
| 158 | `pcre2_substitute` | empty replacement, empty subject, replacement longer than subject | [x] |

### M. `pcre2_serialize_*`

| # | entry point(s) | configuration | [ ] |
|---|----------------|---------------|-----|
| 159 | `pcre2_serialize_encode` | 1 code, N codes (2,3,10) — compare the whole serialized byte stream | [x] |
| 160 | `pcre2_serialize_encode` | codes compiled with all sorts of options (UTF, UCP, named groups, classes) | [x] |
| 161 | `pcre2_serialize_encode` | with `NULL` and custom gcontext | [x] |
| 162 | `pcre2_serialize_decode` | round-trip, then match with the decoded codes; `number_of_codes` < available | [x] |
| 163 | `pcre2_serialize_decode` | decode C-produced bytes with Rust and vice-versa (cross-compat) | [x] |
| 164 | `pcre2_serialize_get_number_of_codes` | valid stream | [x] |
| 165 | `pcre2_serialize_free` | valid stream and `NULL` | [x] |

### N. `pcre2_pattern_convert`

| # | entry point(s) | configuration | [ ] |
|---|----------------|---------------|-----|
| 166 | `pcre2_pattern_convert` | `CONVERT_POSIX_BASIC` × random BRE patterns — compare output bytes + `*blength` | [x] |
| 167 | `pcre2_pattern_convert` | `CONVERT_POSIX_EXTENDED` × random ERE patterns | [x] |
| 168 | `pcre2_pattern_convert` | `CONVERT_GLOB` × random globs | [x] |
| 169 | `pcre2_pattern_convert` | `CONVERT_GLOB_NO_WILD_SEPARATOR` | [x] |
| 170 | `pcre2_pattern_convert` | `CONVERT_GLOB_NO_STARSTAR` | [x] |
| 171 | `pcre2_pattern_convert` | each of the above + `CONVERT_UTF` and + `CONVERT_NO_UTF_CHECK` | [x] |
| 172 | `pcre2_pattern_convert` | each of the above × `glob_separator` ∈ {`/`,`\`,`.`} × `glob_escape` ∈ {0,`\`,`!`} | [x] |
| 173 | `pcre2_pattern_convert` | `PCRE2_ZERO_TERMINATED` vs. explicit length; empty pattern | [x] |
| 174 | `pcre2_pattern_convert` | with `bufflenptr == NULL` (no length wanted) and non-NULL | [x] |
| 175 | `pcre2_pattern_convert` | converted output then fed to `pcre2_compile` + `pcre2_match` | [x] |
| 176 | `pcre2_converted_pattern_free` | valid buffer and `NULL` | [x] |

### O. `pcre2_get_error_message` / `pcre2_next_match`

| # | entry point(s) | configuration | [ ] |
|---|----------------|---------------|-----|
| 177 | `pcre2_get_error_message` | every error code in 1..220 and -1..-76 with an ample buffer — compare bytes and rc | [x] |
| 178 | `pcre2_get_error_message` | buffer sizes 1..len+1 for a few codes (truncation behaviour) | [x] |
| 179 | `pcre2_next_match` | after non-empty match, empty match, at end of subject, after NOMATCH/PARTIAL — compare return, `*pstart_offset`, `*poptions` | [x] |
| 180 | `pcre2_next_match` | full global-match loop over a subject, UTF and non-UTF, CRLF newline convention | [x] |


---

## Coverage map (which test file proves which section)

| CONFIGS.md section | rows | test file |
|---|---|---|
| A · `pcre2_config` | 1-3 | `tests/t09_context_config.rs` |
| B · character tables | 4-7 | `tests/t01_lowlevel.rs`, `tests/t02_compile.rs`, `tests/t09_context_config.rs` |
| C · low-level `_pcre2_*` | 8-27 | `tests/t01_lowlevel.rs`, `tests/t11_internal.rs` |
| D · contexts and setters | 28-46 | `tests/t09_context_config.rs`, `tests/t02_compile.rs` |
| E · `pcre2_compile` × options | 47-83 | `tests/t02_compile.rs`, `tests/t11_internal.rs` |
| F · `pcre2_pattern_info` | 84-89 | `tests/t02_compile.rs` |
| G · `pcre2_match_data_*` | 90-94 | `tests/t09_context_config.rs`, `tests/t03_match.rs` |
| H · `pcre2_match` | 95-118 | `tests/t03_match.rs` |
| I · `pcre2_dfa_match` | 119-128 | `tests/t04_dfa.rs` |
| J · `pcre2_jit_*` | 129-134 | `tests/t07_serialize_jit.rs`, `tests/t11_internal.rs` |
| K · `pcre2_substring_*` | 135-142 | `tests/t06_substring.rs` |
| L · `pcre2_substitute` | 143-158 | `tests/t05_substitute.rs` |
| M · `pcre2_serialize_*` | 159-165 | `tests/t07_serialize_jit.rs` |
| N · `pcre2_pattern_convert` | 166-176 | `tests/t08_convert.rs` |
| O · error messages / `next_match` | 177-180 | `tests/t09_context_config.rs`, `tests/t10_compile_errors.rs`, `tests/t06_substring.rs` |

## What "identical" means for each entry point

The differential logs compare, byte for byte:

* **compile** — `pcre2_compile`'s return-null-ness, `*errorptr`, `*erroroffset`,
  all 27 `PCRE2_INFO_*` values (including the 32-byte first-code-unit bitmap and
  the complete name table), and the **entire serialized bytecode** produced by
  `pcre2_serialize_encode`, which is the compiled program itself. A byte-equal
  serialization means the two compilers emit identical opcodes.
* **match / dfa_match** — the return code, `pcre2_get_ovector_count`, the
  *defined* prefix of the ovector, `pcre2_get_startchar`, `pcre2_get_mark`'s
  pointee bytes, `pcre2_get_match_data_size` and
  `pcre2_get_match_data_heapframes_size` (so even the internal heap-frame growth
  must agree), plus every field of every callout block and the callout call order.
* **substitute** — the return code, the output buffer bytes, `*blength`, and
  every field of every substitute-callout block.
* **convert** — the return code, the converted pattern bytes and `*blength`, in
  all three buffer modes, plus the match behaviour of the converted pattern.
* **serialize** — the whole byte stream, in both directions (a stream produced by
  C decodes in Rust and vice versa, and the decoded codes behave identically).
* **data tables** — every exported read-only table, at the exact byte length
  recorded in the C `.so`'s ELF symbol table.

## Feature combinations

`translation/Cargo.toml` declares **no `[features]` section**, hence exactly one
combination exists. `./run_verification.sh` enumerates the feature power set
mechanically from `Cargo.toml`, so it will widen automatically if features are
ever added; today it degenerates to the two equivalent no-feature runs below.

| combo | command | [x] |
|---|---|---|
| default (= only) | `cargo test --offline --release` | [x] |
| explicit no-default | `cargo test --offline --release --no-default-features` (identical) | [x] |
