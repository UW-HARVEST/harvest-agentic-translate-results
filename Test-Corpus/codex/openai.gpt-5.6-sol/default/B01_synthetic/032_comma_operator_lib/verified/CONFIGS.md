# Configuration Surface

The public header exposes one entry point and no runtime options, modes, flags,
formats, element types, byte-order choices, or compile-time feature branches.
The only C branch is the loop condition `i < x`, which distinguishes the input
shapes below.

| # | entry point(s) | configuration (options set + input shape) | [ ] |
|---|----------------|--------------------------------------------|-----|
| 1 | `driver` | no options; `x < 0`, empty output | [x] |
| 2 | `driver` | no options; `x == 0`, empty output | [x] |
| 3 | `driver` | no options; `x == 1`, one output line | [x] |
| 4 | `driver` | no options; `x > 1`, many output lines | [x] |

## Feature combinations

`Cargo.toml` declares no features. The sole build configuration is the default
empty feature set.
