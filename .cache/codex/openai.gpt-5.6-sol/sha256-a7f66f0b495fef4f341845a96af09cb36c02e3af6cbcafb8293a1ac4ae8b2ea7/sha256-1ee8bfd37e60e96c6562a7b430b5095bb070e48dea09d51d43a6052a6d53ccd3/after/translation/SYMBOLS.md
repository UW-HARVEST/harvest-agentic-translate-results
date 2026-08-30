# Dynamic Symbol Surface

Source library: `../c_src/build/libdriver.so`

Command: `nm -D --defined-only ../c_src/build/libdriver.so`

| C symbol | C type | Rust symbol | Status |
|----------|--------|-------------|--------|
| `driver` | `T` | `driver` (`T`) | [x] |

The C library has no other dynamically defined public symbols. Its only strong
undefined application dependency is versioned libc `puts`; weak toolchain
symbols are not library API. The Rust library also resolves `puts` from libc.

Completion check: **0 missing C API symbols; 0 undefined non-libc C API
symbols.**
