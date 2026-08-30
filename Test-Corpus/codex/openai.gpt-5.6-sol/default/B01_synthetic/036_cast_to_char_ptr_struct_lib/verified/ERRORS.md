# Error Surface

Mechanical review covered all `return`, `assert`, `ERROR`, `NULL`, range-check,
minimum/maximum, and public-API branches in `../c_src/include/driver.h` and
`../c_src/src/driver.c`.

| # | function | trigger (the exact invalid input/condition) | expected C result | Verified |
|---|----------|---------------------------------------------|-------------------|----------|

The C library has no rejection paths. Its only public function is
`void driver(int floors)`: it accepts every value representable by C `int`,
has no pointer or length parameters, and returns no status. Null pointers,
zero/oversized lengths, and invalid enum discriminants are therefore not
applicable to this API.

