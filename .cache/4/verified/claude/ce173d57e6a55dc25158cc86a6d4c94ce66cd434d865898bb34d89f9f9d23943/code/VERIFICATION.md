# VERIFICATION.md — result of the C ↔ Rust differential verification

Library under test: `c_src/{include/lib.h, src/lib.c}` → `src/lib.rs`
(one public entry point: `float pow43(int x);`).

Everything below was produced by loading **both** shared libraries with
`libloading` and calling their exported `pow43` symbol — the Rust
implementation is never called directly, so the `#[unsafe(no_mangle)] extern
"C"` wrapper is part of what is verified.

Reproduce with:

```sh
./verify.sh all          # feature combos × profiles + symbols + C -O levels
cargo test               # the differential suite alone
```

## Phase A — surface map

| artifact | content |
|----------|---------|
| `SYMBOLS.md` | `nm -D` of both `.so`s. C exports exactly one symbol (`pow43`); Rust exports the same one. Diff empty, no unresolved non-libc imports. Nothing was left untranslated: `CMakeLists.txt` compiles only `src/lib.c`, and both of its items (the 145-entry `g_pow43` table and `pow43`) are present in `src/lib.rs`. |
| `ERRORS.md` | 16 rows. Mechanical grep proves the C has **no** rejection path at all (no assert/error code/errno/null/length check), so the rows are its two explicit range comparisons plus every input whose unchecked table index leaves `g_pow43[0..=144]`. |
| `CONFIGS.md` | 21 rows: the cross-product of the axes the C actually branches on (branch A/B/C, `sign ∈ {0,64}`, `frac <0/=0/>0`, `mult ∈ {16,256}`, table-index extremes, boundaries) plus ordering/repetition/threading/reload axes. |

Feature combinations (Phase A step 1): `Cargo.toml` declares no optional
features (`default = []`) and `c_src` has no `#ifdef`/CMake option, so there is
exactly **one** build configuration. `cargo check` was run for
`--no-default-features`, the default, and `--all-features` — all clean, no
compile errors, no warnings.

## Phase B — valid-path differential tests (`tests/differential.rs`, 21 tests)

* **All 21 `CONFIGS.md` rows pass**, each with randomized inputs from a
  fixed-seed SplitMix64 PRNG (2 000 samples per row, 20 000 for row 17,
  8×5 000 across threads for row 19).
* Row 16 is an **exhaustive sweep of the entire defined domain**: all
  8 240 inputs `x ∈ [-16, 8223]` (every input whose table index is inside
  `g_pow43`) agree **bit-for-bit** — compared via `f32::to_bits()`, so `-0.0`
  vs `+0.0` and NaN payloads would also be caught.
* Every one of the 145 table entries is read and compared (branch A covers
  indices 0…144 for `x = -16…128`).

## Phase C — error-path differential tests (`tests/error_paths.rs`, 15 tests)

* **All 16 `ERRORS.md` rows have a passing test.**
* Generic boundaries covered: `INT_MIN`, `INT_MIN+1`, `INT_MAX-1`, `INT_MAX`,
  the first/last defined inputs (`-16`, `8223`), one step past each end
  (`-17`, `8224`), `x = 0`, division-by-zero reachability, and 3 000 random
  full-range `i32` values.
* Null-pointer / zero-length / oversized-length / out-of-range-enum rows are
  *mechanically shown to be inapplicable*: `err13_api_has_no_pointer_or_length_args`
  parses `c_src/include/lib.h` and asserts the entire public API is
  `float pow43(int x);` — no pointer, no second parameter, no `enum`. Because
  the single parameter is a plain `int`, **every** 32-bit pattern is a legal
  input, and those are covered by the exhaustive in-domain sweep plus random
  full-range sampling.
* Inputs whose C behaviour is **undefined** (out-of-bounds table read for
  `x < -16` or `x > 8223`) are called **out of process**
  (`tests/common/child.rs`) so a fault is observed instead of killing the run:
  `INT_MAX`/`INT_MIN` fault with `SIGSEGV` in *both* images; `-17`/`8224`
  return image-dependent values in both. `err16` proves the divergence region
  is *exactly* the out-of-table region: scanning `x ∈ [-256, 8500]`, **no
  in-domain input diverges** and every diverging input has an index outside
  `0..=144`. See "Why rows 5, 7, 8, 9 and 16 cannot assert value equality" in
  `ERRORS.md`.

## Phase D — parity, feature combos, completion gate

```
=== feature combinations found (2) ===
  * cargo <cmd> --no-default-features
  * cargo <cmd> <default>
  (Cargo.toml declares no optional features: one build configuration)
=== cargo check, every feature combination ===        [OK] [OK]
=== cargo test <debug>, every feature combination === [OK] [OK]
=== cargo test --release, every combination ===       [OK] [OK]
=== symbol parity (nm -D) ===
  [ OK ]   exported symbol sets are identical: pow43
  [ OK ]   no unresolved (non-libc) symbols in the Rust .so
=== differential tests against the C library at several optimization levels ===
  [ OK ]   differential vs C default(-O0)   [ OK ]   differential vs C -O1
  [ OK ]   differential vs C -O2            [ OK ]   differential vs C -O3
  [ OK ]   differential vs C -Os
=== result ===  ALL CHECKS PASSED
```

Test totals: **39 tests, 39 passing** (21 valid-path + 15 error-path + 3 symbol
parity; `child_worker` is the intentionally-`#[ignore]`d out-of-process helper),
in both the `dev` and `release` profiles, and for every feature combination.

## Two harness pitfalls that were found and fixed

1. **Stale-artifact trap (would have invalidated the whole suite).** For a crate
   whose only library target is a `cdylib`, `cargo test` does *not* rebuild that
   `.so` — the integration tests do not link it. The first version of the
   harness therefore `dlopen`ed a `.so` left over from an earlier `cargo build`,
   and *passed with deliberately broken Rust code*. `tests/common/mod.rs` now
   builds the `cdylib` itself (into `target/test-cdylib`, so it cannot deadlock
   with the enclosing `cargo test`) and additionally asserts the artifact is
   newer than every file in `src/`.
2. **Sensitivity proof (mutation testing).** To show the suite can actually
   fail, 21 mutations were injected into `src/lib.rs` (rebuilt and re-run each
   time). **19 of 21 were caught**; the 2 survivors were verified to be
   *semantically equivalent*, not test gaps:
   * `81.000000f32 → 81.000001f32` — both decimals round to the same `f32`
     (`0x42A20000`); changing it to `81.00002` *is* caught.
   * `if x < 129 → if x < 128` — only `x = 128` changes path, and there
     `16 * g_pow43[32] == g_pow43[144] == 0x44214518` bit-for-bit, so no input
     can observe the difference.

   Caught mutations included: both polynomial coefficients (`4/3`, `2/9`), the
   `1.f +` sign, the `mult` values (16, 256), the `+16` table offset, the
   `& 64` sign mask, the `& ~63` denominator mask, `+ sign` in the denominator,
   the `x <<= 3` shift, the `>> 6` index shift, the `if (x < 1024)` bound, the
   numerator's `-sign`, and individual table constants (first, middle, last),
   including a `0.0 → -0.0` sign-of-zero change.

## Completion gate

* [x] `SYMBOLS.md`: `nm -D` shows 0 missing symbols in the Rust `.so` and 0
      unresolved non-libc symbols (`diff` empty in both directions).
* [x] Phase B: every one of the 21 `CONFIGS.md` rows passes across randomized
      inputs, plus an exhaustive sweep of all 8 240 defined inputs.
* [x] Phase C: every one of the 16 `ERRORS.md` rows has a passing error-path
      differential test.
* [x] All of the above hold under **every** feature combination
      (`--no-default-features`, default, `--all-features` — the crate has one
      configuration) and in both the `dev` and `release` profiles, and against
      the C library built at `-O0`, `-O1`, `-O2`, `-O3` and `-Os`.

## Conclusion

`src/lib.rs` is a faithful translation of `c_src/src/lib.c`: it produces
**bit-identical** `f32` results for every input in the domain where the C
program is defined (`x ∈ [-16, 8223]`, verified exhaustively and at five C
optimization levels), it exports the same ABI symbol, and no C behaviour was
"fixed", stubbed or invented. The only inputs on which the two libraries differ
are those on which the C itself performs an out-of-bounds read of `g_pow43`
(`x < -16` or `x > 8223`); there the C's own result is a property of its
compiled image rather than of its source, which is documented in `ERRORS.md`
rather than emulated.
