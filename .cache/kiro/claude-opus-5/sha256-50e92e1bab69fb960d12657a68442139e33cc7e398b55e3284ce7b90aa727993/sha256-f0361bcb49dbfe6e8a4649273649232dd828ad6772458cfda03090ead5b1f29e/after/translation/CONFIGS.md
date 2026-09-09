# CONFIGS.md — Configuration-surface table (valid inputs)

Axes derived mechanically from `c_src/include/pcre2.h` (every public option /
mode / info / config constant) and from the `if` / `switch` branches the C takes
on them (`grep -n "options & PCRE2_" c_src/src/*.c`,
`grep -n "optim_flags\|xoptions\|newline_convention\|bsr_convention"`).

Build config: `PCRE2_CODE_UNIT_WIDTH=8`, `SUPPORT_UNICODE`, **no** `SUPPORT_JIT`.
The Rust crate has **no cargo features** (`cargo metadata` → `features: {}`), so
there is exactly one feature combination: the default. Verified in Phase D.

Comparison method per row: call the same sequence on the C `.so` and the Rust
`.so` through `libloading` and compare **byte-for-byte**:
* `pcre2_compile` → `pcre2_serialize_encode` byte stream of the compiled code
  (pointers are zeroed by the C encoder, so it is a deterministic image of the
  whole compiled pattern: opcodes, flags, first/req code unit, start bitmap,
  name table, limits, `blocksize`, …), plus all 27 `pcre2_pattern_info` values.
* match/dfa → return code, full ovector, `startchar`, `mark`, `match_data` sizes.
* substitute/convert → return code, output length, output bytes.

### Test files

| file | rows |
|------|------|
| `tests/a_symbols.rs` | 16–18 (data symbols, default contexts, `pcre2_config`, `maketables`, private JIT helpers) |
| `tests/b_lowlevel.rs` | 1–15 (string helpers, `ord2utf`, `valid_utf`, `ckd_smul`, newline, `extuni`, `script_run`, `memctl_malloc`) |
| `tests/c_compile.rs` | 23–82, 150 (`pcre2_compile` under every option / context, `code_copy`, `callout_enumerate`) |
| `tests/d_match.rs` | 83–114, 149, 159 (`pcre2_match`, `pcre2_dfa_match`, callouts, marks, limits, JIT stubs) |
| `tests/e_substring.rs` | 19–22, 115–122, 147–148, 157 (substrings, serialization, `get_error_message`, contexts) |
| `tests/f_substitute.rs` | 123–135 (`pcre2_substitute`, substitute callout, case callout) |
| `tests/g_convert.rs` | 136–146 (`pcre2_pattern_convert`) |
| `tests/h_err_compile.rs` | ERRORS.md Part 2 (compile error codes) |
| `tests/i_err_api.rs` | ERRORS.md Part 1 + Part 3, plus row 158 |
| `tests/j_internal_helpers.rs` | 14, 79, 105, 150–155 (internal compiler/matcher helpers) |

Every property-style loop is scaled by the `SOAK` environment variable
(`SOAK=25 ./run_tests.sh` etc.) so the same rows can be re-run as a long soak.
The results recorded here were confirmed at `SOAK=1` and again under a soak of
`SOAK=100` for the compile / substitute / convert randomized rows and `SOAK=20`
for the match rows (several million additional randomized calls per row).

**Build note:** `cargo test` does NOT refresh the `cdylib` artifact that the
tests `dlopen`, so `./run_tests.sh` runs `cargo build --release` first. Running
`cargo test` directly can silently test a stale `.so`.

Randomization: `tests/common/mod.rs` has a fixed-seed xorshift PRNG
(`seed = 0x2024_1105_ABCD_EF01`); each row runs over many generated
patterns/subjects (pattern generator emits regex fragments from a token pool;
subject generator emits ASCII, Latin-1, valid-UTF-8 and invalid-UTF-8 bytes).

| # | entry point(s) | configuration (options set + input shape) | [x] |
|---|----------------|--------------------------------------------|-----|
| 1 | `_pcre2_strlen_8` | random byte strings, lengths 0/1/2/…/4096 | [x] |
| 2 | `_pcre2_strcmp_8` | equal, prefix, differing-at-i pairs | [x] |
| 3 | `_pcre2_strcmp_c8_8` | PCRE2 string vs C string, all orderings | [x] |
| 4 | `_pcre2_strncmp_8` | n = 0, < len, == len, > len | [x] |
| 5 | `_pcre2_strncmp_c8_8` | n = 0, < len, == len, > len | [x] |
| 6 | `_pcre2_strcpy_c8_8` | lengths 0..64, resulting buffer compared | [x] |
| 7 | `_pcre2_ord2utf_8` | every code point 0..=0x10FFFF (bytes + length) | [x] |
| 8 | `_pcre2_valid_utf_8` | all-valid strings; every UTF8_ERR1..21 shape; random bytes | [x] |
| 9 | `_pcre2_ckd_smul_8` | random `PCRE2_SIZE` pairs incl. overflow boundary | [x] |
| 10 | `_pcre2_is_newline_8` | each `NLTYPE`/newline convention × CR/LF/CRLF/NEL/LS/PS/other | [x] |
| 11 | `_pcre2_was_newline_8` | each newline convention × same char set, at buffer start | [x] |
| 12 | `_pcre2_extuni_8` | random code points × UTF on/off × `endsubject` boundary | [x] |
| 13 | `_pcre2_script_run_8` | random UTF-8 / Latin-1 runs × utf on/off | [x] |
| 14 | `_pcre2_find_bracket_8` | compiled code from many patterns, every bracket number, utf on/off | [x] |
| 15 | `_pcre2_memctl_malloc_8` | sizes 0/1/16/4096; default memctl | [x] |
| 16 | `_pcre2_default_tables_8` / `pcre2_maketables` | full 1088-byte table compare | [x] |
| 17 | data exports | byte-for-byte compare of all 31 exported data symbols (sizes from `nm -S`) | [x] |
| 18 | `pcre2_config` | every `PCRE2_CONFIG_*` 0..=16, `where=NULL` and non-NULL | [x] |
| 19 | `pcre2_general_context_create/copy/free` | default allocator, custom allocator | [x] |
| 20 | `pcre2_compile_context_create/copy/free` | NULL gcontext, custom gcontext; field defaults compared via compile output | [x] |
| 21 | `pcre2_match_context_create/copy/free` | NULL gcontext, custom gcontext | [x] |
| 22 | `pcre2_convert_context_create/copy/free` | NULL gcontext, custom gcontext | [x] |
| 23 | `pcre2_compile` | no options, no ccontext (NULL) — baseline, random patterns | [x] |
| 24 | `pcre2_compile` | `PCRE2_ANCHORED` | [x] |
| 25 | `pcre2_compile` | `PCRE2_ENDANCHORED` | [x] |
| 26 | `pcre2_compile` | `PCRE2_ALLOW_EMPTY_CLASS` | [x] |
| 27 | `pcre2_compile` | `PCRE2_ALT_BSUX` | [x] |
| 28 | `pcre2_compile` | `PCRE2_AUTO_CALLOUT` | [x] |
| 29 | `pcre2_compile` | `PCRE2_CASELESS` | [x] |
| 30 | `pcre2_compile` | `PCRE2_DOLLAR_ENDONLY` | [x] |
| 31 | `pcre2_compile` | `PCRE2_DOTALL` | [x] |
| 32 | `pcre2_compile` | `PCRE2_DUPNAMES` | [x] |
| 33 | `pcre2_compile` | `PCRE2_EXTENDED` | [x] |
| 34 | `pcre2_compile` | `PCRE2_EXTENDED_MORE` | [x] |
| 35 | `pcre2_compile` | `PCRE2_FIRSTLINE` | [x] |
| 36 | `pcre2_compile` | `PCRE2_LITERAL` | [x] |
| 37 | `pcre2_compile` | `PCRE2_MATCH_UNSET_BACKREF` | [x] |
| 38 | `pcre2_compile` | `PCRE2_MULTILINE` | [x] |
| 39 | `pcre2_compile` | `PCRE2_NO_AUTO_CAPTURE` | [x] |
| 40 | `pcre2_compile` | `PCRE2_NO_AUTO_POSSESS` | [x] |
| 41 | `pcre2_compile` | `PCRE2_NO_DOTSTAR_ANCHOR` | [x] |
| 42 | `pcre2_compile` | `PCRE2_NO_START_OPTIMIZE` | [x] |
| 43 | `pcre2_compile` | `PCRE2_UCP` | [x] |
| 44 | `pcre2_compile` | `PCRE2_UNGREEDY` | [x] |
| 45 | `pcre2_compile` | `PCRE2_UTF` | [x] |
| 46 | `pcre2_compile` | `PCRE2_UTF|PCRE2_UCP` | [x] |
| 47 | `pcre2_compile` | `PCRE2_UTF|PCRE2_NO_UTF_CHECK` | [x] |
| 48 | `pcre2_compile` | `PCRE2_MATCH_INVALID_UTF` (implies UTF); matching with it is compared on valid-UTF subjects — combined with an invalid-UTF subject the C reads out of bounds (see ERRORS.md "Not comparable") | [x] |
| 49 | `pcre2_compile` | `PCRE2_ALT_CIRCUMFLEX` | [x] |
| 50 | `pcre2_compile` | `PCRE2_ALT_VERBNAMES` | [x] |
| 51 | `pcre2_compile` | `PCRE2_ALT_EXTENDED_CLASS` | [x] |
| 52 | `pcre2_compile` | `PCRE2_USE_OFFSET_LIMIT` | [x] |
| 53 | `pcre2_compile` | `PCRE2_NEVER_UCP` / `NEVER_UTF` / `NEVER_BACKSLASH_C` | [x] |
| 54 | `pcre2_compile` | random 2-way and 3-way combinations of the above compile options (fixed seed) | [x] |
| 55 | `pcre2_compile` + `set_compile_extra_options` | `PCRE2_EXTRA_ALLOW_SURROGATE_ESCAPES` | [x] |
| 56 | `pcre2_compile` + extra | `PCRE2_EXTRA_BAD_ESCAPE_IS_LITERAL` | [x] |
| 57 | `pcre2_compile` + extra | `PCRE2_EXTRA_MATCH_WORD` | [x] |
| 58 | `pcre2_compile` + extra | `PCRE2_EXTRA_MATCH_LINE` | [x] |
| 59 | `pcre2_compile` + extra | `PCRE2_EXTRA_ESCAPED_CR_IS_LF` | [x] |
| 60 | `pcre2_compile` + extra | `PCRE2_EXTRA_ALT_BSUX` | [x] |
| 61 | `pcre2_compile` + extra | `PCRE2_EXTRA_ALLOW_LOOKAROUND_BSK` | [x] |
| 62 | `pcre2_compile` + extra | `PCRE2_EXTRA_CASELESS_RESTRICT` | [x] |
| 63 | `pcre2_compile` + extra | `PCRE2_EXTRA_ASCII_BSD` / `BSS` / `BSW` / `POSIX` / `DIGIT` | [x] |
| 64 | `pcre2_compile` + extra | `PCRE2_EXTRA_PYTHON_OCTAL` | [x] |
| 65 | `pcre2_compile` + extra | `PCRE2_EXTRA_NO_BS0` | [x] |
| 66 | `pcre2_compile` + extra | `PCRE2_EXTRA_NEVER_CALLOUT` | [x] |
| 67 | `pcre2_compile` + extra | `PCRE2_EXTRA_TURKISH_CASING` (+ `PCRE2_CASELESS`, + `UTF`) | [x] |
| 68 | `pcre2_compile` + extra | random combinations of extra options × `PCRE2_UTF`/`UCP`/`CASELESS` | [x] |
| 69 | `pcre2_compile` + `set_newline` | each of `CR, LF, CRLF, ANY, ANYCRLF, NUL` × patterns using `.`/`$`/`^`/`\R` | [x] |
| 70 | `pcre2_compile` + `set_bsr` | `BSR_UNICODE`, `BSR_ANYCRLF` × patterns with `\R` | [x] |
| 71 | `pcre2_compile` + `set_max_varlookbehind` | limits 0,1,2,255,65535 × variable lookbehind patterns | [x] |
| 72 | `pcre2_compile` + `set_parens_nest_limit` | limits 0,1,5,250,65535 × nested patterns | [x] |
| 73 | `pcre2_compile` + `set_max_pattern_length` | limits 0,1,len-1,len,len+1 | [x] |
| 74 | `pcre2_compile` + `set_max_pattern_compiled_length` | limits 0,1,small,large | [x] |
| 75 | `pcre2_compile` + `set_character_tables` | `pcre2_maketables` output, and `_pcre2_default_tables` | [x] |
| 76 | `pcre2_compile` + `set_optimize` | `OPTIMIZATION_NONE`, `OPTIMIZATION_FULL`, and each directive 64..=67 on/off | [x] |
| 77 | `pcre2_compile` + `set_compile_recursion_guard` | guard that always returns 0, and one that returns 1 at depth n | [x] |
| 78 | `pcre2_compile` | pattern length shapes: 0, 1, `PCRE2_ZERO_TERMINATED`, embedded NUL, 4 KiB | [x] |
| 79 | `pcre2_compile` | in-pattern option settings: `(*UTF)`, `(*UCP)`, `(*CR)`, `(*LF)`, `(*CRLF)`, `(*ANY)`, `(*ANYCRLF)`, `(*NUL)`, `(*BSR_UNICODE)`, `(*BSR_ANYCRLF)`, `(*LIMIT_MATCH=n)`, `(*LIMIT_DEPTH=n)`, `(*LIMIT_HEAP=n)`, `(*NO_START_OPT)`, `(*NO_AUTO_POSSESS)`, `(*NO_DOTSTAR_ANCHOR)`, `(*NOTEMPTY)`, `(*NOTEMPTY_ATSTART)`, `(*NO_JIT)` | [x] |
| 80 | `pcre2_pattern_info` | every `PCRE2_INFO_*` 0..=26 over the full compiled-pattern corpus | [x] |
| 81 | `pcre2_callout_enumerate` | patterns with numeric callouts, string callouts, `AUTO_CALLOUT` | [x] |
| 82 | `pcre2_code_copy` / `pcre2_code_copy_with_tables` | copy → serialize → byte compare; copy → match | [x] |
| 83 | `pcre2_match_data_create` | oveccount 1,2,3,16,65535 × `pcre2_get_ovector_count`/`_size`/`_heapframes_size` | [x] |
| 84 | `pcre2_match_data_create_from_pattern` | over full pattern corpus | [x] |
| 85 | `pcre2_match` | no options, over pattern × subject corpus (baseline) | [x] |
| 86 | `pcre2_match` | `PCRE2_ANCHORED` | [x] |
| 87 | `pcre2_match` | `PCRE2_ENDANCHORED` | [x] |
| 88 | `pcre2_match` | `PCRE2_NOTBOL` | [x] |
| 89 | `pcre2_match` | `PCRE2_NOTEOL` | [x] |
| 90 | `pcre2_match` | `PCRE2_NOTEMPTY` | [x] |
| 91 | `pcre2_match` | `PCRE2_NOTEMPTY_ATSTART` | [x] |
| 92 | `pcre2_match` | `PCRE2_PARTIAL_SOFT` | [x] |
| 93 | `pcre2_match` | `PCRE2_PARTIAL_HARD` | [x] |
| 94 | `pcre2_match` | `PCRE2_NO_UTF_CHECK` — full 32-bit option-bit sweep and randomized combos on **valid**-UTF subjects and on non-UTF patterns. Invalid-UTF subjects are exercised with the check ENABLED (they must return the right `UTF8_ERRn`); `NO_UTF_CHECK` + invalid UTF is undefined in the C (see ERRORS.md "Not comparable") | [x] |
| 95 | `pcre2_match` | `PCRE2_COPY_MATCHED_SUBJECT` | [x] |
| 96 | `pcre2_match` | `PCRE2_NO_JIT` | [x] |
| 97 | `pcre2_match` | random combinations of the match options above | [x] |
| 98 | `pcre2_match` | `start_offset` sweep 0..=len for each subject | [x] |
| 99 | `pcre2_match` | `mcontext` with `match_limit` 1,10,100,1000,default | [x] |
| 100 | `pcre2_match` | `mcontext` with `depth_limit` 1,10,100,1000,default | [x] |
| 101 | `pcre2_match` | `mcontext` with `heap_limit` 0,1,10,1000,default | [x] |
| 102 | `pcre2_match` | `PCRE2_USE_OFFSET_LIMIT` + `set_offset_limit` 0,1,len/2,len,UNSET | [x] |
| 103 | `pcre2_match` | `set_callout` callback: record every callout block field; callback returns 0 / 1 / −1 | [x] |
| 104 | `pcre2_match` | `MARK`/`(*MARK:x)` patterns → `pcre2_get_mark` | [x] |
| 105 | `pcre2_match` | `pcre2_get_startchar` after match / no-match / partial | [x] |
| 106 | `pcre2_next_match` | after `pcre2_match` on patterns with `\K`/lookarounds and multi-match subjects | [x] |
| 107 | `pcre2_dfa_match` | no options, baseline corpus, wscount 20/64/1000 | [x] |
| 108 | `pcre2_dfa_match` | `PCRE2_DFA_SHORTEST` | [x] |
| 109 | `pcre2_dfa_match` | `PCRE2_DFA_RESTART` after a `PCRE2_ERROR_PARTIAL` (proper 2-call sequence) | [x] |
| 110 | `pcre2_dfa_match` | `PCRE2_PARTIAL_SOFT` / `PCRE2_PARTIAL_HARD` | [x] |
| 111 | `pcre2_dfa_match` | `PCRE2_ANCHORED` / `ENDANCHORED` / `NOTBOL` / `NOTEOL` / `NOTEMPTY` / `NOTEMPTY_ATSTART` | [x] |
| 112 | `pcre2_dfa_match` | `PCRE2_COPY_MATCHED_SUBJECT`, `PCRE2_NO_UTF_CHECK` | [x] |
| 113 | `pcre2_dfa_match` | match/depth/heap limits, offset limit | [x] |
| 114 | `pcre2_dfa_match` | UTF patterns × valid and invalid UTF subjects (invalid ones with the UTF check enabled) | [x] |
| 115 | `pcre2_substring_length_bynumber` | every group number 0..=top+1 after match | [x] |
| 116 | `pcre2_substring_copy_bynumber` | buffer sizes: exact, exact−1, exact+1, 0 | [x] |
| 117 | `pcre2_substring_get_bynumber` | every group; compare returned bytes and length | [x] |
| 118 | `pcre2_substring_length_byname` / `copy_byname` / `get_byname` | named patterns, unique & duplicate names | [x] |
| 119 | `pcre2_substring_nametable_scan` | name present / absent / duplicated; NULL first/last ptrs | [x] |
| 120 | `pcre2_substring_number_from_name` | unique, duplicate, absent | [x] |
| 121 | `pcre2_substring_list_get` + `list_free` | over corpus; compare all strings and lengths | [x] |
| 122 | `pcre2_serialize_encode`/`decode`/`get_number_of_codes`/`free` | 1, 2, 8 codes; then match with each decoded code | [x] |
| 123 | `pcre2_substitute` | no options, single replacement, `$1`-style refs | [x] |
| 124 | `pcre2_substitute` | `PCRE2_SUBSTITUTE_GLOBAL` | [x] |
| 125 | `pcre2_substitute` | `PCRE2_SUBSTITUTE_EXTENDED` (`\U \L \u \l \E`, `${n:-x}`, `${n:+a:b}`) | [x] |
| 126 | `pcre2_substitute` | `PCRE2_SUBSTITUTE_UNSET_EMPTY` | [x] |
| 127 | `pcre2_substitute` | `PCRE2_SUBSTITUTE_UNKNOWN_UNSET` | [x] |
| 128 | `pcre2_substitute` | `PCRE2_SUBSTITUTE_OVERFLOW_LENGTH` with too-small buffer | [x] |
| 129 | `pcre2_substitute` | `PCRE2_SUBSTITUTE_LITERAL` | [x] |
| 130 | `pcre2_substitute` | `PCRE2_SUBSTITUTE_MATCHED` (pre-existing match_data) | [x] |
| 131 | `pcre2_substitute` | `PCRE2_SUBSTITUTE_REPLACEMENT_ONLY` | [x] |
| 132 | `pcre2_substitute` | `set_substitute_callout` — every block field compared, callback returns 0/1/−1 | [x] |
| 133 | `pcre2_substitute` | `set_substitute_case_callout` with LOWER/UPPER/TITLE_FIRST | [x] |
| 134 | `pcre2_substitute` | random combos of substitute options × UTF on/off × global | [x] |
| 135 | `pcre2_substitute` | `outlength` = 0, needed−1, needed, needed+1; NULL buffer probe | [x] |
| 136 | `pcre2_pattern_convert` | `PCRE2_CONVERT_POSIX_BASIC` | [x] |
| 137 | `pcre2_pattern_convert` | `PCRE2_CONVERT_POSIX_EXTENDED` | [x] |
| 138 | `pcre2_pattern_convert` | `PCRE2_CONVERT_GLOB` | [x] |
| 139 | `pcre2_pattern_convert` | `PCRE2_CONVERT_GLOB_NO_WILD_SEPARATOR` | [x] |
| 140 | `pcre2_pattern_convert` | `PCRE2_CONVERT_GLOB_NO_STARSTAR` | [x] |
| 141 | `pcre2_pattern_convert` | each of the above × `PCRE2_CONVERT_UTF` | [x] |
| 142 | `pcre2_pattern_convert` | each of the above × `PCRE2_CONVERT_NO_UTF_CHECK` | [x] |
| 143 | `pcre2_pattern_convert` | glob rows × `set_glob_separator` `/`, `\`, `.` | [x] |
| 144 | `pcre2_pattern_convert` | glob rows × `set_glob_escape` 0, `\`, `!`, `~` | [x] |
| 145 | `pcre2_pattern_convert` | caller-allocated buffer (`*blength` in, 2-pass) vs library-allocated | [x] |
| 146 | `pcre2_pattern_convert` | `PCRE2_ZERO_TERMINATED` length vs explicit length | [x] |
| 147 | `pcre2_get_error_message` | every code −200..=200, buffer sizes 0/1/short/exact/large | [x] |
| 148 | `pcre2_maketables` / `maketables_free` | default gcontext, custom gcontext; 1088-byte compare | [x] |
| 149 | `pcre2_jit_compile` / `jit_match` / `jit_stack_*` / `jit_free_unused_memory` | all entry points with JIT absent (stub paths); `_pcre2_jit_free`, `_pcre2_jit_free_rodata`, `_pcre2_jit_get_size`, `_pcre2_jit_get_target` | [x] |
| 150 | `_pcre2_study_8` | invoked indirectly through compile for every corpus pattern (start bitmap, minlength, first/req cu are in the serialized image) | [x] |
| 151 | `_pcre2_auto_possessify_8` | via compile with auto-possess ON vs OFF (`set_optimize`) on the corpus | [x] |
| 152 | `_pcre2_xclass_8` | via match on `\p{...}` / `[[:...:]]` / big-class patterns, utf on/off | [x] |
| 153 | `_pcre2_check_escape_8` | via compile of every `\x` escape form (`\d \D \s \S \w \W \h \H \v \V \R \X \C \K \b \B \A \Z \z \G \Q..\E \o{} \x{} \N{U+} \g \k \p \P`) | [x] |
| 154 | `_pcre2_compile_class_nested_8` / `_not_nested_8` / `_pcre2_eclass_8` / `_pcre2_update_classbits_8` | via compile+match of `PCRE2_ALT_EXTENDED_CLASS` set operations `[[a-z]&&[^aeiou]]`, `[\p{L}--\p{Lu}]`, nested `[[...][...]]` | [x] |
| 155 | `_pcre2_compile_*` name-table helpers (`add_name_to_table`, `find_dupname_details`, `find_named_group`, `get_hash_from_name`, `parse_recurse_args`, `parse_scan_substr_args`) | via compile of many named/duplicate-named/recursive patterns (name table is in the serialized image) | [x] |
| 156 | binary driver | none — the project builds no executable (CMake: `add_library` only; Cargo: `crate-type = ["cdylib"]`, no `[[bin]]` target; confirmed by `cargo metadata`, whose only target is the `cdylib`). Nothing to compare stdout for. | n/a |
| 157 | `pcre2_serialize_decode` **cross-library** | the Rust `.so` decodes the C `.so`'s byte stream and vice versa, then both re-encode and match identically | [x] |
| 158 | `_pcre2_memctl_malloc_8` + every `*_create` entry point | a custom allocator whose `malloc` starts returning NULL after *n* calls; the C and Rust NULL/non-NULL sequences are compared per allocation budget | [x] |
| 159 | `pcre2_match` / `pcre2_dfa_match` | the documented global-iteration loop (`pcre2_match` + `pcre2_next_match` to exhaustion), whole match sequence compared | [x] |
