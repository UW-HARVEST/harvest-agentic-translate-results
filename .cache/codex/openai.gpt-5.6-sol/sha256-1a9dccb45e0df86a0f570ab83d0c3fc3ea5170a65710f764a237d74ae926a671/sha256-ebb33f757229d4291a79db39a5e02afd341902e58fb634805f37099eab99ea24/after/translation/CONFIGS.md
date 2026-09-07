# Configuration Surface

The C source has no runtime flags, modes, options, conditional branches,
switches, preprocessor feature branches, element types, formats, byte-order
choices, pointer inputs, or variable-length buffers. Its observable axes are
the selected exported entry point, the signed `int` value, and accumulated
process-global house state.

| # | entry point(s) | configuration (options set + input shape) | status |
|---|----------------|--------------------------------------------|-----|
| 1 | `run` | Fresh library state; one direct low-level call; randomized negative, zero, and positive `int` values, including arithmetic boundaries | [x] |
| 2 | `run` | Fresh library state; many direct low-level calls in sequence; randomized signed values exercise accumulated floors, bathrooms, and bedrooms | [x] |
| 3 | `driver` | Fresh library state; one wrapper call (exactly two internal `run` calls); randomized negative, zero, and positive `int` values, including arithmetic boundaries | [x] |
| 4 | `driver` | Fresh library state; many wrapper calls in sequence; randomized signed values exercise accumulated state across repeated composed operations | [x] |
| 5 | `run`, `driver` | Fresh library state; mixed low-level and wrapper calls in both orders with randomized signed values, exercising shared state across entry points | [x] |

There are no Cargo features in `Cargo.toml`, so the default build is the only
feature combination.

Each row is covered by the correspondingly numbered test in
`tests/differential.rs`, using a fixed-seed generator and loading both shared
objects through `libloading`.
