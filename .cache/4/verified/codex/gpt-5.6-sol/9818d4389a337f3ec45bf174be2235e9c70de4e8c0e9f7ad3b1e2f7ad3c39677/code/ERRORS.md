# Error surface

Mechanically scanned source scope: `c_src/src/**/*.{c,h}`.

Patterns checked include error returns, `RETURN_ERROR`, `assert`, `if`,
`switch`, preprocessor conditionals, null checks, range comparisons, enums,
and min/max constants. The source has no explicit rejection, error return,
assertion, range check, null check, pointer/length parameter, or enum input.

| # | function | trigger (the exact invalid input/condition) | expected C result | [ ] |
|---|----------|---------------------------------------------|-------------------|-----|

Generic FFI boundaries are not applicable: both exported entry points take
either no arguments or one by-value C `int`; there are no pointers, lengths,
or enum parameters. Zero and the full C `int` boundary are covered by the
valid-path `driver` row in `CONFIGS.md`. `main`'s ignored `scanf` matching and
input failures are valid observable behaviors and are covered there as well.

Phase C completion: **[x]** (zero rejection rows; generic boundaries covered
or inapplicable as described above).
