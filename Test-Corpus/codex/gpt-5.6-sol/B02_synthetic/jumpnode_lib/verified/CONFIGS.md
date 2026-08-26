# Configuration Surface

## Build-time configurations

`Cargo.toml` has no `[features]` table and CMake has no options or conditional
compilation. The complete build matrix is one combination:

| # | Rust feature set | C configuration | checked |
|---|------------------|-----------------|---------|
| B1 | empty (`--no-default-features`) | default | [x] |

`cargo check --no-default-features` passes.

## Runtime configurations

The public header declares only `jumpnode`. There are no pointer, buffer,
element-type, byte-order, or caller-controlled state shapes. Rows below are
the cross-product classes that the implementation actually distinguishes:
operation switch arms, decimal formatting shape, and the low-seven-bit flag
projection.

| # | entry point(s) | configuration (options set + input shape) | tested |
|---|----------------|--------------------------------------------|--------|
| C1 | `jumpnode` | mode `0001`; empty internal node store; randomized `node_id`, `depth`, and `flags` across the full `int` domain | [x] |
| C2 | `jumpnode` | mode `0002`; empty internal node store; randomized `node_id`, `depth`, and `flags` across the full `int` domain | [x] |
| C3 | `jumpnode` | mode `0003`; `node_id` and `depth` jointly cover every representable decimal text length (1-11 characters each, including zero, positive, negative, `INT_MIN`, and `INT_MAX`); low flag bits cover every value `0..=0177` | [x] |
| C4 | `jumpnode` | mode `0003`; flags have only bits outside `0177` set, including positive, negative, `INT_MIN`, and high-bit boundary values; those bits are masked away | [x] |
| C5 | `jumpnode` | mode `0003`; randomized mixed low/high flag bits across the full `int` domain, combined with randomized signed `node_id` and `depth` values | [x] |
| C6 | `jumpnode` | mode `0004`; empty internal node store; randomized `node_id`, `depth`, and `flags` across the full `int` domain | [x] |
| C7 | `jumpnode` | default switch arm at immediate boundaries `operation_mode == 0` and `operation_mode == 5`, with integer-boundary arguments | [x] |
| C8 | `jumpnode` | default switch arm for randomized operation values outside `1..=4`, including negative values, `INT_MIN`, and `INT_MAX` | [x] |

Modes 1, 2, and 4 are valid mode selections, but the only externally
constructible library state takes their explicit not-found branches. Their
exact rejection values are also tracked in `ERRORS.md`.

## Unreachable internal configurations

The source contains branches for mode 1 parent traversal, mode 2 backward
array processing, mode 4 numeric processing, `node_count > 2`, and the
100-node capacity boundary. No public API initializes or mutates the static
node store, so these configurations cannot be produced by an external caller
using the shared-library ABI and cannot be differentially tested without
modifying the ground-truth C source.
