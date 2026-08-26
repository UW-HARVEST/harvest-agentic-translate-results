# Configuration Surface

## Build-time configurations

`Cargo.toml` declares no `[features]` table, and `c_src/CMakeLists.txt` declares
no options or compile definitions. The complete valid feature combination set
therefore contains one member:

| # | Cargo invocation | CMake configuration | [ ] |
|---|------------------|---------------------|-----|
| B1 | `--no-default-features` (empty feature set) | default | [x] |

## Runtime configurations

The public API has no runtime options, modes, flags, formats, element types,
length arguments, or byte-order settings. The rows below are the pruned
cross-product of the branches actually taken by `smallestValue`: whether the
loop executes and the observed sequence of `value < smallest` outcomes.
Randomized inputs for every row include negative, zero, positive, duplicate,
and `int` boundary values where applicable.

| # | entry point(s) | configuration (options set + input shape) | [ ] |
|---|----------------|-------------------------------------------|-----|
| 1 | `smallestValue` | non-null one-node list; loop executes zero times; any `int` value including `INT_MIN` and `INT_MAX` | [x] |
| 2 | `smallestValue` | non-null list of 2+ nodes; every later value is greater than or equal to the first minimum, covering equality; comparison branch is always false | [x] |
| 3 | `smallestValue` | non-null strictly descending list of 2+ nodes; every comparison branch is true and each node replaces the minimum | [x] |
| 4 | `smallestValue` | non-null list of 3+ nodes with both true and false comparison outcomes in arbitrary order, including duplicate and integer-boundary values | [x] |

The null-list shape is an error configuration and appears in `ERRORS.md`.
