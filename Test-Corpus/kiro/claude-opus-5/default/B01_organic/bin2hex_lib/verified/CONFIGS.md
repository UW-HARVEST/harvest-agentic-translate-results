# CONFIGS.md — Configuration-surface table (valid inputs)

Derived mechanically from `c_src/src/lib.c` + `c_src/include/lib.h`.

## Axis enumeration

**Runtime options / modes / flags:** *none.* `grep -n 'if \|switch\|#if\|#ifdef\|static\|global' c_src/src/lib.c`
finds only the single `abort()` guard. The library has no configuration struct,
no context object, no flags parameter, no globals, and no `#ifdef`s. The header
declares exactly one function and no macros or enums. So the "options set"
column collapses to the function arguments themselves.

**Build-time features:** `translation/Cargo.toml` has **no `[features]`**
section, so there is exactly one feature combination (default ==
`--no-default-features`).

**Public entry points (complete set, lowest-level included):**

| entry point | notes |
|-------------|-------|
| `bin2hex(char *hex, size_t hex_maxlen, const uint8_t *bin, size_t bin_len)` | the only exported symbol; it *is* the lowest-level entry point — there is no convenience wrapper and no internal layering |

**Input shapes the C actually distinguishes** (branch/value analysis of the loop
body, not guesswork):

1. `bin_len`: `0` (loop never runs) vs `>= 1` (loop runs). Sub-shapes for
   `i * 2U` index arithmetic: 1, 2, small, 255/256/257 (byte-count boundary),
   large.
2. `hex_maxlen`: exactly `bin_len*2 + 1` (minimum accepted) vs strictly greater
   (slack — bytes past `bin_len*2` must stay untouched). `hex_maxlen` is
   otherwise unused after the guard.
3. **Per-nibble value class.** The branch-free digit expression
   `(unsigned char)(87U + n + (((n - 10U) >> 8) & ~38U))` takes two
   *arithmetically* distinct paths per nibble: `n <= 9` (the `n - 10U`
   subtraction wraps, `>> 8` yields `0x00FFFFFF`, adjust `= 0x00FFFFD9`, digit
   `'0'..'9'`) vs `n >= 10` (no wrap, adjust `= 0`, digit `'a'..'f'`). Applied
   independently to the high nibble `b = byte >> 4` and the low nibble
   `c = byte & 0xf`, giving a 2×2 cross-product of value classes per byte.
4. **Nibble emission order.** `x = digit(c) << 8 | digit(b)`, then
   `hex[2i] = (char)x` (low byte = high nibble's digit) and
   `hex[2i+1] = (char)(x >> 8)` (low nibble's digit). A byte with two *different*
   digit classes (e.g. `0x0A`, `0xA0`) is required to detect a swapped order;
   uniform bytes cannot.
5. Pointer alignment: `char`/`uint8_t` accesses have no alignment requirement,
   but the Rust translation forms slices, so misaligned/odd-offset buffers are a
   distinct shape worth covering.
6. NUL terminator placement: written at `hex[bin_len*2]` after the loop, using
   the post-loop value of `i`.
7. `bin == NULL` combined with `bin_len == 0` (accepted; never dereferenced).

Rows below are the pruned cross-product of axes 1–7 — the combinations the C
treats differently. Every row is driven through the `.so` exports of **both**
libraries with **many randomized inputs** (seeded, reproducible `SplitMix64`
PRNG, fixed seed per row) and compared byte-for-byte over the whole output
buffer, plus the returned pointer.

## Table

| # | entry point(s) | configuration (options set + input shape) | [x] |
|---|----------------|--------------------------------------------|-----|
| C1 | `bin2hex` | `bin_len = 0`, `hex_maxlen = 1` (minimum accepted); loop never runs, only the NUL is written | [x] |
| C2 | `bin2hex` | `bin_len = 0`, `hex_maxlen` randomized in `2..=4096` (slack); tail past `hex[0]` must be untouched | [x] |
| C3 | `bin2hex` | `bin_len = 0`, `bin = NULL`, `hex_maxlen = 1` and randomized slack — accepted, no deref | [x] |
| C4 | `bin2hex` | `bin_len = 1`, `hex_maxlen = 3` (exact min), **all 256 byte values exhaustively** — covers the full 2×2 nibble-class cross-product and both digit ranges | [x] |
| C5 | `bin2hex` | `bin_len = 1`, `hex_maxlen` randomized slack `4..=64`, all 256 byte values — verifies untouched tail | [x] |
| C6 | `bin2hex` | `bin_len` randomized `2..=16`, uniform-random bytes, `hex_maxlen` exact min | [x] |
| C7 | `bin2hex` | `bin_len` randomized `2..=16`, uniform-random bytes, `hex_maxlen` randomized slack | [x] |
| C8 | `bin2hex` | `bin_len` randomized `17..=4096`, uniform-random bytes, `hex_maxlen` exact min | [x] |
| C9 | `bin2hex` | `bin_len` large `4096..=65536`, uniform-random bytes, `hex_maxlen` randomized slack | [x] |
| C10 | `bin2hex` | nibble class **both `<10`** (BCD bytes: hi ∈ `0..=9`, lo ∈ `0..=9`), randomized `bin_len 1..=512` → output is all `'0'..'9'` | [x] |
| C11 | `bin2hex` | nibble class **hi `>=10`, lo `<10`** (bytes `0xA0..=0xF9` with lo ≤ 9), randomized `bin_len` → alternating letter/digit; catches swapped emission order | [x] |
| C12 | `bin2hex` | nibble class **hi `<10`, lo `>=10`** (bytes `0x0A..=0x9F` with lo ≥ 10), randomized `bin_len` → alternating digit/letter; catches swapped emission order | [x] |
| C13 | `bin2hex` | nibble class **both `>=10`** (bytes with hi ≥ 10 and lo ≥ 10), randomized `bin_len` → output all `'a'..'f'` | [x] |
| C14 | `bin2hex` | boundary byte values only, drawn from `{0x00,0x01,0x09,0x0A,0x0F,0x10,0x90,0x99,0x9A,0x9F,0xA0,0xA9,0xAA,0xAF,0xF0,0xF9,0xFA,0xFF}` — the nibble-class transition points 9↔10 and 15↔0 | [x] |
| C15 | `bin2hex` | `bin_len ∈ {255, 256, 257}` (byte-count / index boundary), randomized bytes, both exact-min and slack `hex_maxlen` | [x] |
| C16 | `bin2hex` | `hex_maxlen = usize::MAX` with small randomized `bin_len` (maximum slack accepted by the guard; `bin_len*2` does not wrap) | [x] |
| C17 | `bin2hex` | **misaligned buffers**: `hex` and `bin` at odd byte offsets `1,2,3,5,7` inside their allocations, randomized `bin_len`/bytes | [x] |
| C18 | `bin2hex` | **repeated calls on the same output buffer** with different `bin_len` (long then short), verifying no residual state and that the NUL lands at `hex[bin_len*2]` leaving later stale bytes untouched | [x] |
| C19 | `bin2hex` | **returned pointer identity** across all of the above: the return value must equal the `hex` argument exactly, for both `.so`s | [x] |
| C20 | `bin2hex` | `bin_len` at the largest value that is *practically* allocatable while still exercising the guard's accept path near the limit: `bin_len` randomized in `1..=8` with `hex_maxlen = bin_len*2 + 1` and separately `bin_len*2 + 2` (off-by-one on the accept side of `hex_maxlen <= bin_len*2`) | [x] |

## How the rows are driven

`tests/valid_paths.rs` contains one `#[test]` per row (`c1_…` … `c20_…`). Every
row:

* loads **both** `.so`s with `libloading` and calls only the exported `bin2hex`
  symbol — the Rust function is never called directly, so the
  `#[no_mangle] extern "C"` wrapper is under test too;
* allocates the output buffer with `GUARD = 32` bytes of untouched padding after
  the usable region (and optional padding before it, for the misalignment rows),
  pre-filled with a per-iteration pattern, then compares the **entire
  allocation** of the C run against the entire allocation of the Rust run — so
  both an incorrect byte and an out-of-bounds write fail the row;
* asserts the input buffer is left unmodified by both;
* asserts the return value is exactly the `hex` argument;
* uses a seeded `SplitMix64` PRNG (one fixed seed per row) so failures are
  reproducible.

Row counts: C4/C5 are exhaustive over all 256 byte values; C6/C7 run 2 000
random inputs each, C14 1 000, C2/C16/C19 300–512, C10–C13 400 each, C8 300,
C17 768, C18 200 trials × 9 calls, C9 40 large inputs.

Total: **20 rows, 20 passing tests**, under every feature combination and both
build profiles (see `verify_all_features.sh`).
