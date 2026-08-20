# ERRORS.md — Error / rejection surface of `c_src/src/lib.c`

Derived mechanically by grepping every rejection site in the C source:

* `assert(` — 2 hits (lines 40, 60)
* `return NULL;` — 2 hits (lines 77, 102)
* `return string;` inside the scan loop (line 52) — the "reject" exit of
  `w_utf8_drop`
* the `else` arm of the validation chain in `w_utf8_filter` (line 96) — the
  "reject a byte" path
* every `&&` clause of `valid_1` / `valid_2` / `valid_3` / `valid_4` — each
  clause is an explicit range/bit check that can reject a candidate sequence
* constants: `REPLACEMENT_INC 4096`, the literal `3` in `if (repl < 3)`,
  `0xC2`, `0xA0`, `0xBF`, `0xF4`, `0x90`, `0x8F`

`w_utf8_drop` / `w_utf8_filter` have **no error enum and no errno use**. The only
failure signals are: process abort (assert), `NULL` return (allocation failure),
and "byte rejected" (pointer returned / byte dropped or replaced).

Legend for "expected C result":
* `SIGABRT` — `__assert_fail` → `abort()`, process dies with signal 6.
* `NULL` — function returns a null pointer.
* `ret=&s[k]` — `w_utf8_drop` returns a pointer to input offset `k`.
* `drop` / `U+FFFD` — `w_utf8_filter` omits the byte / emits `EF BF BD`.

| #  | function | trigger (the exact invalid input/condition) | expected C result |
|----|----------|---------------------------------------------|-------------------|
| 1  | `w_utf8_drop` | `string == NULL` (line 40 `assert(string != NULL)`) | `SIGABRT` (`__assert_fail`, exit by signal 6) |
| 2  | `w_utf8_filter` | `string == NULL` (line 60 `assert(string != NULL)`) | `SIGABRT` (`__assert_fail`, exit by signal 6) |
| 3  | `w_utf8_filter` | `malloc(strlen+1)` fails (line 76) — reachable only when the string contains an invalid byte | `NULL` |
| 4  | `w_utf8_filter` | `realloc(copy, size)` fails (line 101) — reachable only with `replacement == true` and ≥1 invalid byte | `NULL` |
| 5  | `w_utf8_drop` | `valid_1` fails: `s[0] & 0x80 != 0` and no multi-byte form matches | `ret=&s[k]` at that byte |
| 6  | `w_utf8_drop` | lone continuation byte `0x80..0xBF` at `s[k]` (fails `&0x80`, `&0xE0`, `&0xF0`, `&0xF8` tests) | `ret=&s[k]` |
| 7  | `w_utf8_drop` | `valid_2` clause 1: `(s[0] & 0xE0) != 0xC0` (e.g. lead byte `0xA0`) | falls through to `valid_3/4`; if those fail → `ret=&s[k]` |
| 8  | `w_utf8_drop` | `valid_2` clause 2: overlong lead `0xC0` or `0xC1` (`(char)s[0] < (char)0xC2`, signed compare) | `ret=&s[k]` |
| 9  | `w_utf8_drop` | `valid_2` clause 3: `(s[1] & 0xC0) != 0x80` — bad/absent continuation after `0xC2..0xDF` (incl. truncation, `s[1]=='\0'`) | `ret=&s[k]` |
| 10 | `w_utf8_drop` | `valid_3` clause 1: `(s[0] & 0xF0) != 0xE0` | falls through to `valid_4`; if that fails → `ret=&s[k]` |
| 11 | `w_utf8_drop` | `valid_3` clause 2: `(s[1] & 0xC0) != 0x80` after an `0xE0..0xEF` lead (incl. `s[1]=='\0'`) | `ret=&s[k]` |
| 12 | `w_utf8_drop` | `valid_3` clause 3: `(s[2] & 0xC0) != 0x80` (incl. `s[2]=='\0'`, 2-of-3 truncation) | `ret=&s[k]` |
| 13 | `w_utf8_drop` | `valid_3` clause 4: overlong — `s[0]==0xE0 && (unsigned char)s[1] < 0xA0` | `ret=&s[k]` |
| 14 | `w_utf8_drop` | `valid_3` clause 5: UTF-16 surrogate — `s[0]==0xED && (unsigned char)s[1] >= 0xA0` | `ret=&s[k]` |
| 15 | `w_utf8_drop` | `valid_3` clause 6: `s[0]==0xEF && (unsigned char)s[1] > 0xBF` — **unreachable** (clause 2 already forces `s[1] <= 0xBF`); must stay unreachable in Rust too | never rejects |
| 16 | `w_utf8_drop` | `valid_4` clause 1: `(s[0] & 0xF8) != 0xF0` — lead `0xF8..0xFF` | `ret=&s[k]` |
| 17 | `w_utf8_drop` | `valid_4` clause 2: `(unsigned char)s[0] > 0xF4` — leads `0xF5,0xF6,0xF7` (these *pass* clause 1) | `ret=&s[k]` |
| 18 | `w_utf8_drop` | `valid_4` clause 3/4/5: `(s[1..3] & 0xC0) != 0x80` (incl. `'\0'` truncation at 1, 2 or 3 bytes in) | `ret=&s[k]` |
| 19 | `w_utf8_drop` | `valid_4` clause 6: overlong — `s[0]==0xF0 && (unsigned char)s[1] < 0x90` | `ret=&s[k]` |
| 20 | `w_utf8_drop` | `valid_4` clause 7: beyond U+10FFFF — `s[0]==0xF4 && (unsigned char)s[1] > 0x8F` | `ret=&s[k]` |
| 21 | `w_utf8_drop` | zero-length input `""` — loop body never runs | `ret=&s[0]` (pointer to the NUL, *not* an error) |
| 22 | `w_utf8_filter` | any rejected byte with `replacement == 0` (line 96 `else` arm, `if (replacement)` false) | byte `drop`ped, scan advances 1 byte |
| 23 | `w_utf8_filter` | any rejected byte with `replacement != 0` | `U+FFFD` = `EF BF BD` written, scan advances 1 byte |
| 24 | `w_utf8_filter` | `replacement` passed a **non-normalized `_Bool`** (2, 0xFF, 0x100, 0xFF00 …) — C tests `cmpb $0x0,-0x3c(%rbp)`, i.e. low byte ≠ 0 | low byte ≠ 0 ⇒ behaves as `true`; low byte == 0 (e.g. `0x100`) ⇒ behaves as `false` |
| 25 | `w_utf8_filter` | `repl < 3` boundary (line 98): replacement #1366, #2731, … re-trigger `realloc` | same bytes out, `size` grew by 4096 |
| 26 | `w_utf8_filter` | first byte is already invalid ⇒ `i == 0` ⇒ `memcpy(copy, string, 0)` (line 79, zero-length copy) | no prefix copied, valid output |
| 27 | `w_utf8_filter` | whole string valid ⇒ `*valid == '\0'` ⇒ `strdup` path (line 66); includes `""` | fresh copy identical to input (never `NULL` here unless `strdup` OOM) |
| 28 | `w_utf8_drop`, `w_utf8_filter` | bytes *after* the first `'\0'` must be ignored entirely (loop condition `while (*string)`) | terminator honoured, trailing garbage untouched |

## Check-off (Phase C)

All rows pass in **both** the `dev` and `release` profiles.
Test names refer to `tests/errors.rs` (and `tests/configs.rs` for rows 25/26).

| # | test | pass |
|---|------|------|
| 1  | `errors::abort_on_null_drop` (+ `errors::abort_message_matches`) | [x] |
| 2  | `errors::abort_on_null_filter` for `replacement` ∈ {0, 1, 0xFF} (+ `errors::abort_message_matches`) | [x] |
| 3  | `errors::malloc_failure_returns_null` — allocator drained in a forked child (`exhaust_allocator`), both `replacement` values | [x] |
| 4  | `errors::realloc_failure_returns_null` — reserve exactly `strlen+1`, drain, release the reserve ⇒ the first `malloc` succeeds and the first `realloc` fails; plus a `replacement=0` control that must SUCCEED, proving the realloc branch (not the malloc branch) was hit | [x] |
| 5  | `errors::rejection_table` (row 5: every byte `0x80..0xFF` alone) | [x] |
| 6  | `errors::rejection_table` (row 6: all `0x80..0xBF`, followed by ASCII and by 3 continuations) | [x] |
| 7  | `errors::rejection_table` (row 7: every lead `0x80..0xFF` with `(lead & 0xE0) != 0xC0`) | [x] |
| 8  | `errors::rejection_table` (row 8: `0xC0`/`0xC1` × all 256 second bytes) | [x] |
| 9  | `errors::rejection_table` (row 9: `0xC2..0xDF` × all 256 second bytes, accept/reject asserted per byte) | [x] |
| 10 | `errors::rejection_table` (row 10: `0x80..0xBF` + 3 continuations; plus the fall-through-to-`valid_4`/`valid_2` cases) | [x] |
| 11 | `errors::rejection_table` (row 11: `0xE0..0xEF` × all 256 second bytes) | [x] |
| 12 | `errors::rejection_table` (row 12: `0xE0..0xEF` × guard-satisfying b1 × all 256 third bytes) | [x] |
| 13 | `errors::rejection_table` (row 13: `0xE0` × `0x80..0x9F` reject, `0xA0..0xBF` accept) | [x] |
| 14 | `errors::rejection_table` (row 14: `0xED` × `0xA0..0xBF` reject, `0x80..0x9F` accept) | [x] |
| 15 | `errors::valid3_ef_clause_unreachable` (proves the clause is dead *and* that `0xEF` behaves like any other `0xE_` lead) | [x] |
| 16 | `errors::rejection_table` (row 16: leads `0xF8..0xFF`) | [x] |
| 17 | `errors::rejection_table` (row 17: leads `0xF5..0xF7`, which *pass* the `& 0xF8` mask) × `0x80..0xBF` | [x] |
| 18 | `errors::rejection_table` (row 18: truncation after 1/2/3 bytes + bad 2nd/3rd/4th byte for every lead `0xF0..0xF4`) | [x] |
| 19 | `errors::rejection_table` (row 19: `0xF0` × `0x80..0x8F` reject, `0x90..0xBF` accept) | [x] |
| 20 | `errors::rejection_table` (row 20: `0xF4` × `0x90..0xBF` reject, `0x80..0x8F` accept) | [x] |
| 21 | `errors::empty_string` | [x] |
| 22 | `errors::replacement_false_drops` (output compared against an independent model) | [x] |
| 23 | `errors::replacement_true_emits_fffd` (output compared against an independent model) | [x] |
| 24 | `errors::non_normalized_bool` — 4096 random `u32`s + exhaustive low byte × 4 upper-bit patterns | [x] |
| 25 | `configs::row36_37_repl_threshold_boundary`, `configs::row46_allocation_schedule` | [x] |
| 26 | `errors::invalid_at_offset_zero` | [x] |
| 27 | `errors::strdup_path` (also asserts the result is a fresh allocation, not the input pointer) | [x] |
| 28 | `errors::bytes_after_nul_ignored` | [x] |

### Generic FFI-boundary boundaries also covered

| condition | test |
|-----------|------|
| null pointer to both entry points | `errors::abort_on_null_drop`, `errors::abort_on_null_filter` |
| zero length (`""`) | `errors::empty_string`, `configs::row01_drop_empty`, `configs::row16_17_filter_empty_both_flags` |
| oversized length (40 KB … 64 MB inputs) | `configs::row15/42/43`, `errors::malloc_failure_returns_null` |
| one step past a documented valid range | `errors::rejection_table` rows 8/13/14/17/19/20 assert accept on one side of every constant (`0xC2`, `0xA0`, `0x8F`, `0x90`, `0xF4`) and reject on the other |
| **out-of-range "enum" value across FFI** — C `_Bool` accepts any int; the only enum-like parameter in this API is `replacement` | `errors::non_normalized_bool`, `configs::row39_40_non_normalized_bool` (all 256 low-byte values × 4 upper-bit patterns + 4096 random `u32`s; verified against gcc's `cmpb $0x0` semantics) |
| read past the end of the input buffer | `configs::row47_guard_page_no_overread` (`PROT_NONE` page immediately after the NUL) |
| write past the end of the output buffer | one-sided `malloc_usable_size >= len+1` assertion on every `cmp_filter` call, plus `configs::row46_allocation_schedule` |

### Documented non-divergence

* Row 15 is a **dead** clause in the C source. The Rust keeps it verbatim; the
  test proves it can never fire in either implementation.
* The `assert()` failure text is byte-identical, including glibc's
  `<file>:<line>: <function>: Assertion \`string != NULL\' failed.` The Rust
  derives `__FILE__` from `CARGO_MANIFEST_DIR + "/c_src/src/lib.c"`, which is
  exactly the absolute path CMake hands to the C compiler for the same checkout
  (`errors::abort_message_matches` asserts full stderr equality).
