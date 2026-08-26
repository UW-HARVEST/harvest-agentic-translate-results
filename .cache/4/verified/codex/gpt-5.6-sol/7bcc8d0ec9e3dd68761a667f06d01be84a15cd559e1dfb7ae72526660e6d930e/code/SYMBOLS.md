# Dynamic Symbol Surface

Source: `nm -D --defined-only c_src/build/libdriver_c.so`, where the shared
object is built from the complete CMake source list (`mdcore.c`, `mdmain.c`)
with `OP=add`, `REPEAT=5`, and `-fPIC`.

| C symbol | Kind | Rust symbol | Status |
|----------|------|-------------|--------|
| `G_OP` | writable function-pointer object | `G_OP` | [x] |
| `G_OP_NAME` | writable string-pointer object | `G_OP_NAME` | [x] |
| `helper_call` | function | `helper_call` | [x] |
| `helper_ptr` | function | `helper_ptr` | [x] |
| `main` | function | `main` | [x] |
| `op_add` | function | `op_add` | [x] |
| `op_mul` | function | `op_mul` | [x] |
| `op_sub` | function | `op_sub` | [x] |
| `use_generated` | function | `use_generated` | [x] |

The macro-generated `accum_<OP>` function is declared `static` in
`DEFINE_ACCUM`, so it is intentionally absent from the C dynamic symbol table.

Missing C symbols in Rust: **0**
