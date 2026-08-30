# Error Surface

The C source and public header were mechanically searched for rejection and
error constructs, including `RETURN_ERROR`, `return`, `NULL`, `assert`,
`abort`, `exit`, `if`, `switch`, loops, comparisons, range constants, and
enums.

| # | function | trigger (the exact invalid input/condition) | expected C result |
|---|----------|---------------------------------------------|-------------------|

There are no rejection branches or error-return paths in this library.
Neither exported function accepts pointers, lengths, enum values, modes, or
flags. Both accept one by-value C `int`, return `void`, and treat zero as a
normal valid value. Consequently, the generic null, zero-length, oversized
length, and out-of-range-enum checks are not applicable.

Phase C rows: **0 (complete)**

