# Dynamic Symbol Surface

Source library: `c_src/build/libtranslated_rust.so`

Generated from:

```text
nm -D c_src/build/libtranslated_rust.so
```

| C type | symbol | C definition | Rust `nm -D` parity |
|--------|--------|--------------|---------------------|
| `w` | `_ITM_deregisterTMCloneTable` | weak undefined toolchain symbol | [x] present |
| `w` | `_ITM_registerTMCloneTable` | weak undefined toolchain symbol | [x] present |
| `w` | `__cxa_finalize@GLIBC_2.2.5` | weak undefined libc symbol | [x] present |
| `w` | `__gmon_start__` | weak undefined toolchain symbol | [x] present |
| `U` | `fmodf@GLIBC_2.2.5` | undefined libm dependency | [x] present |
| `T` | `hsl_to_rgb` | public API definition | [x] exported |

The public definitions were also extracted with:

```text
nm -D --defined-only --extern-only c_src/build/libtranslated_rust.so
```

| symbol | Rust implementation |
|--------|---------------------|
| `hsl_to_rgb` | `src/lib.rs` |

- [x] Zero C-defined public symbols are missing from the Rust shared library.
- [x] Zero C undefined non-libc/non-toolchain symbols are missing from the Rust shared library.
