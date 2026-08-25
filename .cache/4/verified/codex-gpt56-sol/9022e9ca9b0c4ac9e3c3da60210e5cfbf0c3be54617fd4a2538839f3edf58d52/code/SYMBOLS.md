# Dynamic Symbol Surface

Source artifact: `c_src/build/libdriver_c.so`

Extraction command:

```text
nm -D --defined-only c_src/build/libdriver_c.so
```

Only globally defined dynamic symbols are listed. Undefined libc/runtime
imports and ELF toolchain bookkeeping symbols are not public library entry
points.

| C symbol | C source | Rust export | Status |
|----------|----------|-------------|--------|
| `call_fma` | `c_src/src/main.c:32` | `src/lib.rs` | [x] |
| `fma_array` | `c_src/src/main.c:26` | `src/lib.rs` | [x] |
| `main` | `c_src/src/main.c:48` | `src/lib.rs` | [x] |

The Rust export check uses:

```text
nm -D --defined-only target/debug/libdriver.so
```

- [x] Every C-defined dynamic symbol is exported by the Rust shared object.
- [x] The exact-name C-minus-Rust symbol diff is empty.
