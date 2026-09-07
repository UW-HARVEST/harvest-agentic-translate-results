# Dynamic symbol surface

Source command:

```text
nm -D ../c_src/build/libdriver.so
```

Complete C dynamic-symbol output, classified mechanically:

| symbol | nm type | classification | Rust parity |
|--------|---------|----------------|-------------|
| `_ITM_deregisterTMCloneTable` | `w` | undefined weak toolchain hook | present |
| `_ITM_registerTMCloneTable` | `w` | undefined weak toolchain hook | present |
| `__cxa_finalize@GLIBC_2.2.5` | `w` | undefined weak libc/toolchain hook | present |
| `__gmon_start__` | `w` | undefined weak toolchain hook | present |
| `strrchr@GLIBC_2.2.5` | `U` | undefined libc dependency | present |
| `tool_basename` | `T` | defined public API export | present |

## Required defined-export parity

| C symbol | Rust symbol | status |
|----------|-------------|--------|
| `tool_basename` | `tool_basename` | [x] |

No C-defined public symbol is currently missing from the Rust shared library.

Verified with an empty diff between `nm -D --defined-only` outputs under both
the default build and `--no-default-features`.
