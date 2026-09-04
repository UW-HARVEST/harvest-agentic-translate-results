# PCRE2 (8-bit) configuration-surface table

Mechanically derived from `c_src/`. Every row is a **valid** configuration (no error-path
rows unless the "error" is the documented result of a legal input, e.g. `PCRE2_ERROR_NOMEMORY`
from a deliberate buffer-overflow query). Build assumptions from `CONVENTIONS.md`:
`PCRE2_CODE_UNIT_WIDTH == 8`, `LINK_SIZE == 2`, `SUPPORT_UNICODE` on, `SUPPORT_JIT` off,
`EBCDIC` off. Because JIT is not compiled, `pcre2_jit_compile` returns
`PCRE2_ERROR_JIT_UNSUPPORTED`, `pcre2_jit_match` returns `PCRE2_ERROR_JIT_BADOPTION`, and
`pcre2_match` never takes the JIT branch — those rows verify exactly that.

Option names are joined with `+` instead of `|` to keep the markdown table intact.
Library defaults referenced below: `MATCH_LIMIT 10000000`, `MATCH_LIMIT_DEPTH 10000000`,
`HEAP_LIMIT 20000000` (KiB), `PARENS_NEST_LIMIT 250`, `MAX_NAME_SIZE 128`,
`MAX_NAME_COUNT 10000`, `MAX_VARLOOKBEHIND 255`, `max_pattern_length = PCRE2_SIZE_MAX`,
`max_pattern_compiled_length = PCRE2_SIZE_MAX`, DFA minimum `wscount == 20`.

| # | entry point(s) | configuration (options set + input shape) | [x] |
|---|----------------|--------------------------------------------|-----|


## Coverage map — which test proves each section

Every row below is marked `[x]`: the configuration is driven through BOTH `.so`
exports and the outputs compared byte-for-byte, across randomized inputs with
fixed seeds. Run `./run_tests.sh` to reproduce.

| section | rows | test(s) that cover it |
|---|---|---|
| A. compile core options | 1–42 | `compile_match.rs`: `compile_match_default_options`, `compile_match_each_single_compile_option`, `compile_match_utf_and_ucp`, `compile_match_newline_x_bsr`, `randomized_patterns`, `randomized_raw_byte_patterns` |
| B. extra compile options | 43–56 | `compile_match.rs`: `compile_match_each_extra_option` (all 17 `PCRE2_EXTRA_*`) |
| C. compile context | 57–70 | `compile_match.rs`: `own_character_tables`, `varlookbehind_and_parens_nest_limits`, `pattern_length_limits`, `optimize_flags`, `code_copy_variants`; `api_errors.rs`: `rows283_292_setter_validation` (incl. order-sensitive `set_optimize` sequences) |
| D. pattern shapes | 71–100 | `compile_match.rs`: the 200-pattern corpus in every option combination, `randomized_patterns` (4 000), `randomized_raw_byte_patterns` (15 000); `compile_errors.rs` for the shapes that must be rejected |
| E. pattern_info / callout_enumerate | 101–106 | `common/diff.rs::assert_pattern_info_eq` runs on EVERY compile in the suite (all 27 selectors incl. `FIRSTBITMAP`/`NAMETABLE` bytes); `api_errors.rs`: `rows304_312_pattern_info_validation`, `rows313_316_callout_enumerate` |
| F. pcre2_match | 107–140 | `compile_match.rs`: `compile_match_each_match_option`, `subject_lengths_and_all_start_offsets`, `ovector_sizes`, `match_depth_heap_limits`, `offset_limit`, `randomized_subjects`, `randomized_utf_subjects`; `misc_errors.rs`: `rows348_358_jit_surface_non_jit_build` |
| G. pcre2_dfa_match | 141–153 | `compile_match.rs`: `dfa_shortest_and_partial`, `dfa_restart_after_partial` (documented partial→restart flow, workspace bytes compared), `dfa_workspace_sizes`; every other test also runs `Engine::Dfa` |
| H. pcre2_next_match | 154–159 | `misc_errors.rs`: `rows359_360_next_match` (full iteration sequences, both engines); `substring.rs` drives it to exhaustion |
| I. pcre2_substitute | 160–188 | `substitute.rs` (22 tests) |
| J. pcre2_pattern_convert | 189–202 | `convert.rs` (7 tests) |
| K. substring extraction | 203–213 | `substring.rs` (13 tests) |
| L. serialization | 214–219 | `serialize.rs` (10 tests, incl. cross-decode) |
| M. config / errors / match-data | 220–226 | `lowlevel.rs`: `get_error_message_all_codes`, `maketables_*`; `api_errors.rs`: `rows299_302_config_selectors`; `misc_errors.rs`: `rows334_335_match_data_create_clamps_oveccount` |
| N. low-level `_pcre2_*` helpers | 227–244 | `lowlevel.rs` (34 tests) |
| O. exported DATA symbols | 245–260 | `lowlevel.rs`: `exported_data_tables_are_byte_identical` (27 tables), `exported_default_contexts_match`, `exported_unicode_version_string_matches` |

## A. pcre2_compile — core compile option axes

| # | entry point(s) | configuration (options set + input shape) | [x] |
|---|----------------|--------------------------------------------|-----|
| 1 | `pcre2_compile_8`, `pcre2_code_free_8` | `options=0`, pattern `abc`, `patlen=PCRE2_ZERO_TERMINATED`, `ccontext=NULL`; compare `PCRE2_INFO_SIZE`, `ALLOPTIONS`, `ARGOPTIONS`, `CAPTURECOUNT`, whole code block bytes | [x] |
| 2 | `pcre2_compile_8` | `options=0`, pattern `abc` in a non-NUL-terminated buffer with explicit `patlen=3`; then same with `patlen=2` (`ab`) to prove length is honoured | [x] |
| 3 | `pcre2_compile_8` | empty pattern: `pattern=""`,`patlen=0`; and `pattern=NULL`,`patlen=0`; check `MINLENGTH=0`, `MATCHEMPTY=1` | [x] |
| 4 | `pcre2_compile_8`, `pcre2_match_8` | `PCRE2_ANCHORED` at compile only vs at match only vs both; pattern `a+`, subject `"bbaa"` startoffset 0 | [x] |
| 5 | `pcre2_compile_8`, `pcre2_match_8` | `PCRE2_ENDANCHORED` compile-time, pattern `a+`, subjects `"aaa"`, `"aaab"`; then `PCRE2_ANCHORED+PCRE2_ENDANCHORED` | [x] |
| 6 | `pcre2_compile_8`, `pcre2_match_8` | `PCRE2_CASELESS`, ASCII-only pattern `[a-z]+K`, subject `"ABCk"`; default tables | [x] |
| 7 | `pcre2_compile_8`, `pcre2_match_8` | `PCRE2_CASELESS+PCRE2_UTF`, pattern `\x{130}\x{131}\x{17f}\x{212a}` (chars with `UCD_CASESET != 0`), subject with the other-case forms | [x] |
| 8 | `pcre2_compile_8`, `pcre2_set_compile_extra_options_8` | `PCRE2_CASELESS+PCRE2_UTF` with `PCRE2_EXTRA_CASELESS_RESTRICT`; pattern `k` vs subject `\x{212a}` (KELVIN SIGN) — must NOT match; also inline `(?r)k` and `(*CASELESS_RESTRICT)k` | [x] |
| 9 | `pcre2_compile_8`, `pcre2_match_8` | `PCRE2_DOLLAR_ENDONLY`, pattern `abc$`, subjects `"abc"`, `"abc\n"`; then with `PCRE2_MULTILINE` too | [x] |
| 10 | `pcre2_compile_8`, `pcre2_match_8` | `PCRE2_DOTALL`, pattern `a.b`, subject `"a\nb"`; and without DOTALL for contrast; check `PCRE2_INFO_FIRSTCODETYPE`/`MINLENGTH` | [x] |
| 11 | `pcre2_compile_8`, `pcre2_match_8` | `PCRE2_MULTILINE`, pattern `^b`, subject `"a\nb"`, startoffset 0 and 2; check `PCRE2_INFO_ALLOPTIONS` has `PCRE2_STARTLINE` effect via `FIRSTBITMAP` absence | [x] |
| 12 | `pcre2_compile_8`, `pcre2_match_8` | `PCRE2_ALT_CIRCUMFLEX+PCRE2_MULTILINE`, pattern `^`, subject `"a\n"` matched repeatedly to offset==length (allows `^` after a final newline) | [x] |
| 13 | `pcre2_compile_8`, `pcre2_substring_number_from_name_8` | `PCRE2_DUPNAMES`, pattern `(?<n>a)\|(?<n>b)\|(?<n>c)`; check `NAMECOUNT=3`, `NAMEENTRYSIZE`, full `NAMETABLE` bytes, and `NOUNIQUESUBSTRING` from `number_from_name` | [x] |
| 14 | `pcre2_compile_8` | `PCRE2_EXTENDED`, pattern `"a b # comment\n c"`; then `PCRE2_EXTENDED_MORE` on `"a b[c d]"` (spaces inside classes also ignored); then both together | [x] |
| 15 | `pcre2_compile_8` | `options=0` with inline `(?x)a b(?-x)c d`, and `(?xx)`/`(?-xx)`, and `(?^)` resetting `imnsx`+`(?r)`; check `ARGOPTIONS` vs `ALLOPTIONS` | [x] |
| 16 | `pcre2_compile_8`, `pcre2_match_8` | `PCRE2_FIRSTLINE`, pattern `b`, subject `"a\nb"` (must not match) and `"ab\nb"`; with newline convention LF and CRLF | [x] |
| 17 | `pcre2_compile_8`, `pcre2_pattern_info_8` | `PCRE2_NO_AUTO_CAPTURE`, pattern `(a)(b)`; check `CAPTURECOUNT=0`, `BACKREFMAX=0`; plus `(?n)` inline form | [x] |
| 18 | `pcre2_compile_8` | `PCRE2_NO_AUTO_POSSESS` vs default, pattern `\d+\D`; byte-compare compiled code (OP_PLUS vs OP_POSPLUS); plus `(*NO_AUTO_POSSESS)` inline | [x] |
| 19 | `pcre2_compile_8` | `PCRE2_NO_DOTSTAR_ANCHOR` vs default, pattern `.*abc`; check `PCRE2_INFO_ALLOPTIONS` for `PCRE2_ANCHORED`; plus `(*NO_DOTSTAR_ANCHOR)` | [x] |
| 20 | `pcre2_compile_8`, `pcre2_match_8` | `PCRE2_NO_START_OPTIMIZE`, pattern `(?C1)abc`, subject `"xxabc"`; count callouts (must fire at every start position) vs default | [x] |
| 21 | `pcre2_compile_8` | `PCRE2_UNGREEDY`, pattern `a+?b*`; byte-compare code with the non-UNGREEDY build; plus `(?U)` inline | [x] |
| 22 | `pcre2_compile_8`, `pcre2_match_8` | `PCRE2_UTF` only, pattern `.`, subjects: 1-byte `"A"`, 2-byte `"\xc3\xa9"`, 3-byte `"\xe4\xb8\xad"`, 4-byte `"\xf0\x9f\x98\x80"`; startoffset 0 | [x] |
| 23 | `pcre2_compile_8`, `pcre2_match_8` | `PCRE2_UCP` **without** `PCRE2_UTF`, pattern `\w+\d\s`; subject bytes 0x80–0xFF; verify UCP semantics apply to single bytes | [x] |
| 24 | `pcre2_compile_8`, `pcre2_match_8` | `PCRE2_UTF+PCRE2_UCP`, pattern `\p{Greek}+[\x{370}-\x{3ff}\x{1f00}-\x{1fff}]`; subject 2/3-byte UTF-8, startoffset 0 and at a character boundary mid-subject | [x] |
| 25 | `pcre2_compile_8`, `pcre2_match_8` | `PCRE2_MATCH_INVALID_UTF` (implies UTF, check `ALLOPTIONS` contains UTF), pattern `a`, subject with a valid prefix, an invalid byte `0xFF`, and a valid suffix; startoffset 0 and past the bad byte | [x] |
| 26 | `pcre2_compile_8`, `pcre2_match_8` | `PCRE2_MATCH_UNSET_BACKREF`, pattern `(a)?\1b`, subject `"b"`; also with `PCRE2_INFO_MINLENGTH` (must be 0 for the backref) | [x] |
| 27 | `pcre2_compile_8`, `pcre2_pattern_info_8` | `PCRE2_NEVER_BACKSLASH_C` with pattern containing no `\C`; and default options with pattern `\C` → `PCRE2_INFO_HASBACKSLASHC=1` | [x] |
| 28 | `pcre2_compile_8` | `PCRE2_NEVER_UCP` with a pattern that does not request UCP; `PCRE2_NEVER_UTF` with a pattern that does not request UTF; both together | [x] |
| 29 | `pcre2_compile_8`, `pcre2_callout_enumerate_8` | `PCRE2_AUTO_CALLOUT`, pattern `a(b\|c)*d`; enumerate all callouts and byte-compare every `pattern_position`/`next_item_length`/`callout_number` (255 for auto) | [x] |
| 30 | `pcre2_compile_8`, `pcre2_match_8` | `PCRE2_ALLOW_EMPTY_CLASS`, pattern `[]a`, `[^]a`, `a[]*b`; subject exercising the empty class | [x] |
| 31 | `pcre2_compile_8`, `pcre2_match_8` | `PCRE2_ALT_BSUX`, pattern `A\x41\U`; then `PCRE2_EXTRA_ALT_BSUX` with `\u{41}` (brace form only enabled by the EXTRA bit) | [x] |
| 32 | `pcre2_compile_8`, `pcre2_get_mark_8` | `PCRE2_ALT_VERBNAMES`, pattern `(*MARK:a b\x41)` with `PCRE2_EXTENDED` (verb name is processed for escapes/whitespace); compare the mark bytes returned | [x] |
| 33 | `pcre2_compile_8` | `PCRE2_LITERAL` alone, pattern `a.*b[c` (all metacharacters literal); check `CAPTURECOUNT=0`, `MINLENGTH=6` | [x] |
| 34 | `pcre2_compile_8` | `PCRE2_LITERAL` with each other member of `PUBLIC_LITERAL_COMPILE_OPTIONS`: `+ANCHORED`, `+AUTO_CALLOUT`, `+CASELESS`, `+ENDANCHORED`, `+FIRSTLINE`, `+MATCH_INVALID_UTF`, `+NO_START_OPTIMIZE`, `+NO_UTF_CHECK`, `+USE_OFFSET_LIMIT`, `+UTF` (one row per bit is ideal) | [x] |
| 35 | `pcre2_compile_8`, `pcre2_match_8`, `pcre2_set_offset_limit_8` | `PCRE2_USE_OFFSET_LIMIT` compiled in; `pcre2_set_offset_limit(mc, 0)`, `=3`, `=length`, `=PCRE2_UNSET`; pattern `b`, subject `"aaab"` | [x] |
| 36 | `pcre2_compile_8`, `pcre2_match_8` | `PCRE2_ALT_EXTENDED_CLASS`, patterns `[a-z--[aeiou]]`, `[\d&&[0-4]]`, `[[a-f]~~[c-h]]`, `[!a-z]`; nested 2 and 3 deep | [x] |
| 37 | `pcre2_compile_8`, `pcre2_match_8` | Perl extended class `(?[ [a-z] - [aeiou] ])` with default options (no `ALT_EXTENDED_CLASS`); union/intersection/difference/symmetric-difference and one nested parenthesised operand | [x] |
| 38 | `pcre2_compile_8` | `PCRE2_NO_UTF_CHECK` at compile with `PCRE2_UTF` and a *valid* UTF-8 pattern (skips the pattern validity scan); byte-compare code against the checked build | [x] |
| 39 | `pcre2_compile_8` | `patlen == PCRE2_ZERO_TERMINATED` vs explicit length on a pattern containing an embedded NUL (`"a\0b"`, len 3) — the explicit form must compile 3 items | [x] |
| 40 | `pcre2_compile_8`, `pcre2_pattern_info_8` | in-pattern start-of-pattern directives, one row each: `(*UTF)`, `(*UCP)`, `(*NOTEMPTY)`, `(*NOTEMPTY_ATSTART)`, `(*NO_JIT)`, `(*NO_AUTO_POSSESS)`, `(*NO_DOTSTAR_ANCHOR)`, `(*NO_START_OPT)`, `(*CASELESS_RESTRICT)`, `(*TURKISH_CASING)` — check `ALLOPTIONS`/`EXTRAOPTIONS` | [x] |
| 41 | `pcre2_compile_8`, `pcre2_match_8` | `(*LIMIT_MATCH=100)`, `(*LIMIT_DEPTH=50)`, `(*LIMIT_HEAP=64)` in the pattern combined with `pcre2_set_match_limit(1000)`/`set_depth_limit(1000)`/`set_heap_limit(1024)` — the *minimum* of the two must win; read back via `PCRE2_INFO_MATCHLIMIT`/`DEPTHLIMIT`/`HEAPLIMIT` | [x] |
| 42 | `pcre2_compile_8`, `pcre2_pattern_info_8` | in-pattern newline/BSR directives, one row each: `(*CR)`, `(*LF)`, `(*CRLF)`, `(*ANY)`, `(*ANYCRLF)`, `(*NUL)`, `(*BSR_ANYCRLF)`, `(*BSR_UNICODE)`; read back `PCRE2_INFO_NEWLINE` and `PCRE2_INFO_BSR` | [x] |

## B. pcre2_compile — extra compile options (`pcre2_set_compile_extra_options`)

| # | entry point(s) | configuration (options set + input shape) | [x] |
|---|----------------|--------------------------------------------|-----|
| 43 | `pcre2_set_compile_extra_options_8`, `pcre2_compile_8` | `PCRE2_EXTRA_ALLOW_SURROGATE_ESCAPES` + `PCRE2_UTF`, pattern `\x{d800}\x{dfff}` | [x] |
| 44 | `pcre2_set_compile_extra_options_8`, `pcre2_compile_8` | `PCRE2_EXTRA_BAD_ESCAPE_IS_LITERAL`, pattern `\j\y[\i]` (unknown escapes become literals, in and out of a class) | [x] |
| 45 | `pcre2_set_compile_extra_options_8`, `pcre2_match_8` | `PCRE2_EXTRA_MATCH_WORD`, pattern `abc`, subjects `"abc"`, `"xabc"`, `"abcx"`; compare against a hand-written `\b(?:abc)\b` | [x] |
| 46 | `pcre2_set_compile_extra_options_8`, `pcre2_match_8` | `PCRE2_EXTRA_MATCH_LINE`, pattern `abc`, subject `"abc"` and `"abc\n"`; and `MATCH_LINE+MATCH_WORD` together (LINE wins) | [x] |
| 47 | `pcre2_set_compile_extra_options_8`, `pcre2_compile_8` | `PCRE2_EXTRA_ESCAPED_CR_IS_LF`, pattern containing a literal `\` followed by CR (0x0D) | [x] |
| 48 | `pcre2_set_compile_extra_options_8`, `pcre2_compile_8` | `PCRE2_EXTRA_ALT_BSUX`, patterns `\u{1F600}` (with UTF) and `A` | [x] |
| 49 | `pcre2_set_compile_extra_options_8`, `pcre2_match_8` | `PCRE2_EXTRA_ALLOW_LOOKAROUND_BSK`, pattern `(?=a\Kb)` / `(?<=a\K)b`; match and check `ovector[0] < start_offset` is tolerated (no `PCRE2_ERROR_BAD_BACKSLASH_K`) | [x] |
| 50 | `pcre2_set_compile_extra_options_8`, `pcre2_match_8` | `PCRE2_EXTRA_ASCII_BSD` (`\d`), `_BSS` (`\s`), `_BSW` (`\w`), each with `PCRE2_UCP`; subjects with U+0660 ARABIC-INDIC DIGIT ZERO and U+00A0 NBSP — one row per bit plus the combined `(?a)` form | [x] |
| 51 | `pcre2_set_compile_extra_options_8`, `pcre2_match_8` | `PCRE2_EXTRA_ASCII_POSIX` and `PCRE2_EXTRA_ASCII_DIGIT` with `PCRE2_UCP`, pattern `[[:alpha:][:digit:][:xdigit:]]+`; inline `(?aP)` and `(?aT)` variants | [x] |
| 52 | `pcre2_set_compile_extra_options_8`, `pcre2_compile_8` | `PCRE2_EXTRA_PYTHON_OCTAL`, patterns `\0`, `\7`, `\10`, `\077`, `\o{17}` with 0/1/8 capture groups (changes octal-vs-backref resolution) | [x] |
| 53 | `pcre2_set_compile_extra_options_8`, `pcre2_compile_8` | `PCRE2_EXTRA_NO_BS0`, pattern `\0` (and `\00`) | [x] |
| 54 | `pcre2_set_compile_extra_options_8`, `pcre2_compile_8` | `PCRE2_EXTRA_NEVER_CALLOUT` with a pattern containing no callout; and `PCRE2_AUTO_CALLOUT` is still allowed/denied — record which | [x] |
| 55 | `pcre2_set_compile_extra_options_8`, `pcre2_match_8` | `PCRE2_EXTRA_TURKISH_CASING+PCRE2_UTF+PCRE2_CASELESS`, pattern `i` / `I` / `\x{130}` / `\x{131}`; subjects covering all four dotted/dotless forms; compare `_pcre2_ucd_turkish_dotted_i_caseset_8` usage | [x] |
| 56 | `pcre2_set_compile_extra_options_8`, `pcre2_compile_8` | `PCRE2_EXTRA_CASELESS_RESTRICT` combined with `PCRE2_UCP` (no UTF) and with backreferences `(a)\1` under `PCRE2_CASELESS` (sets `REFI_FLAG_CASELESS_RESTRICT`) | [x] |

## C. Compile context — setters, limits and tables

| # | entry point(s) | configuration (options set + input shape) | [x] |
|---|----------------|--------------------------------------------|-----|
| 57 | `pcre2_compile_context_create_8`, `pcre2_compile_context_copy_8`, `pcre2_compile_context_free_8` | create with `gcontext=NULL`; create with a `pcre2_general_context` holding custom malloc/free; copy and byte-compare the two context blocks | [x] |
| 58 | `pcre2_set_newline_8`, `pcre2_compile_8`, `pcre2_match_8` | all six conventions `PCRE2_NEWLINE_CR/LF/CRLF/ANY/ANYCRLF/NUL` × pattern `.` , `$`, `^` with `PCRE2_MULTILINE`, `\R`; subject `"a\r\nb\x85c\xe2\x80\xa8d\0e"` — one row per convention | [x] |
| 59 | `pcre2_set_bsr_8`, `pcre2_compile_8`, `pcre2_match_8` | `PCRE2_BSR_UNICODE` vs `PCRE2_BSR_ANYCRLF`, pattern `\R+`, subject containing CR, LF, CRLF, VT, FF, NEL, U+2028, U+2029 (with and without `PCRE2_UTF`) | [x] |
| 60 | `pcre2_set_max_pattern_length_8`, `pcre2_compile_8` | limit = exact pattern length (succeeds), limit = length+1, limit = `PCRE2_SIZE_MAX` (default); pattern `abcdef` with `patlen=6` and with `PCRE2_ZERO_TERMINATED` | [x] |
| 61 | `pcre2_set_max_pattern_compiled_length_8`, `pcre2_compile_8`, `pcre2_pattern_info_8` | read `PCRE2_INFO_SIZE` for pattern `(a\|b\|c){3,10}`, then set the limit to exactly that value (succeeds) and to value+1 | [x] |
| 62 | `pcre2_set_max_varlookbehind_8`, `pcre2_compile_8`, `pcre2_pattern_info_8` | default 255 with `(?<=a{200})b`; limit set to 1 with `(?<=a)b`; limit set to 5 with `(?<=ab{0,4})c`; read back `PCRE2_INFO_MAXLOOKBEHIND` | [x] |
| 63 | `pcre2_set_parens_nest_limit_8`, `pcre2_compile_8` | default 250 with 250 nested `(`; limit 1 with `(a)`; limit 300 with 260 nested groups | [x] |
| 64 | `pcre2_set_compile_recursion_guard_8`, `pcre2_compile_8` | guard returning 0 always, on a pattern with 100 nested groups; record every `depth` value the guard is called with (order and count must match) | [x] |
| 65 | `pcre2_set_character_tables_8`, `pcre2_maketables_8`, `pcre2_maketables_free_8`, `pcre2_compile_8` | `tables=NULL` (default `_pcre2_default_tables_8`) vs `pcre2_maketables(NULL)` in the "C" locale (must be byte-identical, 1088 bytes) vs a hand-built 1088-byte table with swapped case; pattern `[[:alpha:]]+` with `PCRE2_CASELESS` | [x] |
| 66 | `pcre2_set_optimize_8`, `pcre2_compile_8` | `PCRE2_OPTIMIZATION_NONE`, `PCRE2_OPTIMIZATION_FULL`, and each toggle `PCRE2_AUTO_POSSESS`/`_OFF`, `PCRE2_DOTSTAR_ANCHOR`/`_OFF`, `PCRE2_START_OPTIMIZE`/`_OFF`; pattern `.*\d+\D`; byte-compare compiled code and `PCRE2_INFO_ALLOPTIONS` | [x] |
| 67 | `pcre2_set_optimize_8`, `pcre2_compile_8` | `PCRE2_OPTIMIZATION_FULL` then `PCRE2_AUTO_POSSESS_OFF` (order matters: later calls modify the flag word) and the reverse order | [x] |
| 68 | `pcre2_general_context_create_8`, `pcre2_general_context_copy_8`, `pcre2_general_context_free_8`, `_pcre2_memctl_malloc_8` | custom malloc/free that records every (size, user_data) pair; drive a full compile+match+substitute and compare the allocation trace | [x] |
| 69 | `pcre2_code_copy_8`, `pcre2_code_free_8` | copy a code compiled with default tables and one compiled with `pcre2_maketables`; byte-compare the copy against the original except the `tables` pointer and `memctl`; then match with the copy | [x] |
| 70 | `pcre2_code_copy_with_tables_8`, `pcre2_code_free_8` | same as above, then free the original code *and* the tables, and still match with the copy (tables must be embedded) | [x] |

## D. pcre2_compile — pattern shapes the parser/compiler special-cases

| # | entry point(s) | configuration (options set + input shape) | [x] |
|---|----------------|--------------------------------------------|-----|
| 71 | `pcre2_compile_8`, `pcre2_pattern_info_8` | capture-group counts 0, 1, 2, 100, 65534 (near `top_bracket` limit); check `CAPTURECOUNT`, `FRAMESIZE`, `SIZE` | [x] |
| 72 | `pcre2_compile_8`, `pcre2_substring_nametable_scan_8` | named groups: 0 names; 1 name; 3 names in non-alphabetical source order (table must be sorted); a 128-char name (`MAX_NAME_SIZE`); names differing only in the last character | [x] |
| 73 | `pcre2_compile_8`, `_pcre2_compile_get_hash_from_name8` | `(?<a>)(?<ab>)(?<b>)` — call the exported hash function directly on each name and length and byte-compare the `uint16_t` results, including names of length 1 and 128 | [x] |
| 74 | `pcre2_compile_8`, `_pcre2_compile_find_dupname_details8` | `PCRE2_DUPNAMES` with `(?<n>a)(?<m>b)(?<n>c)(?<n>d)`; check the `index`/`count` pair for `n` (3 groups) and for `m` (1 group) | [x] |
| 75 | `pcre2_compile_8` | character classes: `[abc]`, `[^abc]`, `[a-z0-9]`, `[[:alpha:][:^digit:]]`, `[\d\s\w]`, `[\D\S\W]`, `[\x{100}-\x{200}]` (forces OP_XCLASS), `[^\x{100}]`, `[\p{L}\p{^Nd}]`, `[\P{Greek}]`, `[a\x{ff}]` non-UTF (wide byte), all with and without `PCRE2_UTF` | [x] |
| 76 | `pcre2_compile_8`, `_pcre2_compile_class_not_nested_8` | single non-nested class producing each of OP_CLASS, OP_NCLASS, OP_XCLASS, OP_ALLANY: `[ab]`, `[^ab]`, `[\x{100}]`, `[\s\S]`; byte-compare the emitted opcode bytes | [x] |
| 77 | `pcre2_compile_8`, `_pcre2_compile_class_nested_8`, `_pcre2_eclass_8` | `PCRE2_ALT_EXTENDED_CLASS` nested classes emitting `ECL_AND`/`ECL_OR`/`ECL_XOR`/`ECL_NOT`/`ECL_ANY`/`ECL_NONE`/`ECL_XCLASS`; then call `_pcre2_eclass_8` directly over the emitted data for code points 0x41, 0xFF, 0x100, 0x10FFFF | [x] |
| 78 | `pcre2_compile_8`, `_pcre2_update_classbits_8` | for each `PT_*` type (`PT_LAMP`, `PT_GC`, `PT_PC`, `PT_SC`, `PT_SCX`, `PT_ALNUM`, `PT_SPACE`, `PT_PXSPACE`, `PT_WORD`, `PT_CLIST`, `PT_UCNC`, `PT_BIDICL`, `PT_BOOL`, `PT_ANY`, `PT_PXGRAPH`, `PT_PXPRINT`, `PT_PXPUNCT`, `PT_PXXDIGIT`) call with `negated=FALSE` and `TRUE` and byte-compare the 32-byte `classbits` | [x] |
| 79 | `pcre2_compile_8`, `pcre2_match_8` | all 14 POSIX classes `[:alpha:] [:lower:] [:upper:] [:alnum:] [:ascii:] [:blank:] [:cntrl:] [:digit:] [:graph:] [:print:] [:punct:] [:space:] [:word:] [:xdigit:]`, positive and `[:^name:]`, with and without `PCRE2_UCP` (UCP switches to `posix_substitutes`) | [x] |
| 80 | `pcre2_compile_8`, `pcre2_match_8` | quantifier bounds: `a*`, `a+`, `a?`, `a{0}`, `a{3}`, `a{0,}`, `a{2,}`, `a{2,5}`, `a{0,1}`, `a{65535}`; lazy `*? +? ??` and possessive `*+ ++ ?+ {2,5}+` forms of each | [x] |
| 81 | `pcre2_compile_8`, `pcre2_match_8` | backreferences: `(a)\1`, `(?<n>a)\k<n>`, `\g{1}`, `\g{-1}`, `(?P=n)`, forward reference `\1(a)` with `PCRE2_MATCH_UNSET_BACKREF`, and a backref with `PCRE2_CASELESS`; check `PCRE2_INFO_BACKREFMAX` | [x] |
| 82 | `pcre2_compile_8`, `pcre2_match_8` | recursion/subroutine calls: `(a(?1)?b)`, `(?R)`, `(?1)`, `(?+1)`, `(?-1)`, `(?&name)`, `(?P>name)`, `\g<1>`, `\g<-1>`; plus `(?(DEFINE)(?<x>a))(?&x)` | [x] |
| 83 | `pcre2_compile_8`, `pcre2_match_8` | lookarounds: `(?=)`, `(?!)`, `(?<=abc)`, `(?<!abc)`, variable-length lookbehind `(?<=a{1,4})`, alternation-length lookbehind `(?<=ab\|cde)`, `(?*)`/`(?<*)` non-atomic forms `(*napla:)`/`(*naplb:)`, and each alpha-assertion spelling (`(*pla:`, `(*plb:`, `(*nla:`, `(*nlb:`, `(*positive_lookahead:`, `(*negative_lookbehind:` …) | [x] |
| 84 | `pcre2_compile_8`, `pcre2_match_8` | atomic groups `(?>a\|ab)`, possessive-group equivalents `(?:a\|ab)++`, `(*atomic:...)`; script runs `(*sr:...)`, `(*asr:...)`, `(*script_run:...)`, `(*atomic_script_run:...)` with `PCRE2_UTF` | [x] |
| 85 | `pcre2_compile_8`, `pcre2_match_8`, `_pcre2_compile_parse_scan_substr_args8` | scan-substring assertions `(*scs:1:a)`, `(*scs:1,2:a)`, `(*scan_substring:name:a)`, `(*scs:-1:a)`, with `PCRE2_DUPNAMES` duplicate names in the capture list | [x] |
| 86 | `pcre2_compile_8`, `_pcre2_compile_parse_recurse_args8` | `(?1)` / `(?&n)` / `(?R)` inside a pattern with duplicate group numbers from `(?\|(a)\|(b))`; verify the parsed recursion targets | [x] |
| 87 | `pcre2_compile_8`, `pcre2_match_8` | conditionals: `(?(1)a\|b)`, `(?(<n>)a\|b)`, `(?(R)a\|b)`, `(?(R1)a\|b)`, `(?(R&n)a\|b)`, `(?(DEFINE)...)`, `(?(?=x)a\|b)`, `(?(VERSION>=10.0)a\|b)`, `(?(+1)a\|b)` | [x] |
| 88 | `pcre2_compile_8`, `pcre2_match_8` | all backtracking verbs: `(*ACCEPT)`, `(*FAIL)`, `(*F)`, `(*MARK:x)`, `(*:x)`, `(*COMMIT)`, `(*COMMIT:x)`, `(*PRUNE)`, `(*PRUNE:x)`, `(*SKIP)`, `(*SKIP:x)`, `(*THEN)`, `(*THEN:x)`; verb name at `MAX_MARK` length | [x] |
| 89 | `pcre2_compile_8`, `pcre2_match_8` | `\K` at top level, inside an atomic group, inside a positive lookahead (with `PCRE2_EXTRA_ALLOW_LOOKAROUND_BSK`); `\K` after a quantifier | [x] |
| 90 | `pcre2_compile_8`, `pcre2_match_8` | `\b`, `\B`, `\A`, `\Z`, `\z`, `\G`, `\R`, `\X`, `\C`, `\N`, `\h`, `\H`, `\v`, `\V` — each alone and each quantified `+`/`*`/`{2,3}`, with `PCRE2_UTF` on and off | [x] |
| 91 | `pcre2_compile_8`, `pcre2_pattern_info_8` | alternation counts 1, 2, 3, 100, 1000 branches of `a` — check `MINLENGTH`, `FIRSTBITMAP` (256 bytes), `FIRSTCODETYPE`, `SIZE` | [x] |
| 92 | `pcre2_compile_8` | pattern sizes straddling the `LINK_SIZE == 2` boundary: a group whose body compiles to just under, exactly, and just over 65535 code units (e.g. `(` + `a{N}` + `)` sweeping N); check `PCRE2_INFO_SIZE` and successful match | [x] |
| 93 | `pcre2_compile_8`, `pcre2_callout_enumerate_8` | numeric callouts `(?C)`, `(?C0)`, `(?C255)` and string callouts with each delimiter in `_pcre2_callout_start_delims_8` (`` ` `` `'` `"` `^` `%` `#` `$` `{`) — enumerate and compare `callout_string_offset`/`_length`/bytes | [x] |
| 94 | `pcre2_compile_8`, `pcre2_match_8` | `\p{...}` / `\P{...}` across every `_pcre2_utt_8` shape: general category `\p{L}`, particular `\p{Lu}`, `\p{L&}`, script `\p{Greek}`, script-extension `\p{Scx:Han}`, boolean `\p{Bidi_Control}`, bidi class `\p{Bidi_Class:AL}`, `\p{Any}`, `\p{Xan}`, `\p{Xps}`, `\p{Xsp}`, `\p{Xuc}`, `\p{Xwd}`; with and without `PCRE2_UTF` | [x] |
| 95 | `pcre2_compile_8`, `_pcre2_auto_possessify_8` | patterns whose auto-possessification depends on the `PT_TABSIZE` matrix: `\d+\D`, `\w+\W`, `\p{L}+\P{L}`, `[a-z]+[0-9]`, `a++b`, `\X+\d`, with `PCRE2_UCP` on/off; byte-compare compiled code with `PCRE2_NO_AUTO_POSSESS` | [x] |
| 96 | `pcre2_compile_8`, `_pcre2_find_bracket_8` | after compiling `(a)(?<n>b)(?:c)((d))`, call `_pcre2_find_bracket_8(code, utf, N)` for N = 1..4, N = 0, N = 5 (not found → NULL), and `number < 0` (lookbehind search) on `(?<=a)b` | [x] |
| 97 | `pcre2_compile_8`, `_pcre2_study_8`, `pcre2_pattern_info_8` | patterns hitting each `set_start_bits` outcome: SSB_DONE (`abc`), SSB_CONTINUE (`a?bc`), SSB_FAIL (`.*x`, `\1(a)`, `(?R)`, `(*ACCEPT)`, `[^a]`), one-bit → `FIRSTSET` (`abc`), two-case-bits → `FIRSTSET+FIRSTCASELESS` (`(?i)abc`), many bits → `FIRSTMAPSET` (`[abc]x`); compare `FIRSTCODETYPE`, `FIRSTCODEUNIT`, all 256 `FIRSTBITMAP` bytes, `LASTCODETYPE`, `LASTCODEUNIT`, `MINLENGTH` | [x] |
| 98 | `pcre2_compile_8`, `_pcre2_study_8` | minlength edge cases: `a*a` (first/last unit collapse), `(?:a\|bb\|ccc)` (min 1), `(a)\1` with and without `PCRE2_MATCH_UNSET_BACKREF`, `(?\|(a)\|(bb))\1` (DUPCAPUSED → 0), `\X` (1), `\R` (1), `(?<=a)b`, 129 backrefs (over `MAX_CACHE_BACKREF`), a pattern exceeding the 1000-item complexity counter, `a{70000}` (UINT16_MAX clamp) | [x] |
| 99 | `pcre2_compile_8`, `pcre2_pattern_info_8` | `PCRE2_INFO_HASCRORLF` = 1 for `a\r`, `a\n`, `a\x0d`, `[\r]`; = 0 for `a\R`, `.`; `PCRE2_INFO_JCHANGED` = 1 for `(?J)(?<n>a)(?<n>b)`, 0 for `PCRE2_DUPNAMES` supplied externally | [x] |
| 100 | `pcre2_compile_8`, `pcre2_pattern_info_8` | `PCRE2_INFO_MATCHEMPTY` = 1 for `a*`, `(?:)`, `(?=a)`; = 0 for `a`, `a+` | [x] |

## E. pcre2_pattern_info / callout_enumerate

| # | entry point(s) | configuration (options set + input shape) | [x] |
|---|----------------|--------------------------------------------|-----|
| 101 | `pcre2_pattern_info_8` | `where == NULL` (field-length query) for every request code 0–26 — compare the returned length for each; and request 27 (unknown) | [x] |
| 102 | `pcre2_pattern_info_8` | `PCRE2_INFO_DEPTHLIMIT`/`MATCHLIMIT`/`HEAPLIMIT` on a pattern **without** `(*LIMIT_…)` (returns `PCRE2_ERROR_UNSET`) and **with** each directive present | [x] |
| 103 | `pcre2_pattern_info_8` | `PCRE2_INFO_FRAMESIZE` and `PCRE2_INFO_SIZE` for 0, 1, 10, 100 capture groups; `PCRE2_INFO_JITSIZE` with no JIT compiled (0) | [x] |
| 104 | `pcre2_pattern_info_8` | `PCRE2_INFO_NAMETABLE` + `NAMECOUNT` + `NAMEENTRYSIZE`: byte-compare the whole name table for 0 names, 1 short name, 3 names of mixed lengths, one 128-char name, and `PCRE2_DUPNAMES` duplicates | [x] |
| 105 | `pcre2_pattern_info_8` | `PCRE2_INFO_ALLOPTIONS` vs `ARGOPTIONS` vs `EXTRAOPTIONS` for a pattern that sets options both by argument and in-pattern (`(*UTF)(?i)` with `PCRE2_MULTILINE` passed in) | [x] |
| 106 | `pcre2_callout_enumerate_8` | pattern with 0 callouts (callback never fires); with 1 numeric callout; with mixed numeric + string callouts inside classes, lookarounds, and after `\x{100}` with `PCRE2_UTF` (exercises the `HAS_EXTRALEN`/`OP_PROP` skips); callback returning non-zero to abort early | [x] |

## F. pcre2_match — option and input-shape axes

| # | entry point(s) | configuration (options set + input shape) | [x] |
|---|----------------|--------------------------------------------|-----|
| 107 | `pcre2_match_8`, `pcre2_match_data_create_8` | `subject=NULL`, `length=0` (internal `null_str` substitute), pattern `a*`; check `rc`, `ovector[0..1]`, `pcre2_get_startchar` | [x] |
| 108 | `pcre2_match_8` | `length=PCRE2_ZERO_TERMINATED` vs explicit length, subject with an embedded NUL `"a\0b"`; pattern `\0` and `.` | [x] |
| 109 | `pcre2_match_8` | `startoffset` = 0, 1, mid-subject, `== length`; pattern `a*` and `\G a` and `^a` (with/without `PCRE2_MULTILINE`) | [x] |
| 110 | `pcre2_match_8`, `pcre2_get_ovector_count_8`, `pcre2_get_ovector_pointer_8` | pattern `(a)(b)(c)` (top_bracket 3): `pcre2_match_data_create(0,…)` (clamped to 1), `(1,…)`, `(2,…)` → `rc==0` overflow, `(4,…)` exact, `(10,…)` oversized; byte-compare all ovector slots including the `PCRE2_UNSET` fill | [x] |
| 111 | `pcre2_match_data_create_8` | `oveccount=0` → 1, `=1`, `=UINT16_MAX`, `=UINT16_MAX+1` → clamped; compare `pcre2_get_match_data_size` for each | [x] |
| 112 | `pcre2_match_data_create_from_pattern_8` | patterns with 0, 1, 3, 1000 capture groups; `gcontext=NULL` (allocator taken from the code) vs a custom general context | [x] |
| 113 | `pcre2_match_8` | `PCRE2_NOTBOL`, `PCRE2_NOTEOL`, both; patterns `^a`, `a$`, `^a$`, `(?m)^a`, `(?m)a$`; subject `"a"`, `"a\n"`, `"b\na"` | [x] |
| 114 | `pcre2_match_8` | `PCRE2_NOTEMPTY` vs `PCRE2_NOTEMPTY_ATSTART` vs neither; pattern `a*`, subject `"bba"`, startoffset 0 and 2 | [x] |
| 115 | `pcre2_match_8`, `pcre2_compile_8` | pattern-level `(*NOTEMPTY)` and `(*NOTEMPTY_ATSTART)` with the corresponding match option *not* supplied (flags are folded into `options`) | [x] |
| 116 | `pcre2_match_8` | `PCRE2_PARTIAL_SOFT` on pattern `abc`, subjects `"ab"`, `"abc"`, `"xab"`; then `PCRE2_PARTIAL_HARD` on the same; then both set (HARD wins); check `ovector[0]`, `ovector[1]==length`, `pcre2_get_startchar` | [x] |
| 117 | `pcre2_match_8` | partial match with a lookbehind pattern `(?<=abc)d` (`allowemptypartial` via `max_lookbehind>0`) and with `PCRE2_INFO_MATCHEMPTY` pattern `a*`; empty partial allowed | [x] |
| 118 | `pcre2_match_8` | `PCRE2_NO_UTF_CHECK` with `PCRE2_UTF` and a valid subject; and with `PCRE2_MATCH_INVALID_UTF` compiled in (the check still runs) | [x] |
| 119 | `pcre2_match_8` | `PCRE2_UTF` pattern, `startoffset` pointing at a continuation byte of a 2/3/4-byte character: without `NO_UTF_CHECK` → `PCRE2_ERROR_BADUTFOFFSET`; with `PCRE2_MATCH_INVALID_UTF` → the bad start is skipped forward | [x] |
| 120 | `pcre2_match_8` | `PCRE2_MATCH_INVALID_UTF` fragment matching: subject `valid + 0xFF + valid`, patterns `^a` (NOTBOL applied to later fragments), `a$` (NOTEOL applied), `.+`; startoffset 0, before and after the bad byte | [x] |
| 121 | `pcre2_match_8`, `pcre2_match_data_free_8` | `PCRE2_COPY_MATCHED_SUBJECT` on a successful match (compare the returned `subject` pointer differs and contents equal); with `length==0` (subject becomes NULL, flag still set); reusing the same match_data for a second match (previous copy freed) | [x] |
| 122 | `pcre2_match_8` | `PCRE2_NO_JIT` (no-op in this build) and `PCRE2_DISABLE_RECURSELOOP_CHECK` on pattern `(a*)*` / `(?R)`-style self recursion `(a\|(?R))*`; without the flag a `PCRE2_ERROR_RECURSELOOP` is possible, with it the match_limit terminates instead | [x] |
| 123 | `pcre2_match_8`, `pcre2_set_match_limit_8` | limit = 1, 10, exactly the count a known pattern needs, `UINT32_MAX`; pattern `(a+)+b` on subject `"aaaaaaaaaaaaaaaaaaaac"` | [x] |
| 124 | `pcre2_match_8`, `pcre2_set_depth_limit_8`, `pcre2_set_recursion_limit_8` | depth limit 1, 10, default; a deeply nested pattern `(((((…a…)))))`; verify `pcre2_set_recursion_limit` is an exact synonym (same resulting `PCRE2_INFO_DEPTHLIMIT` behaviour) | [x] |
| 125 | `pcre2_match_8`, `pcre2_set_heap_limit_8`, `pcre2_get_match_data_heapframes_size_8` | heap limit 0, 1, 10, 20000000 (default) on a pattern with 500 capture groups (large `frame_size`); check the pre-match `PCRE2_ERROR_HEAPLIMIT` when `1024*limit < frame_size`, and the frame-vector growth/reuse across two matches (`heapframes_size` non-decreasing) | [x] |
| 126 | `pcre2_match_8`, `pcre2_set_offset_limit_8` | `PCRE2_USE_OFFSET_LIMIT` compiled; offset limit 0, 2, `length-1`, `length`, `PCRE2_UNSET`; pattern `b`, subject `"aaab"`; also with `PCRE2_ANCHORED` (limit irrelevant) | [x] |
| 127 | `pcre2_match_8` | start optimizations enabled (default): patterns with `PCRE2_FIRSTSET` (`abc`), `FIRSTSET+FIRSTCASELESS` (`(?i)abc`), `FIRSTMAPSET` (`[bc]d`), `STARTLINE` (`(?m)^x`), `LASTSET` req-code-unit (`a.*z`); each on a long subject (> `REQ_CU_MAX`) and a short one; compare `pcre2_get_startchar` | [x] |
| 128 | `pcre2_match_8` | `PCRE2_FIRSTLINE` compiled: subject with the newline before, at, and after the only possible match; newline conventions LF, CRLF and ANY; combined with `PCRE2_PARTIAL_SOFT` | [x] |
| 129 | `pcre2_match_8` | bumpalong CRLF skip: pattern `.` with `PCRE2_NEWLINE_CRLF`/`ANY`/`ANYCRLF`, subject `"a\r\nb"`, startoffset 0 and 1, and a pattern containing `\r` (`PCRE2_HASCRORLF` suppresses the skip) | [x] |
| 130 | `pcre2_match_8`, `pcre2_get_mark_8` | `(*MARK:one)a\|(*MARK:two)b` on subjects `"a"`, `"b"`, `"c"`; `(*SKIP:x)` (does not set nomatch_mark) vs `(*THEN:x)`/`(*PRUNE:x)`/`(*COMMIT:x)` (do); mark after NOMATCH vs after success | [x] |
| 131 | `pcre2_match_8` | verb control flow: `a(*COMMIT)b` on `"xab"` (no bumpalong), `a(*PRUNE)b`, `a(*SKIP)b`, `a(*SKIP:m)b` with the mark never set (retry at the same position), `(*THEN)`, `(*ACCEPT)` inside a group, `(*FAIL)` | [x] |
| 132 | `pcre2_match_8` | `PCRE2_ENDANCHORED` at match time with `(*ACCEPT)` mid-pattern (returns without backtracking) vs at OP_END (backtracks) | [x] |
| 133 | `pcre2_match_8`, `pcre2_get_startchar_8` | `\K` moving `ovector[0]` backwards inside a lookaround, with `PCRE2_EXTRA_ALLOW_LOOKAROUND_BSK` set (legal) and the resulting `ovector[0] < startoffset`; also `\K` producing `ovector[0] > ovector[1]` | [x] |
| 134 | `pcre2_match_8`, `pcre2_set_callout_8` | callout returning 0 (continue), > 0 (fail this path), < 0 (abandon with that code); pattern `(?C1)a(?C2)b`, subject `"ab"`, `"xb"`; byte-compare every field of the `pcre2_callout_block` (version 2, `callout_flags` STARTMATCH/BACKTRACK, `capture_top`, `capture_last`, `offset_vector`, `mark`, `pattern_position`, `next_item_length`) on each invocation | [x] |
| 135 | `pcre2_match_8`, `pcre2_set_callout_8` | callout with `PCRE2_NO_START_OPTIMIZE` (fires at every start offset) vs without; and `PCRE2_AUTO_CALLOUT`-compiled pattern with a counting callout | [x] |
| 136 | `pcre2_match_8` | `mcontext=NULL` (defaults from `_pcre2_default_match_context_8`) vs a fresh `pcre2_match_context_create(NULL)` vs a copied context; all three must give identical results | [x] |
| 137 | `pcre2_match_context_create_8`, `pcre2_match_context_copy_8`, `pcre2_match_context_free_8`, `pcre2_set_recursion_memory_management_8` | create/copy/free round trip, byte-compare the blocks; `set_recursion_memory_management` is a documented no-op returning 0 | [x] |
| 138 | `pcre2_match_8` | match_data reuse: run three matches with the same match_data over a pattern/subject pair, checking `rc`, `mark`, `startchar`, `subject`, `subject_length`, `start_offset`, `options` after each; include a NOMATCH between two matches | [x] |
| 139 | `pcre2_match_8` | UTF subject where `max_lookbehind > 0`: `check_subject` rewind. Pattern `(?<=\x{100}\x{100})a`, subject with the lookbehind text *before* `startoffset`, with and without `PCRE2_NO_UTF_CHECK` (rewind only happens when the check runs) | [x] |
| 140 | `pcre2_jit_compile_8`, `pcre2_match_8`, `pcre2_jit_match_8`, `pcre2_jit_free_unused_memory_8`, `pcre2_jit_stack_create_8`, `pcre2_jit_stack_assign_8`, `pcre2_jit_stack_free_8`, `_pcre2_jit_get_size_8`, `_pcre2_jit_get_target_8`, `_pcre2_jit_free_8`, `_pcre2_jit_free_rodata_8` | no-JIT build: `pcre2_jit_compile(code, PCRE2_JIT_COMPLETE)` → `PCRE2_ERROR_JIT_UNSUPPORTED` (also for `PARTIAL_SOFT`, `PARTIAL_HARD`, `INVALID_UTF`, and an unknown bit); `pcre2_jit_match` → `PCRE2_ERROR_JIT_BADOPTION`; `pcre2_jit_stack_create(1,1,NULL)` → NULL; the free/unused-memory entry points are no-ops; `PCRE2_INFO_JITSIZE` == 0 | [x] |

## G. pcre2_dfa_match

| # | entry point(s) | configuration (options set + input shape) | [x] |
|---|----------------|--------------------------------------------|-----|
| 141 | `pcre2_dfa_match_8` | workspace `wscount` = 20 (minimum), 21, 100, 1000; pattern `(a\|ab\|abc)` on `"abc"`; also a pattern/subject pair that exhausts a 20-int workspace at *run* time (`PCRE2_ERROR_DFA_WSSIZE` from ADD_ACTIVE) | [x] |
| 142 | `pcre2_dfa_match_8` | multiple-match ovector semantics: pattern `a\|ab\|abc` on `"abcd"` with `oveccount` = 1, 2, 3, 4, 10 — verify matches are returned **longest first**, and `rc == 0` when they overflow | [x] |
| 143 | `pcre2_dfa_match_8` | `PCRE2_DFA_SHORTEST` on the same pattern/subject (single, shortest match) with and without `PCRE2_ANCHORED` | [x] |
| 144 | `pcre2_dfa_match_8` | `PCRE2_DFA_RESTART`: first call with `PCRE2_PARTIAL_HARD` on a truncated subject, then a restart call on the continuation with the *same* workspace; also validate `workspace[0]&~1==0` / `workspace[1]` boundary values 1 and `(wscount-2)/INTS_PER_STATEBLOCK`; note RESTART forces anchored and disables start optimizations | [x] |
| 145 | `pcre2_dfa_match_8` | `PCRE2_PARTIAL_SOFT` vs `PARTIAL_HARD` on `abc` with `"ab"`, `"abc"`, `"abcx"`; plus partial at a newline (`partial_newline`) with `PCRE2_NEWLINE_CRLF` | [x] |
| 146 | `pcre2_dfa_match_8` | `PCRE2_NOTBOL`, `PCRE2_NOTEOL`, `PCRE2_NOTEMPTY`, `PCRE2_NOTEMPTY_ATSTART`, `PCRE2_ANCHORED`, `PCRE2_ENDANCHORED`, `PCRE2_COPY_MATCHED_SUBJECT`, `PCRE2_NO_UTF_CHECK` — one row per bit on a common pattern/subject pair (note `PCRE2_DISABLE_RECURSELOOP_CHECK` and `PCRE2_NO_JIT` are **not** accepted here) | [x] |
| 147 | `pcre2_dfa_match_8` | constructs the DFA *accepts* by recursing: `(?=abc)a`, `(?!x)a`, `(?<=a)b`, `(?<!a)b`, `(?>ab)`, `(a)++`/`(?:a)*+` possessive groups, `(a(?1)?b)` recursion, `(?(?=a)b\|c)` assertion condition, `(?(DEFINE)(?<x>a))(?&x)`, `(?(R)a\|b)` | [x] |
| 148 | `pcre2_dfa_match_8` | UTF: `PCRE2_UTF` pattern with 2/3/4-byte characters, `startoffset` 0 and at a character boundary; `\X` on a grapheme cluster; `.` with and without `PCRE2_DOTALL` | [x] |
| 149 | `pcre2_dfa_match_8`, `pcre2_set_callout_8` | callout returning 0, > 0 (kills only that thread), < 0 (abandons); verify the DFA callout block has `capture_top == 1`, `capture_last == 0`, `mark == NULL`; plus the `OP_COND` auto-callout path with `PCRE2_AUTO_CALLOUT` | [x] |
| 150 | `pcre2_dfa_match_8`, `pcre2_set_heap_limit_8`, `pcre2_set_match_limit_8`, `pcre2_set_depth_limit_8` | recursive-workspace (RWS) growth bounded by heap limit 1 / 10 / default on a deeply nested lookaround pattern; match limit 1/10; depth limit 1/10 (depth = nested `internal_dfa_match` calls) | [x] |
| 151 | `pcre2_dfa_match_8` | `mcontext=NULL` (offset limit never consulted) vs `mcontext` with `offset_limit` set and pattern compiled `PCRE2_USE_OFFSET_LIMIT`, values 0 / 2 / `length` / `PCRE2_UNSET` | [x] |
| 152 | `pcre2_dfa_match_8`, `pcre2_get_startchar_8`, `pcre2_get_ovector_pointer_8` | after a DFA match compare `subject_length`, `start_offset`, `startchar`, `matchedby == PCRE2_MATCHEDBY_DFA_INTERPRETER`, `mark == NULL`, and `options == original_options`; then the same fields after PARTIAL and after NOMATCH | [x] |
| 153 | `pcre2_dfa_match_8` | `offsetcount` odd (rounded down to even): `oveccount` 1 and 3 with a pattern producing 2 and 3 matches | [x] |

## H. pcre2_next_match

| # | entry point(s) | configuration (options set + input shape) | [x] |
|---|----------------|--------------------------------------------|-----|
| 154 | `pcre2_next_match_8`, `pcre2_match_8` | after a **non-empty** match of `a` on `"aba"` startoffset 0: returns TRUE, `*pstart_offset == ovector[1]`, `*poptions == 0`; then drive the full global loop to exhaustion | [x] |
| 155 | `pcre2_next_match_8`, `pcre2_match_8` | after an **empty** match of `a*` on `"bb"` at offset 0: TRUE, same offset, `*poptions == PCRE2_NOTEMPTY_ATSTART`; and an empty match at `offset == subject_length` → FALSE | [x] |
| 156 | `pcre2_next_match_8`, `pcre2_match_8` | after NOMATCH, after `PCRE2_ERROR_PARTIAL`, and after any hard error (`rc < 0`) → FALSE in every case, output pointers untouched | [x] |
| 157 | `pcre2_next_match_8`, `pcre2_match_8` | the `\K` bumpalong branch: `ovector[0] != start_offset && ovector[1] == start_offset` via `(?<=a\K)` with `PCRE2_EXTRA_ALLOW_LOOKAROUND_BSK`; `start_offset < subject_length` (bumpalong) and `== subject_length` (FALSE) | [x] |
| 158 | `pcre2_next_match_8` | `do_bumpalong` variants: subject `"a\r\nb"` with `PCRE2_NEWLINE_CRLF`, `ANY`, `ANYCRLF` (offset+2) vs `CR`, `LF`, `NUL` (offset+1); and `PCRE2_UTF` with a 2/3/4-byte character at the bump position | [x] |
| 159 | `pcre2_next_match_8`, `pcre2_dfa_match_8` | after a DFA match with several ovector pairs — only pair 0 is inspected; drive a global loop over `a\|ab` on `"abab"` | [x] |

## I. pcre2_substitute

| # | entry point(s) | configuration (options set + input shape) | [x] |
|---|----------------|--------------------------------------------|-----|
| 160 | `pcre2_substitute_8` | baseline: `options=0`, pattern `b`, subject `"abc"` (`PCRE2_ZERO_TERMINATED`), replacement `"X"`, buffer 16; check return 1, `*blength`, buffer bytes and NUL | [x] |
| 161 | `pcre2_substitute_8` | `PCRE2_SUBSTITUTE_GLOBAL` on pattern `a` / subject `"aaa"`; and on empty-matching pattern `a*` / subject `"bab"` (empty-match advance via `NOTEMPTY_ATSTART` retry) | [x] |
| 162 | `pcre2_substitute_8` | `PCRE2_SUBSTITUTE_REPLACEMENT_ONLY` alone; with `GLOBAL`; with `startoffset > 0` (the pre-offset text must be absent) | [x] |
| 163 | `pcre2_substitute_8` | `PCRE2_SUBSTITUTE_REPLACEMENT_ONLY+PCRE2_PARTIAL_SOFT` and `+PCRE2_PARTIAL_HARD` on pattern `abc`, subject `"ab"` (the only legal way to pass PARTIAL_*) | [x] |
| 164 | `pcre2_substitute_8` | `PCRE2_SUBSTITUTE_LITERAL` with a replacement containing `$1`, `\n`, `\Q`, `$$` — all must be copied verbatim; also `LITERAL+EXTENDED` (EXTENDED inert) | [x] |
| 165 | `pcre2_substitute_8` | `$` forms without EXTENDED: `$$`, `$&`, `` $` ``, `$'`, `$_`, `$1`, `$12`, `${1}`, `${name}`, `$name`, `$<name>`, `$+`, `$*MARK`, `${*MARK}` — one row each on pattern `(?<name>b)(c)?` / subject `"abc"` | [x] |
| 166 | `pcre2_substitute_8` | `$+` sub-cases: pattern with `top_bracket == 0`; pattern `(a)(b)?` where only group 1 is set; a match_data with `oveccount < top_bracket+1` (`PCRE2_ERROR_UNAVAILABLE`); all groups unset with and without `PCRE2_SUBSTITUTE_UNSET_EMPTY` | [x] |
| 167 | `pcre2_substitute_8` | `PCRE2_SUBSTITUTE_UNSET_EMPTY` with an unset group `(a)(b)?` and replacement `"[$2]"`; then without it (`PCRE2_ERROR_UNSET` with `*blength` = offset in the replacement) | [x] |
| 168 | `pcre2_substitute_8` | `PCRE2_SUBSTITUTE_UNKNOWN_UNSET` alone (unknown group → `PCRE2_ERROR_UNSET`) and `+UNSET_EMPTY` (→ empty); replacement `"$9"` and `"${nosuch}"` on a 1-group pattern | [x] |
| 169 | `pcre2_substitute_8` | `PCRE2_SUBSTITUTE_EXTENDED` escapes producing one character, one row each: `\a \e \f \n \r \t \b \v \0 \07 \077 \o{101} \x41 \x{1F600}` (with UTF) `\cA`, and `\$ \\ \{ \} \[ \] \( \) \^ \? \* \+ \| \. \_ \: \; \< \= \> \@` | [x] |
| 170 | `pcre2_substitute_8` | `PCRE2_SUBSTITUTE_EXTENDED` `\Q…\E` literal spans: `"\\Q$1\\E"`, an unterminated `\Q` (state persists into the next global iteration), `\E` with no `\Q` | [x] |
| 171 | `pcre2_substitute_8` | `PCRE2_SUBSTITUTE_EXTENDED` case forcing with **no** case callout: `\U…\E`, `\L…\E`, `\u`, `\l`, `\u\L…\E`, `\l\U…\E`, `\u` at the very end of the replacement (digraph not formed), and case forcing applied to a `$1` expansion and to `$*MARK` | [x] |
| 172 | `pcre2_substitute_8` | case forcing over non-ASCII: `PCRE2_UTF` pattern, replacement `"\\U$1"` where `$1` is `\x{e9}` / `\x{1f0}` / a character with `UCD_CASESET != 0`; and `PCRE2_UCP` without UTF over bytes 0x80–0xFF | [x] |
| 173 | `pcre2_substitute_8`, `pcre2_set_substitute_case_callout_8` | a case callout returning: a length `<= ch1_cap`, a length in `(ch1_cap, max_ch1_cap]` (forces the memmove retry loop), a length `> max_ch1_cap` (ch1_overflow, second half called with a 0-capacity buffer), and `~(PCRE2_SIZE)0` (`PCRE2_ERROR_REPLACECASE`); verify the callout only ever receives `to_case` 1/2/3 | [x] |
| 174 | `pcre2_substitute_8`, `pcre2_set_substitute_case_callout_8` | with a case callout installed, `\u\L`, `\l\U` and `\u`/`\l` single-char modes are decomposed — record the exact sequence of (input, to_case) pairs the callout sees | [x] |
| 175 | `pcre2_substitute_8` | `PCRE2_SUBSTITUTE_EXTENDED` backreference escapes: `\1`…`\9`, a multi-digit `\10` on a 12-group pattern, `\g{1}`, `\g<1>`, `\g<0>`, `\g<name>` | [x] |
| 176 | `pcre2_substitute_8` | `PCRE2_SUBSTITUTE_EXTENDED` conditional forms: `${1:-default}` group set and unset; `${1:+ifset}`; `${1:+ifset:ifnotset}`; `${1:+:}` (both parts empty); a `${…:-…}` whose default itself contains `$2` and `\U` (recursive reprocessing); nesting 1, 5 and 10 deep (`PTR_STACK_SIZE == 20` → max 10) | [x] |
| 177 | `pcre2_substitute_8` | `${1:-…}` where the default text contains `\Q…\E` spanning a `:` and a `}` (terminator scanning must ignore them), and one containing `\L`/`\u` | [x] |
| 178 | `pcre2_substitute_8` | buffer sizing: `*blength` exactly `needed+1` (success), exactly `needed` (overflow), 1, 0; `buffer=NULL` + `*blength=0`; with and without `PCRE2_SUBSTITUTE_OVERFLOW_LENGTH` | [x] |
| 179 | `pcre2_substitute_8` | `PCRE2_SUBSTITUTE_OVERFLOW_LENGTH` two-pass protocol: first call with length 0 to obtain the required size, then a second call with exactly that size must succeed; repeat with `GLOBAL` and with a case-forced replacement (where `pessimistic_case_inflation` may need a third call) | [x] |
| 180 | `pcre2_substitute_8` | overflow at each distinct `CHECKMEMCPY` site — pre-startoffset copy, inter-match fragment, whole `LITERAL` replacement, group contents, `${*MARK}`, an escape-produced char, a literal code unit, the trailing tail, and the trailing NUL — by sizing the buffer one unit short of each | [x] |
| 181 | `pcre2_substitute_8` | `match_data=NULL` (internal block created from the pattern) vs an external match_data with `oveccount == top_bracket+1` vs one with `oveccount < top_bracket+1` (rc normalised to `oveccount`; affects `$+` and duplicate-name resolution); check `match_data->rc` is written only in the external non-MATCHED case | [x] |
| 182 | `pcre2_substitute_8`, `pcre2_match_8` | `PCRE2_SUBSTITUTE_MATCHED` with a prior successful `pcre2_match`; with a prior `PCRE2_ERROR_NOMATCH` (0 substitutions, output == subject); with a prior `rc == 0` (ovector too small); `MATCHED+GLOBAL` (first substitution from the supplied match, the rest from fresh matches) | [x] |
| 183 | `pcre2_substitute_8` | `PCRE2_SUBSTITUTE_MATCHED` where the subject is supplied by pointer identity, and where it comes from a `PCRE2_COPY_MATCHED_SUBJECT` match_data (content-comparison path); verify the caller's match_data is never mutated | [x] |
| 184 | `pcre2_substitute_8`, `pcre2_set_substitute_callout_8` | substitute callout returning 0 (accept), > 0 (reject: original text restored, still counted) and < 0 (reject and stop the global loop); with and without `REPLACEMENT_ONLY` (which drops the matched text on rejection); byte-compare `version`, `input`, `output`, `ovector`, `oveccount`, `subscount`, `output_offsets[0..1]` on each call | [x] |
| 185 | `pcre2_substitute_8`, `pcre2_set_substitute_callout_8` | `PCRE2_SUBSTITUTE_OVERFLOW_LENGTH` + a rejecting callout: the callout is not called during the overflow pass; the reported length must be large enough for a successful second call | [x] |
| 186 | `pcre2_substitute_8` | `subject=NULL`+`length=0`; `replacement=NULL`+`rlength=0`; `rlength=PCRE2_ZERO_TERMINATED` vs explicit; a replacement containing an embedded NUL with explicit `rlength`; `startoffset` 0, mid-subject, `== length` | [x] |
| 187 | `pcre2_substitute_8` | `PCRE2_UTF` pattern with `PCRE2_NO_UTF_CHECK` clear (replacement is UTF-validated once) vs set; and `GLOBAL` where the flag is forced on internally after the first iteration | [x] |
| 188 | `pcre2_substitute_8` | non-substitute match options forwarded through to `pcre2_match`: `PCRE2_NOTBOL`, `PCRE2_NOTEOL`, `PCRE2_NOTEMPTY`, `PCRE2_NOTEMPTY_ATSTART`, `PCRE2_ANCHORED`, `PCRE2_ENDANCHORED`, `PCRE2_COPY_MATCHED_SUBJECT` (stripped when an internal match_data is used) | [x] |

## J. pcre2_pattern_convert

| # | entry point(s) | configuration (options set + input shape) | [x] |
|---|----------------|--------------------------------------------|-----|
| 189 | `pcre2_pattern_convert_8`, `pcre2_converted_pattern_free_8` | the three buffer modes on the same input: (a) `buffptr=NULL` (length-only query), (b) `*buffptr=NULL` (library allocates, freed with `pcre2_converted_pattern_free`), (c) caller buffer with `*bufflenptr` = needed+1 (exact fit), needed (one short → `PCRE2_ERROR_NOMEMORY`), 0 | [x] |
| 190 | `pcre2_pattern_convert_8` | `PCRE2_CONVERT_POSIX_BASIC` (BRE) inputs: `a*`, `*a`, `^*`, `^a`, `a^b`, `\(a\)\1`, `a\{2,3\}`, `a?`, `a+`, `a\|b`, `a{b`, `.`, `$`, `a$b`, `\*`, `\.`, `\\` | [x] |
| 191 | `pcre2_pattern_convert_8` | `PCRE2_CONVERT_POSIX_EXTENDED` (ERE) inputs: `(a\|b)+`, `a{2,3}`, `^a$`, `a)b` (unmatched `)` → literal), `(a`, `?a`, `**a`, `\?`, `\(`, `.` | [x] |
| 192 | `pcre2_pattern_convert_8` | POSIX bracket expressions in both dialects: `[[:alpha:]]`, `[[:digit:][:space:]]`, `[[:foo:]]` (unknown name), `[[:alpha]`, `[]]`, `[^]]`, `[]abc]`, `[a-z]`, `[\]`, `[a:b]`, an unterminated `[` | [x] |
| 193 | `pcre2_pattern_convert_8` | `PCRE2_CONVERT_GLOB` defaults (separator `/`, escape `\`): `*`, `**`, `a*b`, `*a`, `a*`, `?`, `a?b`, `**/foo`, `a/**/b`, `a/**`, `***`, `\*`, `\\`, a trailing `\`, plain `abc` | [x] |
| 194 | `pcre2_pattern_convert_8` | `PCRE2_CONVERT_GLOB+PCRE2_CONVERT_GLOB_NO_WILD_SEPARATOR`: `*`, `*a`, `a*`, `?`, `**`, `[!a]` — the generated `[^sep]`/`(?<!sep)` fragments must become `.` | [x] |
| 195 | `pcre2_pattern_convert_8` | `PCRE2_CONVERT_GLOB+PCRE2_CONVERT_GLOB_NO_STARSTAR`: `**`, `**/a`, `a/**/b`, `***` (collapse to a single `*`); and both GLOB_NO_* bits together | [x] |
| 196 | `pcre2_pattern_convert_8`, `pcre2_set_glob_separator_8` | separator `/` (`with_escape` FALSE) vs `\` and `.` (`with_escape` TRUE) — each on `*`, `?`, `[!a]`, `**/a`, `a/**/b`; and separator `\` with the default escape `\` (separator == escape) | [x] |
| 197 | `pcre2_pattern_convert_8`, `pcre2_set_glob_escape_8` | escape `0` (no escape character at all: `\` becomes an ordinary member), escape `\`, escape `` ` ``, escape `!`, escape `~`; each on `\*`, `[\]]`, `a\b`, trailing escape | [x] |
| 198 | `pcre2_pattern_convert_8` | glob bracket expressions: `[abc]`, `[!abc]`, `[^abc]`, `[]abc]`, `[!]abc]`, `[a-z]`, `[z-a]` reversed, `[a-c-e]`, `[-a]`, `[a-]`, `[/]` (separator member → `(?<!/)` lookbehind), `[!/]`, `[[:alpha:]]`, `[[:foo:]]`, `[[:alpha:]-z]`, an unterminated `[`, an escaped member `[\]]` | [x] |
| 199 | `pcre2_pattern_convert_8` | `PCRE2_CONVERT_UTF` with a valid multi-byte pattern (validated) and `+PCRE2_CONVERT_NO_UTF_CHECK` (skipped); glob and both POSIX types; multi-byte characters as class members and as range endpoints | [x] |
| 200 | `pcre2_pattern_convert_8` | `pattern=NULL`+`plength=0`; `plength=PCRE2_ZERO_TERMINATED` vs explicit length; `ccontext=NULL` (defaults from `_pcre2_default_convert_context_8`) vs a created/copied convert context | [x] |
| 201 | `pcre2_convert_context_create_8`, `pcre2_convert_context_copy_8`, `pcre2_convert_context_free_8` | create with `gcontext=NULL` and with a custom general context; set separator+escape, copy, and byte-compare both blocks | [x] |
| 202 | `pcre2_pattern_convert_8`, `pcre2_compile_8`, `pcre2_match_8` | end-to-end: convert each of the three types, compile the result, and match a subject — confirming the converted pattern is itself valid PCRE2 | [x] |

## K. Substring extraction

| # | entry point(s) | configuration (options set + input shape) | [x] |
|---|----------------|--------------------------------------------|-----|
| 203 | `pcre2_substring_length_bynumber_8` | `stringnumber` 0, 1, `top_bracket`, `top_bracket+1` (`NOSUBSTRING`), `>= oveccount` (`UNAVAILABLE`), an unset group (`UNSET`); `sizeptr=NULL` (validity check only); after a match with `rc == 0` (ovector too small) | [x] |
| 204 | `pcre2_substring_length_bynumber_8`, `pcre2_dfa_match_8` | after a DFA match: `stringnumber` 0 with `count != 0`, `stringnumber >= count` (`UNSET`), `stringnumber >= oveccount` (`UNAVAILABLE`) | [x] |
| 205 | `pcre2_substring_length_bynumber_8` | after `PCRE2_ERROR_PARTIAL`: `stringnumber == 0` (allowed, uses `count = 0`) vs `> 0` (`PCRE2_ERROR_PARTIAL`) | [x] |
| 206 | `pcre2_substring_copy_bynumber_8`, `pcre2_substring_free_8` | buffer size exactly `len+1` (success), exactly `len` (`NOMEMORY`), 0; a zero-length capture; a capture containing an embedded NUL | [x] |
| 207 | `pcre2_substring_get_bynumber_8`, `pcre2_substring_free_8` | groups 0, 1, an unset group, a zero-length capture, `gcontext` from the match data; then free | [x] |
| 208 | `pcre2_substring_length_byname_8`, `pcre2_substring_copy_byname_8`, `pcre2_substring_get_byname_8` | unique name; name not present (`NOSUBSTRING`); `PCRE2_DUPNAMES` with the first duplicate unset and a later one set (must pick the set one); all duplicates unset; a duplicate whose number `>= oveccount` | [x] |
| 209 | `pcre2_substring_copy_byname_8`, `pcre2_dfa_match_8` | any `*_byname` call on a DFA-produced match_data → `PCRE2_ERROR_DFA_UFUNC` (one row per by-name entry point) | [x] |
| 210 | `pcre2_substring_nametable_scan_8` | `firstptr`/`lastptr` both NULL (returns the group number, or `NOUNIQUESUBSTRING` for duplicates) vs both supplied (returns `entrysize` and the first/last table entries); name not present; 1, 2 and 3 duplicates; a name that is a prefix of another | [x] |
| 211 | `pcre2_substring_number_from_name_8` | unique name, unknown name, duplicate name (`NOUNIQUESUBSTRING`); pattern with 0 named groups | [x] |
| 212 | `pcre2_substring_list_get_8`, `pcre2_substring_list_free_8` | `lengthsptr=NULL` vs supplied; pattern with 0 groups; with 3 groups where the middle one is unset; after `rc == 0` (uses `oveccount`); captures containing embedded NULs; then free | [x] |
| 213 | `pcre2_substring_free_8`, `pcre2_substring_list_free_8` | called with NULL (must be no-ops) | [x] |

## L. Serialization

| # | entry point(s) | configuration (options set + input shape) | [x] |
|---|----------------|--------------------------------------------|-----|
| 214 | `pcre2_serialize_encode_8`, `pcre2_serialize_get_number_of_codes_8`, `pcre2_serialize_decode_8`, `pcre2_serialize_free_8` | 1 code compiled with default options; byte-compare the whole serialized blob, then decode and byte-compare the decoded code block against the original | [x] |
| 215 | `pcre2_serialize_encode_8`, `pcre2_serialize_decode_8` | `number_of_codes` = 1, 2, 5 with mixed patterns (UTF, named groups, callouts, `PCRE2_ALT_EXTENDED_CLASS`); decode all, then decode a subset (`number_of_codes < data->number_of_codes`) | [x] |
| 216 | `pcre2_serialize_encode_8` | all codes sharing `pcre2_maketables` tables (succeeds); all codes with `tables == NULL` default; a mix of default-tables and custom-tables codes (`PCRE2_ERROR_MIXEDTABLES`) | [x] |
| 217 | `pcre2_serialize_encode_8`, `pcre2_serialize_decode_8` | `gcontext=NULL` vs a custom general context (allocation trace compared); `pcre2_serialize_free(NULL)` no-op | [x] |
| 218 | `pcre2_serialize_decode_8`, `pcre2_match_8` | decode into a `codes` array, match with each decoded code, and confirm `pcre2_pattern_info` for every request code matches the original; then `pcre2_code_free` each | [x] |
| 219 | `pcre2_serialize_get_number_of_codes_8` | on a blob produced from 1, 3 and 10 codes — return value must equal the count without decoding | [x] |

## M. pcre2_config, error messages, match-data accessors

| # | entry point(s) | configuration (options set + input shape) | [x] |
|---|----------------|--------------------------------------------|-----|
| 220 | `pcre2_config_8` | `where == NULL` (size query) for every code 0–16; then the value for each of `PCRE2_CONFIG_BSR`, `COMPILED_WIDTHS`, `DEPTHLIMIT`, `EFFECTIVE_LINKSIZE`, `HEAPLIMIT`, `JIT`, `LINKSIZE`, `MATCHLIMIT`, `NEVER_BACKSLASH_C`, `NEWLINE`, `PARENSLIMIT`, `STACKRECURSE`, `TABLES_LENGTH`, `UNICODE` — one row per code | [x] |
| 221 | `pcre2_config_8` | string codes `PCRE2_CONFIG_JITTARGET` (no JIT), `PCRE2_CONFIG_UNICODE_VERSION`, `PCRE2_CONFIG_VERSION`: first with `where=NULL` to get the length, then into an exact-size buffer; byte-compare the strings | [x] |
| 222 | `pcre2_get_error_message_8` | every compile error 101–220 and every match/UTF error −1…−76 into a 256-byte buffer; byte-compare each message and its returned length | [x] |
| 223 | `pcre2_get_error_message_8` | buffer exactly `len+1` (success), exactly `len` (`PCRE2_ERROR_NOMEMORY` with a truncated NUL-terminated message), size 1, size 0 (`NOMEMORY`); error number 0 and 100 (invalid → `PCRE2_ERROR_BADDATA`) | [x] |
| 224 | `pcre2_get_match_data_size_8`, `pcre2_get_ovector_count_8`, `pcre2_get_match_data_heapframes_size_8` | for `oveccount` 1, 4, 100, `UINT16_MAX`: before any match (heapframes 0) and after a match on a pattern with many groups (heapframes non-zero, and unchanged by a second smaller match) | [x] |
| 225 | `pcre2_get_mark_8`, `pcre2_get_startchar_8`, `pcre2_get_ovector_pointer_8` | on a fresh match_data (before any match), after success, after NOMATCH, after PARTIAL, after a hard error — one row per state | [x] |
| 226 | `pcre2_maketables_8`, `pcre2_maketables_free_8` | `gcontext=NULL` (malloc) and a custom general context; byte-compare all 1088 bytes against `_pcre2_default_tables_8` in the "C" locale; free through both paths | [x] |

## N. Low-level exported `_pcre2_*` helpers

| # | entry point(s) | configuration (options set + input shape) | [x] |
|---|----------------|--------------------------------------------|-----|
| 227 | `_pcre2_valid_utf_8` | valid strings of every length class: empty, 1-byte ASCII, 2-byte, 3-byte, 4-byte, and a mixed 64-byte string; `length` exact and `length` covering only part of the last character; compare the returned `erroroffset` | [x] |
| 228 | `_pcre2_valid_utf_8` | one row per documented `PCRE2_ERROR_UTF8_ERR1`…`ERR21`: truncated 2/3/4/5/6-byte sequences, bad continuation in byte 2/3/4/5/6, 5- and 6-byte leads, `> 0x10FFFF`, surrogates `0xD800`–`0xDFFF`, overlong 2/3/4/5/6-byte forms, isolated `0x80`, bytes `0xFE`/`0xFF`; check both the code and `erroroffset` | [x] |
| 229 | `_pcre2_ord2utf_8` | code points 0x0, 0x7F, 0x80, 0x7FF, 0x800, 0xFFFF, 0x10000, 0x10FFFF, and each boundary in `_pcre2_utf8_table1` (0x7F/0x7FF/0xFFFF/0x1FFFFF/0x3FFFFFF); byte-compare the buffer and the returned unit count | [x] |
| 230 | `_pcre2_strlen_8`, `_pcre2_strcmp_8`, `_pcre2_strcmp_c8_8`, `_pcre2_strncmp_8`, `_pcre2_strncmp_c8_8`, `_pcre2_strcpy_c8_8` | empty strings; equal strings; differing at position 0, mid, and only in length; `n` = 0, less than the difference offset, greater than both lengths; high-bit bytes ≥ 0x80 (sign-extension sensitivity); `strcpy_c8` return value and buffer bytes | [x] |
| 231 | `_pcre2_ckd_smul_8` | (0,0), (1,1), (2,3), (`INT_MAX`,1), (`INT_MAX`,2) (overflow), (0x10000,0x10000), (`INT_MAX`,`INT_MAX`); compare both the BOOL and `*r` | [x] |
| 232 | `_pcre2_is_newline_8` | `type` = `NLTYPE_FIXED` with `nllen` 1 and 2, `NLTYPE_ANY`, `NLTYPE_ANYCRLF`; `utf` TRUE and FALSE; input at CR, LF, CRLF, CR at `endptr` (no following LF), VT, FF, NEL (`0x85` raw and `\xc2\x85` UTF), U+2028, U+2029, NUL, and an ordinary char; compare the BOOL and `*lenptr` | [x] |
| 233 | `_pcre2_was_newline_8` | same matrix as above but scanning backwards from a position immediately after each newline form, with `startptr` at and one before the newline (so CRLF vs lone LF is distinguished); `utf` TRUE and FALSE | [x] |
| 234 | `_pcre2_extuni_8` | grapheme clusters: base+combining mark, CRLF, Hangul L+V+T, Regional-Indicator pairs (even and odd preceding counts), ZWJ + Extended_Pictographic preceded and not preceded by Extended_Pictographic, Prepend, SpacingMark, an isolated Extend at the string start; `utf` TRUE and FALSE; `xcount` NULL and non-NULL | [x] |
| 235 | `_pcre2_script_run_8` | single-script runs (Latin, Greek, Han); Han+Hiragana, Han+Katakana, Han+Bopomofo, Han+Hangul (the `SCRIPT_HANPENDING`/`HANHIRAKATA`/`HANBOPOMOFO`/`HANHANGUL` states); Common/Inherited-only runs; a mixed Latin+Cyrillic run (FALSE); digits from two different decimal sets; empty and single-character runs; `utf` TRUE and FALSE | [x] |
| 236 | `_pcre2_xclass_8` | compile `[\x{100}-\x{200}]`, `[^\x{100}]`, `[\p{L}\x{2000}]`, `[\P{Greek}]`, `[a-z\x{100}]` (XCL_MAP present), and a class large enough to use `XCL_LIST`; then call `_pcre2_xclass_8(c, data, char_lists_end, utf)` for c = 0x41, 0xFF, 0x100, 0x200, 0x201, 0xFFFF, 0x10000, 0x10FFFF with `utf` TRUE and FALSE, negated and not | [x] |
| 237 | `_pcre2_eclass_8` | compile `PCRE2_ALT_EXTENDED_CLASS` patterns exercising `ECL_AND`, `ECL_OR`, `ECL_XOR`, `ECL_NOT`, `ECL_ANY`, `ECL_NONE`, `ECL_XCLASS` and an `ECL_MAP` prefix; call directly for c = 0x00, 0x41, 0x7F, 0xFF, 0x100, 0x10FFFF, `utf` TRUE and FALSE | [x] |
| 238 | `_pcre2_find_bracket_8` | code from `(a)(?:b)(?<n>c)((d)(e))` searching numbers 1–5, 0 and 6; a code containing `OP_XCLASS`, `OP_ECLASS`, `OP_CALLOUT_STR`, and (UTF) a character with `HAS_EXTRALEN` before the target bracket; `number < 0` on a pattern with and without a lookbehind | [x] |
| 239 | `_pcre2_study_8` | call directly on codes for `abc`, `(?i)abc`, `[abc]d`, `a*`, `.*x`, `(?m)^a`, `\1(a)`, `(?R)`, `(*ACCEPT)a`, `[^a]`, `\X`, `\R`, a 1001-item pattern (complexity cut-off), and a 1001-deep nesting (SSB_TOODEEP); byte-compare `re->flags`, `first_codeunit`, `last_codeunit`, all 256 `start_bitmap` bytes and `minlength` | [x] |
| 240 | `_pcre2_auto_possessify_8` | call directly on compiled code for `\d+\D`, `\w+\W`, `[a-z]+[0-9]`, `\p{L}+\P{L}`, `a+b`, `\X+a`, `(?i)a+b`, each with `PCRE2_UCP` and `PCRE2_UTF` on/off; byte-compare the mutated code against the `PCRE2_NO_AUTO_POSSESS` build | [x] |
| 241 | `_pcre2_check_escape_8` | `isclass` FALSE and TRUE × `options` {0, `PCRE2_UTF`, `PCRE2_ALT_BSUX`, `PCRE2_EXTENDED`} × `xoptions` {0, `PCRE2_EXTRA_ALT_BSUX`, `PCRE2_EXTRA_PYTHON_OCTAL`, `PCRE2_EXTRA_NO_BS0`, `PCRE2_EXTRA_BAD_ESCAPE_IS_LITERAL`, `PCRE2_EXTRA_ALLOW_SURROGATE_ESCAPES`, `PCRE2_EXTRA_ESCAPED_CR_IS_LF`} on inputs `\n \t \e \a \f \r \0 \07 \077 \o{101} \x41 \x{100} \cA \d \D \s \S \w \W \h \H \v \V \R \X \b \B \A \Z \z \G \K \N \Q \E \p{L} \P{L} \1 \g{1} \g<1> \k<n> A \u{41} \-`; vary `bracount` 0/1/9/12 (backref-vs-octal resolution); compare the escape code, `*chptr` and the advanced pointer | [x] |
| 242 | `_pcre2_memctl_malloc_8` | `memctl=NULL` (default malloc/free installed at the block head) and a custom `pcre2_memctl`; byte-compare the installed `pcre2_memctl` header for both | [x] |
| 243 | `_pcre2_compile_find_named_group8`, `_pcre2_compile_add_name_to_table8` | during a compile of `(?<b>)(?<a>)(?<c>)` verify the name table ends up alphabetically sorted and `find_named_group` returns the right `named_group` for each name, for a name of length 1 and of length 128, and for a name not present | [x] |
| 244 | `_pcre2_compile_class_not_nested_8`, `_pcre2_compile_class_nested_8` | `lengthptr != NULL` (sizing pass) and `lengthptr == NULL` (emit pass) on the same class META stream; `negate_class` TRUE/FALSE; `has_bitmap` out-parameter for `[ab]` vs `[\x{100}]`; `options`/`xoptions` combinations `PCRE2_CASELESS`, `PCRE2_UTF`, `PCRE2_UCP`, `PCRE2_EXTRA_CASELESS_RESTRICT` | [x] |

## O. Exported DATA symbols — byte-for-byte comparison C vs Rust

| # | entry point(s) | configuration (options set + input shape) | [x] |
|---|----------------|--------------------------------------------|-----|
| 245 | `_pcre2_default_tables_8` | byte-compare all 1088 bytes (lcc, fcc, cbits, ctypes sections) | [x] |
| 246 | `_pcre2_OP_lengths_8` | byte-compare all 173 bytes, and cross-check that every entry equals the length the compiler actually emits for that opcode | [x] |
| 247 | `_pcre2_hspace_list_8`, `_pcre2_vspace_list_8` | byte-compare 20×`uint32_t` and 8×`uint32_t` respectively, including the terminating `NOTACHAR` | [x] |
| 248 | `_pcre2_callout_start_delims_8`, `_pcre2_callout_end_delims_8` | byte-compare 9×`uint32_t` each, and verify each start delimiter is accepted by `pcre2_compile` in a `(?C…)` string callout | [x] |
| 249 | `_pcre2_utf8_table1`, `_pcre2_utf8_table1_size`, `_pcre2_utf8_table2`, `_pcre2_utf8_table3`, `_pcre2_utf8_table4` | byte-compare 6×`int`, 1×`unsigned`, 6×`int`, 6×`int`, 64×`uint8_t` | [x] |
| 250 | `_pcre2_ucp_gentype_8`, `_pcre2_ucp_gbtable_8` | byte-compare 30×`uint32_t` and 15×`uint32_t` | [x] |
| 251 | `_pcre2_posix_class_maps8` | byte-compare 42×`int` (14 triples) and cross-check against the `[:name:]` compile results | [x] |
| 252 | `_pcre2_utt_8`, `_pcre2_utt_names_8`, `_pcre2_utt_size_8` | byte-compare 518×6 bytes, 3834 bytes of names, and the size value; verify every name resolves through `\p{name}` | [x] |
| 253 | `_pcre2_ucd_records_8`, `_pcre2_ucd_stage1_8`, `_pcre2_ucd_stage2_8` | byte-compare 1563×12, 8704×`uint16_t`, 40192×`uint16_t`; then spot-check `UCD_*` lookups for U+0041, U+00DF, U+0130, U+0660, U+1F600, U+10FFFF | [x] |
| 254 | `_pcre2_ucd_caseless_sets_8`, `_pcre2_ucd_digit_sets_8` | byte-compare 118×`uint32_t` and 78×`uint32_t`; exercise via `PCRE2_CASELESS+PCRE2_UTF` on `\x{1c5}` and via `\d` script-run digit checks | [x] |
| 255 | `_pcre2_ucd_script_sets_8`, `_pcre2_ucd_boolprop_sets_8` | byte-compare 476×`uint32_t` and 382×`uint32_t`; exercise via `\p{Scx:Han}` and `\p{Bidi_Control}` | [x] |
| 256 | `_pcre2_ucd_nocase_ranges_8`, `_pcre2_ucd_nocase_ranges_size_8` | byte-compare 84×`uint32_t` and the size value | [x] |
| 257 | `_pcre2_ucd_turkish_dotted_i_caseset_8`, `_pcre2_unicode_version_8` | byte-compare the caseset `uint32_t` and the version string (including NUL); cross-check against `pcre2_config(PCRE2_CONFIG_UNICODE_VERSION)` | [x] |
| 258 | `_pcre2_default_compile_context_8` | byte-compare the whole struct: `memctl` (default malloc/free/NULL), `tables == NULL`, `extra_options == 0`, `max_pattern_length == PCRE2_SIZE_MAX`, `max_pattern_compiled_length == PCRE2_SIZE_MAX`, `bsr_convention`, `newline_convention`, `parens_nest_limit == 250`, `max_varlookbehind == 255`, `optimization_flags == PCRE2_OPTIMIZATION_ALL`, `stack_guard == NULL` | [x] |
| 259 | `_pcre2_default_match_context_8` | byte-compare the whole struct: `memctl`, `callout == NULL`, `substitute_callout == NULL`, `substitute_case_callout == NULL`, `offset_limit == PCRE2_UNSET`, `heap_limit == HEAP_LIMIT`, `match_limit == MATCH_LIMIT`, `depth_limit == MATCH_LIMIT_DEPTH` | [x] |
| 260 | `_pcre2_default_convert_context_8` | byte-compare the whole struct: `memctl`, `glob_separator == '/'`, `glob_escape == '\\'` (non-Windows build) | [x] |
