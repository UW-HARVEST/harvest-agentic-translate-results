# Configuration-surface table

Mechanical scans covered the public header and all C implementation branches.
There are no runtime options, flags, modes, `if`/`switch` statements, feature
conditionals, or multiple entry points. `driver` accepts every 64-bit IEEE-754
bit pattern by value. The single row therefore spans the complete input
domain, with explicit coverage of signed zero, subnormal, normal, infinity,
and NaN encodings plus fixed-seed randomized bit patterns.

| # | entry point(s) | configuration (options set + input shape) | [ ] |
|---|----------------|--------------------------------------------|-----|
| 1 | `driver(double)` | No options; all binary64 bit patterns, including both signs and all IEEE-754 classes | [x] |

The row passed with 20 explicit edge encodings and 32,768 fixed-seed
randomized encodings under both the default and `--no-default-features`
invocations. `Cargo.toml` defines no feature table, so there are no additional
feature combinations.
