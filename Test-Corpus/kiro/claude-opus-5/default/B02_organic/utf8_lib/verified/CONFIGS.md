# CONFIGS.md — Configuration surface table (Phase B gate)

Derived mechanically from `c_src/src/lib.c` + `c_src/include/lib.h`.

## Axes the C actually branches on

**Runtime options.** The public API has exactly one option: the `bool
replacement` parameter of `w_utf8_filter`. It gates the whole
`if (replacement) { … }` block (the `realloc` growth, the `EF BF BD` write and
the `repl` bookkeeping). There are no globals, no `#ifdef`-selected features
and no init/config struct.

**Public entry points (both, including the low-level one).**
* `w_utf8_drop(const char *)` — the low-level scanner. Not in the header, but
  non-static and exported, so it is a real public entry point and is tested
  directly, not only through the wrapper.
* `w_utf8_filter(const char *, bool)` — the one-shot wrapper that calls
  `w_utf8_drop` and then re-runs the same `valid_1..4` ladder itself.

**Input shapes the code special-cases.**
* length: empty / 1 byte / short / long / crossing `REPLACEMENT_INC` (4096)
* which of the four `valid_N` ladder arms accepts a sequence (1/2/3/4 bytes)
* boundary lead/continuation values: `0xC2`, `0xDF`, `0xE0`+`0xA0`, `0xED`+`0x9F`,
  `0xEF`+`0xBF`, `0xF0`+`0x90`, `0xF4`+`0x8F`
* position of the first rejected byte: none at all (`strdup` fast path),
  offset 0 (`memcpy` length 0), middle (`memcpy` length > 0), last byte
* density of rejected bytes: none / one / many / all — with `replacement=true`
  this decides how many `realloc` rounds happen
* whether a multi-byte sequence is truncated by the `NUL` terminator

## Rows (pruned cross-product of the axes above)

Every row is exercised with **many randomized inputs** (fixed seed, xorshift64
PRNG in `tests/differential.rs`) and both `.so`s' outputs are compared
byte-for-byte.

| #  | entry point(s) | configuration (options set + input shape) | [x] |
|----|----------------|-------------------------------------------|-----|
| 1  | `w_utf8_drop` | empty string `""` | [x] |
| 2  | `w_utf8_drop` | random pure-ASCII `0x01`–`0x7F`, lengths 1…64 | [x] |
| 3  | `w_utf8_drop` | random valid 2-byte sequences only (`0xC2`–`0xDF` + `0x80`–`0xBF`), incl. `0xC2`/`0xDF` boundaries | [x] |
| 4  | `w_utf8_drop` | random valid 3-byte sequences only, incl. `E0 A0`, `ED 9F`, `EF BF` boundaries | [x] |
| 5  | `w_utf8_drop` | random valid 4-byte sequences only, incl. `F0 90`, `F4 8F` boundaries | [x] |
| 6  | `w_utf8_drop` | random mix of all four valid widths (fully valid input → `NUL` return) | [x] |
| 7  | `w_utf8_drop` | uniform random bytes `0x01`–`0xFF` (mostly invalid, dense rejections) | [x] |
| 8  | `w_utf8_drop` | valid prefix + single invalid byte injected at a random offset | [x] |
| 9  | `w_utf8_drop` | invalid byte at offset 0 | [x] |
| 10 | `w_utf8_drop` | truncated multi-byte sequence at end of string (lead byte(s) then `NUL`) | [x] |
| 11 | `w_utf8_drop` | long inputs (1 KiB…16 KiB) of mixed valid/invalid | [x] |
| 12 | `w_utf8_filter` | `replacement=false`, empty string (`strdup` fast path) | [x] |
| 13 | `w_utf8_filter` | `replacement=true`, empty string (`strdup` fast path) | [x] |
| 14 | `w_utf8_filter` | `replacement=false`, fully valid input (`strdup` fast path, all four widths) | [x] |
| 15 | `w_utf8_filter` | `replacement=true`, fully valid input (`strdup` fast path, all four widths) | [x] |
| 16 | `w_utf8_filter` | `replacement=false`, single invalid byte at offset 0 (`memcpy` len 0) | [x] |
| 17 | `w_utf8_filter` | `replacement=true`, single invalid byte at offset 0 (`memcpy` len 0) | [x] |
| 18 | `w_utf8_filter` | `replacement=false`, single invalid byte in the middle (`memcpy` len > 0) | [x] |
| 19 | `w_utf8_filter` | `replacement=true`, single invalid byte in the middle (`memcpy` len > 0) | [x] |
| 20 | `w_utf8_filter` | `replacement=false`, invalid byte as the last byte | [x] |
| 21 | `w_utf8_filter` | `replacement=true`, invalid byte as the last byte | [x] |
| 22 | `w_utf8_filter` | `replacement=false`, uniform random bytes (dense rejections), lengths 1…64 | [x] |
| 23 | `w_utf8_filter` | `replacement=true`, uniform random bytes (dense rejections), lengths 1…64 | [x] |
| 24 | `w_utf8_filter` | `replacement=false`, random mix of valid widths + random invalid bytes | [x] |
| 25 | `w_utf8_filter` | `replacement=true`, random mix of valid widths + random invalid bytes | [x] |
| 26 | `w_utf8_filter` | `replacement=false`, all bytes invalid (`0x80`, `0xFF`, `0xC0`, `0xF5`…), 1…4096 long | [x] |
| 27 | `w_utf8_filter` | `replacement=true`, ≥ 1366 invalid bytes → forces the **2nd** `realloc` (`repl` wraps through `1 < 3`) | [x] |
| 28 | `w_utf8_filter` | `replacement=true`, ≥ 5000 invalid bytes → forces **4+** `realloc` rounds | [x] |
| 29 | `w_utf8_filter` | `replacement=false`, input longer than `REPLACEMENT_INC` (4096) with sparse invalid bytes | [x] |
| 30 | `w_utf8_filter` | `replacement=true`, input longer than `REPLACEMENT_INC` (4096) with sparse invalid bytes | [x] |
| 31 | `w_utf8_filter` | `replacement=false`, truncated multi-byte sequences at end | [x] |
| 32 | `w_utf8_filter` | `replacement=true`, truncated multi-byte sequences at end | [x] |
| 33 | `w_utf8_filter` | `replacement` passed as an out-of-range `_Bool` (2, 3, 0x7F, 0xFF) | [x] |
| 34 | `w_utf8_drop` → `w_utf8_filter` | composed pipeline: use `w_utf8_drop`'s offset to predict `w_utf8_filter`'s `memcpy` prefix, and assert the prefix of the filtered output equals that prefix, for both `.so`s, over random inputs | [x] |
| 35 | `w_utf8_filter` | both flag values on the *same* random corpus (cross-check that only the invalid-byte handling differs) | [x] |
| 36 | `w_utf8_drop` | every single byte `0x01`–`0xFF` as a 1-byte string (exhaustive lead-byte sweep) | [x] |
| 37 | `w_utf8_drop` | exhaustive 2-byte sweep: all 65 025 `(b0, b1)` pairs with `b0, b1 ∈ 0x01..0xFF` | [x] |
| 38 | `w_utf8_filter` | exhaustive 2-byte sweep, both `replacement` values | [x] |
| 39 | `w_utf8_drop` | exhaustive 3-byte sweep over leads `0xE0`–`0xEF` × all `(b1, b2)` | [x] |
| 40 | `w_utf8_drop` | exhaustive 4-byte sweep over leads `0xF0`–`0xF4` × boundary-focused `(b1, b2, b3)` | [x] |
