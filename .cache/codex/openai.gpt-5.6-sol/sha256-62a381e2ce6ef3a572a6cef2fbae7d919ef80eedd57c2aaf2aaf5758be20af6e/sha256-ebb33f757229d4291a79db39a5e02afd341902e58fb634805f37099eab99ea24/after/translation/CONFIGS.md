# Configuration Surface

Mechanical inspection found one public entry point and no runtime options,
modes, flags, state, switches, conditionals, feature branches, element types,
formats, byte-order choices, counts, pointers, or variable-sized inputs.

The only input shape is the complete pair of by-value C `int` bit patterns.
The Phase B test for this row must include deterministic boundary pairs and
many fixed-seed randomized pairs, comparing captured stdout byte-for-byte.

| # | entry point(s) | configuration (options set + input shape) | status |
|---|----------------|--------------------------------------------|--------|
| 1 | `driver(int x, int y)` | No options; all pairs in the full signed 32-bit C `int` domain, including zero, ±1, `INT_MIN`, `INT_MAX`, equal/opposite values, and randomized bit patterns | [x] |
