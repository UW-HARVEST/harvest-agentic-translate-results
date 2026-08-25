# Dynamic Symbol Surface

Derived from:

```sh
nm -D c_src/build/libdriver.so
nm -D --defined-only c_src/build/libdriver.so
nm -D --defined-only target/release/libdriver.so
```

## Library-owned exports

| symbol | C type | Rust type | status |
|--------|--------|-----------|--------|
| `driver` | `T` | `T` | present |
| `run` | `T` | `T` | present |

The exact-name export diff is empty:

```sh
comm -23 \
  <(nm -D --defined-only c_src/build/libdriver.so | awk '{print $3}' | sort -u) \
  <(nm -D --defined-only target/release/libdriver.so | awk '{print $3}' | sort -u)
```

## C shared-library imports

These are the remaining entries emitted by `nm -D`; they are dynamic
dependencies rather than symbols implemented by this library.

| symbol | type | provider / role |
|--------|------|-----------------|
| `_ITM_deregisterTMCloneTable` | `w` | toolchain weak import |
| `_ITM_registerTMCloneTable` | `w` | toolchain weak import |
| `__cxa_finalize@GLIBC_2.2.5` | `w` | libc weak import |
| `__errno_location@GLIBC_2.2.5` | `U` | libc import |
| `__gmon_start__` | `w` | toolchain weak import |
| `printf@GLIBC_2.2.5` | `U` | libc import |
| `puts@GLIBC_2.2.5` | `U` | libc import |
| `strtol@GLIBC_2.2.5` | `U` | libc import |

Completion status: [x] zero C-defined symbols are missing from Rust; [x] zero
undefined non-libc library symbols require a Rust implementation.
