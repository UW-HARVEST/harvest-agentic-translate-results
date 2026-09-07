# Dynamic Symbol Surface

Generated from:

```text
nm -D --defined-only ../c_src/build/libdriver.so
```

| symbol | C type | C location | Rust export | status |
|---|---|---|---|---|
| `call_fma` | `int call_fma(const int *data, int len)` | `src/driver.c:34` | `src/lib.rs` | present |
| `driver` | `void driver(const char *in)` | `include/driver.h:27`, `src/driver.c:50` | `src/lib.rs` | present |
| `fma_array` | `void fma_array(int *restrict out, const int *mul1, const int *mul2, const int *add, int len)` | `src/driver.c:28` | `src/lib.rs` | present |

## Raw C output

```text
00000000000011c9 T call_fma
00000000000013b4 T driver
0000000000001139 T fma_array
```

## Phase D parity

- [x] Every globally defined dynamic C symbol is exported by the Rust shared object with the exact name.
- [x] Missing C symbols in Rust: 0.
- [x] Undefined non-libc symbols introduced by the Rust translation: 0.

