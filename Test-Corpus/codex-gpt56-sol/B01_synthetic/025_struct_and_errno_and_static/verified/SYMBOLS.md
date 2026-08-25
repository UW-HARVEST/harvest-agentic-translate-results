# Dynamic Symbol Surface

Source: `nm -D --defined-only c_src/build/libdriver_c.so`, built from
`c_src/src/main.c` with `cc -std=c11 -fPIC -shared`.

| symbol | C type | Rust export | parity |
|--------|--------|-------------|--------|
| `main` | `T` | `main` | [x] |
| `run` | `T` | `run` | [x] |

The sorted C-minus-Rust symbol diff is empty. Both libraries export exactly
these two globally defined dynamic symbols. Rust's remaining undefined dynamic
symbols are provided by glibc or the compiler's unwind runtime; it has no
undefined project/library symbols.
