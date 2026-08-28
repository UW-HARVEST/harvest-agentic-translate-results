# VERIFICATION.md — completion gate

Library under verification: `c_src/{include/lib.h,src/lib.c}` → `translation/src/lib.rs`.
Single public entry point: `int wcscat(wchar_t *dst, size_t numElem, const wchar_t *src)`.

Everything below is measured, not asserted from memory. Reproduce with:

```
cd c_src && mkdir -p build && cd build && cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON && cmake --build .
cd translation
cargo check
cargo test --release          # 61 differential tests
./run_all_configs.sh          # every feature combo x cdylib profile
./mutation_check.sh           # proves the suite is actually sensitive
```

## How the tests call the code

Both libraries are `dlopen`ed with `libloading` and driven **only** through
`dlsym(handle, "wcscat")` — no Rust function is ever called directly, so the
`#[no_mangle] extern "C"` wrapper is itself under test. `wcscat` collides with
glibc's 2-argument `wcscat`; `RTLD_LOCAL` + per-handle `dlsym` was confirmed with
`dladdr` to resolve the *library's* definition, and
`smoke_resolved_symbol_is_the_library_not_glibc` guards against regression.

## Gate

- [x] **`SYMBOLS.md`: `nm -D` shows 0 missing/undefined non-libc symbols in Rust.**
      The C `.so` exports exactly one global, `wcscat`; the Rust `.so` exports it
      with the identical name. The symbol diff is empty. Enforced continuously by
      `phase_a_every_c_symbol_is_exported_by_rust` (which shells out to `nm -D`)
      and by the per-artifact `comm -23` check inside `run_all_configs.sh`.
      Nothing was stubbed: `c_src` is a single translation unit with a single
      function definition, all of which is translated.

- [x] **Phase B: EVERY row in `CONFIGS.md` passes across randomized inputs.**
      28 rows → 28 tests (`cfg01_*` … `cfg28_*`, `tests/phase_b_valid.rs`), driven
      from a fixed-seed xorshift64\* PRNG (`SEED = 0x5EED_C0DE_1234_5678`).
      Row 26 alone runs 200 000 randomized cases; row 27 runs 20 000; the
      per-row tests add ~25 000 more. Every assertion compares the return code,
      the **entire** `dst` allocation (window **and** guard tail) and the `src`
      allocation, and cross-checks against an independent re-derivation of the C
      semantics. Observed return-code histogram over row 26:
      `0 → 29 161`, `22 → 28 194`, `34 → 142 645` (all three paths exercised).

- [x] **Phase C: EVERY row in `ERRORS.md` has a passing error-path differential test.**
      17 table rows + 7 generic-boundary rows → 22 tests (`err01_*` … `err17_*`,
      `boundary_*`, `extreme_wchar_values`, `oversized_numelem_no_overflow`,
      `return_code_domain_is_closed`, `tests/phase_c_errors.rs`). Each asserts the
      **same specific** code (22 / 34), never merely "both failed". Covered:
      null `dst`, null `src`, both null, `numElem == 0`, short-circuit precedence
      between the three checks, `numElem == 1`, one-slot-left, exact off-by-one at
      the fit boundary, unterminated `dst`, terminator outside the window,
      `numElem == SIZE_MAX` and four other `dst + numElem` pointer-overflow
      witnesses, and out-of-Unicode / negative `wchar_t` payloads. The API has no
      enum parameters, so the equivalent out-of-domain-scalar class (`size_t` and
      `wchar_t` values with no valid meaning) is covered instead — see `ERRORS.md`
      row G7.

- [x] **All of the above hold under EVERY feature combination.**
      `Cargo.toml` declares no `[features]`, so the combination space is a single
      point. `run_all_configs.sh` derives the list from `Cargo.toml`
      mechanically (so a future feature is picked up automatically) and still runs
      `--no-default-features` and default explicitly, against **both** a debug and
      a release `cdylib`, checking symbol parity for each artifact:

      ```
      check  OK   --no-default-features
      check  OK   (default)
      tests  OK   --no-default-features / cdylib=debug    (61 tests passed)
      tests  OK   --no-default-features / cdylib=release  (61 tests passed)
      tests  OK   (default) / cdylib=debug                (61 tests passed)
      tests  OK   (default) / cdylib=release              (61 tests passed)
      === ALL CONFIGURATIONS PASSED ===
      ```

## Extra assurance beyond the four gate items

**Hardware bounds equivalence** (`tests/phase_d_bounds.rs`, 6 tests). Value
comparison cannot see an out-of-bounds *read*. These tests `mmap`
`PROT_NONE | RW | PROT_NONE` page triples and place `dst` so that `dst + numElem`
is exactly the first unmapped byte (and `src` so its terminator is the last mapped
element). Any access the Rust makes that the C does not kills the process. Both
implementations pass for window sizes 1 … 1024, for the src-terminator boundary,
for `src == NULL`, and for the pointer-overflow `numElem` values.

**Mutation testing** (`./mutation_check.sh`). 16 plausible mistranslations are
injected into `src/lib.rs` one at a time; the suite must catch each. Result:
**16 / 16 caught, 0 escaped.** Including:

| mutation | caught by |
|----------|-----------|
| `22` → `21` on the `!dst`/`numElem==0` branch | `err01`/`err02`/… |
| `22` → `21` on the `!src` branch | `err06`/`err07`/`err08` |
| `34` → `33` on truncation | 20 tests |
| `numElem == 0` also zeroes `dst[0]` | `err02_numelem_zero` |
| `!src` checked before `numElem == 0` | `err05`, `cfg25` |
| copy-loop bound `<` → `<=` | 20 tests |
| **scan-loop bound `<` → `<=` (out-of-bounds read only, identical values)** | **guard-page SIGSEGV in `phase_d_bounds`** |
| NUL-terminate at `dst[numElem-1]` instead of `dst[0]` | 5 tests |
| drop the `dst[0] = 0` on truncation | 19 tests |
| saturating instead of wrapping `dst + numElem` | `err15`/`cfg19` |
| `end = dst + numElem - 1` | 20 tests |
| terminator consumed but not written | `cfg04`/`cfg05`/… |
| scan stops on negative `wchar_t` (signedness bug) | 21 tests |
| `wchar_t` as `i16` instead of `i32` | value comparison |
| `src` pointer never advanced | value comparison |
| scan loop removed entirely | value comparison |

The scan-loop row is the important one: it is invisible to output comparison and
is only caught because of the guard pages.

## Divergences found and fixed

**None.** `translation/src/lib.rs` was already a faithful, bug-for-bug transcription
of `c_src/src/lib.c` and required no changes:

* the three-way check order (`!dst || numElem == 0`, then `!src`) is preserved,
  including the fact that the `numElem == 0` path does **not** zero `dst[0]`
  while the `!src` path does;
* `dst.wrapping_add(num_elem)` reproduces the C's `dst + numElem` pointer
  overflow for `numElem == SIZE_MAX` and friends (empirically: both return 34 and
  touch only `dst[0]`);
* truncation clobbers `dst[0]` rather than NUL-terminating at the end, matching
  the C exactly — deliberately *not* "fixed";
* `wchar_t` is `i32` on this target, matching `sizeof(wchar_t) == 4` and signed,
  as measured from the C build.

The only work required was fixing the test harness itself (`cargo test` does not
build a `cdylib` target, so the harness builds it into a separate
`CARGO_TARGET_DIR` and can also be pointed at any artifact via `WCSCAT_RUST_SO`).

## Files

| file | purpose |
|------|---------|
| `SYMBOLS.md` | Phase A symbol surface + ABI facts + feature inventory |
| `ERRORS.md` | Phase A error-surface table (17 rows + 7 generic boundaries) |
| `CONFIGS.md` | Phase A configuration-surface table (28 rows) |
| `tests/common/mod.rs` | harness: `.so` loading, case model, guard buffers, PRNG, sequencing |
| `tests/phase_a_symbols.rs` | symbol parity, `dlsym` provenance, ABI assumptions (5 tests) |
| `tests/phase_b_valid.rs` | Phase B, one test per `CONFIGS.md` row (28 tests) |
| `tests/phase_c_errors.rs` | Phase C, one test per `ERRORS.md` row (22 tests) |
| `tests/phase_d_bounds.rs` | guard-page bounds equivalence (6 tests) |
| `run_all_configs.sh` | Phase D feature-combo × profile matrix runner |
| `mutation_check.sh` | sensitivity proof for the whole suite |
