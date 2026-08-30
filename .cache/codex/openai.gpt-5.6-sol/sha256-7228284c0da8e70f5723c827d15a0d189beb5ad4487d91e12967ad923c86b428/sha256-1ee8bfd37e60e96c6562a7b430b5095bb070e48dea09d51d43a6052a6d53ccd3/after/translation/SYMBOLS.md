# Dynamic Symbol Surface

Generated from:

```text
nm -D ../c_src/build/libdriver.so
```

## Defined public symbols

| C symbol | Type | Rust symbol | Status |
|----------|------|-------------|--------|
| `driver` | `T` | `driver` (`T`) | present |

The defined-symbol parity command is:

```text
comm -23 \
  <(nm -D --defined-only ../c_src/build/libdriver.so | awk '{print $3}' | sort -u) \
  <(nm -D --defined-only target/release/libdriver.so | awk '{print $3}' | sort -u)
```

Its output is empty.

## Undefined runtime dependencies

These are the remaining entries from the complete C `nm -D` output. They are
imports, not symbols implemented or exported by this library.

| C symbol | Type | Provider |
|----------|------|----------|
| `_ITM_deregisterTMCloneTable` | `w` | compiler runtime, optional weak import |
| `_ITM_registerTMCloneTable` | `w` | compiler runtime, optional weak import |
| `__cxa_finalize@GLIBC_2.2.5` | `w` | libc, optional weak import |
| `__gmon_start__` | `w` | profiling runtime, optional weak import |
| `div@GLIBC_2.2.5` | `U` | libc |
| `printf@GLIBC_2.2.5` | `U` | libc |

