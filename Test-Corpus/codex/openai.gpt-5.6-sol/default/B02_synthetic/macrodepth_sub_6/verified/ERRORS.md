# Error Surface

The table was derived by grepping all files under `c_src/src/` for returns,
assertions, null checks, range checks, conditionals, switch statements, and
min/max constants.

| # | function | trigger (the exact invalid input/condition) | expected C result | status |
|---|----------|----------------------------------------------|-------------------|--------|
| 1 | `main` | `argc < 3` | writes `usage: %s A B\n` to `stderr`, using `argv[0]` for `%s`, and returns `2` | [x] |

Audit notes:

- `mdcore.c` contains no rejection/error branches, assertions, null checks, or
  range checks.
- `use_generated` does not reject out-of-range `n`: values outside `0..=6`
  take the `default` switch branch and return the selected operation's initial
  accumulator. Those are valid configurations covered in `CONFIGS.md`.
- The C source defines no public enum inputs and no length-taking APIs.
- Generic FFI pointer boundaries for `main` are tested separately: null
  `argv`, null argument entries, zero `argc`, and oversized/extra `argc`.
- Row 1 and all generic boundaries pass for every one of the 24 valid feature
  combinations.
