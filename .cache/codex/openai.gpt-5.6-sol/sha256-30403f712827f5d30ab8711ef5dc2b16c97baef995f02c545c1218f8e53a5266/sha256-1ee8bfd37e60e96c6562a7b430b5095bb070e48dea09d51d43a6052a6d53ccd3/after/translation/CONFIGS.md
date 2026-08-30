# Configuration Surface

Mechanically derived from the public dynamic definitions and all `if`/loop
branches in `../c_src/src/driver.c`. There are no runtime options, modes,
flags, element formats, byte-order branches, Cargo features, or C preprocessor
feature branches. Static `goodG2B` is covered through `good` and `driver`;
static `goodB2G` is covered across both sides of its range check.

| # | entry point(s) | configuration (options set + input shape) | status |
|---|----------------|-------------------------------------------|--------|
| C1 | `printLine` | non-null NUL-terminated byte string, including empty and non-empty strings | [x] |
| C2 | `printIntLine` | arbitrary C `int`, including zero, signs, and `INT_MIN`/`INT_MAX` | [x] |
| C3 | `bad` | `data` in `0..=9`; index zero, interior, and index nine | [x] |
| C4 | `good` | `data` in `0..=9`; fixed `goodG2B` plus valid `goodB2G` index | [x] |
| C5 | `driver` | `goodData` in `0..=9` crossed with `badData` in `0..=9` | [x] |

Every row is exercised repeatedly with a fixed-seed generator through both
shared-library FFI boundaries.
