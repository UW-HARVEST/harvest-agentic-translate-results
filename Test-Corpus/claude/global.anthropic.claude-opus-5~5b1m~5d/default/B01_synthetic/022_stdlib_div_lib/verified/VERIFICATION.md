# Verification report

`c_src` (ground truth) vs `translation` (Rust). Every comparison is made
**through the FFI boundary**: both the C `.so` and the Rust `.so` are `dlopen`ed
with `libloading` and `driver` is resolved with `dlsym`. No Rust function is
ever called directly, so the `#[no_mangle]` export wrapper is itself under test.

## How to reproduce

```sh
cd translation && ./run_verification.sh
```

The script builds the C `.so`, enumerates the Cargo feature power set, and runs
all three test files for every feature combination × {release, debug}.

> **Note:** crates.io is unreachable from this sandbox, so cargo is invoked with
> `--offline`. `libloading 0.8.9` was already present in the local registry cache.

## Result: ALL CONFIGURATIONS PASSED

| phase | test file | tests | status |
|-------|-----------|-------|--------|
| A/D — symbol parity | `tests/symbol_parity.rs` | 3 | ok |
| B — valid paths (`CONFIGS.md` C1–C15) | `tests/valid_paths.rs` | 16 | ok |
| C — error paths (`ERRORS.md` E1–E5) | `tests/error_paths.rs` | 4 | ok |

Run for `profile=release` and `profile=debug`; the debug run loads
`target/debug/libdriver.so` and the release run `target/release/libdriver.so`
(the harness derives the profile from `current_exe()`, so a stale artifact
cannot be silently re-verified).

## Completion gate

- [x] **`SYMBOLS.md`: `nm -D` shows 0 missing / 0 undefined non-libc symbols.**
      The C `.so` exports exactly one symbol, `driver`; the Rust `.so` exports it
      under the identical name. `comm -23` of the two symbol lists is empty, and
      every undefined symbol in the Rust `.so` resolves to libc/libgcc. Enforced
      as a test (`rust_so_exports_every_c_symbol`,
      `rust_so_has_no_unresolved_project_symbols`) so it cannot regress.
- [x] **Phase B: every `CONFIGS.md` row passes across randomized inputs.**
      All 15 rows. Inputs come from a fixed-seed SplitMix64 PRNG
      (`0x243F6A8885A308D3`), 400 pairs per shaped row plus deterministic
      boundary anchors, plus a 20 000-pair sweep over the entire `i32` domain
      (row C14) and the full 9×9 boundary cross-product (row C13). Roughly
      24 000 distinct argument pairs, each compared byte-for-byte.
- [x] **Binary stdout diff: N/A.** `c_src/CMakeLists.txt` has no
      `add_executable` — the C project builds only a shared library, and
      `Cargo.toml` declares only `crate-type = ["cdylib"]` with no `[[bin]]`.
      Pinned by the `c_project_builds_no_executable` test so this stays true.
- [x] **Phase C: every `ERRORS.md` row has a passing error-path test.**
      All 5 rows. Each faulting input runs in a forked child so the fault is
      observable, and the test asserts the C and Rust children die from the
      **same specific signal (8 / SIGFPE)** with identical (empty) stdout — not
      merely that "both failed". Divide-by-zero is additionally generalised
      across 35 numerators. Generic boundaries are covered by
      `generic_ffi_boundaries`; null-pointer / length / enum boundaries are
      structurally N/A for the `void driver(int, int)` ABI and that reasoning is
      recorded in `ERRORS.md` rather than left implicit.
- [x] **All of the above hold under every feature combination.** `Cargo.toml`
      declares no `[features]` table and `src/` contains no
      `#[cfg(feature = …)]`, so the feature power set is the single empty
      combination; it is still run explicitly via `--no-default-features`, and
      the whole matrix is additionally run under both the release and debug
      profiles (debug enables overflow checks, a genuinely different codegen
      configuration for arithmetic).

## Why the translation is correct

`driver` is branch-free and delegates to two libc routines. The Rust translation
calls **the same two libc symbols** (`div@GLIBC_2.2.5`, `printf@GLIBC_2.2.5`)
rather than reimplementing them, which is what makes the degenerate inputs trap
identically instead of being silently "fixed":

* `y == 0` and `INT_MIN / -1` are **undefined behaviour in C** that x86-64
  realises as `SIGFPE`. A natural Rust translation using `/`, `checked_div`, or
  `wrapping_div` would panic, return `None`, or return `INT_MIN` — all
  divergences. Delegating to libc `div` reproduces the fault exactly.
* Using libc `printf` (not Rust's `println!`) keeps the `%d` formatting and the
  stdout buffering byte-identical, including the 11-character `-2147483648`.

## Negative controls (harness validated, not just green)

To prove the suite can actually detect divergence, two mutants were injected
into the Rust `driver`, tested, and then reverted:

| mutant | effect | caught by |
|--------|--------|-----------|
| replace `div` with `wrapping_div`/`wrapping_rem` guarded on `y == 0` | valid outputs unchanged, but faults are swallowed | Phase C: 3 of 4 tests FAILED — reported `driver(7, 0)`: `C = Some(8), Rust = None`, and `driver(INT_MIN, -1)` exiting 0 after printing `quotient: -2147483648, remainder: 0` |
| force a Euclidean (non-negative) remainder | remainder sign wrong for negative dividends | Phase B: 9 of 16 tests FAILED — rows C3, C5, C7, C10, C12, C13, C14, C15 all reported concrete diverging pairs |

Both mutants were removed and the pristine translation re-verified; `src/lib.rs`
contains no mutant residue.

## Files

| file | role |
|------|------|
| `SYMBOLS.md` | Phase A public-symbol map and parity proof |
| `ERRORS.md` | Phase A error-surface table (E1–E5) + generic-boundary reasoning |
| `CONFIGS.md` | Phase A configuration-surface table (C1–C15) |
| `tests/common/mod.rs` | dlopen/dlsym harness, fd-1 stdout capture, fork-and-report-signal runner, seeded PRNG |
| `tests/valid_paths.rs` | Phase B |
| `tests/error_paths.rs` | Phase C |
| `tests/symbol_parity.rs` | Phase D |
| `run_verification.sh` | full matrix driver |

Nothing in `c_src/` was modified.
