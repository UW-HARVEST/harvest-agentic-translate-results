# CONFIGS.md — Phase A configuration-surface table

## Mechanical derivation of the axes

Public API, from `c_src/include/driver.h` plus every external-linkage symbol in
`c_src/src/driver.c`:

- `void fma_array(int *out, const int *mul1, const int *mul2, const int *add, int len)`
  — the **lowest-level** entry point (declared only in the `.c`, but exported;
  it is in `nm -D`, so a real consumer can and does call it directly).
- `void driver(const int *data, int len)` — the convenience / one-shot wrapper.
  Internally: VLA + `memcpy` + `static inner()` → `fma_array` with **full
  self-aliasing** → `printf("%d\n")` loop.

### Runtime options / modes / flags

**None.** Grepping the C for `if`/`switch`/`#ifdef` on any flag yields only the
header's include guard (see `ERRORS.md`). There is no config struct, no mode
enum, no global state, no `setopt`-style function. The only `#include`s are
`stdio.h` and `string.h`.

### Compile-time configurations

**None.** `c_src/CMakeLists.txt` adds no `target_compile_definitions` and no
options; `translation/Cargo.toml` declares **no `[features]` table**. Therefore
there is exactly one feature combination to verify: the default (empty) one.
`--no-default-features` is equivalent to the default here. This is asserted
mechanically by `tests/feature_combos.rs` / `check_all_features.sh`.

### Input shapes the code actually distinguishes

Derived from the branches the C actually takes:

- `len` vs the loop guard `i < len` (`driver.c:30`, `:37`): the code
  distinguishes *nothing* but `len <= 0` (no iterations) from `len >= 1`; but
  the *data-dependent* paths make element count and element values matter, so:
  empty (0), single (1), pair (2), many (small/medium/large).
- element values feeding `mul1[i] * mul2[i] + add[i]` (`driver.c:31`): in-range
  vs multiply-overflow vs add-overflow vs sign combinations vs zeros/ones vs
  `INT_MIN`/`INT_MAX` boundaries.
- **aliasing of the four pointers** — this is the axis the C itself exercises:
  `inner` calls `fma_array(out, out, out, out, len)` (`driver.c:36`), i.e. total
  aliasing, despite the `const` qualifiers. A direct `fma_array` consumer may
  pass any aliasing pattern, and the forward element-local loop makes each
  pattern produce a *different* defined result. Patterns: all-distinct,
  `out==mul1`, `out==mul2`, `out==add`, `mul1==mul2` (square), all-same,
  overlapping-with-offset.
- `driver`'s stdout shape: number of `printf("%d\n")` lines and the exact digit
  formatting of negative / zero / `INT_MIN` values.
- `driver`'s `memcpy` length `len * sizeof(int)` (`driver.c:44`) vs the VLA size
  `len` (`:43`) — exercised across the same `len` shapes.

## Configuration-surface table

Every row is run against **many randomized inputs** (fixed seed
`0x5EED_C0DE_1234_5678`, SplitMix64 PRNG) unless the row is a fixed boundary
vector. All rows compare the C `.so` against the Rust `.so`, both loaded via
`libloading`, byte-for-byte (`out` buffer bytes and, for `driver`, captured
stdout bytes).

| # | entry point(s) | configuration (options set + input shape) | [x] |
|---|----------------|--------------------------------------------|-----|
| C1 | `fma_array` | no options; `len = 0`, four distinct buffers pre-filled with a random sentinel pattern (asserts `out` untouched) | [x] |
| C2 | `fma_array` | no options; `len = 1`, four distinct buffers, randomized `i32` over the full range | [x] |
| C3 | `fma_array` | no options; `len = 2`, four distinct buffers, randomized full-range `i32` | [x] |
| C4 | `fma_array` | no options; `len` random in `3..=64`, four distinct buffers, randomized full-range `i32` (overflow reached naturally) | [x] |
| C5 | `fma_array` | no options; `len` random in `65..=1024` (medium), four distinct buffers, randomized full-range `i32` | [x] |
| C6 | `fma_array` | no options; `len = 4096` (large), four distinct buffers, randomized full-range `i32` | [x] |
| C7 | `fma_array` | no options; `len` random; values drawn from the **small** set `{-2,-1,0,1,2}` (no overflow, exercises sign handling) | [x] |
| C8 | `fma_array` | no options; `len` random; values drawn from the **boundary** set `{INT_MIN, INT_MIN+1, -1, 0, 1, INT_MAX-1, INT_MAX}` (dense overflow) | [x] |
| C9 | `fma_array` | no options; all four buffers all-zero | [x] |
| C10 | `fma_array` | no options; all four buffers all-one (`out[i] = 2`) | [x] |
| C11 | `fma_array` | **aliasing** `out == mul1`, `mul2`/`add` distinct; random `len`, randomized full-range `i32` | [x] |
| C12 | `fma_array` | **aliasing** `out == mul2`, `mul1`/`add` distinct; random `len`, randomized full-range `i32` | [x] |
| C13 | `fma_array` | **aliasing** `out == add`, `mul1`/`mul2` distinct; random `len`, randomized full-range `i32` | [x] |
| C14 | `fma_array` | **aliasing** `mul1 == mul2` (square), `out`/`add` distinct; random `len`, randomized full-range `i32` | [x] |
| C15 | `fma_array` | **aliasing** `out == mul1 == mul2 == add` (exactly what `inner` does); random `len`, randomized full-range `i32` | [x] |
| C16 | `fma_array` | **aliasing** `out == mul1 == mul2 == add`, boundary value set (self-square overflow) | [x] |
| C17 | `fma_array` | **overlapping with offset**: `mul1 = out + 1` within one shared buffer, forward-loop read-after-write dependence; random `len`, random values | [x] |
| C18 | `fma_array` | **overlapping with offset**: `out = buf + 1`, `mul1 = buf`, write-after-read dependence; random `len`, random values | [x] |
| C19 | `fma_array` | no options; `len` = each of `0,1,2,3,4,5,7,8,9,15,16,17,31,32,33,63,64,65` (SIMD/unroll-width sweep), randomized values per width | [x] |
| C20 | `driver` | no options; `len = 0` (empty stdout) | [x] |
| C21 | `driver` | no options; `len = 1`, randomized full-range `i32` | [x] |
| C22 | `driver` | no options; `len = 2`, randomized full-range `i32` | [x] |
| C23 | `driver` | no options; `len` random in `3..=64`, randomized full-range `i32` | [x] |
| C24 | `driver` | no options; `len` random in `65..=1024`, randomized full-range `i32` | [x] |
| C25 | `driver` | no options; `len = 4096`, randomized full-range `i32` (large stdout volume) | [x] |
| C26 | `driver` | no options; values from the **small** set `{-2,-1,0,1,2}`; checks `-1 → 0`, `2 → 6` formatting | [x] |
| C27 | `driver` | no options; values from the **boundary** set incl. `INT_MIN`/`INT_MAX` (asserts `INT_MIN` prints as `-2147483648`) | [x] |
| C28 | `driver` | no options; all-zero data (prints `0` `len` times) | [x] |
| C29 | `driver` | no options; `len` width sweep `0,1,2,3,4,5,7,8,9,15,16,17,31,32,33,63,64,65` | [x] |
| C30 | `driver` | no options; `data` points into the **middle** of a larger buffer (non-zero offset, exercises that only `len` elements are copied and that the source is not modified) | [x] |
| C31 | `driver` + `fma_array` | **cross-check of the composed pipeline**: `driver(data, len)` stdout must equal the stdout implied by calling the low-level `fma_array(buf, buf, buf, buf, len)` on a copy of `data` — verifies `inner`'s composition, not just each wrapper | [x] |
| C32 | `driver` | **repeated invocation / statelessness**: the same `.so` handle called many times in sequence with different `len`/data; asserts no residual state between calls and identical stdout each time | [x] |
| C33 | `fma_array` | **repeated invocation / statelessness**: many sequential calls on the same handle, interleaved lengths and aliasing patterns | [x] |
| C34 | `driver` | `data` is a valid non-null pointer with `len = 0` while the buffer is non-empty (boundary between C20 and C21) | [x] |

## Feature combinations

`translation/Cargo.toml` has no `[features]`, so the complete set of
combinations is `{ default }` ≡ `{ --no-default-features }`. Both are run by
`check_all_features.sh`; there is no additional code path to cover.
