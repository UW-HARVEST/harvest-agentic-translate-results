# SYMBOLS.md — exported-symbol parity

Derived mechanically:

```
cd translated_rust/c_src && mkdir -p build && cd build \
  && cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON && cmake --build .
cargo build --release
nm -D --defined-only c_src/build/libdriver.so   | sort -k3
nm -D --defined-only target/release/libdriver.so | sort -k3
```

## C translation units contributing symbols

| C file | notes |
|--------|-------|
| `c_src/src/file-queue.c` | `merror`, `Init_FileQueue`, `Read_FileMon` global; `file_sleep`, `GetFile_Queue`, `Handle_Queue` are `static` → **not** exported |
| `c_src/src/read-alert.c` | `FreeAlertData`, `GetAlertData` global; also drags in `shared.h`'s three **non-static header-defined** functions → `os_calloc`, `os_realloc`, `os_strdup` |
| `c_src/src/driver.c` | `driver` global |

`s_month[]` is `static const` → not exported. No namespacing/aliasing macros are
used anywhere, so linker names == source-level names.

## Symbol table (T = global text)

| # | symbol | in C `.so` | in Rust `.so` | Rust definition site |
|---|--------|-----------|---------------|----------------------|
| 1 | `FreeAlertData`  | T | T | `src/read_alert.rs` `#[unsafe(no_mangle)] extern "C"` |
| 2 | `GetAlertData`   | T | T | `src/read_alert.rs` |
| 3 | `Init_FileQueue` | T | T | `src/file_queue.rs` |
| 4 | `Read_FileMon`   | T | T | `src/file_queue.rs` |
| 5 | `driver`         | T | T | `src/driver.rs` |
| 6 | `merror`         | T | T | `src/file_queue.rs` |
| 7 | `os_calloc`      | T | T | `src/shared.rs` |
| 8 | `os_realloc`     | T | T | `src/shared.rs` |
| 9 | `os_strdup`      | T | T | `src/shared.rs` |

**Missing from Rust: none.** No stubs, no `unimplemented!()` — every one of the
nine is a full translation of the corresponding C body. All three C source files
plus both public headers plus `shared.h` are translated
(`file_queue.rs`, `read_alert.rs`, `driver.rs`, `shared.rs`, `cbits.rs`).

## `nm -D` diff

```
$ diff <(nm -D --defined-only c_src/build/libdriver.so   | awk '{print $3}' | sort) \
       <(nm -D --defined-only target/release/libdriver.so | awk '{print $3}' | sort)
(no output — identical)
```

Verified by `tests/symbols.rs::symbol_parity_c_vs_rust`, which shells out to
`nm -D` on both objects at test time and asserts the defined-symbol sets are
equal.

## Undefined symbols in the Rust `.so`

`nm -D --undefined-only target/release/libdriver.so` lists **only** libc
(`GLIBC_*`), libgcc unwinder (`_Unwind_*@GCC_*`) and the standard weak ELF
symbols (`__gmon_start__`, `_ITM_*TMCloneTable`, `__cxa_finalize`,
`__cxa_thread_atexit_impl`, `gettid`, `statx`). **0 missing/undefined non-libc
symbols.** `ldd` resolves fully against `libgcc_s.so.1` + `libc.so.6`.

## Build configurations

`Cargo.toml` has **no `[features]` table** and `grep -rn 'cfg(feature' src/`
returns nothing, so the crate has exactly **one** build configuration. Both
`cargo check --no-default-features` and `cargo check` (identical inputs) are
run by `tmp/check_combos.sh`; both succeed with no warnings/errors.
`c_src/CMakeLists.txt` likewise has no options, no `#ifdef`-gated sources and no
`target_compile_definitions` — a single unconditional `SHARED` target from the
three `.c` files.
