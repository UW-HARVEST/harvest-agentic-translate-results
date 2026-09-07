# Error surface

Mechanically inspected `../c_src/include/driver.h` and
`../c_src/src/driver.c` for error returns, null checks, assertions, explicit
range checks, error enums, and min/max constants.

| # | function | trigger (the exact invalid input/condition) | expected C result |
|---|----------|----------------------------------------------|-------------------|

There are **0 rejection paths**. The sole public function returns `void` and
accepts every bit pattern representable by C `int`. It has no pointer, length,
enum, option, allocation, or fallible-operation inputs. Consequently, generic
null-pointer, zero/oversized-length, and invalid-enum error cases are not
applicable to this API. Integer boundaries remain valid inputs and are covered
by `CONFIGS.md`.

