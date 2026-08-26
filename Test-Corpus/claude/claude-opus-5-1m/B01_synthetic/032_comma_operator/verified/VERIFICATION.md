# VERIFICATION.md — completion gate

C ground truth: `c_src/src/main.c` (36 lines, two functions: `driver`, `main`).
Rust translation: `src/imp.rs`, exported as a C ABI surface by `src/lib.rs`
(`cdylib`) and as an executable by `src/main.rs`.

## Completion checklist

- [x] **`SYMBOLS.md`: `nm -D` shows 0 missing/undefined non-libc symbols in Rust.**
      The C `.so` exports `driver` and `main`; the Rust `.so` exports both under
      the exact same names. The symbol diff is empty and there are no undefined
      non-libc symbols. Verified by `./check_symbols.sh` in both the `debug` and
      `release` profiles.
- [x] **Phase B: every row in `CONFIGS.md` passes across randomized inputs.**
      32/32 rows green (`tests/phase_b_configs.rs`), ~1 000 distinct inputs from
      fixed PRNG seeds.
- [x] **Phase C: every row in `ERRORS.md` has a passing error-path differential
      test.** 25/25 table rows plus 4 extra generic-boundary cases = 29/29 green
      (`tests/phase_c_errors.rs`). Each asserts identical stdout bytes *and* the
      identical wait status (exit code + terminating signal).
- [x] **All of the above hold under every feature combination.** `Cargo.toml`
      declares `[features] default = []` with no optional features and
      `c_src/CMakeLists.txt` has no options or `-D` flags, so the cross-product
      has exactly one member (`<none>` ≡ default); `check_all_features.sh`
      enumerates it mechanically and `run_difftests.sh` runs symbol parity and
      both suites for each. The suites additionally pass under the `release`
      profile (the crate's other build-time configuration, `panic = "abort"`).

## How to reproduce

```sh
./run_difftests.sh          # Phase A checks + Phase D symbol parity + Phases B/C
./check_all_features.sh     # feature-combination enumeration + cargo check
./check_symbols.sh          # nm -D parity only
cargo test --offline        # the two differential suites
cargo test --offline --release
```

`run_difftests.sh` ends with `ALL CHECKS PASSED` and exit status 0.

Nothing under `c_src/` was modified. The C reference artifacts used by the tests
(`target/<profile>/c_ref/libdriver_c.so`, `.../driver_c`) are compiled from
`c_src/src/main.c` by the test harness on every run, so they can never be stale;
`c_src/build/` additionally holds the plain CMake build (`cmake .. && cmake --build .`)
and was spot-checked to produce byte-identical output to the harness' `cc` build.

## Everything that was changed in the Rust side, and why

| change | reason |
|--------|--------|
| `src/imp.rs` (new) | the translation itself, moved out of `src/main.rs` so the cdylib and the binary share one copy and cannot drift |
| `src/lib.rs` (new) | `#[no_mangle] extern "C"` wrappers exporting `driver` and `main`, matching the C `.so`'s symbol set exactly |
| `src/main.rs` | now a thin entry point over `imp`, **plus** `restore_default_sigpipe()` |
| `Cargo.toml` | `[lib] crate-type = ["cdylib"], test = false, doctest = false`; `[features] default = []`; the two `harness = false` test targets; `libloading` dev-dependency |
| `src/imp.rs` `driver_stdout` | flushes the `StdoutLock` as well as the `BufWriter`, so a caller that only has the C ABI (no way to ask Rust to flush) sees the bytes as soon as `driver` returns |

### The one behavioural bug found

Rust's runtime sets `SIGPIPE` to `SIG_IGN` before `main`. The C program keeps the
default disposition, so `./driver | head -1` kills the C process with signal 13
(shell status 141) while the first Rust translation ran to completion and exited
0. `src/main.rs` now restores `SIG_DFL` for `SIGPIPE` as its first action, and
`err_24_main_sigpipe` fails if that line is removed. The reset lives only in the
binary: the `main` exported from the shared object must inherit the host
process's disposition, which is what the C shared object does.

### The one test-infrastructure bug found

`cargo test` builds the package's binaries but **not** the `cdylib` artifact, so
the first version of the suite silently compared the C `.so` against a *stale*
`libdriver.so` — deliberately broken Rust code still "passed". `tests/common/mod.rs`
now runs `cargo build` itself before the first comparison and hard-fails if any
Rust artifact is older than `src/`. Mutation testing (table in `ERRORS.md`)
confirms the suite now detects every non-equivalent mutation.
