# Configuration Surface

## Build-time configurations

`Cargo.toml` has no `[features]` table and CMake defines no options or
conditional compilation. The complete valid feature set is therefore:

| # | Cargo invocation feature set | matching C configuration |
|---|------------------------------|--------------------------|
| 1 | `--no-default-features` (no features enabled) | default CMake build |

## Runtime and input configurations

The public headers expose only `void driver(float x)`. There are no runtime
options, modes, flags, state, element counts, formats, or byte-order controls.
The C code does not branch on the float value: it prints all `sizeof(float)`
object-representation bytes in native memory order. The row therefore covers
the complete `u32` bit-pattern domain, including positive/negative zero,
normal and subnormal finite values, infinities, and all NaN payloads.

| # | entry point(s) | configuration (options set + input shape) | [ ] |
|---|----------------|--------------------------------------------|-----|
| 1 | `driver` | no options; every 32-bit float object representation, emitted as four lowercase hexadecimal bytes plus newline in native byte order | [x] |

Public entry points covered: **1 of 1**.
