# Dynamic Symbol Surface

Source library: `../c_src/build/libStaticLoop.so`

Mechanical inventory command:

```text
nm -D --defined-only ../c_src/build/libStaticLoop.so
```

Only globally defined function symbols are part of the C API surface; undefined
libc imports are excluded.

| C symbol | C type | Rust export | Status |
|----------|--------|-------------|--------|
| `driver` | `T` | `driver` | present |
| `static_sum` | `T` | `static_sum` | present |

Missing Rust symbols: **0**

## Phase D Verification

- [x] Default release build exports both C API symbols.
- [x] `--no-default-features` release build exports both C API symbols.
- [x] The C-minus-Rust dynamic-symbol diff is empty.
- [x] `ldd -r target/release/libStaticLoop.so` reports no unresolved symbols.
