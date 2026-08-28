# ERRORS.md — ERROR-SURFACE TABLE (Phase A / gate for Phase C)

Derived mechanically from `c_src/src/lib.c`. Every `assert`, every
`return NULL` / early `return`, every explicit range/`!=`/`<`/`>` comparison
inside the `valid_1` … `valid_4` rejection macros, and every named constant is
listed. One row per **distinct** rejection branch.

Grep evidence:

```
$ grep -n 'assert\|return\|NULL\|>=\|<=\|!=\|#define' c_src/src/lib.c
7:#define REPLACEMENT_INC 4096
10:#define valid_1(x) (((x)[0] & 0x80) == 0)
14,15: valid_2 …  (x)[0] >= (char)0xC2 …
20-25: valid_3 …  (x)[0] != (char)0xE0 || … >= 0xA0 …  != (char)0xED || … < 0xA0 …  != (char)0xEF || … <= 0xBF
30-36: valid_4 …  (unsigned char)(x)[0] <= 0xF4 …  != (char)0xF0 || … >= 0x90 …  != (char)0xF4 || … <= 0x8F
40:    assert(string != NULL);                (w_utf8_drop)
52:        return string;                     (first invalid byte)
56:    return string;                         (end of string)
60:    assert(string != NULL);                (w_utf8_filter)
64:    if (*valid == '\0') { … strdup … }
66:        copy = strdup(string);             (may return NULL)
76-78: copy = malloc(size); if (copy == NULL) return NULL;
100-103: copy = realloc(copy, size); if (copy == NULL) return NULL;
97:            if (replacement) {             (else: byte silently dropped)
```

Legend for "expected C result": `SIGABRT` = process aborts via
`__assert_fail`; `p+k` = pointer `k` bytes into the input; `NULL` = null
return value.

| #   | function | trigger (the exact invalid input/condition) | expected C result | test | ✔ |
|-----|----------|---------------------------------------------|-------------------|------|---|
| E1  | `w_utf8_drop`   | `string == NULL` → `assert(string != NULL)` at line 40 | `SIGABRT` (signal 6), stderr `…lib.c:40: w_utf8_drop: Assertion \`string != NULL' failed.` | `e1_null_drop_aborts` | [x] |
| E2  | `w_utf8_filter` | `string == NULL` → `assert(string != NULL)` at line 60 | `SIGABRT` (signal 6), stderr `…lib.c:60: w_utf8_filter: Assertion \`string != NULL' failed.` | `e2_null_filter_aborts` | [x] |
| E3  | `w_utf8_drop`   | any byte sequence at offset `k` matching none of `valid_1..4` → `return string` (line 52) | returns `p+k`, **not** the NUL terminator | `e3_drop_returns_first_invalid` | [x] |
| E4  | `w_utf8_filter` | input fully valid **and** `strdup` fails (OOM, line 66) | `NULL` | `e4_strdup_oom` | [x] |
| E5  | `w_utf8_filter` | input has ≥1 invalid byte **and** `malloc(strlen+1)` fails (line 76) | `NULL` | `e5_malloc_oom` | [x] |
| E6  | `w_utf8_filter` | `replacement != 0`, ≥1 invalid byte, **and** the first `realloc(copy, size+4096)` fails (line 101) | `NULL` (old buffer leaked — unobservable) | `e6_realloc_oom` | [x] |
| E7  | `w_utf8_filter` | invalid byte reached with `replacement == 0` (line 97 false) | byte **silently dropped**, no U+FFFD, output shorter | `e7_drop_mode_elides_invalid` | [x] |
| E8  | `w_utf8_filter` | invalid byte reached with `replacement != 0` | 3 bytes `EF BF BD` appended, exactly **one** input byte consumed | `e8_replacement_mode_emits_fffd` | [x] |
| E9  | `valid_1` (line 10) | `(b0 & 0x80) != 0` (b0 ≥ 0x80) → not a 1-byte form; falls through to `valid_2` | rejection of the 1-byte form | `e9_valid1_high_bit` | [x] |
| E10 | `valid_2` (line 14) | `(b0 & 0xE0) != 0xC0`, e.g. b0 = 0x80…0xBF, 0xE0…0xFF | 2-byte form rejected | `e10_valid2_lead_mask` | [x] |
| E11 | `valid_2` (line 15) | `b0 >= (char)0xC2` fails — **signed** `char` compare, so exactly b0 ∈ {0xC0, 0xC1} (overlong 2-byte) | 2-byte form rejected → byte is invalid | `e11_valid2_overlong_c0_c1` | [x] |
| E12 | `valid_2` (line 15) | `(b1 & 0xC0) != 0x80` (bad/absent continuation, incl. b1 == `'\0'` at end of string) | 2-byte form rejected | `e12_valid2_bad_cont` | [x] |
| E13 | `valid_3` (line 20) | `(b0 & 0xF0) != 0xE0` | 3-byte form rejected | `e13_valid3_lead_mask` | [x] |
| E14 | `valid_3` (line 21) | `(b1 & 0xC0) != 0x80` (incl. b1 == `'\0'`, 1-byte truncation) | 3-byte form rejected; `x[2]` **not read** | `e14_valid3_bad_cont1` | [x] |
| E15 | `valid_3` (line 22) | `(b2 & 0xC0) != 0x80` (incl. b2 == `'\0'`, 2-byte truncation) | 3-byte form rejected | `e15_valid3_bad_cont2` | [x] |
| E16 | `valid_3` (line 23) | b0 == 0xE0 **and** `(unsigned char)b1 < 0xA0` (b1 ∈ 0x80…0x9F) — overlong | 3-byte form rejected | `e16_valid3_overlong_e0` | [x] |
| E17 | `valid_3` (line 24) | b0 == 0xED **and** `(unsigned char)b1 >= 0xA0` (b1 ∈ 0xA0…0xBF) — UTF-16 surrogate half | 3-byte form rejected | `e17_valid3_surrogate_ed` | [x] |
| E18 | `valid_3` (line 25) | b0 == 0xEF **and** `(unsigned char)b1 > 0xBF` — **unreachable** (line 21 already forces b1 ≤ 0xBF); still exercised as a clause | never rejects on its own; 0xEF sequences stay valid | `e18_valid3_ef_clause_unreachable` | [x] |
| E19 | `valid_4` (line 30) | `(b0 & 0xF8) != 0xF0` (b0 ∉ 0xF0…0xF7) | 4-byte form rejected | `e19_valid4_lead_mask` | [x] |
| E20 | `valid_4` (line 31) | `(unsigned char)b0 > 0xF4` → b0 ∈ {0xF5, 0xF6, 0xF7} | 4-byte form rejected → byte invalid | `e20_valid4_lead_gt_f4` | [x] |
| E21 | `valid_4` (line 32) | `(b1 & 0xC0) != 0x80` (incl. `'\0'`) | 4-byte form rejected; `x[2]`, `x[3]` **not read** | `e21_valid4_bad_cont1` | [x] |
| E22 | `valid_4` (line 33) | `(b2 & 0xC0) != 0x80` (incl. `'\0'`) | 4-byte form rejected; `x[3]` **not read** | `e22_valid4_bad_cont2` | [x] |
| E23 | `valid_4` (line 34) | `(b3 & 0xC0) != 0x80` (incl. `'\0'`, 3-byte truncation) | 4-byte form rejected | `e23_valid4_bad_cont3` | [x] |
| E24 | `valid_4` (line 35) | b0 == 0xF0 **and** `(unsigned char)b1 < 0x90` (b1 ∈ 0x80…0x8F) — overlong | 4-byte form rejected | `e24_valid4_overlong_f0` | [x] |
| E25 | `valid_4` (line 36) | b0 == 0xF4 **and** `(unsigned char)b1 > 0x8F` (b1 ∈ 0x90…0xBF) — beyond U+10FFFF | 4-byte form rejected | `e25_valid4_f4_above_max` | [x] |
| E26 | both | **empty string** `""` (zero length): `while (*string)` never runs | `w_utf8_drop` → `p+0`; `w_utf8_filter` → `strdup("")` = `""` | `e26_empty_string` | [x] |
| E27 | `w_utf8_filter` | out-of-range value for the C `_Bool` parameter: 2, 3, 0x7F, 0x80, 0xFE, 0xFF (no valid `_Bool` variant) | GCC emits `cmpb $0x0` → **every non-zero byte is true** | `e27_noncanonical_bool_byte` | [x] |
| E28 | `w_utf8_filter` | `_Bool` parameter register carrying garbage in the upper 56 bits (`0x100`, `0xFFFFFF00`, `0xDEADBEEF00`, `0x1_0000_0001`) | only the **low byte** is examined → `0x…00` is false, `0x…01` is true | `e28_noncanonical_bool_upper_bits` | [x] |
| E29 | both | input consisting of a lone lead byte immediately followed by the NUL terminator (0xC2, 0xE0, 0xEF, 0xF0, 0xF4 …) — one step past the end of the buffer must **not** be read | rejected at the NUL; scan never advances past the terminator | `e29_lead_byte_then_nul` | [x] |
| E30 | `w_utf8_filter` | `REPLACEMENT_INC` (4096) boundary: `repl` bookkeeping means a `realloc` happens on replacement #1 and then only every 1365 replacements (4096 = 3·1365 + 1) | identical byte output **and** identical `malloc_usable_size` at 1, 1364, 1365, 1366, 2730, 2731, 4096 invalid bytes | `e30_replacement_inc_boundary` | [x] |
| E31 | `w_utf8_filter` | very large input (oversized length) that is fully valid vs. fully invalid, 1 MiB | no overflow, byte-identical, allocation sizes identical | `e31_oversized_length` | [x] |
| E32 | both | **one byte past the end of the buffer**: input placed so that its NUL is the last readable byte before a `PROT_NONE` guard page. The `valid_2/3/4` macros must short-circuit on the terminator and never read `x[1]`/`x[2]`/`x[3]` past it | both complete normally (no `SIGSEGV`) and return identical results | `e32_no_read_past_terminator` | [x] |

## Notes / non-rows

* There is no error-code enum, no `errno` use and no `RETURN_ERROR`-style macro
  in this library; the only failure channels are `NULL` returns and
  `assert()`-driven `SIGABRT`.
* `w_utf8_drop`'s line-56 `return string` is the *success* return (Phase B),
  not a rejection.
* Rows E9…E25 are the individual clauses of the four validity macros. Each is
  reachable via `w_utf8_drop` (as the returned offset) **and** via
  `w_utf8_filter` (as a dropped/replaced byte); the Phase C tests drive both.
* **One documented, deliberate difference (E1/E2).** `assert()` expands to
  `__assert_fail(expr, __FILE__, __LINE__, __func__)`. The C's `__FILE__` is
  the *absolute build path* that CMake happened to pass to GCC
  (`/…/harvest-work-…/c_src/src/lib.c` on this machine), so it is not
  reproducible and cannot be part of a portable contract; the Rust passes the
  repository-relative `c_src/src/lib.c`. Everything that *is* part of the
  contract is asserted identical: the abort signal (`SIGABRT`), the assertion
  expression (`string != NULL`), the line number (40 / 60) and the function
  name (`w_utf8_drop` / `w_utf8_filter`). The mutation check confirms all four
  of those are actually tested (mutations 27–29).
* The `NULL` returns for E4/E5/E6 are reached with a deterministic allocator
  interposer (`tests/fixtures/failalloc.c`, loaded with `LD_PRELOAD` in a child
  process) rather than by exhausting real memory, so those rows are exact, not
  best-effort. The injection is restricted to requests ≥ the input length and
  reports how many failures actually fired, so each row asserts `fired=1`
  (the library's own allocation was the one that failed) — or `fired=0` where
  the C must *not* allocate at all (arming the 2nd `realloc` when the input only
  triggers one, and arming `realloc` with `replacement == 0`).
