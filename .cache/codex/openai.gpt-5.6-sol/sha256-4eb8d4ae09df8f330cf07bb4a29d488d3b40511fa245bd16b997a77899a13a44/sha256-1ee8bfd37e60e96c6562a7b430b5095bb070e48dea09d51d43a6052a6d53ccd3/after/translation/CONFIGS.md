# Configuration Surface

Mechanical search covered the public header and all C implementation branches.
There are no runtime options, modes, flags, conditional branches, compile-time
feature branches, pointer/array shapes, formats, byte-order choices, or
multiple entry points.

| # | entry point(s) | configuration (options set + input shape) | status |
|---|----------------|--------------------------------------------|--------|
| 1 | `driver` | No options; one by-value C `int`. Exercise zero, positive, negative, arithmetic wraparound-producing values, `INT_MIN`, `INT_MAX`, and fixed-seed randomized values across the full 32-bit domain. | [x] |

## Feature Matrix

`Cargo.toml` declares no features, so the only feature combination is the
default/no-feature build.
