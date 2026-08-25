# Dynamic Symbol Surface

Generated from:

```text
nm -D c_src/build/libdriver.so
nm -D --defined-only c_src/build/libdriver.so
```

## Defined public symbols

| # | symbol | C type | Rust export | status |
|---|--------|--------|-------------|--------|
| 1 | `bad` | `T` | `bad` | [x] |
| 2 | `driver` | `T` | `driver` | [x] |
| 3 | `good` | `T` | `good` | [x] |
| 4 | `printIntLine` | `T` | `printIntLine` | [x] |
| 5 | `printLine` | `T` | `printLine` | [x] |

## Undefined runtime symbols shown by `nm -D`

These are toolchain/libc imports, not symbols defined by the C library.

| symbol | C type | provider |
|--------|--------|----------|
| `_ITM_deregisterTMCloneTable` | `w` | compiler runtime (optional weak import) |
| `_ITM_registerTMCloneTable` | `w` | compiler runtime (optional weak import) |
| `__cxa_finalize@GLIBC_2.2.5` | `w` | libc (optional weak import) |
| `__gmon_start__` | `w` | compiler profiling runtime (optional weak import) |
| `printf@GLIBC_2.2.5` | `U` | libc |
| `puts@GLIBC_2.2.5` | `U` | libc |

## Parity command

```sh
comm -23 \
  <(nm -D --defined-only c_src/build/libdriver.so | awk '{print $3}' | sort -u) \
  <(nm -D --defined-only target/release/libdriver.so | awk '{print $3}' | sort -u)
```

Current result: empty (zero missing C exports).
