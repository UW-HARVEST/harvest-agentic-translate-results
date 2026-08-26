# C Dynamic Symbol Surface

Source: `nm -D --defined-only --format=posix c_src/build/libdriver_c.so`.

The C shared object exports three public symbols. The Rust shared object must
export each exact name.

| # | symbol | kind | C source | Rust parity |
|---|--------|------|----------|-------------|
| 1 | `array` | `B` (global object, 1,048,576 bytes) | `src/main.c:33` | [x] |
| 2 | `main` | `T` (function) | `src/main.c:49` | [x] |
| 3 | `perform_expensive_operations` | `T` (function) | `src/main.c:36` | [x] |

No C implementation is absent from the translation. These symbols were
initially missing because the Rust package only had a binary target and its
corresponding functions and global state were private. `src/lib.rs` now
provides the required C ABI exports. The final `comm` comparison between C and
Rust defined dynamic symbol names is empty.
