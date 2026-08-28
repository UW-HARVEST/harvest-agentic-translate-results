# CONFIGS.md — CONFIGURATION-SURFACE TABLE (Phase A / gate for Phase B)

Mechanically enumerated from the branches `c_src/src/lib.c` actually takes.

## Axis 1 — public entry points (ALL of them, lowest level included)

| entry point | declared in | level |
|-------------|-------------|-------|
| `w_utf8_drop(const char *)` | **not** in `lib.h`, exported from `lib.c` (line 39) | low-level scanner, callable directly |
| `w_utf8_filter(const char *, bool)` | `lib.h` line 3 | high-level; internally *composes* `w_utf8_drop` + the copy loop |

There is no init/teardown, no context object, no global state.

## Axis 2 — runtime options / flags

| option | values the C distinguishes | branch |
|--------|---------------------------|--------|
| `replacement` (`_Bool`) | `0` → false; **any** non-zero byte → true (`cmpb $0x0`, line 97) | `if (replacement)` |
| `REPLACEMENT_INC` | compile-time `4096` (line 7); drives the `repl < 3` branch (line 98) | `if (repl < 3)` realloc-or-not |

There are **no** `#ifdef`s and no `[features]` in `Cargo.toml`, so the flag
cross-product is `{0, 1, other-non-zero}`.

## Axis 3 — input shapes the code special-cases

* **length**: 0 (empty), 1, 2, 3, 4, 5, small random, ≥ 4096, ≥ 1 MiB
* **element width** taken by the scanner: 1 / 2 / 3 / 4 byte forms
  (`valid_1` … `valid_4`, tried strictly in that order)
* **lead-byte special cases**: `0xC2` (lowest legal 2-byte lead), `0xE0`
  (overlong guard), `0xED` (surrogate guard), `0xEF` (the extra `<= 0xBF`
  clause), `0xF0` (overlong guard), `0xF4` (upper-limit guard)
* **code-point boundary values**: U+0001, U+007F, U+0080, U+07FF, U+0800,
  U+D7FF, U+E000, U+FFFD, U+FFFF, U+10000, U+10FFFF
* **validity mix**: all-valid / all-invalid / mixed
* **position of the first invalid byte**: offset 0 (⇒ `memcpy` of 0 bytes),
  middle, last byte, "none" (⇒ the `strdup` fast path at line 64)
* **run length of invalid bytes** (drives the `repl`/`realloc` bookkeeping):
  0, 1, 2, 3, 1364, 1365, 1366, 2730, 2731, 4096, 100 000
* **truncation**: multi-byte form cut short by the NUL terminator (1, 2 or 3
  bytes missing) — the boundary-read case
* **byte order / element type**: n/a (byte-oriented API, no endianness, no
  element-type parameter)

## Row table

`R` column: value(s) of `replacement` exercised. Every row is run with **many
randomized inputs** (splitmix64, fixed seed per row) unless it says "fixed".

| #   | entry point(s) | configuration (options set + input shape) | R | test | ✔ |
|-----|----------------|-------------------------------------------|---|------|---|
| C1  | `w_utf8_drop` | empty string, length 0 | — | `c1_drop_empty` | [x] |
| C2  | `w_utf8_drop` | all-ASCII (1-byte forms only), lengths 1…64 + random | — | `c2_drop_ascii` | [x] |
| C3  | `w_utf8_drop` | valid 2-byte forms only (leads 0xC2…0xDF), random | — | `c3_drop_valid2` | [x] |
| C4  | `w_utf8_drop` | valid 3-byte forms only (all leads 0xE0…0xEF incl. 0xE0/0xED/0xEF guards), random | — | `c4_drop_valid3` | [x] |
| C5  | `w_utf8_drop` | valid 4-byte forms only (leads 0xF0…0xF4 incl. both guards), random | — | `c5_drop_valid4` | [x] |
| C6  | `w_utf8_drop` | mixed valid forms, widths 1–4 interleaved, random | — | `c6_drop_valid_mixed` | [x] |
| C7  | `w_utf8_drop` | code-point boundary table (fixed): U+0001, U+007F, U+0080, U+07FF, U+0800, U+D7FF, U+E000, U+FFFD, U+FFFF, U+10000, U+10FFFF | — | `c7_drop_boundary_codepoints` | [x] |
| C8  | `w_utf8_drop` | uniform random bytes 0x01…0xFF (mostly invalid), lengths 1…64 | — | `c8_drop_uniform_random` | [x] |
| C9  | `w_utf8_drop` | bytes drawn only from the *interesting* boundary set (0x7F, 0x80, 0x9F, 0xA0, 0xBF, 0xC0, 0xC1, 0xC2, 0xDF, 0xE0, 0xED, 0xEE, 0xEF, 0xF0, 0x8F, 0x90, 0xF4, 0xF5, 0xF7, 0xF8, 0xFF), lengths 1…40 | — | `c9_drop_interesting_bytes` | [x] |
| C10 | `w_utf8_drop` | valid prefix of random length followed by one invalid sequence then more junk | — | `c10_drop_valid_prefix_then_invalid` | [x] |
| C11 | `w_utf8_drop` | truncated multi-byte forms at end of buffer (2/3/4-byte forms cut by NUL, every cut position) | — | `c11_drop_truncated_tail` | [x] |
| C12 | `w_utf8_drop` | long input (64 KiB) of mixed valid/invalid | — | `c12_drop_long_mixed` | [x] |
| C13 | `w_utf8_filter` | fully valid input ⇒ `strdup` fast path; shapes of C2…C7 | 0 | `c13_filter_valid_strdup_r0` | [x] |
| C14 | `w_utf8_filter` | fully valid input ⇒ `strdup` fast path; shapes of C2…C7 | 1 | `c14_filter_valid_strdup_r1` | [x] |
| C15 | `w_utf8_filter` | empty string ⇒ `strdup("")` | 0, 1 | `c15_filter_empty` | [x] |
| C16 | `w_utf8_filter` | first invalid byte at offset 0 (`memcpy` length 0) | 0 | `c16_filter_invalid_at_0_r0` | [x] |
| C17 | `w_utf8_filter` | first invalid byte at offset 0 | 1 | `c17_filter_invalid_at_0_r1` | [x] |
| C18 | `w_utf8_filter` | first invalid byte in the middle (non-zero `memcpy`) | 0 | `c18_filter_invalid_mid_r0` | [x] |
| C19 | `w_utf8_filter` | first invalid byte in the middle | 1 | `c19_filter_invalid_mid_r1` | [x] |
| C20 | `w_utf8_filter` | invalid byte is the **last** byte of the string | 0, 1 | `c20_filter_invalid_last` | [x] |
| C21 | `w_utf8_filter` | uniform random bytes 0x01…0xFF, lengths 1…64 | 0 | `c21_filter_uniform_r0` | [x] |
| C22 | `w_utf8_filter` | uniform random bytes 0x01…0xFF, lengths 1…64 | 1 | `c22_filter_uniform_r1` | [x] |
| C23 | `w_utf8_filter` | interesting-byte-set inputs, lengths 1…40 | 0 | `c23_filter_interesting_r0` | [x] |
| C24 | `w_utf8_filter` | interesting-byte-set inputs, lengths 1…40 | 1 | `c24_filter_interesting_r1` | [x] |
| C25 | `w_utf8_filter` | mixed valid forms + injected invalid sequences of every invalid class | 0 | `c25_filter_mixed_classes_r0` | [x] |
| C26 | `w_utf8_filter` | mixed valid forms + injected invalid sequences of every invalid class | 1 | `c26_filter_mixed_classes_r1` | [x] |
| C27 | `w_utf8_filter` | truncated multi-byte tails (every cut position) | 0, 1 | `c27_filter_truncated_tail` | [x] |
| C28 | `w_utf8_filter` | runs of exactly N invalid bytes, N ∈ {1,2,3,4,1364,1365,1366,2730,2731,4096} — crosses the `repl < 3` / `REPLACEMENT_INC` realloc boundary | 1 | `c28_filter_realloc_boundary_r1` | [x] |
| C29 | `w_utf8_filter` | same run lengths as C28 but `replacement = 0` (no realloc at all, output shrinks) | 0 | `c29_filter_runs_no_realloc_r0` | [x] |
| C30 | `w_utf8_filter` | invalid bytes spread through a long (64 KiB) mixed buffer ⇒ many realloc cycles | 0, 1 | `c30_filter_long_mixed` | [x] |
| C31 | `w_utf8_filter` | 1 MiB fully-valid input (`strdup` of a large block) | 0, 1 | `c31_filter_large_valid` | [x] |
| C32 | `w_utf8_filter` | 1 MiB fully-invalid input (≈350 000 reallocs) | 1 | `c32_filter_large_invalid_r1` | [x] |
| C33 | `w_utf8_filter` | non-canonical `replacement` byte values 2, 3, 0x7F, 0x80, 0xFE, 0xFF on mixed input | 2,3,0x7F,0x80,0xFE,0xFF | `c33_filter_noncanonical_bool` | [x] |
| C34 | `w_utf8_filter` | `replacement` register carrying garbage upper bits (0x100, 0x1FF, 0xFFFFFF00, 0xFFFFFFFF, 0xDEADBEEF00, 0xDEADBEEF01) on mixed input | wide | `c34_filter_wide_bool_register` | [x] |
| C35 | `w_utf8_drop` → `w_utf8_filter` | **composed pipeline**: call `w_utf8_drop`, then `w_utf8_filter` on the returned suffix pointer, then `w_utf8_drop` on the filter's output (must reach the terminator) | 0, 1 | `c35_composed_pipeline` | [x] |
| C36 | `w_utf8_filter` | idempotence/stability: filter twice, second pass on the already-filtered buffer | 0, 1 | `c36_filter_twice` | [x] |
| C37 | both | every single-byte input 0x01…0xFF (fixed, exhaustive) | 0, 1 | `c37_exhaustive_len1` | [x] |
| C38 | both | every two-byte input 0x01…0xFF × 0x01…0xFF (fixed, exhaustive, 65 025 cases) | 0, 1 | `c38_exhaustive_len2` | [x] |
| C39 | both | exhaustive 3-byte sweep over the *interesting* byte set (23³ = 12 167) plus random 3-byte inputs | 0, 1 | `c39_exhaustive_len3_interesting` | [x] |
| C40 | both | exhaustive 4-byte sweep over the *interesting* byte set (23⁴ = 279 841) | 0, 1 | `c40_exhaustive_len4_interesting` | [x] |
| C41 | both | 0xEF-lead sequences specifically (the `<= 0xBF` clause) — all 0xEF x y combinations over the interesting set | 0, 1 | `c41_ef_lead_sweep` | [x] |
| C42 | both | 0xE0 / 0xED / 0xF0 / 0xF4 lead bytes with **all** 256 possible second bytes (fixed, exhaustive) | 0, 1 | `c42_guarded_lead_all_second_bytes` | [x] |
| C43 | both | repeated calls on the same buffer (statelessness) and interleaved C/Rust calls | 0, 1 | `c43_repeated_calls_stateless` | [x] |
| C44 | both | buffers with **interior NUL bytes** (1 and several) — the API is `strlen`-based, so everything from the first NUL must be ignored; plus a buffer that starts with the terminator | 0, 1 | `c44_interior_nul_terminates` | [x] |
| C45 | both | the same content at **every start offset 0…15** inside a larger allocation (unaligned input pointer), short and 400-byte buffers | 0, 1 | `c45_unaligned_start_pointer` | [x] |

Verification for every row: `w_utf8_drop` → returned pointer offset must be
identical; `w_utf8_filter` → NUL-terminated output bytes identical **and**
`malloc_usable_size()` of the returned block identical (catches an allocation
arithmetic divergence that byte comparison alone would hide).
