# Error Surface

Mechanically derived from all rejection/error patterns, null checks, range
checks, assertions, and error returns in `c_src/include/` and `c_src/src/`.
There are no enums, lengths, range constants, assertions, or other error
returns in this API.

| # | function | trigger (the exact invalid input/condition) | expected C result | |
|---|----------|---------------------------------------------|-------------------|-|
| 1 | `smallestValue` | `head == NULL` (`if (head)` is false) | returns `-1` | [x] |
