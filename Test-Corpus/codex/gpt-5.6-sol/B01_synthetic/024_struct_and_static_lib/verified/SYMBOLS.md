# Dynamic Symbol Surface

Generated from:

```text
nm -D --defined-only c_src/build/libdriver.so
```

| symbol | C type | Rust type | parity |
|--------|--------|-----------|--------|
| `driver` | `T` | `T` | [x] |
| `run` | `T` | `T` | [x] |

`run` is not declared in `include/driver.h`, but it is externally visible in
the C shared object and is therefore part of the dynamic symbol surface.

The C shared object's complete undefined-symbol set is the libc symbol
`printf@GLIBC_2.2.5`. Its remaining dynamic entries are weak toolchain
hooks (`_ITM_*`, `__cxa_finalize`, and `__gmon_start__`), not library exports.

Defined-symbol diff:

```text
(empty)
```

- [x] The Rust shared object is missing zero C-defined dynamic symbols.
- [x] The C shared object has zero undefined non-libc library symbols.
