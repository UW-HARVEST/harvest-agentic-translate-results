# Configuration Surface

Mechanically derived from the complete dynamic export set, the public header,
and every `if` branch in `../c_src/src/driver.c`. There are no compile-time
feature branches, runtime mode objects, lengths, element types, byte-order
options, formats, or binaries. The only caller-controlled branch axes are
`printLine` nullness and `driver` integer truthiness.

The null `printLine` case is invalid/rejection-like and is tracked in
`ERRORS.md`; all valid configurations and every exported entry point are below.

| # | entry point(s) | configuration (options set + input shape) | verified |
|---|----------------|--------------------------------------------|----------|
| 1 | `printLine` | Non-null, NUL-terminated byte string; randomized content and lengths, including empty, one-byte, and many-byte strings. | [x] |
| 2 | `good` | No arguments; fixed internal non-null string `"string"`. | [x] |
| 3 | `bad` | No arguments; passes its uninitialized automatic `char *data` to `printLine`. Compare the externally observed process status and stdout of the built libraries. | [x] |
| 4 | `driver` | `useGood == 0`; dispatches to `bad()`. Compare the externally observed process status and stdout of the built libraries. | [x] |
| 5 | `driver` | `useGood != 0`; randomized positive and negative nonzero `int` values, including `INT_MIN` and `INT_MAX`; dispatches to `good()`. | [x] |

Feature combinations from `Cargo.toml`: one combination only (no features are
declared), represented by the default/no-feature build.
