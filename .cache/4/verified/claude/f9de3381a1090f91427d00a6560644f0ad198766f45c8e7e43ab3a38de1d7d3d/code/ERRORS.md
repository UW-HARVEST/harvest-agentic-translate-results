# ERRORS.md — Error-surface table (Phase C gate)

Mechanically derived from `c_src/src/lib.c`. Every rejection/error path in the
translation unit was found by grepping for `return`, `NULL`, `assert`, range
checks and min/max constants:

```
$ grep -n 'return\|assert\|NULL\|if (' c_src/src/lib.c
33:    if (!src) {
34:        return NULL;      <-- rejection #1
37:    if (!size) {
42:    if (!out) {
43:        return NULL;      <-- rejection #2
82:    return out;           <-- success
```

Findings:

* There are **exactly two** `return NULL` statements — i.e. two distinct
  rejection mechanisms.
* There are **no** `assert()`s, **no** error enums / error codes, **no**
  `errno` writes, and **no** named min/max constants in the library.
* The only failure signal in the whole ABI is the sentinel **`NULL` return**.
* `static char encode(unsigned char u)` has **no** rejection path: it is total
  over `0..=255` (final `return '/'` is the catch-all), so it contributes no
  rows. Note the callers only ever pass `0..=63`.
* There are **no `enum` parameters** in the public API (`lib.h` declares
  `char *encode_base64(int size, const char *src)`), so the "out-of-range enum
  value across FFI" class maps onto the only scalar parameter, `int size`,
  whose *entire* `i32` domain is a legal FFI input. Rows 5-16 below cover that
  domain, including `INT_MIN` and values one step outside the implicitly
  documented `size >= 0` range.

## Rejection rows

| # | function | trigger (the exact invalid input/condition) | expected C result | done |
|---|----------|----------------------------------------------|-------------------|-----|
| 1 | `encode_base64` | `src == NULL`, `size == 0` — hits `if (!src)` L33 | returns `NULL` | [x] |
| 2 | `encode_base64` | `src == NULL`, `size > 0` (e.g. `1`, `3`, `4096`) — null check precedes everything | returns `NULL` | [x] |
| 3 | `encode_base64` | `src == NULL`, `size < 0` (e.g. `-1`, `-4`) | returns `NULL` | [x] |
| 4 | `encode_base64` | `src == NULL`, `size == i32::MIN` and `size == i32::MAX` — the null check short-circuits before any arithmetic, so even the UB-inducing sizes are safe | returns `NULL` | [x] |
| 5 | `encode_base64` | `size == -4`, `src` valid ⇒ `size*4/3+4 == -1` ⇒ `(size_t)(-1)` = `SIZE_MAX` ⇒ `calloc` fails ⇒ `if (!out)` L42 | returns `NULL` | [x] |
| 6 | `encode_base64` | `size == -5`, `src` valid ⇒ `nbytes == -2` ⇒ huge `size_t` ⇒ `calloc` fails | returns `NULL` | [x] |
| 7 | `encode_base64` | `size == -6`, `src` valid ⇒ `nbytes == -4` ⇒ `calloc` fails | returns `NULL` | [x] |
| 8 | `encode_base64` | `size == -7`, `src` valid ⇒ `nbytes == -5` ⇒ `calloc` fails | returns `NULL` | [x] |
| 9 | `encode_base64` | `size == -100`, `src` valid ⇒ `nbytes == -129` ⇒ `calloc` fails | returns `NULL` | [x] |
| 10 | `encode_base64` | `size` randomized in `[-536870000, -4]` (no `int` wrap) ⇒ `nbytes < 0` ⇒ `calloc` fails | returns `NULL` for every such `size` | [x] |
| 11 | `encode_base64` | **boundary, NOT rejected:** `size == -3` ⇒ `nbytes == 0` ⇒ `calloc(1, 0)` returns a non-NULL zero-length block; loop skipped | returns non-`NULL`, empty string | [x] |
| 12 | `encode_base64` | **boundary, NOT rejected:** `size == -1` (one step past the valid `size >= 0` range) ⇒ `nbytes == 3` ⇒ `calloc` succeeds; loop skipped | returns non-`NULL`, empty string | [x] |
| 13 | `encode_base64` | **boundary, NOT rejected:** `size == -2` ⇒ `nbytes == 2` ⇒ `calloc` succeeds; loop skipped | returns non-`NULL`, empty string | [x] |
| 14 | `encode_base64` | **boundary, NOT rejected:** `size == i32::MIN` ⇒ `size*4` wraps to `0` ⇒ `nbytes == 4` ⇒ `calloc` succeeds; `0 < INT_MIN` is false so loop skipped | returns non-`NULL`, empty string | [x] |
| 15 | `encode_base64` | **boundary, NOT rejected:** `size == i32::MIN + 1 .. i32::MIN + 8` ⇒ `size*4` wraps to a small positive `int` ⇒ `calloc` succeeds; loop skipped | returns non-`NULL`, empty string | [x] |
| 16 | `encode_base64` | zero length: `size == 0` with `src == ""` ⇒ `strlen == 0` ⇒ `size` stays `0` ⇒ `nbytes == 4` ⇒ succeeds, loop skipped | returns non-`NULL`, empty string | [x] |

Rows 11-16 are *non*-rejections that sit immediately adjacent to rejection rows
5-10; they are included because the C's rejection boundary is exactly
`nbytes < 0`, which is **not** the same as `size < 0`. A translation that
"sanitised" negative sizes into an error would pass rows 5-10 and fail 11-15.

## Oversized / out-of-range inputs that are UB in the C (excluded, with reason)

| trigger | why not executed |
|---------|------------------|
| `size >= 2^29` (`INT_MAX`, `2^29`, ...) with a **valid** `src` | `size*4` overflows `int`; the resulting `nbytes` is small or negative, so either `calloc` fails (⇒ `NULL`, benign) or `calloc` returns a tiny buffer *and the loop still runs `size/3` iterations*, writing gigabytes out of bounds. The C ground truth segfaults, so no differential comparison is possible. Both implementations compute `nbytes` with identical wrapping `int` arithmetic (`wrapping_mul(4).wrapping_div(3).wrapping_add(4)`), verified by source inspection. |
| `size > 0` with a `src` buffer shorter than `size` bytes | out-of-bounds *read* in the C by construction. |

## Row → test mapping (all rows verified)

Each test asserts the two implementations return the **same** sentinel, i.e.
both NULL or both a non-NULL buffer with identical contents — never merely
"both failed somehow".

| ERRORS row | test in `tests/differential.rs` | asserted result |
|------------|---------------------------------|-----------------|
| 1  | `err01_null_src_size_zero`                     | both NULL |
| 2  | `err02_null_src_positive_size`                 | both NULL (13 fixed + 2 000 random sizes) |
| 3  | `err03_null_src_negative_size`                 | both NULL (10 fixed + 2 000 random sizes) |
| 4  | `err04_null_src_extreme_sizes`                 | both NULL (`i32::MIN`, `i32::MAX`, …) |
| 5  | `err05_size_minus4_calloc_fails`               | both NULL |
| 6  | `err06_size_minus5_calloc_fails`               | both NULL |
| 7  | `err07_size_minus6_calloc_fails`               | both NULL |
| 8  | `err08_size_minus7_calloc_fails`               | both NULL |
| 9  | `err09_size_minus100_calloc_fails`             | both NULL |
| 10 | `err10_random_negative_sizes_reject`           | both NULL (4 000 random + all of `-4000..=-4`) |
| 11 | `err11_size_minus3_zero_alloc_not_rejected`    | both non-NULL, empty |
| 12 | `err12_size_minus1_not_rejected`               | both non-NULL, empty |
| 13 | `err13_size_minus2_not_rejected`               | both non-NULL, empty |
| 14 | `err14_size_int_min_not_rejected`              | both non-NULL, empty |
| 15 | `err15_sizes_just_above_int_min_not_rejected`  | both non-NULL, empty |
| 16 | `err16_zero_length_not_rejected`               | both non-NULL, empty |
| generic | `boundary_size_domain_sweep_with_valid_src` | NULL src rejected for every i32 boundary size; valid src compared |
| generic | `boundary_null_vs_nonnull_agreement_table`  | the exact accept/reject boundary, both pointer kinds |

All 16 rows pass, under every build configuration and against the C library
compiled at `-O0`, `-O2` and `-O3`.
