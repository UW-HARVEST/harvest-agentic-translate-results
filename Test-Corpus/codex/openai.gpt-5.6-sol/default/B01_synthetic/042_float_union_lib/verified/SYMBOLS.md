# Dynamic symbol surface

Generated from:

```text
nm -D --defined-only ../c_src/build/libdriver.so
```

| C symbol | C type | Rust export | Status |
|----------|--------|-------------|--------|
| `driver` | `T` | `driver` | present |

The C library has no other defined dynamic symbols. Its only strong undefined
dependency is the libc function `printf`; the remaining undefined entries are
weak toolchain/runtime hooks. The Rust library also resolves `printf` from
libc.

Missing C symbols in Rust: **0**.

Final verification:

- The sorted `nm -D --defined-only` C-minus-Rust symbol diff is empty.
- `ldd -r target/release/libdriver.so` reports no unresolved relocations.
