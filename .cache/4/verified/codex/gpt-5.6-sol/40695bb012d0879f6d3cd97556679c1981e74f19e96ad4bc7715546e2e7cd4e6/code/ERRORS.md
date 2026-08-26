# Error Surface

Mechanically inspected `c_src/include/lib.h` and `c_src/src/lib.c` for error
returns, null/range checks, assertions, error enums, and min/max constants.

| # | function | trigger (the exact invalid input/condition) | expected C result | tested |
|---|----------|----------------------------------------------|-------------------|--------|

There are no defined rejection branches in the C source. `md5_digest` returns
`void` and unconditionally dereferences both pointers. Null or undersized
pointers therefore invoke undefined C behavior rather than producing an error
code or sentinel. The differential test suite probes null pointers in isolated
child processes and verifies that neither implementation accepts them. There
are no length parameters or enums to probe; every `uint32_t` bit pattern is
valid, and the valid-path tests include zero and `UINT32_MAX`.

Defined C rejection rows tested: **0 of 0**

Generic boundary probes: **complete**
