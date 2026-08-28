# Dynamic symbol surface

Generated from:

```text
nm -D --defined-only ../c_src/build/libharvest-work-M8dUuU.so
```

Toolchain support symbols and undefined imports are excluded by
`--defined-only`. The C library has one public dynamic symbol.

| C symbol | Type | Rust symbol | Status |
|----------|------|-------------|--------|
| `rgb_to_hsv` | `T` (global function) | `rgb_to_hsv` | [x] present |

## Parity

```text
C-only symbols:    0
Rust-only API symbols: 0
Undefined non-libc symbols required by the C API: 0
```

The Rust comparison command is:

```text
nm -D --defined-only target/release/librgb_to_hsv_lib.so
```
