# Verification report — C ↔ Rust differential testing

Project: `StaticAlias` (`c_src/src/main.c`, 72 lines, one translation unit
exporting `static_alias` and `main`) versus its Rust translation
(`src/lib.rs` + `src/main.rs` + `examples/capi.rs`).

## What is compared, and how

| layer | C artifact | Rust artifact | how they are compared |
|-------|-----------|----------------|------------------------|
| C ABI exports | `libcdriver.so` (`cc -shared -fPIC c_src/src/main.c`) | `target/<profile>/examples/libcapi.so` (`cargo build --example capi`) | both `dlopen`ed with `libloading`, symbols resolved with `dlsym`, called through raw `extern "C"` function pointers; stdout captured at the file-descriptor level (`dup2` + `fflush(NULL)`) |
| program | `c_src/build/driver` (CMake, as `c_src/CMakeLists.txt` prescribes) | `target/<profile>/driver` | spawned as processes; stdout bytes, stderr bytes, exit code, terminating signal compared |

No Rust function is ever called directly by the tests — every Rust call goes
through the `#[no_mangle]` export in the loaded `.so`, so the export wrappers are
under test too.

Each scenario loads a *private copy* of both `.so` files, because `dlopen`
refcounts by file identity: distinct copies give distinct images and therefore a
pristine `static int inner = 1;` — the equivalent of a freshly started process.
Scenarios that test state accumulation deliberately share one image and replay
the identical call sequence against both.

## Phase A — surface maps

* `SYMBOLS.md` — `nm -D` surface of the C `.so`: `main`, `static_alias` (plus
  weak CRT symbols). Both are exported by the Rust `.so` under the same names.
* `ERRORS.md` — 20 rows: 4 rejection/range checks that exist in the C source
  (`argc != 3`, `end == argv[1]`, `end == argv[2]`, `i < iterations`), 12 generic
  C-API boundary rows (`strtol` saturation, `long`→`int` narrowing, trailing
  garbage, empty/oversized strings, wild `argc`, extra `argv`, `int` boundary
  values in `static_alias`) and 4 NULL-pointer rows.
* `CONFIGS.md` — 30 rows covering the valid surface: both entry points, both
  branches of `static_alias`, the self-aliasing configuration, the state of the
  hidden static (`1`, grown, `0`, negative, `INT_MIN`, wrapped), the caller's
  storage class, `argv` string shapes, iteration counts, repeated and interleaved
  calls, and the process-level environment (SIGPIPE, closed stdout, locale, empty
  `argv`).

## Phase B/C — tests

| test binary | contents |
|-------------|----------|
| `tests/ffi_static_alias.rs` | 10 tests, CONFIGS rows 1-9, ERRORS rows 14-16. Compares, after **every** call: pointer identity of the result (caller's object vs hidden static), `*result`, and the caller's object. Includes 6 randomized 500-step call sequences (fixed seeds), randomized values per branch/state/storage class, and a non-destructive `INT_MIN` probe of the hidden static after each step. |
| `tests/ffi_main.rs` | CONFIGS rows 10-20, 25, 26 — 1387 scenarios / 1457 `main()` call pairs, comparing the return value and the exact stdout bytes. Every row carries randomized cases in addition to its enumerated ones (650 randomized argument strings from the alphabet `strtol` reacts to, randomized values/counts per branch, randomized boundary neighbourhoods). |
| `tests/ffi_errors.rs` | one row per `ERRORS.md` row 1-16, 319 call pairs (enumerated + randomized: digit-free strings, saturating 20..40-digit magnitudes, values outside `int` but inside `long`). Each row additionally **pins the expected C result** (exit code + exact message bytes), so it asserts the specific error rather than "both failed somehow". |
| `tests/ffi_null_ub.rs` | ERRORS rows 17-20 (NULL pointers) in re-executed child processes, comparing the terminating signal. |
| `tests/cli_diff.rs` | 11 tests, CONFIGS rows 21-24 and 27-30: 300 randomized argument pairs, a 29×12 shape matrix plus 80 randomized shape pairs, 180 randomized digit-free (unparsable) strings, oversized arguments, 20 randomized long-output streams, SIGPIPE on an early-closing reader, closed stdout, locale environment, empty `argv`. |
| `tests/symbols.rs` | Phase D symbol parity, re-derived with `nm -D` at test time. |
| `tests/smoke.rs` | infrastructure self-check. |

## Divergences found and fixed

1. **`SIGPIPE` disposition** (process level). Rust's runtime sets `SIGPIPE` to
   `SIG_IGN` before `main`; the C program keeps the default. `driver 1 20000000 |
   head -2` therefore died with signal 13 (status 141) in C but exited 0 in Rust.
   Fixed in `src/main.rs` (`restore_default_sigpipe`), regression test
   `cli_closed_pipe_sigpipe`.
2. **Hidden static shared across entry points.** The first translation modelled
   the aliasing with a boolean and a function-local variable, which is
   indistinguishable at the CLI but wrong for a library consumer: `static_alias`
   was not exported at all and the state did not survive across calls. The
   translation now uses a real `static mut INNER` and raw pointers, so
   `static_alias` and `main` share state exactly as the C does (tests
   `cfg18_repeat_*`, `cfg19_interleaved*`, `alias_*`).

## Test-suite validation (mutation testing)

Passing tests only mean something if they can fail. `scripts/mutation_check.sh`
injects 12 mutations into `src/lib.rs` (`>=` → `>`, wrapping → saturating
arithmetic in both branches, initial value of the static, altered message text,
removed parse check, removed `strtol` saturation, loop off-by-one, changed
`printf` format, `\v` dropped from the whitespace set, relaxed `argc` check,
clamped instead of truncating narrowing) and requires the suite to fail for each:
**12/12 detected**. `scripts/mutation_check_ffi.sh` repeats 8 of them against
*only* the `dlopen`-based test binaries: **8/8 detected**, i.e. the FFI-level
tests alone are sufficient.

## Phase D — configurations

`Cargo.toml` declares no `[features]` and `c_src/CMakeLists.txt` no options, so
the feature cross-product is the single empty combination.
`scripts/check_features.sh` derives the list from `Cargo.toml` mechanically and
runs `cargo check`/`cargo test` for every combination plus the default and
`--all-features` sets; all pass. Both cargo profiles were exercised end to end
(`dev` and `release`, the latter with `panic = "abort"`), and the Rust
implementation was additionally compared against the C compiled at `-O0`, `-O2`
and `-O3` (81 cases including every signed-overflow case) — identical output.

## Completion gate

- [x] `SYMBOLS.md`: `nm -D` shows 0 C symbols missing from the Rust `.so`; no
      undefined non-libc symbols (dlopen of the Rust `.so` succeeds).
- [x] Phase B: every one of the 30 `CONFIGS.md` rows passes, with randomized
      inputs (fixed seeds) per row.
- [x] Phase C: every one of the 20 `ERRORS.md` rows has a passing differential
      test that also pins the expected C result.
- [x] All of the above under every feature combination (one: the empty set) and
      under both the `dev` and `release` profiles.

Reproduce:

```sh
cd c_src && mkdir -p build && cd build && cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON && cmake --build . && cd ../..
cargo test --all-targets
cargo test --release --all-targets
scripts/check_features.sh test
scripts/mutation_check.sh
scripts/mutation_check_ffi.sh
```
