# CONFIGS.md — Phase B configuration-surface table

## Axes the C code actually branches on

Derived from `c_src/src/lib.c` and `c_src/include/lib.h`.

* **Runtime options / modes / flags:** none. The public header declares exactly
  one entry point, `int wcscat(wchar_t *dst, size_t numElem, const wchar_t *src)`,
  and there is no global state, no init/config call, no `#ifdef` in the source.
  So the configuration surface is entirely made of *input shapes*.
* **Full set of public entry points:** `wcscat` — it *is* the lowest-level entry
  point; there is no convenience wrapper layer to skip past.
* **Input shapes the code special-cases:**
  * `dst` NUL-terminated inside `numElem` vs. not terminated (decides whether the
    copy loop body runs at all).
  * position of `dst`'s terminator: 0 (empty), 1, middle, `numElem-1` (last
    element, i.e. the seek loop stops on the final element).
  * `src` length: 0 (empty), 1, many.
  * slack = `numElem - (wcslen(dst) + wcslen(src) + 1)`: > 0 (room to spare),
    == 0 (exact fit — the terminator lands in the very last element, allowed
    because the bound is `ptr < dst + numElem`), < 0 (overflow → `34`).
  * `numElem`: 1 (minimum accepted), small, large; larger than the real buffer.
  * residual bytes in `dst` past its terminator (must be observably overwritten
    or preserved exactly as C leaves them).
  * element values: ASCII, non-BMP, negative (`wchar_t` is signed 32-bit on
    Linux), `INT_MIN`, `INT_MAX`, surrogate range, `> 0x10FFFF`.
  * `wchar_t` width/signedness: fixed by the platform (4-byte signed `int` on
    the Linux/glibc target); asserted by the test harness.

Every row is exercised with many randomized inputs (fixed seed, deterministic
xorshift PRNG) and compared C-vs-Rust over the **whole** buffer plus guard
padding, not just the return value.

| #  | entry point(s) | configuration (options set + input shape) | ok |
|----|----------------|-------------------------------------------|----|
| 1  | `wcscat` | `dst` empty (`dst[0]==0`), `src` empty, slack > 0 | [x] |
| 2  | `wcscat` | `dst` empty, `src` length 1, slack > 0 | [x] |
| 3  | `wcscat` | `dst` empty, `src` long, slack > 0 | [x] |
| 4  | `wcscat` | `dst` empty, `src` long, **exact fit** (slack == 0) | [x] |
| 5  | `wcscat` | `dst` non-empty (terminator in the middle), `src` empty, slack > 0 | [x] |
| 6  | `wcscat` | `dst` non-empty, `src` length 1, slack > 0 | [x] |
| 7  | `wcscat` | `dst` non-empty, `src` long, slack > 0 | [x] |
| 8  | `wcscat` | `dst` non-empty, `src` long, **exact fit** (slack == 0) | [x] |
| 9  | `wcscat` | `dst`'s terminator sits in the **last** element (`dst[numElem-1]==0`), `src` empty → the terminator is rewritten in place, returns 0 | [x] |
| 10 | `wcscat` | `dst`'s terminator sits in the last element, `src` non-empty → no room, returns 34 (valid-shape input that lands on the overflow path) | [x] |
| 11 | `wcscat` | `numElem == 1`, `dst[0]==0`, `src` empty (smallest possible success) | [x] |
| 12 | `wcscat` | `dst` contains non-zero residual data **after** its terminator; append shorter than the residue, so the tail must be preserved byte-for-byte | [x] |
| 13 | `wcscat` | `dst` **unterminated** within `numElem` (seek loop exhausts the buffer), `src` non-empty → 34, `src` untouched | [x] |
| 14 | `wcscat` | `dst` unterminated, `src` empty → 34 | [x] |
| 15 | `wcscat` | overflow by exactly one element (slack == -1) | [x] |
| 16 | `wcscat` | overflow by many elements: `src` far longer than the remaining room → partial fill then `dst[0]=0`, 34 | [x] |
| 17 | `wcscat` | `numElem` smaller than the real allocation (guard region past `numElem` must stay pristine — proves no over-write) | [x] |
| 18 | `wcscat` | `numElem` **larger** than the logical data but within the real allocation ("oversized length" that is never noticed) | [x] |
| 19 | `wcscat` | element values: random full-range 32-bit non-zero `wchar_t` (negative values included) in both `dst` and `src` | [x] |
| 20 | `wcscat` | element values: extremes only — `INT_MIN`, `INT_MAX`, `-1`, `0x110000`, `0xD800` surrogates | [x] |
| 21 | `wcscat` | `src` and `dst` are the **same length class** but randomized across all of the above simultaneously (fuzz sweep over `numElem` 1..=24, random terminator position or none, random `src` length) — the cross-product row | [x] |
| 22 | `wcscat` | `dst` buffer of size 1..=4 swept exhaustively against `src` of length 0..=4 with every terminator position, including none (exhaustive small-shape cross-product) | [x] |
