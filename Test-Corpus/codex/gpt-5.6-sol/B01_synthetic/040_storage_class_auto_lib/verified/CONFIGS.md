# Configuration-Surface Table

Mechanical inspection covered the public header and every branch-like
construct in `c_src/src/driver.c`. There are no runtime options, flags,
conditionals, switches, preprocessor configurations, data buffers, formats,
element types, or alternate entry points. The complete API is
`void driver(int x)`.

`driver` applies the same straight-line operation to every representable C
`int`, including zero, sign boundaries, and values whose compiled GNU C
arithmetic wraps. Those values form one configuration because the C source
does not branch on them.

| # | entry point(s) | configuration (options set + input shape) | status |
|---|----------------|--------------------------------------------|--------|
| 1 | `driver` | no options; one by-value `c_int`, full representable domain | [x] |
