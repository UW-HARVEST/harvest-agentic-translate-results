# Verification summary

C ground truth: `c_src/src/main.c` (one translation unit, 6 functions).
Rust translation: `src/imp.rs` (behaviour) + `src/lib.rs` (C-ABI exports) +
`src/main.rs` (executable).

Run everything with:

```
./verify.sh          # every feature combination + release sanity
cargo test --offline # the differential suite for the current combination
```

## Completion gate

| gate | evidence | result |
|------|----------|--------|
| `SYMBOLS.md`: `nm -D` shows 0 missing/undefined non-libc symbols in Rust | `comm -23 c.syms r.syms` → 0; `tests/symbol_parity.rs` (4 tests) | ✅ |
| Phase B: every `CONFIGS.md` row passes across randomized inputs | `tests/differential_valid.rs`, 20 tests = rows C1–C20, ~1 800 randomized cases from `SEED = 0x20260818` | ✅ |
| Phase C: every `ERRORS.md` row has a passing error-path differential test | `tests/differential_errors.rs`, 10 tests = rows E1–E10 (E11 = N/A, no enum in the C API; covered by E9) | ✅ |
| Holds under every feature combination | `Cargo.toml` declares no features, so the only combination is the empty one; `verify.sh` runs `--no-default-features`, default and `--all-features` — 34 tests pass in each | ✅ |

## What the differential tests actually compare

Both implementations are loaded as shared objects and called **only** through
their exported symbols (`libloading` + `dlsym`), so the `#[no_mangle]` wrappers
are part of what is under test:

* C: `target/cdiff/libcdriver.so` — `cc -shared -fPIC c_src/src/main.c`
  (nothing in `c_src/` is modified; the CMake `add_executable` build is also run
  and used for the end-to-end executable comparison).
* Rust: `target/cdiff/rustlib/debug/libdriver.so` — rebuilt by the harness on
  every run, because `cargo test` does *not* refresh the `cdylib`.

The library's only observable effect is bytes on fd 1, so each call is made with
fd 1 redirected to a temporary file; the C stdio buffer is flushed with
`fflush(NULL)` and the captured bytes are compared exactly, plus the `int`
returned by `main`.

## Harness pitfalls that were found and fixed

1. **Stale `cdylib`.** `cargo test` builds the library only as an `rlib`;
   `target/debug/libdriver.so` was left over from an earlier `cargo build`, so
   the tests were verifying old code. The harness now performs a nested
   `cargo build --lib` into a private target dir before loading.
2. **Foreign writers to fd 1.** libtest writes its progress lines to the
   process-global `io::stdout()` from another thread, and those bytes landed in
   the capture file. `capture()` now holds the `StdoutLock` across the whole
   redirect window, which blocks libtest until fd 1 is restored — no reliance on
   `--test-threads=1`.

## Mutation testing (proof the suite is not vacuous)

Each mutation was applied to the Rust side only, the suite was run, and the
mutation was then reverted (sources verified byte-identical afterwards):

| mutation | detected by | outcome |
|----------|-------------|---------|
| `bad()` also calls `helper_bad()` (the C `static helperBad` is never called) | E3, E9, E10, C11, C16, C17, C18 | ✅ FAILED as expected |
| `print_line` renders through `String::from_utf8_lossy` (the classic `&str` mistranslation) | E3, E5, E6, C5, C7, C16, C19 | ✅ FAILED as expected |
| `#[no_mangle]` removed from `good` | `phase_d_rust_so_exports_every_c_symbol`, `phase_d_every_c_symbol_is_dlsym_resolvable_in_rust` | ✅ FAILED as expected (`missing: ["good"]`) |

## Translation notes (C behaviour deliberately preserved)

* `printLine` takes raw bytes (`*const c_char` → `CStr::to_bytes()`), never
  `&str`: C copies the bytes verbatim and does not validate UTF-8.
* NULL is silently ignored (`if (line != NULL)`), with no error signalled.
* `helperBad` is translated but never called — `bad()` calls only `printLine`,
  exactly like the C.
* The exported `main` ignores `argc`/`argv` and returns 0 without terminating
  the process; the executable's `main` exits with that status.
* `static` helpers are not exported by either `.so`.
