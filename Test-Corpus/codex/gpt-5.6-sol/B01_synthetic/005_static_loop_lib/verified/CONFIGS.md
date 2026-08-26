# Configuration Surface

## Build-time configurations

`Cargo.toml` has no `[features]` table and `c_src/CMakeLists.txt` defines no
options or conditional sources. The complete build-time configuration set is:

| # | Cargo invocation | CMake configuration |
|---|------------------|---------------------|
| 1 | `--no-default-features` (empty feature set) | default |

## Runtime configurations

The public header exposes exactly `static_sum(int)` and `driver(int)`. There
are no runtime options, modes, flags, element types, formats, byte-order
choices, pointers, or variable-length inputs. The meaningful distinctions
come from the persistent static accumulator and `driver`'s fixed ten-element
loop.

| # | entry point(s) | configuration (options set + input shape) | [ ] |
|---|----------------|--------------------------------------------|-----|
| 1 | `static_sum` | First call from zero state; `update` sampled across negative, zero, positive, `INT_MIN`, and `INT_MAX` values | [x] |
| 2 | `static_sum` | Repeated calls with persistent nonzero state; randomized mixed-sign `int` updates, including wrapping machine-integer boundaries | [x] |
| 3 | `driver`, `static_sum` | Composed ten-iteration operation from arbitrary accumulated state; randomized negative, zero, positive, `INT_MIN`, and `INT_MAX` strides; compare all printed bytes and resulting state | [x] |

All rows are exercised by `tests/differential.rs` with fixed seed
`0x5a17c9e3d42b806f`.
