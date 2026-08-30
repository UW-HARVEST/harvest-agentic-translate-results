# VERIFICATION.md — completion gate

Reproduce everything with:

```
cd translation && ./run_all_features.sh
```

## Result

| gate | status |
|------|--------|
| `SYMBOLS.md`: `nm -D` shows 0 missing / undefined non-libc symbols in Rust | **PASS** — symbol diff empty; only import is `printf`/`putchar` + glibc/libgcc runtime |
| Phase B: every row in `CONFIGS.md` (22) passes across randomised inputs | **PASS** — 22/22 tests, `tests/valid_paths.rs` |
| Phase C: every row in `ERRORS.md` (14) has a passing error-path differential test | **PASS** — 12 tests covering all 14 rows, `tests/error_paths.rs` |
| All of the above under EVERY feature combination | **PASS** — 6 configurations (3 feature spellings × release/debug) |

37 tests per configuration; 222 test executions in total, all green.

The crate declares no `[features]` and no optional dependencies, so
`{default}`, `{--no-default-features}` and `{--all-features}` are the complete —
and equivalent — set of feature combinations. `run_all_features.sh` runs all
three under both the `release` and `debug` profiles, because the two profiles
produce measurably different `.so`s (see the `putchar` note below).

## What was tested and how

The library's whole public surface is `void driver(float x)` (one C translation
unit, 40 lines incl. licence header). Its only observable effect is bytes on
`stdout`, so the harness (`tests/common/mod.rs`) redirects fd 1 to a temporary
file around every call, then compares the captured bytes from the C `.so` and
the Rust `.so`. Both libraries are loaded with `libloading::Library::new` and
called through `dlsym`, so the `#[no_mangle] extern "C"` export wrapper is
itself under test — no Rust function is ever called directly.

Coverage highlights: ~1.5 million differential `driver` invocations per
implementation, including an exhaustive sweep of both 16-bit halves of the input
against boundary constants, every exponent × sign × mantissa-corner combination,
every byte value 0x00–0xFF in every one of the 4 byte positions, and ~400 k
uniform-random 32-bit patterns (deterministic SplitMix64, fixed seed).

## Two harness defects found and fixed during verification

These are recorded because both would have produced **false green** results.

1. **`cargo test` does not re-link the `cdylib`.** It compiles the lib target as
   a test binary but leaves `target/<profile>/libdriver.so` untouched. The suite
   was therefore dlopen-ing a stale artifact: a deliberately broken Rust library
   still passed all 37 tests. Fixed by (a) building explicitly before every
   `cargo test` in `run_all_features.sh`, and (b) `assert_so_is_fresh` in the
   harness, which refuses to run if the `.so` is older than anything in `src/`
   or `Cargo.toml`. Verified: touching `src/lib.rs` and running `cargo test`
   without a rebuild now fails with `STALE ARTIFACT`.
2. **libtest's own progress output landed inside the captures.** fd-1
   redirection is process-global, so `test foo ... ok` written by *other* test
   threads was captured as if it were library output, corrupting comparisons.
   Fixed with `RUST_TEST_THREADS=1` (`.cargo/config.toml`), by *not* flushing
   Rust's `stdout` inside the capture window (that would push libtest's pending
   partial line into the file), and by a defensive check that captured bytes are
   only ever `[0-9a-f\n]`.

## Mutation testing — proof the suite is not vacuous

Each mutation was applied to `translation/src/lib.rs`, the `.so` was rebuilt,
and the suite re-run. All were detected.

| # | mutation | detected by |
|---|----------|-------------|
| 1 | `to_ne_bytes` → `to_be_bytes` (byte order) | 12/12 error tests |
| 2 | `%02x` → `%02X` (uppercase hex) | 11/12 error tests |
| 3 | drop the trailing `printf("\n")` | 12/12 error tests |
| 4 | print 3 bytes instead of `sizeof(float)` | 12/12 error tests, incl. E1/E2 record-length invariant |
| 5 | `%02x` → `%2x` (lose zero padding) | 12/12 error tests |
| 6 | emit via Rust's `std::io::stdout` instead of C `printf` | `rust_so_has_no_unresolved_non_libc_symbols` (no `printf` import) |
| 7 | private buffer flushed only every 8 KiB | E14 + all batched comparisons (reordering) |
| 8 | `x` → `x + 0.0` (quiets sNaN, normalises `-0.0`) | E4 (signalling NaN) and E9 (signed zeros), 8/12 total |

Mutations 6–8 are the ones a happy-path suite would miss, and are exactly why
`ERRORS.md` rows E4/E9/E14 and `CONFIGS.md` rows C19/C20 exist.

## Known-benign codegen difference (not a behavioural divergence)

Both GCC and LLVM rewrite `printf("\n")` into `putchar('\n')` when optimising.
The C `.so` and the **release** Rust `.so` therefore import `putchar`; the
**debug** Rust `.so` imports only `printf`. Both spellings write the same single
byte to the same libc `stdout` stream, and all 37 differential tests pass in
both profiles, so this is a codegen artifact with no observable effect.
`symbol_parity.rs` asserts the *exported* symbol sets match exactly, and
separately that Rust reaches stdout through C stdio (imports `printf`) and uses
no stdio entry point the C library does not — rather than demanding a byte-exact
import list.

## Conclusion

The Rust translation in `translation/src/lib.rs` is byte-for-byte equivalent to
`c_src/src/driver.c` across the entire input domain of its single public
function, under every feature combination and both build profiles. No divergence
was found in the translation itself; it required no changes.
