# Configuration Surface

The public headers expose two entry points and no options, modes, flags,
pointers, lengths, enums, element types, formats, or compile-time feature
branches. The implementation's meaningful call/input shapes are the persistent
state update, the fixed ten-iteration driver, and their shared-state
composition.

| # | entry point(s) | configuration (options set + input shape) | [ ] |
|---|----------------|--------------------------------------------|-----|
| 1 | `static_sum` | Direct one/many state updates across the full by-value C `int` domain, including zero, positive, negative, `INT_MIN`, and `INT_MAX` | [x] |
| 2 | `driver` | Direct calls with randomized full-domain C `int` strides; capture all ten printed result lines and probe the resulting persistent sum | [x] |
| 3 | `static_sum`, `driver` | Randomized interleaving of low-level updates and composed driver calls against their shared persistent sum state | [x] |

Cargo declares no features. Therefore the sole feature configuration is the
default/empty feature set, which is also exercised explicitly with
`--no-default-features`.
