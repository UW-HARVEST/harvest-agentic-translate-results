# Configuration Surface

Derived mechanically from the exported C entry points and every runtime branch
in `c_src/src/driver.c`. There are no runtime modes, flags, formats, element
types, byte-order choices, length/count parameters, preprocessor feature
branches, or Cargo features. The only data-dependent C branch is the null check
for `printLine`; its valid (`line != NULL`) side is row 1 and its rejected side
is tracked in `ERRORS.md`.

| # | entry point(s) | configuration (options set + input shape) | [ ] |
|---|----------------|--------------------------------------------|-----|
| 1 | `printLine` | non-null, NUL-terminated C string; randomized byte contents and lengths without interior NUL | [x] |
| 2 | `bad` | no arguments; direct lowest-level fixed-message entry point | [x] |
| 3 | `good` | no arguments; composed path through `helperGood` | [x] |
| 4 | `driver` | no arguments; full public pipeline through `good`, `helperGood`, and `bad` | [x] |

Feature combinations: the `[features]` table is absent, so the sole semantic
combination is the featureless build (checked both normally and with
`--no-default-features`).
