# CONFIGS.md — Phase A: configuration-surface table (valid inputs)

## Build-time configuration surface

### Rust: `Cargo.toml`

```
$ grep -n "\[features\]" Cargo.toml   # -> no match
```

`Cargo.toml` has **no `[features]` section**, therefore:

| # | feature combination | cargo invocation | `cargo check` | tests |
|---|---------------------|------------------|---------------|-------|
| F1 | *(none — the only valid combination)* | `cargo check --no-default-features` | [x] clean | [x] |
| F1 | *(same, spelled without the flag)* | `cargo check` | [x] clean | [x] |

There is exactly **one** valid feature combination (the empty set). The powerset
of an empty feature set has one element, so Phase D's "repeat B–C for every
feature combination" is satisfied by the single run — but both spellings are
exercised by `run_all.sh` to prove `--no-default-features` compiles too.

### C: `c_src/CMakeLists.txt`

```
$ grep -nE "option|if\(|IF\(|add_definitions|target_compile_definitions|CMAKE_BUILD_TYPE" c_src/CMakeLists.txt
# -> no match
```

No `option()`, no `#ifdef`/`#if` anywhere in `driver.c`/`driver.h`, no
compile definitions, no build-type branches. One source file, one
configuration. Built as `cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON`
(the default, un-optimised `-O0` configuration, which is also what the
translation's two's-complement wrapping semantics are matched against).

## Runtime configuration surface

```
$ grep -nE "^[a-zA-Z_].*=|static [^v]|extern|global" c_src/src/driver.c
# -> nothing but `static void inner(...)`
```

The library has **no runtime options at all**: no global/static state, no
setter functions, no flags, no modes, no context struct, no byte-order or
format selector. `driver.h` exposes a single declaration. The *entire*
configuration surface is therefore the **shape and values of the input data**
plus **which entry point** is called.

## Public entry points (both are exercised directly, not only via wrappers)

| entry point | signature | level | exported by C `.so` |
|-------------|-----------|-------|---------------------|
| `fma_array` | `void fma_array(int *out, const int *mul1, const int *mul2, const int *add, int len)` | **lowest level** — the element kernel; not in the header but externally linkable | yes (`T`) |
| `driver`    | `void driver(const int *data, int len)` | convenience one-shot: VLA + `memcpy` + `inner` (`fma_array` on 4-way-aliased pointers + `printf` loop) | yes (`T`) |
| `inner`     | `static void inner(int *out, int len)` | internal; reachable only through `driver` | no (`static`) |

## Axes the C actually branches on / distinguishes

| axis | values the code distinguishes | where in the C |
|------|-------------------------------|----------------|
| A. entry point | `fma_array` (raw kernel) / `driver` (VLA + copy + print pipeline) | `driver.c:29`, `driver.c:42` |
| B. `len` count shape | `0` (loop skipped) / `1` (single iteration, no loop-carried effects) / `2` / `3` (odd, small) / `many` (8, 64) / `large` (1 024, 65 536 — crosses page boundaries and glibc `memcpy` size-class thresholds: `<16B`, `16–32B`, `32–64B`, `64–128B`, `>512B` non-temporal path) | loop guard `i < len` (`:30`, `:37`); `len * sizeof(int)` (`:44`) |
| C. pointer aliasing of `fma_array`'s 4 pointers | all 4 distinct / `out==mul1` / `out==mul2` / `out==add` / `mul1==mul2` (squaring, sources aliased but dest distinct) / `mul1==mul2==add` (dest distinct) / **`out==mul1==mul2==add`** (exactly the pattern `inner` uses) / `out` overlapping the sources at ±1, ±2 element offsets | `inner` (`:36`) passes `out` four times; the parameters are **not** `restrict`, so every aliasing pattern is a legal, distinguished input |
| D. element value shape | all-zero / all-one / all `-1` / all `INT_MAX` / all `INT_MIN` / small-magnitude (no overflow: `\|v\| <= 46 340` so `v*v` fits) / full-range random (overflow-heavy, exercises `imul` wraparound) / mixed-sign / values whose products straddle `0x7FFFFFFF` / digit-length spread (1..11 chars incl. `-`), which changes the `printf("%d\n")` byte count | `out[i] = mul1[i]*mul2[i] + add[i]` (`:31`), `printf("%d\n", out[i])` (`:38`) |
| E. `out` buffer pre-state (`fma_array` only) | pre-filled with a poison pattern, to prove which bytes the kernel writes and that nothing outside `[0, len)` is touched | no bounds check exists in `:30–:32` |
| F. `data` alignment / provenance (`driver` only) | naturally `int`-aligned / heap / a byte-misaligned view of a `u8` buffer (legal for `memcpy`) | `memcpy(out, data, len*sizeof(int))` (`:44`) is alignment-agnostic |
| G. stdout buffering state | fully-buffered stream redirected to a file; output compared byte-for-byte after `fflush` | `printf` (`:38`) is the only observable output |

## Configuration-surface table (pruned cross-product of A × B × C × D × E × F)

Every row is driven with **many randomized inputs** (deterministic SplitMix64
PRNG, fixed seed `0x5EED_1234_ABCD_0001` mixed with the row index — reproducible)
unless the row's value shape is itself a fixed constant pattern. Both the C
`.so` and the Rust `.so` are called through `libloading`, and both the returned
`out` buffer **and** the captured stdout bytes are compared byte-for-byte.

| #  | entry point(s) | configuration (options set + input shape) | test | [x] |
|----|----------------|--------------------------------------------|------|-----|
| 1  | `fma_array` | 4 distinct buffers; `len` ∈ {0,1,2,3,4,5,7,8,16,31,32,63,64,127,128,255,256,1024}; random full-range values; poisoned `out` | `cfg_01_fma_distinct_all_lens_random` | [x] |
| 2  | `fma_array` | 4 distinct buffers; `len` = 65 536 (large, crosses many pages); random full-range | `cfg_02_fma_distinct_large` | [x] |
| 3  | `fma_array` | 4 distinct buffers; `len` ∈ {1,2,3,8,64}; small-magnitude values (no signed overflow) | `cfg_03_fma_distinct_no_overflow` | [x] |
| 4  | `fma_array` | 4 distinct buffers; constant patterns: all-`0`, all-`1`, all-`-1`, all-`INT_MAX`, all-`INT_MIN`, alternating `INT_MIN`/`INT_MAX` | `cfg_04_fma_distinct_constant_patterns` | [x] |
| 5  | `fma_array` | `out == mul1` (dest aliases first multiplicand); `len` ∈ {1,2,3,8,64,1024}; random full-range | `cfg_05_fma_alias_out_mul1` | [x] |
| 6  | `fma_array` | `out == mul2` | `cfg_06_fma_alias_out_mul2` | [x] |
| 7  | `fma_array` | `out == add` | `cfg_07_fma_alias_out_add` | [x] |
| 8  | `fma_array` | `mul1 == mul2`, `out`/`add` distinct (squaring kernel) | `cfg_08_fma_alias_mul1_mul2` | [x] |
| 9  | `fma_array` | `mul1 == mul2 == add`, `out` distinct (`x*x+x` into a separate buffer) | `cfg_09_fma_alias_all_sources` | [x] |
| 10 | `fma_array` | **`out == mul1 == mul2 == add`** — the exact 4-way aliasing `inner` uses; `len` ∈ {0,1,2,3,8,64,1024}; random full-range | `cfg_10_fma_alias_four_way` | [x] |
| 11 | `fma_array` | `out` = `base`, sources = `base + 1` element (forward overlap; loop-carried, ascending order matters) | `cfg_11_fma_overlap_plus_one` | [x] |
| 12 | `fma_array` | `out` = `base + 1`, sources = `base` (backward overlap) | `cfg_12_fma_overlap_minus_one` | [x] |
| 13 | `fma_array` | `out` = `base`, `mul1` = `base+1`, `mul2` = `base+2`, `add` = `base+3` (staggered partial overlap of all four) | `cfg_13_fma_overlap_staggered` | [x] |
| 14 | `fma_array` | `out` written into the middle of a poisoned buffer with margins on both sides, `len` ∈ {1,7,64} — verifies the *exact* written extent (no over/under-write) | `cfg_14_fma_write_extent_exact` | [x] |
| 15 | `driver` | `len` ∈ {0,1,2,3,4,5,7,8,16,31,32,63,64,127,128,255,256,1000,1024}; random full-range values (overflow-heavy) | `cfg_15_driver_all_lens_random` | [x] |
| 16 | `driver` | `len` = 65 536 (large; big `memcpy` + 65 536 `printf` lines) | `cfg_16_driver_large` | [x] |
| 17 | `driver` | `len` ∈ {1,2,8,64}; small-magnitude values (no overflow) | `cfg_17_driver_no_overflow` | [x] |
| 18 | `driver` | constant patterns: all-`0`, all-`1`, all-`-1`, all-`2`, all-`INT_MAX`, all-`INT_MIN`, alternating `INT_MIN`/`INT_MAX` | `cfg_18_driver_constant_patterns` | [x] |
| 19 | `driver` | digit-length spread: values chosen so `printf("%d\n")` emits 1..11 characters per line, including `0`, `-1`, `-2147483648`, `2147483647` | `cfg_19_driver_digit_length_spread` | [x] |
| 20 | `driver` | values `v` with `v*v+v` exactly straddling the `int` boundary (`46 340`, `46 341`, `-46 340`, `-46 341`, `65 535`, `65 536`, `0x8000`, `0xFFFF`) | `cfg_20_driver_overflow_boundary_values` | [x] |
| 21 | `driver` | `data` is a **byte-misaligned** view (offset 1,2,3 into a `u8` buffer); `len` ∈ {1,3,17} | `cfg_21_driver_misaligned_source` | [x] |
| 22 | `driver` | `data` points at a heap allocation sized exactly `len*4` (no slack) — proves no read past the end for valid `len`; `len` ∈ {1,2,3,4,5,8,16,17,64,1000} | `cfg_22_driver_exact_sized_source` | [x] |
| 23 | `driver` | `memcpy` size-class sweep: `len*4` ∈ {4,8,12,16,20,32,33..36,64,65,127,128,129,256,512,513,1024,4096,4097} bytes | `cfg_23_driver_memcpy_size_classes` | [x] |
| 24 | `driver` then `fma_array` | equivalence composition: `driver(data,len)` stdout must equal the `printf`-formatted result of `fma_array(buf,buf,buf,buf,len)` on a copy of `data` — verified for **both** libraries and cross-compared, exercising the composed pipeline rather than each wrapper alone | `cfg_24_pipeline_equivalence` | [x] |
| 25 | `driver` + `fma_array` | repeated / interleaved invocation sequence (state-leak check): 200 randomized calls alternating between the two entry points on the *same* loaded library handle, comparing the accumulated stdout stream | `cfg_25_interleaved_call_sequence` | [x] |

## Where each row is tested

`tests/phase_b_configs.rs`, one `#[test]` per row, named `cfg_NN_...`. Every test
loads **both** shared libraries with `libloading` and calls only their exported
`driver` / `fma_array` symbols — never a Rust function directly — so the
`#[no_mangle]` export wrappers are part of what is measured. Each call is made
with fd 1 redirected to a temporary file, and the captured bytes plus the whole
destination buffer are compared.

Cross-checks that keep a *shared* bug from hiding behind "C == Rust":

* `driver`'s output is also compared against an independent Rust model
  (`model_driver_stdout`: `x*x + x`, two's-complement wrapping, `%d\n`);
* row 24 additionally requires `driver`'s stdout to equal the formatted result of
  driving the low-level `fma_array` with the same 4-way aliasing `inner` uses, for
  each library separately;
* row 14 checks the exact written extent, so an off-by-one write outside
  `[0, len)` cannot pass.

## Row-boundary note (valid vs. out-of-range)

Every row above stays **inside** the caller's buffers, which is what makes
byte-for-byte comparison meaningful. `len` values that read past the end of the
caller's buffer are *not* valid configurations: `ERRORS.md` rows 26–27 show, by
measurement against the C `.so`, that neither the resulting bytes nor whether the
process faults is decided by the input there.

## Gate

- [x] Every row above passes byte-for-byte across its randomized inputs, under
      every feature combination in the build-time table (F1), against **both**
      the dev-profile and the release-profile Rust `.so`.
