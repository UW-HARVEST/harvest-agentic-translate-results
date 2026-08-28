# Verification report

Differential verification of the Rust translation in `translation/` against the
C ground truth in `c_src/`. Both are loaded as shared objects with `libloading`
and compared **only** through their exported C symbol — the Rust functions are
never called directly as Rust, so the `#[unsafe(no_mangle)] extern "C"` wrapper
and the C ABI are themselves under test.

Reproduce with:

```sh
cd translation
./verify.sh          # phases A-D                       (~40 s)
./verify.sh --full   # + 2^32 sweeps, mutation check,
                     #   C optimization matrix          (~4 min)
```

## Scope

The library is one function, and this was confirmed mechanically rather than
assumed: `c_src/` is 3 files / 30 lines total, `nm -D` on the C `.so` shows
exactly one exported symbol, and grepping for `#define`/`#ifdef` found no
macro-generated symbol families. Nothing was left untranslated, so no missing
C source had to be written.

```c
tflac_u32 max_size_frame(tflac_u32 blocksize, tflac_u32 channels, tflac_u32 bitdepth);
```

## Result

| gate | status |
|---|---|
| `cargo check` clean (no compile errors to fix) | PASS |
| `SYMBOLS.md`: `nm -D` diff C→Rust empty; 0 undefined non-libc symbols | PASS |
| Phase B: all **30** `CONFIGS.md` rows pass over randomized inputs | PASS |
| Phase C: all **23** `ERRORS.md` rows have a passing error-path test | PASS |
| Phase D: all feature combos × both profiles | PASS (6/6) |
| Row↔test gate (`tests/coverage_gate.rs`) | PASS |
| Mutation sensitivity: 28 injected bugs all detected; 3 equivalent mutants correctly survive | PASS |
| 5 × full 2^32 exhaustive sweeps (21.5 × 10⁹ comparisons) | PASS |
| C rebuilt at `-O0/-O1/-O2/-O3/-Os/-Ofast`, `c99/c11/gnu17`, `-fwrapv`, `-ftrapv`, `-march=native`, clang `-O0/-O2/-O3` | PASS (14/14) |

66 tests across 5 test binaries. **No divergence was found**, so no change to
`src/lib.rs` was required; the translation was already correct. Neither
`c_src/` nor `src/lib.rs` was modified (mutations were injected and reverted
under `mutation_check.sh`, which restores on every exit path).

## Two harness bugs found and fixed

The value of this exercise was almost entirely in *invalidating my own tests
twice*. Both bugs made tests pass while verifying nothing.

**1. `cargo test` does not build `cdylib` artifacts.** The first suite passed
57/57 — and kept passing after I deliberately broke the Rust (changed `18` to
`17`, deleted `#[no_mangle]`, changed the divisor). `cargo test` only builds the
rlib it links into test binaries; `target/<profile>/libmax_size_frame_lib.so`
was a leftover from an earlier `cargo build`, so every test was comparing C
against a **stale** `.so`. All 10 deliberate bugs went undetected.

Fixed in `tests/common/mod.rs`: the harness now runs `cargo build` for its own
profile and feature set before `dlopen`, and additionally asserts that both
`.so`s are newer than their sources (`STALE Rust .so` / `STALE C .so`).
`tests/coverage_gate.rs` asserts these guards stay in place.

**2. The mutation script mutated comments.** After the fix, 6 mutants still
"survived" — because `str.replace(old, new, 1)` hit the *comment* on line 72
(`// 18U + channels + (sum / 8)`) instead of the code on line 73. The reported
blind spots were fictitious. `mutation_check.sh` now splits each line at `//`
and mutates only the code part, verifies the file actually changed, and
distinguishes provably-equivalent mutants (which must survive) from real ones.

A third, smaller instance: the symbol-diff step in `verify.sh` wrote to `/tmp`,
which is read-only in this sandbox, and printed `symbol diff: EMPTY (0 missing)`
from two nonexistent files — a false pass. It now uses `mktemp -d` and aborts if
either symbol list is empty.

The general lesson: a differential test that has never been *seen to fail* is
not evidence of anything. Every gate here is backed by a demonstrated failure.

## What the C actually does (and what the Rust must reproduce)

```
result = 18 + channels + ((blocksize*A + 7) mod 2^32) / 8      [all u32, mod 2^32]
A = 0                                    if channels == 0
A = bitdepth + bitdepth + (bitdepth!=32) if channels == 2
A = bitdepth * channels                  otherwise
```

Behaviours deliberately preserved rather than "fixed":

- `channels == 0` returns a constant `18` (the `channels * (channels != 2)`
  factor annihilates the only live term).
- Invalid FLAC parameters are **accepted**, not rejected: `bitdepth = 0`,
  `bitdepth = 33`, `blocksize = 0`, `blocksize = 65536`, `channels = 9`.
- Unsigned wraparound everywhere, including `bitdepth = 0xFFFFFFFF` making
  `bitdepth + (bitdepth != 32)` wrap to `0` and silently kill the third term,
  and `18 + channels` wrapping past zero (`channels = 0xFFFFFFEE` → `0`).
- Truncating (not rounding) division by 8, and the `+ +7` double unary plus.
- The `bitdepth != 32` correction only ever affects the `channels == 2` branch.

## Files

| file | purpose |
|---|---|
| `SYMBOLS.md` | Phase A: `nm -D` surface, C vs Rust |
| `ERRORS.md` | Phase A: error-surface table (23 rows) + grep evidence that the C has zero explicit rejections |
| `CONFIGS.md` | Phase A: configuration-surface table (30 rows) |
| `tests/common/mod.rs` | loader, freshness guards, SplitMix64 PRNG, reference model |
| `tests/configs.rs` | Phase B: one test per `CONFIGS.md` row |
| `tests/errors.rs` | Phase C: one test per `ERRORS.md` row |
| `tests/symbols.rs` | Phase D: symbol parity |
| `tests/deep_sweeps.rs` | exhaustive/large sweeps (`--ignored` for the 2^32 ones) |
| `tests/coverage_gate.rs` | mechanical row↔test gate + anti-stale-`.so` guard |
| `verify.sh` | end-to-end driver |
| `run_all_features.sh` | feature-combo × profile matrix |
| `mutation_check.sh` | proves the suite detects divergence |
