# Error Surface

The complete C source contains no rejection or error paths: no error-return
macro or statement, `assert`, range check, null check, enum validation, or
minimum/maximum constant. The public API accepts two scalar C `int` values, so
pointer and length boundary cases do not apply.

| # | function | trigger (the exact invalid input/condition) | expected C result |
|---|----------|---------------------------------------------|-------------------|

Total distinct C rejection paths: **0**.
