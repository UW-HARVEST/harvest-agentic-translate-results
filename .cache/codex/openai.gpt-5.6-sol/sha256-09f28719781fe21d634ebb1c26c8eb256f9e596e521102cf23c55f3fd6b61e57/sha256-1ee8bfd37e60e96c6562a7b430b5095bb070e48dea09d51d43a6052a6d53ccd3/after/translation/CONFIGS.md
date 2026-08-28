# Configuration Surface

The public header declares one entry point, `to_barycentric`. The C source has
no option, mode, flag, type, format, byte-order, count, `if`, `switch`, or
preprocessor branches. All four inputs have the single fixed shape `lm_vec2`
(two `float` fields, passed by value).

The rows below partition the floating-point arithmetic surface exposed by the
C expression itself. There are no runtime or Cargo feature axes to cross with
these shapes.

| # | entry point(s) | configuration (options set + input shape) | status |
|---|----------------|-------------------------------------------|--------|
| 1 | `to_barycentric` | no options; finite normal coordinates, nonzero determinant; randomized interior, edge, vertex, and exterior points | [x] |
| 2 | `to_barycentric` | no options; finite degenerate or near-degenerate triangle (coincident/collinear vertices, zero or tiny determinant) | [x] |
| 3 | `to_barycentric` | no options; finite IEEE-754 boundaries (positive/negative zero, subnormal, minimum normal, and maximum finite coordinates) | [x] |
| 4 | `to_barycentric` | no options; non-finite coordinates and varied NaN payloads/signs (positive/negative infinity, quiet NaN, signaling NaN) | [x] |

Feature combinations from `Cargo.toml`: one, because `[features]` is absent.
The required invocation is `cargo test --no-default-features`.
