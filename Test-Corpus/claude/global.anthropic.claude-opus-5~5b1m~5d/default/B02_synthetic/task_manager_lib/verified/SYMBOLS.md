# SYMBOLS.md — Phase A symbol surface

Derived mechanically from:

```
nm -D --defined-only c_src/build/libdriver.so
nm -D --defined-only translation/target/debug/libdriver.so
```

The C build (`c_src/CMakeLists.txt`) compiles exactly three translation units
into one `SHARED` library:

* `src/task_manager.c`
* `src/logger.c`
* `src/driver.c`

All three are translated (`translation/src/task_manager.rs`,
`translation/src/logger.rs`, `translation/src/driver.rs`), plus
`translation/src/cstd.rs` holding the libc `extern` declarations. No C module
is missing.

## Exported (dynamic, defined) symbols

| # | symbol | declared in | C `.so` | Rust `.so` | status |
|---|--------|-------------|---------|------------|--------|
| 1 | `create_task_manager`  | `include/task_manager.h` | T | T | OK |
| 2 | `add_task`             | `include/task_manager.h` | T | T | OK |
| 3 | `print_tasks`          | `include/task_manager.h` | T | T | OK |
| 4 | `destroy_task_manager` | `include/task_manager.h` | T | T | OK |
| 5 | `initialize_logger`    | `include/logger.h`       | T | T | OK |
| 6 | `log_info`             | `include/logger.h`       | T | T | OK |
| 7 | `log_warning`          | `include/logger.h`       | T | T | OK |
| 8 | `log_error`            | `include/logger.h`       | T | T | OK |
| 9 | `finalize_logger`      | `include/logger.h`       | T | T | OK |
| 10 | `driver`              | `src/driver.c` (no header) | T | T | OK |

**Symbol diff (C-defined minus Rust-defined): EMPTY.** Verified by
`tests/symbols.rs::c_exports_are_a_subset_of_rust_exports`, which shells out to
`nm -D --defined-only` on both objects at test time.

There are no macro-generated symbols in this library.

`driver` has no prototype in any installed header (`install(DIRECTORY include/)`
ships only `logger.h` and `task_manager.h`), but it has external linkage and is
exported, so it is part of the ABI surface and is tested.

## Undefined symbols (imports)

The C `.so` imports only glibc: `atoi fclose fopen fprintf free fwrite getenv
malloc printf puts stderr strchr strlen strncpy` (+ weak `_ITM_*`,
`__cxa_finalize`, `__gmon_start__`).

Note `fwrite`/`puts` appear only because gcc rewrites
`fprintf(f, "...%s\n", ...)`-style and `printf("Tasks:\n")` calls into
`fwrite`/`puts`. That is a code-generation detail; the bytes written are
identical, so the Rust side keeping literal `printf`/`fprintf` is behaviourally
equivalent.

The Rust `.so` imports the same glibc set plus the Rust runtime's own libc /
`libgcc_s` unwinder needs (`memcpy`, `memset`, `mmap64`, `_Unwind_*`,
`pthread_key_*`, …). `ldd` resolves to `libc.so.6` and `libgcc_s.so.1` only.

**0 missing / 0 unresolved non-libc symbols in the Rust `.so`.**

## Feature combinations

`translation/Cargo.toml` declares **no `[features]` table**, hence exactly one
build configuration exists (default = no features). "Every feature combination"
therefore reduces to the single default combination; the test suite is
additionally run under `--release` and under `--no-default-features` to prove
the surface is configuration-independent.

## Build note (staleness hazard)

`cargo test` does **not** build a `crate-type = ["cdylib"]` library target — the
test harness cannot link one, so cargo skips it. The `.so` must be produced by
an explicit `cargo build` / `cargo build --release` first, otherwise the tests
either fail to find it or silently exercise an *old* one.

`tests/common/mod.rs::rust_so()` guards both cases: it asserts the `.so` exists
and that every `src/*.rs` is older than it. `run_verification.sh` runs
`cargo build` before `cargo test` for each profile and feature combination.
