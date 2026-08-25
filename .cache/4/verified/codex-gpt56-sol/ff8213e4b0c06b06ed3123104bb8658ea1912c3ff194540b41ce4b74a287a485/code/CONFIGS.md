# Configuration Surface

`Cargo.toml` originally declares no `[features]` table, and
`c_src/CMakeLists.txt` declares no options or conditional compilation. The only
valid Rust feature combination is therefore the empty set:

```text
cargo check --no-default-features
cargo test --no-default-features
```

The rows below come from the two exported entry points, the process-global
`the_house` state, the four unconditional print/mutation stages in `run`, the
two unconditional calls to `run` in `main`, and the `%d` conversion in `main`.

| # | entry point(s) | configuration (options set + input shape) | covered |
|---|----------------|--------------------------------------------|---------|
| 1 | `run` | Fresh library state; one call; randomized `int` across negative, zero, positive, and boundary values | [x] |
| 2 | `run` | Existing library state; randomized sequence of two or more calls, exercising accumulated floors, bathrooms, and bedrooms | [x] |
| 3 | `main` -> `run` | `%d` conversion succeeds after optional C whitespace and optional sign; randomized negative, zero, and positive values; `run` executes twice | [x] |
| 4 | `main` -> `run` | `%d` consumes a valid decimal prefix and ignores trailing nonnumeric input; `run` executes twice with the converted prefix | [x] |
| 5 | `main` -> `run` | `%d` conversion fails on EOF or a nonnumeric leading byte; initialized `x = 0` is retained and `run` executes twice | [x] |

All output comparisons are byte-for-byte and include every intermediate line.
