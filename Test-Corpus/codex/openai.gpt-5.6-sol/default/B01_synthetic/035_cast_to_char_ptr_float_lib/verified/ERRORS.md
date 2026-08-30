# Error Surface

Mechanical inspection covered `../c_src/include/driver.h` and
`../c_src/src/driver.c`. Searches for error returns, `NULL`, assertions,
range checks, min/max constants, and conditional rejection branches found no
rejection paths. The only public function returns `void` and accepts one
by-value `float`.

| # | function | trigger (the exact invalid input/condition) | expected C result | Tested |
|---|----------|---------------------------------------------|-------------------|--------|

There are **0** C rejection branches. Generic pointer, length, and enum boundary
cases do not apply because the public API contains no pointers, lengths, or
enums. Every 32-bit object representation passed by value is accepted,
including signed zero, infinities, subnormals, and NaNs.
