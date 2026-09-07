# Configuration surface

Mechanically derived from the two declarations in `staticalias.h`, the
`*outer >= inner` branch in `static_alias`, pointer aliasing caused by its
returned static address, and the `i < iterations` loop in `driver`.

There are no Cargo features and no C preprocessor configuration branches. The
default Cargo configuration and `--no-default-features` therefore represent
the same sole code configuration, and both are exercised in Phase D.

| # | entry point(s) | configuration (options set + input shape) | |
|---|----------------|--------------------------------------------|-|
| 1 | `static_alias` | caller-owned non-null `outer`; `*outer < inner` branch | [x] |
| 2 | `static_alias` | caller-owned non-null `outer`; boundary `*outer == inner` | [x] |
| 3 | `static_alias` | caller-owned non-null `outer`; `*outer > inner` branch | [x] |
| 4 | `static_alias` | pass the previously returned static `inner` pointer back as `outer` (source and destination alias) | [x] |
| 5 | `driver` | `iterations <= 0` (empty operation; includes zero and negative counts) | [x] |
| 6 | `driver` | `iterations == 1`, `initial_value < current inner` | [x] |
| 7 | `driver` | `iterations == 1`, boundary `initial_value == current inner` (fresh-library value is 1) | [x] |
| 8 | `driver` | `iterations == 1`, `initial_value > current inner` | [x] |
| 9 | `driver` | `iterations > 1`, sufficiently negative `initial_value` that every iteration remains on `*outer < inner` | [x] |
| 10 | `driver` | `iterations > 2`, negative `initial_value` that crosses from `*outer < inner` to `*outer >= inner` after multiple lower-branch iterations | [x] |
| 11 | `driver` | `iterations > 1`, `0 <= initial_value < current inner`, so the first call is lower-branch and the second reaches the upper branch | [x] |
| 12 | `driver` | `iterations > 1`, boundary `initial_value == current inner` (fresh-library value is 1) | [x] |
| 13 | `driver` | `iterations > 1`, `initial_value > current inner` | [x] |
