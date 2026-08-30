# Dynamic Symbol Surface

Generated from:

```text
nm -D ../c_src/build/libdriver.so
```

## Defined C exports

| symbol | C type | Rust export | status |
|--------|--------|-------------|--------|
| `driver` | `T` | `driver` | present |
| `printHexCharLine` | `T` | `printHexCharLine` | present |

The required-export diff is empty:

```text
comm -23 \
  <(nm -D --defined-only ../c_src/build/libdriver.so | awk '$2 ~ /^[TW]$/ {print $3}' | sort -u) \
  <(nm -D --defined-only target/release/libdriver.so | awk '$2 ~ /^[TW]$/ {print $3}' | sort -u)
```

## Undefined and weak dynamic entries

These are not definitions exported by the C library. They are recorded to
cover every entry emitted by `nm -D`.

| symbol | C type | classification |
|--------|--------|----------------|
| `_ITM_deregisterTMCloneTable` | `w` | optional compiler-runtime reference |
| `_ITM_registerTMCloneTable` | `w` | optional compiler-runtime reference |
| `__cxa_finalize@GLIBC_2.2.5` | `w` | optional libc runtime reference |
| `__gmon_start__` | `w` | optional profiling-runtime reference |
| `printf@GLIBC_2.2.5` | `U` | libc import |

