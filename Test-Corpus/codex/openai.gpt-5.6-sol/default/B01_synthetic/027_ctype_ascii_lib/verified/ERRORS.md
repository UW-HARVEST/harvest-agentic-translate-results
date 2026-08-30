# Error Surface

Mechanical searches covered `return`, `assert`, `if`, `switch`, `NULL`,
`ERROR`, `MIN`, `MAX`, and enums in `../c_src/include` and `../c_src/src`.
The sole public function returns `void`, accepts a value (not a pointer), and
contains no rejection, assertion, range check, error enum, or error-return
branch.

| # | function | trigger (the exact invalid input/condition) | expected C result |
|---|----------|---------------------------------------------|-------------------|

There are no error-surface rows. Generic pointer, length, and enum boundaries
are not applicable to the `void driver(char)` API. Every bit pattern of its
only argument is covered as a valid `char` input in `CONFIGS.md`.

Completion: [x] all zero rejection rows have differential coverage.
