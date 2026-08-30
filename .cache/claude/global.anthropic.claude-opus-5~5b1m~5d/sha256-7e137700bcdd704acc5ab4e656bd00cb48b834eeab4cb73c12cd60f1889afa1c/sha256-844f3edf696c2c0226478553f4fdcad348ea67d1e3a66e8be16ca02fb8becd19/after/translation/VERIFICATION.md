# VERIFICATION.md — completion gate

Differential verification of `translation/` (Rust) against `c_src/` (C, the ground
truth). Both sides are exercised **only** through their shared objects, loaded
with `libloading`; no Rust function is ever called in-process, so the
`#[no_mangle] extern "C"` export wrappers and the ELF placement of the exported
globals are themselves under test.

## Completion checklist

- [x] **`SYMBOLS.md`** — `nm -D` shows **0 missing** symbols in the Rust `.so`.
      The C `.so` exports 8 global symbols; the Rust `cdylib` exports all 8 with
      the exact same names, for all 24 C configurations and in both the dev and
      release profiles. 0 unresolved non-libc imports (proved by an `RTLD_NOW`
      load). No module was missing, so nothing needed newly translating, and
      nothing is stubbed.
- [x] **Phase B** — every one of the 28 rows of `CONFIGS.md` passes across
      randomised inputs (fixed seed `0x5EED_C0FF_EE00_1234`).
- [x] **Phase C** — every one of the 26 rows of `ERRORS.md` has a passing
      error-path differential test asserting the *same* sentinel/exit code, not
      merely "both failed".
- [x] **All of the above under every feature combination.** All **2048** feature
      subsets `cargo check --all-targets` cleanly (0 warnings); the 41
      representative configurations run the full test suite in both the dev
      profile (Rust arithmetic overflow checks **on**) and the release profile.

## Reproducing

```sh
./build_c_so.sh                 # 24 C .so + 24 C driver executables from c_src/
./check_all_features.sh full    # cargo check x 2048 feature subsets
CARGO_TARGET_DIR="$PWD/translation/target-cfg" ./run_all_configs.sh
CARGO_EXTRA=--release CARGO_TARGET_DIR="$PWD/translation/target-cfg" \
  ./run_all_configs.sh
```

Latest run: **41/41 configurations, 34/34 tests each, both profiles.**

`run_all_configs.sh` builds *before* testing on purpose: `cargo test` does not
reliably re-emit the `cdylib` when only the feature set changes, so testing
without a preceding build can compare a stale `.so`. `tests/common/mod.rs` also
verifies at load time that the `.so` on disk was built for the active feature set
(via `G_OP_NAME` and `helper_call(0, 0)`), turning that hazard into an explicit
build error rather than a misleading "divergence".

## Divergences found and fixed

Two real fidelity bugs were found in the Rust translation, both invisible to
return-value-only happy-path testing:

1. **`G_OP` / `G_OP_NAME` were not writable.** In C both are mutable objects
   (for `G_OP_NAME` only the *pointee* is `const`), so they live in the writable
   `.data` section and a `dlopen`ing consumer may legally store through the
   `dlsym` address. The Rust versions were immutable `static`s holding relocated
   pointers, which rustc emits into `.data.rel.ro` — read-only after RELRO
   processing, so such a store would `SIGSEGV` where C succeeds. Both are now
   `static mut`; `readelf -SW` confirms they land in `.data`, matching C.
   Covered by `ERRORS.md` rows 15–17 / `tests/globals.rs`.
2. **`helper_ptr` read the wrong thing.** C does
   `int (*fp)(int,int) = OP_FN(OP);` — the *statically selected* operation. The
   Rust version initialised `fp` from the mutable `G_OP` global instead. The two
   agree until a consumer overwrites `G_OP`, after which the C library keeps
   computing with `OP_FN(OP)` while Rust would have followed the new pointer.
   `helper_ptr` now uses the `OP_FN` constant. Covered by the "clobbering `G_OP`
   must not change library behaviour" assertion in `tests/globals.rs`.

## Negative controls (is the harness able to fail?)

The suite was deliberately fed a mismatched C reference to prove it is not
vacuously green:

| injected fault | result |
|---|---|
| C `.so` swapped for a different `REPEAT` (`add_5` → `add_4`) | 3 + 3 tests fail, `divergence in helper_call` |
| C `.so` swapped for a different `OP` (`add_5` → `mul_5`) | 15 tests fail across `valid_paths` + `errors` |
| C `driver` swapped for a different `OP` | 6 `driver_cli` tests fail on stdout bytes |
| Rust `.so` left stale from another feature set | `load_pair` reports "was built for OP=…, but the active feature set resolves to OP=…" |

## What each test file covers

| file | tests | scope |
|---|---|---|
| `tests/common/mod.rs` | — | loads both `.so`s, resolves the active configuration, seeded RNG, corner-value tables, stale-artifact guard |
| `tests/valid_paths.rs` | 10 | Phase B: all 8 exported symbols, lowest-level first; `use_generated` driven directly to reach all seven `switch` arms (the `driver` only ever calls it with `REPEAT`) |
| `tests/stdout_parity.rs` | 1 | Phase B: byte-for-byte comparison of the three `printf` side effects of the `.so` exports (return-value comparison alone would miss a format-string divergence) |
| `tests/errors.rs` | 8 | Phase C rows 1–14 |
| `tests/globals.rs` | 2 | Phase C rows 15–17 — own process, it clobbers `.data` |
| `tests/driver_cli.rs` | 8 | end-to-end `driver` pipeline + Phase C rows 18–21 and 26 (`argc == 0`, NULL `argv[0]`) |
| `tests/symbols.rs` | 5 | Phase D: `nm -D` parity against all 24 C configurations, symbol kinds, `RTLD_NOW`, and that `static accum_<OP>` stays unexported |

## Notes on the C's build-time surface

`c_src/CMakeLists.txt` exposes two cache variables, `OP` (default `add`) and
`REPEAT` (default `5`), passed through as `-DOP= -DREPEAT=`. Only
`OP ∈ {add, sub, mul}` and `REPEAT ∈ 0..=7` compile — anything else is a
token-paste failure, verified with `gcc -fsyntax-only` (`ERRORS.md` rows 24–25).
That is exactly 24 buildable configurations, and it is why the Cargo feature set
is exactly `add`/`sub`/`mul` plus `0`..`7`. Because Cargo features are additive
and cannot be made mutually exclusive, conflicting selections must still compile;
the crate documents and the tests verify the resolution order (`mul > sub > add`;
highest `REPEAT` wins; empty ⇒ the CMake defaults).
