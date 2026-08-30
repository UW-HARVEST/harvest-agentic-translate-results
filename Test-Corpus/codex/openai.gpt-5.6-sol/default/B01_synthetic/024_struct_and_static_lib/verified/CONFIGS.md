# Configuration Surface

Mechanical source inspection found:

- Dynamic public entry points: `run(int)` and `driver(int)`.
- Runtime modes, options, and flags: none.
- Conditional branches, switches, or feature `#ifdef` paths: none.
- Input shape: one by-value C `int`; negative, zero, and positive values all
  follow the same path and are included in each randomized row.
- Stateful axis: fresh initial state versus state accumulated by prior calls.
- Call hierarchy: `driver(x)` invokes `run(x)` twice.
- Cargo feature combinations: default only; `Cargo.toml` declares no features.

The cross-product below contains every combination the implementation treats
differently. Values are constrained to executions where C signed arithmetic is
defined.

| # | entry point(s) | configuration (options set + input shape) | [ ] |
|---|----------------|--------------------------------------------|-----|
| 1 | `run` | Fresh library state; randomized negative, zero, and positive `int` arguments | [x] |
| 2 | `run` | Accumulated state after one or more prior calls; randomized negative, zero, and positive `int` arguments | [x] |
| 3 | `driver` | Fresh library state; randomized negative, zero, and positive `int` arguments; exercises both nested `run` calls | [x] |
| 4 | `driver` | Accumulated state after prior `run`/`driver` calls; randomized negative, zero, and positive `int` arguments | [x] |
