# Dynamic Symbol Surface

Source artifact: `c_src/build/libdriver_c.so`

Command:

```text
nm -D --defined-only c_src/build/libdriver_c.so
```

## C-defined public symbols

| symbol | type | Rust parity |
|--------|------|-------------|
| `main` | `T`  | [x] |

## C external imports

The complete non-weak imports reported by `nm -D --undefined-only` are libc or
libm symbols: `__errno_location`, `fprintf`, `pow`, `printf`, `stderr`, and
`strtod`. The weak toolchain imports are `_ITM_deregisterTMCloneTable`,
`_ITM_registerTMCloneTable`, `__cxa_finalize`, and `__gmon_start__`. There are
no undefined project symbols.

## Completion checks

- [x] Every C-defined public symbol is defined by the Rust shared object.
- [x] Rust has zero undefined non-libc/non-libm project symbols.
