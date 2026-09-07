# Configuration Surface

Mechanical scans of the public header and implementation found one public
entry point and no runtime options, flags, modes, input values, input shapes,
conditional branches, switches, or compile-time feature branches.

| # | entry point(s) | configuration (options set + input shape) | verified |
|---|----------------|--------------------------------------------|----------|
| 1 | `helloworld` | No options and no input; call the zero-argument function, capture stdout bytes, and record the integer return value. | [x] |

Verified by `tests/differential.rs` across 128 fixed-seed randomized invocation
counts under both the default and `--no-default-features` configurations.
