# Error Surface

Mechanical source scan covered `RETURN_ERROR`, negative/NULL returns, error
enums, `assert`, all `if` conditions, `NULL`, and `CHAR_MIN`/`CHAR_MAX`.

| # | function | trigger (the exact invalid input/condition) | expected C result | status |
|---|----------|----------------------------------------------|-------------------|--------|
| 1 | `printLine` | `line == NULL` | Returns `void` without calling `printf`; emits zero bytes and does not crash. | [x] |
| 2 | `goodB2G` (static; reached through `good` and `driver(useGood != 0)`) | `data >= CHAR_MAX / 2` after `data > 0`; concretely `data == CHAR_MAX` | Rejects the multiplication and emits `data value is too large to perform arithmetic safely.\n`. | [x] |

There are no error-return macros/statements, assertions, error enums, length
arguments, or externally supplied enum values in this C source.

The remaining `data > 0` guards are not rejection rows: their local values are
unconditionally assigned to positive constants (`CHAR_MAX` or `2`), so their
false branches are unreachable through every public entry point.
