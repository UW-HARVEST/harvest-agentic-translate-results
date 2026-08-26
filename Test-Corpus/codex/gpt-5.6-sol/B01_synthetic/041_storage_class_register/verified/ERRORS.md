# Error Surface

Mechanical search scope: all files under `c_src/`, including every `return`,
error macro, assertion, null/range check, and min/max token.

| # | function | trigger (the exact invalid input/condition) | expected C result | tested |
|---|---|---|---|---|

There are no rows: the C implementation has no explicit rejection, error
return, assertion, null check, range check, enum, pointer/length argument, or
min/max constant. `main` does not inspect `scanf`'s result; its matching-failure
and EOF behavior are therefore configuration outcomes in `CONFIGS.md`, not
rejections. Generic null, zero/oversized-length, and invalid-enum cases are not
applicable to either scalar/no-argument API. One-step-past-`int` textual inputs
are covered by the differential boundary test.
