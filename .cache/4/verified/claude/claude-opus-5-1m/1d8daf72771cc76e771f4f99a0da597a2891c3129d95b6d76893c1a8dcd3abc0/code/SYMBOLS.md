# SYMBOLS.md — exported-symbol parity (Phase A / Phase D)

Derived mechanically from `nm -D` on both shared libraries.

Commands used:

```sh
# C
cd c_src && mkdir -p build && cd build && \
  cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON && cmake --build .
nm -D --defined-only c_src/build/libdriver.so

# Rust
cargo build --offline
nm -D --defined-only target/debug/libdriver.so
```

## C source surface

`c_src/CMakeLists.txt` builds exactly one translation unit (`src/lib.c`) into
`libdriver.so`. `c_src/include/lib.h` declares exactly one function:

```c
char *decode_base64(const char *src);
```

`lib.c` additionally defines two **`static`** helpers (`decode`, `is_base64`)
which have internal linkage and therefore are *not* part of the dynamic symbol
table — they are exercised only indirectly through `decode_base64`.
There are no namespace/renaming macros in the header, so the linker name equals
the source name.

## Symbol table (defined / exported)

| # | symbol | C `.so` | Rust `.so` | notes |
|---|--------|---------|------------|-------|
| 1 | `decode_base64` | `T` (defined) | `T` (defined) | `#[unsafe(no_mangle)] pub unsafe extern "C" fn decode_base64(*const c_char) -> *mut c_char` |

`static` C functions (must NOT appear, and do not appear, in either `.so`):

| symbol | C `.so` | Rust `.so` |
|--------|---------|------------|
| `decode` | absent (internal linkage) | absent (private `fn`) |
| `is_base64` | absent (internal linkage) | absent (private `fn`) |

## Undefined (imported) symbols

| symbol | C `.so` | Rust `.so` | libc? |
|--------|---------|------------|-------|
| `calloc@GLIBC_2.2.5` | U | U | yes |
| `malloc@GLIBC_2.2.5` | U | U | yes |
| `free@GLIBC_2.2.5` | U | U | yes |
| `strlen@GLIBC_2.2.5` | U | U | yes |
| `_ITM_deregisterTMCloneTable`, `_ITM_registerTMCloneTable`, `__gmon_start__`, `__cxa_finalize` | w (weak) | w (weak) | toolchain-generated |

The Rust `.so` additionally imports the usual weak/toolchain and Rust-runtime
libc symbols (`memcpy`, `_Unwind_*`, `pthread_*`, …) supplied by `libc`/`libgcc`.
Those are *additions* (allowed); no C-exported symbol is missing.

## Result

```
$ comm -23 <(nm -D --defined-only c_src/build/libdriver.so | awk '{print $3}' | sort -u) \
           <(nm -D --defined-only target/debug/libdriver.so | awk '{print $3}' | sort -u)
(empty)
```

* Symbols exported by C but missing from Rust: **0**
* Undefined non-libc / non-toolchain symbols in the Rust `.so`: **0**
* Nothing was stubbed: `decode_base64` is a full translation of the C body,
  including its two `static` helpers.

Automated check: `tests/symbols.rs::c_and_rust_export_identical_symbols`.
