# Dynamic Symbol Surface

Source artifact: `c_src/build/libdriver_c.so`, built from the unchanged
`c_src/src/main.c` with `cc -shared -fPIC`.

Command:

```text
nm -D --defined-only c_src/build/libdriver_c.so
```

| C symbol | Kind | Final Rust status |
|----------|------|-------------------|
| `bad` | `T` | Exported with the exact C ABI and tested through `libloading` |
| `good` | `T` | Exported with the exact C ABI and tested through `libloading` |
| `main` | `T` | Exported with the exact C ABI and tested through `libloading` |
| `printIntPtrLine` | `T` | Exported with the exact C ABI and tested through `libloading` |

Phase D status:

- [x] All four symbols are exported by the Rust shared object.
- [x] The C-to-Rust defined-symbol diff is empty.
- [x] The Rust shared object has no undefined non-libc project symbols.
