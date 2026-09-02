# CONFIGS.md — Phase A: configuration-surface table

## Mechanical derivation of the axes

### Public entry points (complete set)

`c_src/include/driver.h` declares exactly one entity, and `nm -D` on the C
`.so` confirms exactly one exported symbol:

| entry point | signature | level |
|---|---|---|
| `driver` | `void driver(int x)` | this *is* the lowest level; there is no
convenience wrapper and nothing beneath it. The whole public API is one
function. |

There are no `_init`/`_create`/`_destroy` functions, no context/handle struct,
no setters and no one-shot wrapper over a streaming API — so "exercise the
low-level entry points, not just the wrappers" collapses to "call `driver`".

### Runtime options / modes / flags

```sh
grep -nE 'if *\(|switch|#if|#ifdef|extern|static|global|flag|mode|option' \
    c_src/src/driver.c c_src/include/driver.h
```

Non-comment hits: the `for` loop guard, and the `#ifndef DRIVER_H_` include
guard. Therefore:

* runtime options / flags / modes: **0**
* `#ifdef` compile-time configuration: **0** (only the header include guard)
* mutable global / `static` state: **0**
* byte-order, element-type or width selectors: **0**

The library has **exactly one axis: the value of the single `int` parameter
`x`**, plus one derived axis the code branches on implicitly — the state of the
internal `j` accumulator, which is `2*i` and is the only value that can leave
`int` range.

### Input shapes the code actually distinguishes

From `for (int i = 0, j = 0; i < x; i++, j += 2)` and
`printf("%d %d\n", i, j)`, the distinct shapes are:

1. **iteration count**: zero (`x <= 0`, see `ERRORS.md`), exactly one, few,
   many — the loop body count is the only structural variation;
2. **decimal width of `i`**: `printf("%d")` emits a different number of bytes
   at each power-of-ten boundary (1, 2, 3, … digits), so field width is a real
   output-shape axis;
3. **decimal width of `j = 2*i`**: crosses its power-of-ten boundaries at
   *different* `i` than `i` does (e.g. `i=5 → j=10`), so the two widths are
   independent axes and their combinations must be crossed;
4. **`j` overflow**: `j` exceeds `INT_MAX` once `i >= 2^30 = 1073741824`. This
   requires `x > 2^30`, i.e. >1e9 `printf` calls and ~21.9 GB of output. It is
   nonetheless **reachable in ~92 s per library**, so row C11 is executed for
   real (streamed and hashed rather than buffered) instead of being assumed;
5. **stdout stream state**: the C writes through the C runtime's `stdout`
   `FILE*`, so buffering / flush interleaving is observable and is part of the
   contract. Both `.so`s import the same `printf@GLIBC_2.2.5`, and the tests
   assert on the flushed byte stream.

## Configuration-surface table

Each row is a meaningful combination of the axes above. Every row is exercised
against **both** `.so` exports with many randomized inputs drawn from a
fixed-seed SplitMix64 PRNG (seed `0x5DEE_CE66_D0C0_FFEE`, reproducible), not a
single hand-picked value.

| # | entry point(s) | configuration (options set + input shape) | test | [x] |
|---|----------------|-------------------------------------------|------|-----|
| C1 | `driver` | `x == 1` — exactly one iteration; single-digit `i` and `j` (`0 0`) | `differential::cfg_c1_single` | [x] |
| C2 | `driver` | `x == 2` — smallest "many"; boundary where `j` first becomes non-zero | `differential::cfg_c2_two` | [x] |
| C3 | `driver` | every `x` in `1..=16` exhaustively — single-digit `i`, `j` crossing 1→2 digits at `i=5` | `differential::cfg_c3_small_exhaustive` | [x] |
| C4 | `driver` | `x` randomized in `1..=1000`, 200 draws — `i` 1–4 digits, `j` 1–4 digits, widths desynchronized | `differential::cfg_c4_random_small` | [x] |
| C5 | `driver` | `x` randomized in `1..=100_000`, 25 draws — `i` up to 5 digits, `j` up to 6 | `differential::cfg_c5_random_medium` | [x] |
| C6 | `driver` | `x` at every decimal-width boundary of `i`: `1, 9, 10, 11, 99, 100, 101, 999, 1000, 1001, 9999, 10000, 10001, 99999, 100000, 100001` | `differential::cfg_c6_i_width_boundaries` | [x] |
| C7 | `driver` | `x` at every decimal-width boundary of `j = 2*i`: `5, 6, 7, 50, 51, 52, 500, 501, 502, 5000, 5001, 50000, 50001` | `differential::cfg_c7_j_width_boundaries` | [x] |
| C8 | `driver` | large single call, `x == 1_000_000` — repeated stdout buffer refills in the shared `FILE*`; line count asserted | `differential::cfg_c8_large_one_million` | [x] |
| C9 | `driver` | 40 randomized sequences of 8 calls each, valid counts interleaved with rejecting ones — statelessness plus stream concatenation | `differential::cfg_c9_interleaved_sequence` | [x] |
| C10 | `driver` | `x == INT_MAX` — the maximum valid count (~46 GB if run to completion). Started in a forked child per library, compared over an identical 64 MiB prefix, then both killed at the same offset | `heavy::cfg_c10_int_max_prefix` (+ `cfg_c10_prefix_machinery_selfcheck`) | [x] |
| C11 | `driver` | `j` signed-overflow regime: `x == 2^30 + 4096`, so the last 4096 lines have `j < 0` starting at `j == -2147483648`. 21.9 GB streamed and FNV-hashed per library; byte count additionally asserted against a closed-form two's-complement-wrapping model *and* shown to differ from the non-wrapping model, proving the boundary was really crossed | `heavy::cfg_c11_j_signed_overflow` | [x] |
| C12 | `driver` | output to a pipe vs. a regular file — libc picks different default buffering. Fixed values, 40 randomized values, and a 40 000-iteration payload that overruns the 64 KiB pipe capacity so the writer blocks and resumes mid-stream | `differential::cfg_c12_pipe_and_file_buffering` | [x] |

### Measured result for C11

```
x = 1073745920 (2^30 + 4096)
C    : 21955747671 bytes, FNV-1a = 0x317129e0b55bbaca
Rust : 21955747671 bytes, FNV-1a = 0x317129e0b55bbaca   -> identical
wrapping model     = 21955747671  (matches)
non-wrapping model = 21955743575  (excluded)
```

The C's `j += 2` is signed overflow, which is UB in C; gcc's actual codegen
wraps two's-complement, and the Rust `wrapping_add(2)` reproduces exactly that.
The byte-count model confirms the wrap is observable in the output rather than
merely assumed.

## Gate

- [x] Every row passes across its randomized inputs.
- [x] No row is skipped or replaced by a static argument.

## Feature combinations

`translation/Cargo.toml` declares **no `[features]` section**, so the crate has
exactly one configuration: the default. `cargo metadata` confirms the feature
set is empty; `--no-default-features` and the default build are the same build.
The feature-combination sweep therefore has a single member, and it is the one
all tests above run under. Verified by script rather than assumption
(`scripts/check_features.sh`).
