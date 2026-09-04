# Verification report — C `driver` library → Rust `driver` crate

Ground truth: `c_src/` (never modified). Subject: `translation/` (`libdriver.so`).

Every comparison is made **through the FFI boundary**: the suite `dlopen`s both
`c_src/build/libdriver.so` and `translation/target/<profile>/libdriver.so` with
`libloading` and calls the exported `extern "C"` symbols. No Rust function is
ever called directly, so the `#[unsafe(no_mangle)]` export wrappers are under
test as well.

## How to reproduce

```bash
cd translation
./run_all.sh          # builds the C .so, then every feature combo x profile, then the symbol diff
./mutation_check.sh   # proves the suite is not vacuous (10 injected faults, all detected)
```

`cargo test` on its own is **not** sufficient: the crate's only `crate-type` is
`cdylib`, so `cargo test` does not re-emit `libdriver.so`. The harness now
detects and rejects a stale artifact (`assert_so_fresh`); `run_all.sh` always
runs `cargo build` first. The tests must also run single-threaded (fd 1 is
redirected process-wide); `.cargo/config.toml` sets `RUST_TEST_THREADS = "1"`
and the harness asserts it.

## Completion gate

- [x] **`SYMBOLS.md`** — `nm -D` diff (C minus Rust) is **empty** for both the
      debug and the release Rust `.so`. `ldd -r` reports **0** unresolved
      symbols; every import is a versioned glibc/libgcc symbol. `inner` is
      `static` in C and is exported by neither `.so`. No C module was missing,
      so no additional translation was needed.
- [x] **Phase B** — all **44** rows of `CONFIGS.md` pass, each across many
      seeded random inputs (SplitMix64, fixed seed `0x9E3779B97F4A7C15`), plus
      the full `Alias × Shape` and `Shape × Len` cross-products and an
      exhaustive 15×15×15 value grid at `len == 1`.
- [x] **Phase C** — all **16** rows of `ERRORS.md` have a passing differential
      test, including out-of-process signal comparison for the rows where the C
      traps, and the out-of-range-`int` boundary values.
- [x] **Every feature combination** — the crate declares no `[features]`, so
      `--all-features` and `--no-default-features` are the same build; both are
      run, under both the `debug` and the `release` profile (the latter with
      `panic = "abort"`), and additionally against an `-O2` build of the same C
      sources.

## Test inventory (70 tests, all passing)

| target | tests | scope |
|--------|-------|-------|
| `tests/smoke.rs` | 1 | harness self-check against hard-coded C reference values |
| `tests/phase_b_fma.rs` | 30 | `CONFIGS.md` C1–C27, C44 — the low-level `fma_array` entry point driven directly, in 9 pointer-aliasing configurations × 10 value shapes × 19 lengths |
| `tests/phase_b_driver.rs` | 17 | `CONFIGS.md` C28–C43 — the `driver` entry point, byte-exact fd-1 capture |
| `tests/phase_c_errors.rs` | 18 | `ERRORS.md` E1–E16 |
| `tests/phase_d_symbols.rs` | 4 | symbol parity, private-symbol non-leakage, dynamic-link closure |

## Divergences found and fixed in the Rust translation

1. **`driver(NULL, len > 0)` (ERRORS.md E12).** The translation used
   `std::ptr::copy_nonoverlapping`, which carries a debug-only null-pointer
   precondition check; the Rust `.so` therefore died with `SIGABRT` where the C
   died with `SIGSEGV`. Fixed by calling libc `memcpy` — literally the function
   `c_src/src/driver.c` calls — which also removed the `if n > 0` guard the C
   does not have. Now identical in every profile.

No other behavioural divergence was found. In particular the following were
checked and match exactly:

* wrapping two's-complement semantics of `mul1[i]*mul2[i] + add[i]` at every
  sign and overflow combination (signed overflow is UB in C; the built object
  wraps, and so does the Rust — confirmed at `-O0` **and** `-O2`);
* all nine aliasing configurations of `fma_array`'s four pointers, including the
  4-way alias `inner` actually uses, and partial forward/reverse buffer overlap;
* the exact `%d\n` byte stream, including `-2147483648` / `2147483647`;
* the loop bound: neither implementation reads or writes past `len` (sentinel
  tail check), and `driver` never mutates its `const int *` input.

## Known, deliberate divergences (undefined behaviour, out of contract)

`ERRORS.md` rows **E11** and **E13** are inputs on which the C invokes UB and
dies:

* **E11** `driver(data, len < 0)` — `int out[len]` with a negative size, then
  `len * sizeof(int)` converts the negative `int` to `size_t`, so `memcpy` gets
  ~2^64 bytes. Measured: `SIGSEGV`, no stdout, for `len` ∈ {−1, −2, −1000,
  −1000000, `INT_MIN`}. Rust clamps to 0 and returns.
* **E13** `driver(data, len)` with `len` large enough that the VLA overflows the
  stack. Measured with an 8 MiB stack: `len = 2_000_000` succeeds,
  `len = 2_100_000` faults. Rust uses `Vec` (heap) and succeeds.

These are not reproduced on purpose: a crash caused by UB is not a specified
result and is not stable across compiler, optimisation level or `ulimit -s`, so
it cannot be asserted equal. Both rows still have tests, which (a) assert the C
side really does trap — so the divergence is re-flagged if the C build ever
changes — and (b) assert the Rust side emits **no** stdout, i.e. the two never
produce *differing* bytes, since C produces none before dying.

Everything in the defined domain (`len >= 0`, valid pointers, VLA within the
stack limit) is asserted byte-identical.

## Suite is not vacuous

`./mutation_check.sh` injects 10 faults into `src/lib.rs` and confirms each is
detected: non-wrapping multiply, `mul1*mul1` instead of `mul1*mul2`, off-by-one
result, dropped addend, `i <= len` and `i < len-1` loop bounds, `%d` → `%u`,
dropped newline in the format string, a one-element-short `memcpy`, and removing
the negative-`len` clamp. **10 killed, 0 survived.**

This check also uncovered the stale-artifact trap described above: before
`assert_so_fresh` existed, all 10 mutants "passed" the entire suite because
`cargo test` had reused an old `.so`.
