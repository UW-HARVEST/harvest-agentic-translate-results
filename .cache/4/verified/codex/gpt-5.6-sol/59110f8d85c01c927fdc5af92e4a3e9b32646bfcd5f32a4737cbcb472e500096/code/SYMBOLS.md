# Dynamic Symbol Surface

Source library: `c_src/build/liblong.so`

Command:

```text
nm -D --defined-only c_src/build/liblong.so
```

| C symbol | Kind | Rust export | Status |
|----------|------|-------------|--------|
| `array` | `B` (global data) | `array` | [x] |
| `long_exec` | `T` (function) | `long_exec` | [x] |
| `perform_expensive_operations` | `T` (function) | `perform_expensive_operations` | [x] |

The exact-name defined-symbol diff between the C and Rust shared libraries is
empty. The C library's remaining dynamic symbols are undefined libc/toolchain
imports, not exports from this library.

Phase D symbol status: [x] complete (zero missing C exports).
