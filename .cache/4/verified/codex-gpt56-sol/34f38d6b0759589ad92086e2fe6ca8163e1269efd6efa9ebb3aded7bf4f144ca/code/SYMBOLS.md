# Dynamic Symbol Surface

Ground truth: `nm -D --defined-only c_src/build/libdriver_c.so`, built from
`c_src/src/main.c` with `cc -shared -fPIC`.

| symbol | C type | C source | Rust status |
|--------|--------|----------|-------------|
| `driver` | `void driver(float)` | `c_src/src/main.c:33` | [x] Exported by `src/lib.rs`. |
| `main` | `int main()` | `c_src/src/main.c:37` | [x] Exported by `src/lib.rs`. |

Completion check:

- [x] `nm -D` reports no C-defined dynamic symbol missing from the Rust shared object.
- [x] The Rust shared object has no undefined non-libc project symbols.
