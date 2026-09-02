# CONFIGS.md — Phase B configuration-surface table

Derived mechanically from `c_src/src/lib.c` and `c_src/include/lib.h`.

## Axes the C code actually branches on

The C source has exactly five control-flow constructs
(`grep -n 'if\s*(\|while\s*(\|switch' src/lib.c`):

```
 7:    if (!dst || numElem == 0)                 -> A1 (dst nullness), A2 (numElem == 0)
 9:    if (!src) {                               -> A3 (src nullness)
13:    while (ptr < dst + numElem && *ptr != 0)  -> A4 (where dst's NUL is, if any)
15:    while (ptr < dst + numElem) {             -> A5 (does src's NUL land in bounds?)
16:        if ((*ptr++ = *src++) == 0)           -> A5
```

* **Runtime options / modes / flags:** there are **none**. The public header
  declares a single function with no flag, mode, or option parameter, there is
  no global/static state, no `#ifdef`, and `Cargo.toml` declares no
  `[features]`. The only "configuration" a consumer can express is the shape and
  content of the three arguments. That is enumerated exhaustively below.
* **A1 `dst`:** NULL / non-NULL. (NULL → `ERRORS.md`.)
* **A2 `numElem`:** `0` (→ `ERRORS.md`) / `1` / `2` / small / large / oversized
  relative to the real allocation.
* **A3 `src`:** NULL (→ `ERRORS.md`) / non-NULL.
* **A4 existing `dst` state:** NUL at index `0` (empty string) / NUL at
  `0 < k < numElem-1` / NUL exactly at `numElem-1` (last element) / **no NUL**
  within `numElem` (unterminated) / NUL present only at an index `>= numElem`.
* **A5 fit relationship:** `k + wcslen(src) + 1 < numElem` (room to spare) /
  `== numElem` (exact fit, NUL occupies the last element) / `== numElem + 1`
  (one element short — chars fit but NUL does not) / `>> numElem` (grossly
  oversized `src`).
* **A6 `src` length:** `0` (empty string) / `1` / many.
* **A7 `wchar_t` value classes** (the C only ever compares against `0`, so every
  bit pattern is data): ASCII, non-BMP (`> 0xFFFF`), surrogate range
  `0xD800..=0xDFFF`, above `0x10FFFF`, negative, `i32::MIN`, `i32::MAX`.
* **A8 aliasing:** `src` disjoint from `dst` / `src == dst` / `src` pointing
  into the interior of `dst`. No overlap check exists, so these are distinct
  valid inputs.
* **A9 call sequencing:** one call / repeated calls accumulating into the same
  buffer (the real consumer pipeline — `wcscat` is by definition used
  iteratively, and cumulative state is where composed-pipeline bugs hide).

## Full set of public entry points

`nm -D --defined-only` on the C `.so` yields exactly one entry point, `wcscat`,
which *is* the lowest-level entry point — there is no convenience wrapper layer
and no internal helper to reach beneath it. Every row below therefore drives
`wcscat` directly through the `.so` export, never through a Rust-side helper.

## Configuration-surface table

Every row is exercised against **both** `.so`s with many randomized inputs
(fixed seed `0x5EED_1234_ABCD_9876`, xorshift64\*), comparing the return code
**and** the entire physical destination allocation including an 8-element
sentinel guard region past `numElem`, so an out-of-bounds write divergence is
caught too.

| #  | entry point(s) | configuration (options set + input shape) | [ ] |
|----|----------------|-------------------------------------------|-----|
| 1  | `wcscat` | `numElem = 1`, `dst[0] == 0` (empty), `src` empty (`L = 0`) → exact fit, expect `0` | [x] |
| 2  | `wcscat` | `numElem = 2`, `dst` empty, `src` empty → room to spare, expect `0` | [x] |
| 3  | `wcscat` | `numElem = 2`, `dst` empty, `L = 1` → exact fit (NUL at last element), expect `0` | [x] |
| 4  | `wcscat` | `numElem = 2`, `dst` NUL at index 1 (last element), `src` empty → exact fit, expect `0` | [x] |
| 5  | `wcscat` | `numElem = N` (random 3..64), `dst` empty, `L` random with `L + 1 < N` → room to spare, expect `0` | [x] |
| 6  | `wcscat` | `numElem = N`, `dst` empty, `L + 1 == N` → exact fit, NUL in last element, expect `0` | [x] |
| 7  | `wcscat` | `numElem = N`, `dst` NUL at random `k` with `0 < k < N-1`, `k + L + 1 < N` → room to spare, expect `0` | [x] |
| 8  | `wcscat` | `numElem = N`, `dst` NUL at random `k`, `k + L + 1 == N` → exact fit, expect `0` | [x] |
| 9  | `wcscat` | `numElem = N`, `dst` NUL at `k == N-1` (last element), `src` empty → exact fit, expect `0` | [x] |
| 10 | `wcscat` | `numElem = N`, `dst` NUL at `k == N-1`, `L >= 1` → one short, expect `34` + `dst[0] = 0` | [x] |
| 11 | `wcscat` | `numElem = N`, `dst` empty, `L + 1 == N + 1` (chars fit, NUL does not) → expect `34`, partial copy retained in `dst[1..]` | [x] |
| 12 | `wcscat` | `numElem = N`, `dst` NUL at random `k`, `k + L + 1 == N + 1` → expect `34`, partial copy retained | [x] |
| 13 | `wcscat` | `numElem = N`, `dst` NUL at random `k`, `L` grossly oversized (`L = N * 4`) → expect `34` | [x] |
| 14 | `wcscat` | `numElem = N`, `dst` **unterminated** (no `0` in `[0, N)`) → seek exhausts, copy loop skipped, expect `34` + `dst[0] = 0`, `src` unread | [x] |
| 15 | `wcscat` | `numElem = N` smaller than the physical allocation, `dst`'s only NUL sits at an index `>= N` → same as row 14, and **no write past `numElem`** | [x] |
| 16 | `wcscat` | `numElem` oversized (`1 << 20`) w.r.t. the real content, `dst` terminated early, short `src` → expect `0`, writes confined to the real prefix | [x] |
| 17 | `wcscat` | `dst` prefix + `src` drawn from the **ASCII** value class, random `N`/`k`/`L` | [x] |
| 18 | `wcscat` | value class **non-BMP** (`0x10000..=0x10FFFF`) | [x] |
| 19 | `wcscat` | value class **surrogates** (`0xD800..=0xDFFF`) | [x] |
| 20 | `wcscat` | value class **above `0x10FFFF`** (including `i32::MAX`) | [x] |
| 21 | `wcscat` | value class **negative** (including `-1` and `i32::MIN`) — `wchar_t` is signed `int` on Linux | [x] |
| 22 | `wcscat` | value class **mixed / fully random `i32`** bit patterns (never `0` except as the terminator) | [x] |
| 23 | `wcscat` | `src == dst` (self-append) with `dst` empty → expect `0` | [x] |
| 24 | `wcscat` | `src == dst + j` pointing into the interior of `dst`, `dst` non-empty (overlap, no check in C) | [x] |
| 25 | `wcscat` | **repeated calls** (A9): 2..8 successive appends into one buffer until saturation, comparing return code and full buffer after **every** call | [x] |
| 26 | `wcscat` | exhaustive small-sweep: every `(N, k, L)` with `N ∈ 1..=6`, `k ∈ 0..=N`, `L ∈ 0..=8`, unterminated variant included | [x] |
| 27 | `wcscat` | `numElem = usize` boundary-ish large value with early-terminated `dst` inside a padded allocation (no upper-bound check in C) | [x] |
| 28 | `wcscat` | fully randomized fuzz: random `N`, random `dst` fill (possibly unterminated), random `L`, random value class, 20 000 iterations | [x] |

## Feature combinations

`translation/Cargo.toml` has no `[features]` table, so the complete set of
feature combinations is the single default build. `ci/check_features.sh`
enumerates it from `Cargo.toml` mechanically and runs the whole suite for each
(`default`, `--no-default-features`, `--all-features`) rather than assuming.
