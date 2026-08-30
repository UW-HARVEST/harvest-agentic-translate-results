# Configuration Surface

Mechanical inspection found one public entry point and no runtime options,
modes, flags, element types, sizes, formats, byte-order choices, compile-time
features, or conditional data-shape branches.

| # | entry point(s) | configuration (options set + input shape) | status |
|---|----------------|--------------------------------------------|--------|
| 1 | `driver(int)` | No options; scalar C `int` across negative, zero, positive, arithmetic-boundary, and full-domain randomized values; output is the exact bytes written to stdout. | [x] |
