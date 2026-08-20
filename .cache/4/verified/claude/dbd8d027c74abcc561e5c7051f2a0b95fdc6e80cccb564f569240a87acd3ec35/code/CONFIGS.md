# CONFIGS.md — Configuration surface (valid inputs) of `c_src/src/lib.c`

## Axes derived from the C source

**Public entry points** (from `nm -D` + `include/lib.h` + non-`static` definitions
in `src/lib.c`), lowest level first:

| level | entry point | signature |
|-------|-------------|-----------|
| low   | `w_utf8_drop`   | `const char *w_utf8_drop(const char *string)` — the raw scanner. Not declared in the public header, but exported and *is* the primitive `w_utf8_filter` is built on, so it is driven directly. |
| high  | `w_utf8_filter` | `char *w_utf8_filter(const char *string, bool replacement)` — the convenience wrapper. Also driven composed with `w_utf8_drop` (rows 44/45). |

**Runtime option / mode flags** (grep of every `if (` / `switch` on a parameter):

| axis | values the C actually branches on | branch site |
|------|-----------------------------------|-------------|
| `replacement` | `0` (false) vs. non-zero low byte (true) | `if (replacement)` line 97 → drop-vs-U+FFFD |
| `replacement`, non-normalized `_Bool` | `2`, `0xFF`, `0x100` (low byte 0!), `0xFF00`, … | gcc emits `cmpb $0x0,-0x3c(%rbp)`, i.e. **only the low byte** decides |
| `repl` / `size` accounting | `if (repl < 3)` with `REPLACEMENT_INC = 4096` | line 98 → realloc on replacement #1, #1366, #2731, … |

There are **no** compile-time options: `c_src/CMakeLists.txt` has no `option()` /
`target_compile_definitions`, `src/lib.c` has no `#ifdef`, and `Cargo.toml`
declares no features. One build configuration only (see `SYMBOLS.md`).

**Input SHAPE axes the C special-cases** (each is a distinct `if` / `&&` clause):

* which validator matches: `valid_1` (1 byte) / `valid_2` (2) / `valid_3` (3) /
  `valid_4` (4) / none (reject)
* lead-byte class: `0x00`, `0x01..0x7F`, `0x80..0xBF` (continuation),
  `0xC0..0xC1` (overlong-2), `0xC2..0xDF` (valid-2), `0xE0` (overlong-3 guard),
  `0xE1..0xEC`, `0xED` (surrogate guard), `0xEE`, `0xEF` (dead guard),
  `0xF0` (overlong-4 guard), `0xF1..0xF3`, `0xF4` (max-codepoint guard),
  `0xF5..0xF7` (pass the `& 0xF8` mask but fail `<= 0xF4`), `0xF8..0xFF`
* second-byte boundaries `0x80 / 0x8F / 0x90 / 0x9F / 0xA0 / 0xBF`, the
  out-of-continuation values, and `'\0'` (truncation)
* truncation depth: sequence cut after 1 / 2 / 3 bytes by the terminator
* position of the first invalid byte: offset 0 (⇒ `memcpy` length 0) vs. > 0
  (⇒ prefix `memcpy`) vs. none (⇒ `strdup` early return)
* length / count: `""`, 1 byte, "few", and counts that cross the `repl < 3` /
  `REPLACEMENT_INC` accounting boundary (1 / 1365 / 1366 / 1367 / 2730 / 2731 /
  2732 / 4095 / 4096 / 4097 / k·1365 / 50000)
* bytes *after* the terminating NUL (must be ignored)
* memory placement: input pressed against an unmapped guard page, so that the
  short-circuit structure of `valid_2/3/4` (which is what stops the C from
  reading a truncated sequence's missing bytes) is observable

## Configuration table

Every row is exercised against **both** `.so`s through their exported symbols,
with **many randomized inputs** (seeded xorshift64\*, one fixed seed per row ⇒
reproducible) *and* the hand-derived boundary values, asserting byte-identical
results. Test names refer to `tests/configs.rs`.

| #  | entry point(s) | configuration (options set + input shape) | test | [x] |
|----|----------------|-------------------------------------------|------|-----|
| 1  | `w_utf8_drop` | `""` (empty) | `row01_drop_empty` | [x] |
| 2  | `w_utf8_drop` | pure ASCII `0x01..0x7F`: exhaustive single bytes + 4000 random strings, len 0..64 | `row02_drop_ascii` | [x] |
| 3  | `w_utf8_drop` | exhaustive **all 256** single-byte strings | `row03_drop_all_1byte` | [x] |
| 4  | `w_utf8_drop` | exhaustive **all 65 536** two-byte strings | `row04_drop_all_2byte` | [x] |
| 5  | `w_utf8_drop` | exhaustive 3-byte strings, leads `0xE0..0xEF` × all 256 × all 256 (1 048 576) | `row05_drop_all_3byte_e0_ef` | [x] |
| 6  | `w_utf8_drop` | exhaustive 4-byte strings, leads `0xF0..0xF7` × all 256 × 16 sampled × 16 sampled | `row06_drop_4byte_f0_f7` | [x] |
| 7  | `w_utf8_drop` | well-formed 2-byte only: exhaustive `0xC2..0xDF` × `0x80..0xBF`, + 3000 random multi-char | `row07_drop_valid2_only` | [x] |
| 8  | `w_utf8_drop` | well-formed 3-byte only, incl. `E0 A0 80`, `ED 9F BF`, `EE 80 80`, `EF BF BF` + 3000 random | `row08_drop_valid3_only` | [x] |
| 9  | `w_utf8_drop` | well-formed 4-byte only, incl. `F0 90 80 80`, `F4 8F BF BF` + 3000 random | `row09_drop_valid4_only` | [x] |
| 10 | `w_utf8_drop` | mixed well-formed 1/2/3/4-byte, 8000 random strings, 0..64 chars | `row10_drop_mixed_valid` | [x] |
| 11 | `w_utf8_drop` | uniformly random bytes, 20 000 strings, len 0..256 | `row11_drop_uniform_random` | [x] |
| 12 | `w_utf8_drop` | biased random (lead-byte / continuation / boundary heavy), 20 000 strings | `row12_drop_biased_random` | [x] |
| 13 | `w_utf8_drop` | valid prefix + trailing sequence truncated after 1/2/3 of its bytes, 6000 cases | `row13_drop_truncated_tail` | [x] |
| 14 | `w_utf8_drop` | bytes present *after* the NUL terminator, 5000 cases | `row14_drop_after_nul` | [x] |
| 15 | `w_utf8_drop` | long all-valid strings (≥ 40 000 bytes) | `row15_drop_long_valid` | [x] |
| 16 | `w_utf8_filter` | `replacement=0`, `""` ⇒ `strdup` path | `row16_17_filter_empty_both_flags` | [x] |
| 17 | `w_utf8_filter` | `replacement=1`, `""` ⇒ `strdup` path | `row16_17_filter_empty_both_flags` | [x] |
| 18 | `w_utf8_filter` | `replacement=0`, fully valid input ⇒ `strdup` path | `row18_19_filter_valid_strdup_path` | [x] |
| 19 | `w_utf8_filter` | `replacement=1`, fully valid input ⇒ `strdup` path | `row18_19_filter_valid_strdup_path` | [x] |
| 20 | `w_utf8_filter` | `replacement=0`, invalid byte at offset 0 ⇒ `memcpy` length 0 | `row20_21_filter_invalid_at_offset_zero` | [x] |
| 21 | `w_utf8_filter` | `replacement=1`, invalid byte at offset 0 ⇒ `memcpy` length 0 | `row20_21_filter_invalid_at_offset_zero` | [x] |
| 22 | `w_utf8_filter` | `replacement=0`, invalid byte in the middle ⇒ prefix `memcpy` > 0 | `row22_23_filter_invalid_in_middle` | [x] |
| 23 | `w_utf8_filter` | `replacement=1`, invalid byte in the middle ⇒ prefix `memcpy` > 0 | `row22_23_filter_invalid_in_middle` | [x] |
| 24 | `w_utf8_filter` | `replacement=0`, invalid byte / truncated char as the very last byte | `row24_25_filter_invalid_at_end` | [x] |
| 25 | `w_utf8_filter` | `replacement=1`, invalid byte / truncated char as the very last byte | `row24_25_filter_invalid_at_end` | [x] |
| 26 | `w_utf8_filter` | `replacement=0`, exhaustive all-256 1-byte strings | `row26_27_filter_all_1byte` | [x] |
| 27 | `w_utf8_filter` | `replacement=1`, exhaustive all-256 1-byte strings | `row26_27_filter_all_1byte` | [x] |
| 28 | `w_utf8_filter` | `replacement=0`, exhaustive all-65 536 2-byte strings | `row28_29_filter_all_2byte` | [x] |
| 29 | `w_utf8_filter` | `replacement=1`, exhaustive all-65 536 2-byte strings | `row28_29_filter_all_2byte` | [x] |
| 30 | `w_utf8_filter` | `replacement=0`, 3-byte strings over **every** multi-byte lead `0xC0..0xFF` × all 256 × 12 sampled | `row30_31_filter_3byte_leads` | [x] |
| 31 | `w_utf8_filter` | `replacement=1`, same as row 30 | `row30_31_filter_3byte_leads` | [x] |
| 32 | `w_utf8_filter` | `replacement=0`, uniform random bytes, 12 000 strings, len 0..256 | `row32_33_filter_uniform_random` | [x] |
| 33 | `w_utf8_filter` | `replacement=1`, same as row 32 | `row32_33_filter_uniform_random` | [x] |
| 34 | `w_utf8_filter` | `replacement=0`, biased random (well-formed chars ⊕ invalid bytes), len 0..1024 | `row34_35_filter_biased_random` | [x] |
| 35 | `w_utf8_filter` | `replacement=1`, same as row 34 | `row34_35_filter_biased_random` | [x] |
| 36 | `w_utf8_filter` | `replacement=1`, exactly `n` invalid bytes, `n ∈ {0,1,2,3,4,1364,1365,1366,1367,2729,2730,2731,2732,4094,4095,4096,4097}` ⇒ `repl < 3` / `REPLACEMENT_INC` boundary; pure runs *and* interleaved with well-formed chars | `row36_37_repl_threshold_boundary` | [x] |
| 37 | `w_utf8_filter` | `replacement=0`, same `n` sweep as row 36 (no realloc at all) | `row36_37_repl_threshold_boundary` | [x] |
| 38 | `w_utf8_filter` | `replacement=0/1`, 10 000 / 20 000 / 50 000 invalid bytes interleaved with well-formed chars ⇒ dozens of realloc rounds | `row38_filter_many_realloc_rounds` | [x] |
| 39 | `w_utf8_filter` | non-normalized `replacement ∈ {1,2,3,0x7F,0x80,0xFF,0x12345601,0xFFFFFFFF}` (low byte ≠ 0 ⇒ true) | `row39_40_non_normalized_bool` | [x] |
| 40 | `w_utf8_filter` | non-normalized `replacement ∈ {0,0x100,0xFF00,0x12345600,0xFFFFFF00,0x80000000}` (low byte 0 ⇒ false) | `row39_40_non_normalized_bool` | [x] |
| 41 | `w_utf8_filter` | `replacement=0/1`, bytes present after the NUL terminator (compared against the visible prefix alone) | `row41_filter_after_nul` | [x] |
| 42 | `w_utf8_filter` | `replacement=0/1`, ≥ 40 000-byte all-valid input (`strdup` path at scale) | `row42_filter_long_all_valid` | [x] |
| 43 | `w_utf8_filter` | `replacement=0/1`, ≥ 40 000-byte input with scattered invalid bytes | `row43_filter_long_scattered_invalid` | [x] |
| 44 | `w_utf8_drop` + `w_utf8_filter` | composed pipeline: `drop(filter(s))` must report the terminator; raw pointers from both scanners compared, both `replacement` values | `row44_composed_drop_of_filter` | [x] |
| 45 | `w_utf8_filter` | `filter(filter(s))` idempotence, compared C-vs-Rust, both `replacement` values | `row45_filter_idempotent` | [x] |
| 46 | `w_utf8_filter` | **internal allocation schedule**: `n` invalid bytes for `n ∈ {0,1,2,3,1365,1366,2730,2731,4095,4096} ∪ {k·1365, k·1365+1, k·4096 : k=1..8}`, plus the `strdup` path. Measured with `malloc_usable_size` in two children forked back-to-back from an identical heap, so `size = strlen+1 + g·REPLACEMENT_INC` is compared exactly. | `row46_allocation_schedule` | [x] |
| 47 | `w_utf8_drop` + `w_utf8_filter` | **no read past the terminator**: input placed so its NUL is the last readable byte before a `PROT_NONE` guard page. All 1-byte, all 2-byte, 3-byte over leads `0xC0..0xFF`, 4-byte over leads `0xF0..0xF7`, plus 20 000 randomized truncated sequences. | `row47_guard_page_no_overread` | [x] |

## Result

All 47 rows pass, in **both** the `dev` and `release` profiles (release also
switches `panic = "abort"`), for the single valid feature combination.
See `verify_all.sh` for the driver and `mutation_check.sh` for the proof that
these rows actually discriminate (19/19 injected bugs caught).
