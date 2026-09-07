# SYMBOLS.md — Phase A symbol surface

Derived mechanically from:

```sh
nm -D --defined-only c_src/build/libdriver.so
nm -D --defined-only translation/target/release/libdriver.so
```

## C source → symbol provenance

| C file | non-static definitions |
|--------|------------------------|
| `c_src/include/shared.h` | `os_calloc`, `os_realloc`, `os_strdup` (defined **in the header** with external linkage) |
| `c_src/src/file-queue.c`  | `merror`, `Init_FileQueue`, `Read_FileMon` (`file_sleep`, `GetFile_Queue`, `Handle_Queue` are `static` → not exported) |
| `c_src/src/read-alert.c`  | `FreeAlertData`, `GetAlertData` |
| `c_src/src/driver.c`      | `driver` |

## Exported symbol parity (`nm -D`, type `T`)

| # | symbol | C `.so` | Rust `.so` | Rust definition site |
|---|--------|---------|-----------|----------------------|
| 1 | `FreeAlertData` | T | T | `src/read_alert.rs` |
| 2 | `GetAlertData`  | T | T | `src/read_alert.rs` |
| 3 | `Init_FileQueue`| T | T | `src/file_queue.rs` |
| 4 | `Read_FileMon`  | T | T | `src/file_queue.rs` |
| 5 | `driver`        | T | T | `src/driver.rs` |
| 6 | `merror`        | T | T | `src/file_queue.rs` |
| 7 | `os_calloc`     | T | T | `src/shared.rs` |
| 8 | `os_realloc`    | T | T | `src/shared.rs` |
| 9 | `os_strdup`     | T | T | `src/shared.rs` |

**Symbol diff: EMPTY.** 9 / 9 C symbols exported by the Rust `.so` with the
exact same names. No macro-generated symbols exist in this C source.

## Undefined-symbol audit of the Rust `.so`

`nm -D --undefined-only` on the Rust `.so` reports only glibc
(`@GLIBC_*`), libgcc unwinder (`_Unwind_*@GCC_*`) and standard
weak/toolchain entries (`__gmon_start__`, `_ITM_*`). **0 missing / undefined
non-libc symbols.**

Every libc entity used by the translation is declared in `src/cbind.rs` and
resolved against the very same glibc the C object files link to, so string /
stdio / `stat` semantics are shared, not re-implemented.

## Build configurations

`translation/Cargo.toml` declares **no `[features]` section**, so the only
configuration is the default one. `cargo check --no-default-features` and
`cargo check` are the same build. Verified by:

```sh
grep -c '^\[features\]' translation/Cargo.toml   # -> 0
```

Consequently "every feature combination" = the single default combination,
which is what Phases B–D exercise.
