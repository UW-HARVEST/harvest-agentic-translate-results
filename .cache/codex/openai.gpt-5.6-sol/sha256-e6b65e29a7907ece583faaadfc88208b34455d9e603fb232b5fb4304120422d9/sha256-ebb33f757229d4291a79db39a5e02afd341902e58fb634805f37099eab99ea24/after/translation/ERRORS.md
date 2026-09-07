# Error Surface

The C source and public header were mechanically searched for error returns,
error macros, assertions, null checks, range checks, min/max constants, enums,
and conditional rejection branches. None exist. The sole public function
returns `void` and accepts one by-value `int`.

| # | function | trigger (the exact invalid input/condition) | expected C result | status |
|---|----------|----------------------------------------------|-------------------|--------|

There are no error-surface rows. Generic pointer, length, and enum boundary
cases are not applicable because the API contains no pointers, lengths, or
enums. Every `int` bit pattern is accepted by C.

Phase C status: **[x] complete (zero applicable rejection rows)**.
