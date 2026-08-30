# Dynamic Symbol Surface

Source library: `../c_src/build/libdriver.so`

Enumeration command:

```sh
nm -D --defined-only ../c_src/build/libdriver.so
```

Only defined dynamic symbols are API exports. Undefined entries such as
`printf@GLIBC_2.2.5` are shared-library dependencies, not symbols implemented
by this library.

| C symbol | C type | Rust export | Status |
|----------|--------|-------------|--------|
| `driver` | `T` | `driver` | [x] |
| `run` | `T` | `run` | [x] |

Missing C exports in Rust: **0**

Extra Rust API exports: **0**

Undefined non-system symbols required by the C API but absent from Rust: **0**

