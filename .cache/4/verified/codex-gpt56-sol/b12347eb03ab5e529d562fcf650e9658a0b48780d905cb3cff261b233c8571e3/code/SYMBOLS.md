# Dynamic Symbol Surface

Source library: `c_src/build/libdriver.so`

Extraction command:

```sh
nm -D --defined-only c_src/build/libdriver.so
```

Only globally defined dynamic symbols are API exports. Undefined libc,
compiler-runtime, and ELF bookkeeping symbols are imports and are not included.

| C symbol | C type | Rust export | Status |
|----------|--------|-------------|--------|
| `w_utf8_drop` | `T` | `w_utf8_drop` | present |
| `w_utf8_filter` | `T` | `w_utf8_filter` | present |

Missing C exports in Rust: **0**

Undefined non-runtime API symbols in Rust: **0**
