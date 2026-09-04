# Dynamic symbol surface

Derived with:

```text
nm -D --defined-only ../c_src/build/libharvest-work-kocQYC.so
nm -D --defined-only target/release/libcheckshift_lib.so
```

The C shared object has ten defined public symbols. The Rust shared object
exports every one with the exact same spelling.

| # | C symbol | Rust export | Status |
|---|----------|-------------|--------|
| 1 | `add_with_static` | `add_with_static` | [x] |
| 2 | `apply_operation` | `apply_operation` | [x] |
| 3 | `checkshift` | `checkshift` | [x] |
| 4 | `compute_checksum` | `compute_checksum` | [x] |
| 5 | `execute_operation` | `execute_operation` | [x] |
| 6 | `get_operation` | `get_operation` | [x] |
| 7 | `init_state` | `init_state` | [x] |
| 8 | `multiply_with_static` | `multiply_with_static` | [x] |
| 9 | `shift_with_static` | `shift_with_static` | [x] |
| 10 | `xor_operation` | `xor_operation` | [x] |

Missing C exports in Rust: **0**.

The C object's undefined dynamic symbols are only libc/toolchain imports:
`free`, `malloc`, `memcpy`, `printf`, `puts`, and weak ELF runtime hooks.
There are no undefined project-library symbols that Rust must implement.
