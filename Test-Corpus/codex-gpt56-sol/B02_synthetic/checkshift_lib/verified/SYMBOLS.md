# Dynamic Symbol Surface

Generated from:

```text
nm -D --defined-only c_src/build/libtranslated_rust.so
nm -D --defined-only target/release/libcheckshift_lib.so
```

The C shared object defines 10 public dynamic symbols. The Rust shared object
defines every one under the exact same name.

| # | C symbol | C type | Rust type | Status |
|---|----------|--------|-----------|--------|
| 1 | `add_with_static` | `T` | `T` | present |
| 2 | `apply_operation` | `T` | `T` | present |
| 3 | `checkshift` | `T` | `T` | present |
| 4 | `compute_checksum` | `T` | `T` | present |
| 5 | `execute_operation` | `T` | `T` | present |
| 6 | `get_operation` | `T` | `T` | present |
| 7 | `init_state` | `T` | `T` | present |
| 8 | `multiply_with_static` | `T` | `T` | present |
| 9 | `shift_with_static` | `T` | `T` | present |
| 10 | `xor_operation` | `T` | `T` | present |

## Completion

- [x] Final symbol diff is empty under every feature combination.
- [x] Rust has no undefined non-libc symbol originating from the C library.

