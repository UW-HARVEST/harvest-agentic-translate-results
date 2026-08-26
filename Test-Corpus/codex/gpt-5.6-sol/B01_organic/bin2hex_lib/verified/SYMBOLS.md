# Dynamic Symbol Surface

Source library: `c_src/build/libtranslated_rust.so`

Command: `nm -D c_src/build/libtranslated_rust.so`

## Defined public symbols

| C symbol | Type | Rust parity |
|----------|------|-------------|
| `bin2hex` | `T` (global text) | [x] |

## Verification

- C defined public symbols: 1
- C symbols missing from the Rust library: 0
- Unresolved Rust runtime symbols reported by `ldd -r`: 0

## Undefined runtime dependencies

These are dynamic runtime/toolchain dependencies, not library API exports.

| Symbol | Type | Classification |
|--------|------|----------------|
| `_ITM_deregisterTMCloneTable` | `w` | Optional toolchain runtime |
| `_ITM_registerTMCloneTable` | `w` | Optional toolchain runtime |
| `__cxa_finalize@GLIBC_2.2.5` | `w` | libc |
| `__gmon_start__` | `w` | Optional toolchain runtime |
| `abort@GLIBC_2.2.5` | `U` | libc |

## Raw output

```text
                 w _ITM_deregisterTMCloneTable
                 w _ITM_registerTMCloneTable
                 w __cxa_finalize@GLIBC_2.2.5
                 w __gmon_start__
                 U abort@GLIBC_2.2.5
0000000000001109 T bin2hex
```
