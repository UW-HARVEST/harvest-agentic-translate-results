# SYMBOLS.md — Phase A: exported-symbol surface

Mechanically derived from `nm -D --defined-only` on both shared objects.
Regenerate with `./symdiff.sh`.

```
C   .so : c_src/build/libdriver.so     (cmake -DCMAKE_POSITION_INDEPENDENT_CODE=ON)
Rust .so: target/<profile>/[deps/]libdriver.so   (crate-type = ["cdylib", "rlib"])
```

## Build configurations

`Cargo.toml` declares **no `[features]` section**, and `cargo metadata` reports
`driver -> {}` (zero features, not even `default`). The C side has **no
`#ifdef`/`#if` configuration** either — `grep -rn '#if\|#ifdef\|#else\|#elif'
c_src/src c_src/include` matches nothing outside the two `#ifndef` header
guards, and `CMakeLists.txt` sets no `target_compile_definitions` / options.

Therefore the complete set of valid feature combinations is exactly one, the
empty set:

| # | combination | profile | command | result |
|---|-------------|---------|---------|--------|
| 1 | `∅` (empty; identical to `default`) | `dev`     | `cargo check --offline --no-default-features` | OK |
| 2 | `default` (same set, both spellings checked) | `dev`     | `cargo check --offline` | OK |
| 3 | `∅` | `release` | `cargo check --offline --release --no-default-features` | OK |
| 4 | `default` | `release` | `cargo check --offline --release` | OK |

Both optimisation profiles are treated as distinct configurations, because
`debug-assertions` / `overflow-checks` and the optimiser demonstrably change
observable behaviour at the FFI boundary (see the finding at the end of
`ERRORS.md`). Phases B and C are run for all four rows above.

Enumerated + verified mechanically by `./check_all_features.sh check`
(power set of the declared feature list, plus the default set).
No `#[cfg(feature = "...")]` gating is required anywhere in `src/`.

## Exported (defined, dynamic) symbols

All 10 symbols the C `.so` exports are exported by the Rust `.so` under the
**exact same name**. There are no macro-generated / namespaced / renamed
symbols: neither header in `c_src/include/` defines a name-mangling macro, so
linker names equal source-level names.

| # | symbol | C source | type | Rust source | exported by Rust `.so` |
|---|--------|----------|------|-------------|------------------------|
| 1 | `create_task_manager`  | `c_src/src/task_manager.c:32` | `T` (func) | `src/task_manager.rs` `#[unsafe(no_mangle)] extern "C"` | yes |
| 2 | `add_task`             | `c_src/src/task_manager.c:53` | `T` | `src/task_manager.rs` | yes |
| 3 | `print_tasks`          | `c_src/src/task_manager.c:67` | `T` | `src/task_manager.rs` | yes |
| 4 | `destroy_task_manager` | `c_src/src/task_manager.c:74` | `T` | `src/task_manager.rs` | yes |
| 5 | `initialize_logger`    | `c_src/src/logger.c:33`       | `T` | `src/logger.rs` | yes |
| 6 | `log_info`             | `c_src/src/logger.c:47`       | `T` | `src/logger.rs` | yes |
| 7 | `log_warning`          | `c_src/src/logger.c:53`       | `T` | `src/logger.rs` | yes |
| 8 | `log_error`            | `c_src/src/logger.c:59`       | `T` | `src/logger.rs` | yes |
| 9 | `finalize_logger`      | `c_src/src/logger.c:65`       | `T` | `src/logger.rs` | yes |
| 10| `driver`               | `c_src/src/driver.c:32`       | `T` | `src/driver.rs` | yes |

Notes:
* `driver` has no declaration in any public header, but it is a non-`static`
  definition and therefore part of the `.so` ABI. It **is** translated and
  exported.
* `logger.c`'s `static FILE *log_file` is file-scope-static in C, so it is
  correctly **not** an exported symbol on either side (the Rust translation
  models it as a private `static LOG_FILE: AtomicPtr<FILE>`).
* No C source file was skipped by the translation: `task_manager.c`, `logger.c`
  and `driver.c` (the exact three files listed in `CMakeLists.txt`) map 1:1 onto
  `src/task_manager.rs`, `src/logger.rs`, `src/driver.rs`, with `src/cffi.rs`
  holding the libc `extern` declarations.

### Where the artifacts live

`cargo build` uplifts the cdylib to `target/<profile>/libdriver.so`, while
`cargo test` only emits `target/<profile>/deps/libdriver.so`. The test harness
(`tests/common/mod.rs::rust_so_path`) picks whichever of the two was modified
**most recently**, so a stale uplifted copy can never silently shadow the
freshly-built one. (`RUST_DRIVER_SO` / `C_DRIVER_SO` override both.)

Note: `[lib] crate-type` was changed from `["cdylib"]` to `["cdylib", "rlib"]`.
A cdylib-**only** lib target cannot be linked by test targets, so Cargo skips
producing the `.so` during `cargo test` at all and the differential tests had
nothing to `dlopen`. The `rlib` is never linked by the tests — every call still
goes through `libloading` + `dlsym`.

### Symbol diff result

```
C   exported symbols: 10
Rust exported symbols: 10

=== symbols in C .so but MISSING from Rust .so ===
(none)

=== symbols only in Rust .so (extra) ===
(none)

=== Rust .so undefined symbols that are NOT libc/libgcc ===
(none)
```

## Imported (undefined) symbols

The Rust `.so` must not depend on anything outside libc / the GCC unwinder.

C `.so` imports (14 + 4 weak): `atoi fclose fopen fprintf free fwrite getenv
malloc printf puts stderr strchr strlen strncpy`, weak `_ITM_*`,
`__cxa_finalize`, `__gmon_start__`.
(`fwrite`/`puts` are compiler rewrites of `printf("Tasks:\n")` and friends.)

Rust `.so` imports the same 14 plus the extra glibc/`libgcc` surface that any
`std`-linked cdylib pulls in (`memcpy`, `mmap64`, `pthread_key_create`,
`_Unwind_*`, …). Every one is `@GLIBC_*` or `@GCC_*` versioned, i.e. libc /
unwinder: the "non-libc undefined" set is empty. The Rust translation calls
straight through to the same libc entry points (`src/cffi.rs`), which is what
makes `printf`/`fprintf` conversion semantics (e.g. `%s` with a `NULL`
argument printing `(null)`) byte-identical.

### Verified for

| profile | feature set | `nm -D` missing | non-libc undefined | tests |
|---------|-------------|-----------------|--------------------|-------|
| `dev`     | `∅` / `default` | 0 | 0 | all pass |
| `release` | `∅` / `default` | 0 | 0 | all pass |

Checked mechanically by `./symdiff.sh [path-to-rust.so]` and by
`tests/phase_d_symbols.rs` (`d02`, `d03`), which shell out to `nm` and also
`dlsym` all ten symbols out of both libraries.
