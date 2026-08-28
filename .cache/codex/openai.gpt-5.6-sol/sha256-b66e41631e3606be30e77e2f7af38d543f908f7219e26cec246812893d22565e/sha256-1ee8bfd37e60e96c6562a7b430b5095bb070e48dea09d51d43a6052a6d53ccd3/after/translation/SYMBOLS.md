# Dynamic Symbol Surface

Generated from:

```text
nm -D --defined-only ../c_src/build/libharvest-work-Wm9von.so
nm -D --defined-only target/release/libsiphash_lib.so
```

## Defined public symbols

| C symbol | C type | Rust type | Status | C definition |
|----------|--------|-----------|--------|--------------|
| `siphash` | `T` | `T` | present | `src/lib.c:114` |
| `stbds_hash_bytes` | `T` | `T` | present | `src/lib.c:110` |

Missing Rust exports: **none**.

## C dynamic imports and weak runtime symbols

These appear in the complete `nm -D` output but are not definitions exported
by the C library:

| Symbol | Type |
|--------|------|
| `_ITM_deregisterTMCloneTable` | `w` |
| `_ITM_registerTMCloneTable` | `w` |
| `__cxa_finalize@GLIBC_2.2.5` | `w` |
| `__gmon_start__` | `w` |
| `printf@GLIBC_2.2.5` | `U` |
| `puts@GLIBC_2.2.5` | `U` |

All strong undefined C symbols are libc functions. The Rust shared object also
resolves `printf` and `puts` dynamically.

## Parity

- [x] Every defined public C symbol is exported by Rust with the exact name.
- [x] Zero C API symbols are missing from Rust.
