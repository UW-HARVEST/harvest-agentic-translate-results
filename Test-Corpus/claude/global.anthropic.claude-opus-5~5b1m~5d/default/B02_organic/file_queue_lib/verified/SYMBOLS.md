# SYMBOLS.md — Phase A: exported-symbol surface

Derived mechanically from `nm -D` on both shared objects:

```
nm -D --defined-only c_src/build/libdriver.so
nm -D --defined-only translation/target/release/libdriver.so
```

## C `.so` defined dynamic symbols (9 total)

| # | symbol | C definition site | exported by Rust `.so`? | Rust definition site |
|---|--------|-------------------|-------------------------|----------------------|
| 1 | `os_calloc`      | `c_src/include/shared.h:13` (non-`static` definition in a header, pulled in by `read-alert.c`) | YES | `src/shared.rs` `#[no_mangle] os_calloc` |
| 2 | `os_realloc`     | `c_src/include/shared.h:22` | YES | `src/shared.rs` `#[no_mangle] os_realloc` |
| 3 | `os_strdup`      | `c_src/include/shared.h:31` | YES | `src/shared.rs` `#[no_mangle] os_strdup` |
| 4 | `FreeAlertData`  | `c_src/src/read-alert.c:64` | YES | `src/read_alert.rs` `#[no_mangle] FreeAlertData` |
| 5 | `GetAlertData`   | `c_src/src/read-alert.c:93` | YES | `src/read_alert.rs` `#[no_mangle] GetAlertData` |
| 6 | `merror`         | `c_src/src/file-queue.c:24` | YES | `src/file_queue.rs` `#[no_mangle] merror` |
| 7 | `Init_FileQueue` | `c_src/src/file-queue.c:113` | YES | `src/file_queue.rs` `#[no_mangle] Init_FileQueue` |
| 8 | `Read_FileMon`   | `c_src/src/file-queue.c:143` | YES | `src/file_queue.rs` `#[no_mangle] Read_FileMon` |
| 9 | `driver`         | `c_src/src/driver.c:6` | YES | `src/driver.rs` `#[no_mangle] driver` |

`file_sleep`, `GetFile_Queue` and `Handle_Queue` are `static` in
`c_src/src/file-queue.c`, so they are **not** dynamic symbols; the Rust
translation keeps them as private `unsafe fn`s (correctly *not* exported).

There are no macro-generated symbols in this library (all `#define`s are
constants / expression macros, none define functions).

## Symbol diff

```
$ comm -23 <(nm -D --defined-only c_src/build/libdriver.so | awk '{print $3}' | sort) \
           <(nm -D --defined-only translation/target/{debug,release}/libdriver.so | awk '{print $3}' | sort -u)
(empty)
```

**Result: 0 symbols missing from the Rust `.so`.** No stubs were needed —
every C translation unit (`shared.h`, `read-alert.c`, `file-queue.c`,
`driver.c`) has a real, complete Rust counterpart
(`shared.rs`, `read_alert.rs`, `file_queue.rs`, `driver.rs`).

The Rust `.so` additionally references the usual Rust-runtime libc/unwind
imports (`_Unwind_*`, `malloc`, `memcpy`, `mmap64`, …). Those are *undefined*
(imported) symbols, not extra exports, and are irrelevant to ABI parity.
`nm -D -u` on the Rust `.so` shows **no undefined non-libc / non-libgcc
symbol**, i.e. nothing that a plain `dlopen` could fail to resolve.

## Cargo features

`translation/Cargo.toml` declares **no `[features]` table**, so the only
feature combination that exists is the default (empty) one. The
`scripts/check_features.sh` helper enumerates the feature combinations from
`Cargo.toml` and runs the full suite for each; with no features declared it
runs the single default configuration plus `--no-default-features`, in both the
debug and the release profile. Last run: **ALL FEATURE COMBINATIONS PASSED**.
