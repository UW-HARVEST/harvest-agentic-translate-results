# Error Surface

The following source patterns were searched exhaustively in `c_src/include/`
and `c_src/src/`: error-return macros, `return -1`, `return NULL`, assertions,
conditionals, switches, enum declarations, null checks, and min/max constants.
The only preprocessor conditional is the header include guard.

| # | function | trigger (the exact invalid input/condition) | expected C result |
|---|----------|---------------------------------------------|-------------------|

There are no rejection or error paths. `driver` returns `void`, accepts one
`int`, and has no pointer, length, enum, option, or documented range parameter.
Zero and negative values are accepted and produce no output; that boundary is
covered as valid behavior in `CONFIGS.md`.
