# Dynamic symbol surface

Generated from:

```text
nm -D ../c_src/build/libharvest-work-68uRX7.so
nm -D target/release/libmerge_sort_lib.so
```

## C-defined public symbols

| symbol | C `.so` | Rust `.so` | status |
|---|---:|---:|---|
| `merge_sort` | `T` | `T` | [x] exact export present |

## C dynamic dependencies

These are runtime/toolchain dependencies, not library API exports:

| symbol | kind |
|---|---|
| `_ITM_deregisterTMCloneTable` | weak runtime symbol |
| `_ITM_registerTMCloneTable` | weak runtime symbol |
| `__cxa_finalize@GLIBC_2.2.5` | weak libc/runtime symbol |
| `__gmon_start__` | weak runtime symbol |
| `memcpy@GLIBC_2.14` | undefined libc dependency |

Missing C-defined symbols in Rust: **0**.

Final `ldd -r target/release/libmerge_sort_lib.so` check: **0 unresolved
symbols**.
