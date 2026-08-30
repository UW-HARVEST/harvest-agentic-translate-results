# VERIFICATION.md — results

Differential verification of `translation/` (Rust) against `c_src/` (C, ground
truth) for the `liblong` shared library.

Reproduce everything with:

```sh
./verify.sh          # fast suite + soak            (~3 min)
./verify.sh --full   # additionally the 4 full long_exec runs (~11 min)
```

## Scope of the library

| item | value |
|---|---|
| C translation units | 1 (`c_src/src/long.c`), 1 header (`c_src/include/long.h`) |
| exported symbols | 3 (`array`, `long_exec`, `perform_expensive_operations`) |
| runtime options / modes / flags | none |
| Cargo features | none — the default is the *only* combination |
| explicit error returns in the C | none (both functions are `void`) |

## How the libraries are driven

Both `.so` files are loaded with `libloading` and driven **only** through their
exported symbols — the Rust code is never called directly, so the
`#[no_mangle] extern "C"` wrappers and the exported `array` object are part of
what is under test. Two Rust profiles are compared against C on every row:

* `target/debug/liblong.so` — `overflow-checks = true`, so any arithmetic the
  translation did not spell out as `wrapping_*` would abort instead of wrapping
  like the C does;
* `target/release/liblong.so` — `opt-level = 3`, `lto`, the vectorised path.

`tests/symbols.rs::no_global_symbol_interposition_between_the_two_libraries`
proves the two libraries cannot bind to each other's identically-named symbols
(both define `array`, `long_exec` *and* `perform_expensive_operations`), which
would otherwise have silently corrupted every reference value.

## Phase A — surface artifacts

| artifact | content |
|---|---|
| `SYMBOLS.md` | 3 C symbols, all exported by Rust with matching `nm` type and matching `array` `st_size` (`0x100000`). Symbol diff `C \ Rust` = **empty**. |
| `ERRORS.md` | 19 rows. The C has **zero** explicit rejections; the rows enumerate every implicit/UB/boundary condition plus the generic FFI boundaries, and the contract is that Rust must not *invent* a rejection. |
| `CONFIGS.md` | 19 rows over the real axes: 3 entry points (incl. the low-level `array` symbol and the non-header `perform_expensive_operations`), value classes, 8-lane batching offsets, composition depth, seeds. |

## Phase B — valid-path results

`tests/valid_paths.rs` — **17 tests, all passing**, one per `CONFIGS.md` row
1–16 and 19, each comparing the full 1 MiB `array` byte-for-byte after every
call, against both Rust profiles.

`tests/valid_paths.rs::soak_randomised_batches` — **120 randomised batches x
262144 independent starting values = 31,457,280 distinct inputs**, cycling six
distribution families (full-range, non-negative, negative, edge-table,
small-magnitude, mixed), fixed SplitMix64 seed. **All matched.**

## Phase C — error-path results

`tests/error_paths.rs` — **19 tests, all passing**, one per `ERRORS.md` row,
including out-of-range "enum-like" integers across the FFI boundary, a negative
`int` passed for the `unsigned int` seed, null/surplus arguments through
deliberately mismatched prototypes, `INT_MIN`/`INT_MAX` overflow inputs, and a
canary check that nothing is written one past `array`'s valid range.

## Phase D — symbol parity and feature combinations

`tests/symbols.rs` — **6 tests, all passing**. Symbol lists are re-derived with
`nm -D` at test time, so the parity claim cannot go stale.

Feature matrix — `Cargo.toml` declares no `[features]`, so the default,
`--no-default-features` and `--all-features` builds are the complete matrix and
are identical. All three were run; all pass (`verify.sh` step 5 enumerates
them, and `features_surface_is_empty` fails the suite if a `[features]` table is
ever added without extending the matrix).

## Full `long_exec` end-to-end differential

`long_exec` performs `262144 x 100 x 2000 = 5.24e10` `step()` evaluations per
call: ~470 s through the C `.so`, ~56 s through the optimised Rust `.so`. It
cannot be short-circuited, because the orbit never converges — after 40
`perform_expensive_operations` calls essentially every element still changes on
the next call (measured, `CONFIGS.md` row 13). So it was simply run in full, as
one background process per (library, seed) pair:

| seed | C `.so` printed | Rust `.so` printed | verdict |
|------|-----------------|--------------------|---------|
| 0 | `42032659` | `42032659` | MATCH |
| 1 | `42032659` | `42032659` | MATCH |
| 2 | `15573690` | `15573690` | MATCH |
| 12345 | `241792833` | `241792833` | MATCH |

Seeds 0 and 1 agreeing with each other is correct, not a bug: glibc's
`srand(0)` aliases `srand(1)`, which `err01_seed_zero_is_not_rejected` asserts
independently. Seeds 2 and 12345 give distinct values, so the comparison is not
being satisfied by a constant.

## Divergences found

**None.** No changes to `translation/src/lib.rs` were required — `cargo check`
was clean on the first run and every differential row passed. The three initial
test failures were wrong assumptions in the *tests*, corrected against gcc as
ground truth:

| wrong assumption | measured C ground truth |
|---|---|
| `INT_MIN % 7 == -1` | `INT_MIN % 7 == -2` |
| `0` is a fixed point of the inner loop | `step(0) == -3`; one call maps `0` to `-626538949` |
| an all-zero `array` stays all-zero | it becomes uniformly `-626538949` |

Nothing in `c_src/` was modified.

## Completion gate

- [x] `SYMBOLS.md`: `nm -D` shows 0 missing/undefined non-libc symbols in Rust.
- [x] Phase B: every `CONFIGS.md` row passes across randomised inputs (plus a
      31.4M-input soak).
- [x] Phase C: every `ERRORS.md` row has a passing error-path differential test.
- [x] All of the above hold under every feature combination (the crate has
      exactly one) and under both Rust profiles (`debug` with overflow checks
      and `release` with LTO).
