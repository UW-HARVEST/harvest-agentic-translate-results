# CONFIGS.md — Phase A configuration surface table

Mechanically derived from every branch the C in `c_src/src/lib.c` actually
takes on **valid** input. There are no runtime options, flags, modes, `switch`
statements, `#ifdef`s or global state in this library: `grep -n 'if\|while\|#if\|switch' c_src/src/lib.c` yields

```
12:    while (lineIndex < numLines && pos < bufferSize) {   <- outer loop guard (2 sub-conditions)
17:        while ((pos + len < bufferSize) && buffer[pos + len] != '\0') {  <- inner scan (2 sub-conditions)
23:        if (pos < bufferSize) pos++;                     <- conditional NUL skip
27:    if (lineIndex != numLines) {                         <- line-count verification
10:    if (bufferPtrs == NULL) return NULL;                 <- malloc check
```

## Axes the code distinguishes

* **A. `numLines`**: `0` / `1` / `many`; and relative to the number of
  NUL-separated lines really present: `<` present, `==` present.
  (`>` present is an error ⇒ `ERRORS.md`.)
* **B. `bufferSize`**: `0` / `1` / exactly the content length / **larger** than
  the content (trailing padding, which creates extra empty lines) / a value
  that **truncates** the buffer in the middle of a line.
* **C. buffer shape / which loop branches fire**:
  * inner-scan length `len == 0` (line starts with `'\0'` ⇒ empty line) vs `len > 0`.
  * line 23 `pos < bufferSize` **true** (a real NUL was consumed) vs **false**
    (the final line runs flush to the end of the buffer with no terminator ⇒
    `pos` lands exactly on `bufferSize`).
  * all-`'\0'` buffer, all-non-`'\0'` buffer, mixed line lengths, one long line.
* **D. entry point**: there is exactly **one** public entry point,
  `UTIL_createLinePointers` (the lowest level *is* the only level), so every
  row below drives it directly through the `.so` export.

## Rows (cross-product pruned to combinations the C treats differently)

Each row is exercised with many randomized inputs (fixed seed `0x5EED_1234`,
xorshift64* PRNG) and both `.so`s are compared byte-for-byte: NULL-ness of the
result plus every returned pointer converted to its offset from `buffer` (raw
pointers differ per-library only via the heap block, the *offsets into the
caller's buffer* are the observable output and must be identical), for all
`numLines` slots.

| # | entry point(s) | configuration (options set + input shape) | test | [x] |
|---|----------------|--------------------------------------------|------|-----|
| 1 | `UTIL_createLinePointers` | A=`numLines=0`, B=`bufferSize=0`, C=empty buffer ⇒ loop never entered, `malloc(0)` returned as success | `cfg_01_zero_lines_zero_size` | [x] |
| 2 | `UTIL_createLinePointers` | A=`numLines=0`, B=`bufferSize>0` (randomized content) ⇒ loop guard fails on `lineIndex<numLines`, success | `cfg_02_zero_lines_nonempty_buffer` | [x] |
| 3 | `UTIL_createLinePointers` | A=`numLines=1`, B=`bufferSize=1`, C=buffer is a single `'\0'` ⇒ `len==0`, line 23 taken (`pos`0→1) | `cfg_03_one_line_single_nul` | [x] |
| 4 | `UTIL_createLinePointers` | A=`numLines=1`, B=`bufferSize=1`, C=buffer is one non-NUL byte ⇒ `len==1`, `pos` reaches `bufferSize`, line 23 **not** taken | `cfg_04_one_line_single_nonnul` | [x] |
| 5 | `UTIL_createLinePointers` | A=`numLines=1`, B=exact, C=one NUL-terminated line, randomized length/bytes ⇒ `len>0`, line 23 taken | `cfg_05_one_terminated_line` | [x] |
| 6 | `UTIL_createLinePointers` | A=`numLines=1`, B=exact, C=one line with **no** terminator (`bufferSize == strlen`) ⇒ line 23 not taken | `cfg_06_one_unterminated_line` | [x] |
| 7 | `UTIL_createLinePointers` | A=`numLines==` lines present (`many`, 2..16), B=exact, C=all lines NUL-terminated, randomized non-empty lengths | `cfg_07_many_terminated_lines_exact` | [x] |
| 8 | `UTIL_createLinePointers` | A=`numLines==` lines present (`many`), B=exact, C=**last line unterminated** (buffer ends mid-line) ⇒ final iteration skips line 23 | `cfg_08_many_lines_last_unterminated` | [x] |
| 9 | `UTIL_createLinePointers` | A=`numLines==` lines present, C=**empty lines mixed in** (consecutive NULs, `len==0` iterations interleaved with `len>0`) | `cfg_09_mixed_empty_and_nonempty_lines` | [x] |
| 10 | `UTIL_createLinePointers` | A=`numLines==bufferSize`, C=buffer is **all NULs** ⇒ every iteration `len==0`, line 23 always taken | `cfg_10_all_nul_buffer` | [x] |
| 11 | `UTIL_createLinePointers` | A=`numLines <` lines present (strict prefix), B=exact ⇒ outer loop exits on `lineIndex<numLines` while `pos<bufferSize` still true | `cfg_11_fewer_requested_than_present` | [x] |
| 12 | `UTIL_createLinePointers` | B=`bufferSize` **larger** than the string content (trailing NUL padding creates extra empty lines that get consumed) | `cfg_12_trailing_nul_padding` | [x] |
| 13 | `UTIL_createLinePointers` | B=`bufferSize` **truncated** mid-line (caller passes a smaller size than the real content) ⇒ inner scan stops on `pos+len<bufferSize`, not on `'\0'` | `cfg_13_truncated_buffersize_midline` | [x] |
| 14 | `UTIL_createLinePointers` | C=**one long line** (length 1..4096, randomized) with terminator ⇒ long inner scan | `cfg_14_single_long_line` | [x] |
| 15 | `UTIL_createLinePointers` | C=buffer with **no NUL at all** and `numLines==1` ⇒ single line spanning whole buffer | `cfg_15_no_terminator_single_line` | [x] |
| 16 | `UTIL_createLinePointers` | **Fully randomized fuzz**: random `bufferSize` 0..64, random bytes (biased so `'\0'` is frequent), random `numLines` 0..20 — hits every success *and* failure combination indiscriminately | `cfg_16_full_random_fuzz` | [x] |
| 17 | `UTIL_createLinePointers` | Repeated / interleaved invocations on the same buffer with different `numLines` (checks there is no hidden static state between calls) | `cfg_17_repeated_calls_no_state` | [x] |
