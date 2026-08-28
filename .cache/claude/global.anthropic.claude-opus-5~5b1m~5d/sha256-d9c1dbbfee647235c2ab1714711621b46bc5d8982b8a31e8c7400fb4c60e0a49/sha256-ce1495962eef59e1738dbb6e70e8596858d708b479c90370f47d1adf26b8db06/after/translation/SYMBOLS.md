# SYMBOLS.md — Public symbol parity (Phase A / Phase D)

Derived mechanically from `nm -D` on both shared objects. No assumptions.

## Commands used

```sh
# C
cd c_src && mkdir -p build && cd build \
  && cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON && cmake --build .
nm -D --defined-only c_src/build/libdriver.so

# Rust
cd translation && cargo build --release
nm -D --defined-only translation/target/release/libdriver.so
```

## C source surface

`c_src` contains exactly one translation unit and one header:

| file | contents |
|------|----------|
| `c_src/include/lib.h` | `char *custom_strdup(const char *str);` (the whole header — 1 line) |
| `c_src/src/lib.c`     | the single definition of `custom_strdup` (22 lines) |

`c_src/CMakeLists.txt` builds `add_library(driver SHARED src/lib.c)`, so there is
no second module, no macro-generated symbol family, and no conditionally
compiled file. There is therefore **no untranslated C module** — the Rust crate
covers 100% of the C source (1 of 1 functions).

## Exported (defined, dynamic) symbols

| # | symbol | C `.so` | Rust `.so` | status |
|---|--------|---------|------------|--------|
| 1 | `custom_strdup` | `T` (0x1129) | `T` (0x11c60) | **MATCH** |

`nm -D --defined-only` on the C `.so` lists exactly one symbol. The Rust `.so`
exports it with the exact same name via `#[unsafe(no_mangle)] pub unsafe extern
"C" fn custom_strdup`.

**Symbols exported by C but missing from Rust: 0.**
Nothing needed a new `#[no_mangle]` wrapper, and no C module had to be
translated to close a gap. No stubs / `unimplemented!()` were introduced.

## Undefined (imported) symbols

The C `.so` imports only libc plus the standard weak CRT/ITM hooks:

| symbol | kind |
|--------|------|
| `malloc@GLIBC_2.2.5` | libc |
| `memcpy@GLIBC_2.14` | libc |
| `strlen@GLIBC_2.2.5` | libc |
| `_ITM_deregisterTMCloneTable`, `_ITM_registerTMCloneTable`, `__cxa_finalize@GLIBC_2.2.5`, `__gmon_start__` | weak CRT/ITM |

The Rust `.so` imports the same three libc functions (`malloc`, `memcpy`,
`strlen`) because the translation deliberately calls the platform allocator
rather than the Rust global allocator — this keeps the returned pointer
`free()`-able by the caller, exactly as the C contract requires. It additionally
imports libc/`libgcc_s` symbols pulled in by the Rust standard library and its
unwinder (`_Unwind_*`, `abort`, `calloc`, `free`, `realloc`,
`posix_memalign`, `pthread_key_*`, `mmap64`, `dl_iterate_phdr`, …).

**Undefined non-libc symbols in the Rust `.so`: 0.**
`ldd` resolves the Rust `.so` fully against `libgcc_s.so.1`, `libc.so.6` and
`ld-linux-x86-64.so.2` — there are no unresolved application-level symbols.

## Feature combinations

`translation/Cargo.toml` declares **no `[features]` table**, so the only build
configurations that exist are the default one and `--no-default-features`
(which is identical, since there are no default features to remove). Both are
exercised by `run_all.sh`; symbol parity holds in both.

## Verdict

- [x] `nm -D` shows **0 missing** C-exported symbols in the Rust `.so`.
- [x] `nm -D` shows **0 undefined non-libc** symbols in the Rust `.so`.
- [x] Holds under every feature combination (default, `--no-default-features`).
