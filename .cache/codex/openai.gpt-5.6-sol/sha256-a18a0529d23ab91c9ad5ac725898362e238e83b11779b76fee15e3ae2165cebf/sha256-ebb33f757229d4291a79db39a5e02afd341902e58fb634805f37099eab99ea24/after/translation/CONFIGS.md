# Configuration Surface

Mechanically derived from all exported entry points, conditionals, switches,
preprocessor branches, options, flags, and input shapes in
`c_src/include/driver.h` and `c_src/src/driver.c`.

The C implementation has no runtime options, modes, flags, feature branches,
element types, byte-order choices, lengths, or counts. Its only data-dependent
branch is the null check in `printLine`; the null case is tracked in
`ERRORS.md`.

| # | entry point(s) | configuration (options set + input shape) | Verified |
|---|----------------|--------------------------------------------|----------|
| 1 | `printLine` | Non-null, NUL-terminated C string; randomized empty and non-empty byte strings without interior NUL bytes | [x] |
| 2 | `printIntLine` | Any C `int`; randomized values including `INT_MIN`, negatives, zero, positives, and `INT_MAX` | [x] |
| 3 | `bad` | No arguments; complete operation | [x] |
| 4 | `good` | No arguments; complete operation | [x] |
| 5 | `driver` | No arguments; complete composed operation through `printLine`, `good`, and `bad` | [x] |

Cargo feature combinations: default only (no `[features]` table is present).
