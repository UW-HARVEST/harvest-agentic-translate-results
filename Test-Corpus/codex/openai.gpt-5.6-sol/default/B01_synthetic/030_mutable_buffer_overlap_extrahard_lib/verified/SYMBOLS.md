# Dynamic symbol surface

Source library: `../c_src/build/libdriver.so`

Command used:

```text
nm -D --defined-only ../c_src/build/libdriver.so
```

The C shared object has exactly two globally defined dynamic symbols. Both are
also present with the exact same name in `target/release/libdriver.so`.

| # | C symbol | C type | Rust symbol | Status |
|---|----------|--------|-------------|--------|
| 1 | `driver` | `T` | `driver` | [x] |
| 2 | `fma_array` | `T` | `fma_array` | [x] |

Undefined symbols in the C shared object are `memcpy` and `printf` plus normal
weak ELF runtime symbols; these are libc/toolchain dependencies rather than
library API symbols.

Final Phase D verification: sorted `nm -D --defined-only` symbol names have an
empty `comm -3` diff, and `ldd -r` reports no unresolved relocations for either
shared object.
