# VERIFICATION.md — completion gate

Reproduce everything with one command:

```sh
./run_diff_tests.sh          # builds the C reference, loops over every feature
                             # combination × profile, diffs symbols, runs tests
```

## What is compared

| level | C side | Rust side | how |
|-------|--------|-----------|-----|
| `static_sum` (lowest level) | `libcdriver.so` (`gcc -shared -fPIC c_src/src/main.c`) | `target/<prof>/libdriver.so` (`cdylib` from `src/lib.rs`) | `libloading` + `dlsym`, results compared call by call |
| `main(argc, argv)` | same `.so` | same `.so` | `libloading`; fd 1 is `dup2`-redirected to capture what each one prints |
| whole program | `c_src/build/driver` (CMake) | `target/<prof>/driver` | spawned as subprocesses; stdout, stderr, exit code and signal compared |

The Rust code is **never** called directly — always through the exported
`#[no_mangle]` C-ABI symbols of the `.so`, so the export wrappers are part of
what is tested.

## Gate

- [x] **`SYMBOLS.md`** — `nm -D` on the C `.so` exports `{main, static_sum}`;
      the Rust `.so` exports exactly the same two names, so the diff is empty
      (0 missing). `ldd -r` reports 0 unresolved non-libc symbols. Asserted by
      `tests/symbol_parity.rs` and re-checked by `run_diff_tests.sh` for every
      combination × profile.
- [x] **Phase B** — all 35 rows of `CONFIGS.md` pass, each with randomized
      inputs from a fixed seed (`SplitMix64`, `0x5EED_C0DE_1234_5678`):
      `tests/ffi_static_sum.rs` (rows 1–8), `tests/ffi_main.rs` (rows 9–26, 35),
      `tests/cli_diff.rs` (rows 10–23, 29, 31–34).
- [x] **Phase C** — all 23 rows of `ERRORS.md` have a passing differential test
      that asserts the *exact* message and status (`E1`/`E2`/signal), not just
      "both failed": `tests/ffi_errors.rs` (FFI level, incl. bogus `argc`,
      `argv == NULL`, embedded NUL, oversized inputs) and
      `tests/cli_errors.rs` (program level, incl. `SIGPIPE`, `/dev/full`).
- [x] **Every build configuration** — `Cargo.toml` has no `[features]` and
      `c_src/CMakeLists.txt` has no options, so the only combination is
      `--no-default-features`; it is verified in **both** the `dev` and the
      `release` profile (release adds `panic = "abort"`), each with `cargo
      check --all-targets`, `cargo build --lib --bins`, the `nm -D` diff and the
      full test suite.

## Divergences found and fixed (Rust changed, C untouched)

| # | symptom | fix |
|---|---------|-----|
| 1 | `driver 1 > <closed pipe>`: C is killed by `SIGPIPE` (status 141), Rust exited 0 because the Rust runtime sets `SIGPIPE` to `SIG_IGN` before `main` | `src/main.rs` restores `SIG_DFL` for `SIGPIPE` at the start of `main` (`cli_errors::err_epipe_kills_process`) |
| 2 | `static_sum`'s running total was a `thread_local!`, while C's `static int sum` has static storage duration (one instance per process) | `src/logic.rs` uses a process-global `static SUM: AtomicI32` (`ffi_static_sum::ffi_static_sum_is_process_wide_not_thread_local`) |
| 3 | the translation had no C-ABI surface, so nothing could be loaded/compared through `nm -D`/`dlsym` | `src/logic.rs` (shared translation) + `src/lib.rs` (`cdylib` exporting `main` and `static_sum`) + `src/main.rs` (executable); `[lib] test = false` because the exported `main` cannot coexist with libtest's entry point |

## Test-suite sanity (mutation) checks

Deliberate mutations of the Rust side were each caught by the suite before being
reverted:

| mutation | caught by |
|----------|-----------|
| clamp instead of truncate `long` → `int` | `cfg_long_boundaries`, `cfg_int_boundaries`, `cfg_digit_count_sweep`, `cfg_long_digit_runs`, `cfg_random_i64_strings` |
| drop `\v` (0x0b) from the `isspace` set | `err_randomized_reject_decision` |
| remove the `SIGPIPE` restore | `err_epipe_kills_process` |
| make the running total thread-local | `ffi_static_sum_is_process_wide_not_thread_local` |
