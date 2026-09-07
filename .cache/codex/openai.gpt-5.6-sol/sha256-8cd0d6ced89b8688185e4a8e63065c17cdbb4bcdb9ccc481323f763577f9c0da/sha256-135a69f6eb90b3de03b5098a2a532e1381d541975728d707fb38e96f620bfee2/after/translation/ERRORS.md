# Error-surface table

Mechanically derived by searching all C source and headers for returns,
assertions, null checks, explicit range checks, error macros/enums, and
minimum/maximum constants.

The shared-library API has no pointer arguments, lengths, enum arguments,
explicit rejection returns, assertions, or error sentinels. Its only input
type is `int`; `use_generated` deliberately accepts every `int` and sends
values outside `0..=6` through its `default` switch arm.

| # | function | trigger (the exact invalid input/condition) | expected C result |
|---|----------|----------------------------------------------|-------------------|
| 1 | `main` (`driver`) | `argc < 3` | [x] Write `usage: <argv[0]> A B\n` to stderr and return exit status `2`. |
