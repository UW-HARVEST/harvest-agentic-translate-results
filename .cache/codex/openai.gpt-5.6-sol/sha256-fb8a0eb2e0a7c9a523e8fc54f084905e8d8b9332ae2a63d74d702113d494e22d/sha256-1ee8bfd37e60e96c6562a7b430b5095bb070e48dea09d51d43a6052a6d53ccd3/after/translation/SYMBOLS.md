# Dynamic Symbol Surface

Derived from:

```text
nm -D --defined-only ../c_src/build/libdriver.so
nm -D --undefined-only ../c_src/build/libdriver.so
```

## Library-owned exports

| C symbol | C type | Rust export | Status |
|----------|--------|-------------|--------|
| `driver` | `T` | `driver` | present |
| `printLine` | `T` | `printLine` | present |

The exact-name defined-symbol difference is empty:

```text
comm -23 \
  <(nm -D --defined-only ../c_src/build/libdriver.so | awk '{print $3}' | sort -u) \
  <(nm -D --defined-only target/release/libdriver.so | awk '{print $3}' | sort -u)
```

## C shared-library imports

| Symbol | Binding | Classification |
|--------|---------|----------------|
| `_ITM_deregisterTMCloneTable` | weak | toolchain/runtime |
| `_ITM_registerTMCloneTable` | weak | toolchain/runtime |
| `__cxa_finalize@GLIBC_2.2.5` | weak | libc/toolchain runtime |
| `__gmon_start__` | weak | toolchain/runtime |
| `memset@GLIBC_2.2.5` | strong | libc |
| `puts@GLIBC_2.2.5` | strong | libc |
| `strncpy@GLIBC_2.2.5` | strong | libc |

There are no undefined non-libc library API symbols.
