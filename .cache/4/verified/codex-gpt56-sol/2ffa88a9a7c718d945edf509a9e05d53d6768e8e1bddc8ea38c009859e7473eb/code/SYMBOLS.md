# Dynamic Symbol Surface

Source: `nm -D --defined-only c_src/build/libdriver_c.so`, where the shared
object is linked from CMake's position-independent `main.c.o`.

| C symbol | C definition | Resolution | Rust export |
|----------|--------------|------------|-------------|
| `driver` | `c_src/src/main.c:32` | Added the complete C ABI operation in `src/lib.rs` | [x] |
| `fma_array` | `c_src/src/main.c:26` | Added the complete C ABI operation in `src/lib.rs` | [x] |
| `main` | `c_src/src/main.c:39` | Added the complete libc-backed entry point in `src/lib.rs` | [x] |

All three inventory-time omissions were export-completeness failures, not
untranslated modules. `comm` over the two `nm -D --defined-only` symbol lists
now reports zero missing C symbols.
