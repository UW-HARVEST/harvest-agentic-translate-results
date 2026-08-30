# Error Surface

Mechanically derived by inspecting every `if`, null check, return, assertion,
and limit reference in `include/driver.h` and `src/driver.c`. The C source has
no error-return statements, error enums, assertions, length arguments, or enum
arguments. It has the null guard and arithmetic range rejection below.

| # | function | trigger (the exact invalid input/condition) | expected C result | |
|---|----------|---------------------------------------------|-------------------|-|
| 1 | `printLine` | `line == NULL` | Return `void` without output | [x] |
| 2 | `goodB2G` via `good` | Local `data == CHAR_MAX`, so `data < (CHAR_MAX/2)` is false | Reject the multiply and print `data value is too large to perform arithmetic safely.\n` | [x] |

Generic FFI boundaries were also inventoried. No exported function accepts a
length or enum. `driver(int)` accepts the entire C `int` domain and treats
every nonzero value as true; `printHexCharLine(char)` accepts the entire C
`char` domain. The other `data > 0` checks guard fixed positive local values
(`CHAR_MAX` or `2`) and cannot be driven to their false branches by an API
input.
