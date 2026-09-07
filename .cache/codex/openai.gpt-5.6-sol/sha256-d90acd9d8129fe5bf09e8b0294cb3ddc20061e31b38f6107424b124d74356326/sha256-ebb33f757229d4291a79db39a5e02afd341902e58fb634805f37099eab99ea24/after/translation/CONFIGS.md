# Configuration Surface

The public header exposes `static_sum(int)` and `driver(int)`. The C source has
no runtime option fields, modes, flags, switches, data formats, pointers,
lengths, compile-time feature branches, or variable element counts.

The mechanically visible behavioral axes are:

- entry point: low-level `static_sum` or composed `driver`;
- process-static accumulator state: zero/fresh or nonzero/prior calls;
- integer data shape: zero or nonzero C `int` values, including signs and
  boundary bit patterns;
- operation shape: one call, a sequence of calls, or repeated composed calls;
- `driver` always performs exactly ten iterations and emits ten decimal lines.

| # | entry point(s) | configuration (options set + input shape) | [ ] |
|---|----------------|--------------------------------------------|-----|
| 1 | `static_sum` | Fresh accumulator; one update spanning zero, positive, negative, `INT_MIN`, and `INT_MAX` values. | [x] |
| 2 | `static_sum` | Nonzero accumulator; randomized many-call sequences spanning zero, both signs, repeated values, and wrapping boundary bit patterns. | [x] |
| 3 | `driver` → `static_sum` | Fresh accumulator; zero stride; fixed ten-iteration stdout operation. | [x] |
| 4 | `driver` → `static_sum` | Fresh accumulator; randomized nonzero stride spanning both signs and integer boundary bit patterns; fixed ten-iteration stdout operation. | [x] |
| 5 | `static_sum` → `driver` | Preloaded nonzero accumulator; zero stride; compare returned preload state and all ten stdout lines. | [x] |
| 6 | `static_sum` → `driver` | Preloaded nonzero accumulator; randomized nonzero stride spanning both signs and integer boundary bit patterns; compare all ten stdout lines. | [x] |
| 7 | `driver` → `driver` | Repeated composed calls without resetting accumulator; randomized strides including zero, both signs, and boundary bit patterns. | [x] |

## Cargo Feature Combinations

`Cargo.toml` declares no features. The sole combination is the default build
(equivalently `--no-default-features`).

- [x] `cargo test --release`
- [x] `cargo test --release --no-default-features`
