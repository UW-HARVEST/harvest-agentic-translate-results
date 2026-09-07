# Configuration surface

Mechanical enumeration covered the public header and every `if`, `switch`,
`#ifdef`, and loop in the C implementation.

The library has one public entry point, no runtime options/modes/flags, no
public state, no element formats, no variable-size inputs, and no Cargo
features. The only source-level iteration is the unconditional loop over all
`sizeof(int)` bytes in native object-representation order.

| # | entry point(s) | configuration (options set + input shape) | [ ] |
|---|----------------|--------------------------------------------|-----|
| 1 | `driver(int)` | No options; every representable signed C `int` value, emitted as exactly `sizeof(int)` bytes in native memory order, including zero, positive, negative, `INT_MIN`, `INT_MAX`, and randomized full-width bit patterns | [x] |

## Build combinations

Cargo.toml declares no features. The effective feature surface is therefore:

- default feature set (empty)
- `--no-default-features` (also empty; verified separately as the required
  feature-switch invocation)
