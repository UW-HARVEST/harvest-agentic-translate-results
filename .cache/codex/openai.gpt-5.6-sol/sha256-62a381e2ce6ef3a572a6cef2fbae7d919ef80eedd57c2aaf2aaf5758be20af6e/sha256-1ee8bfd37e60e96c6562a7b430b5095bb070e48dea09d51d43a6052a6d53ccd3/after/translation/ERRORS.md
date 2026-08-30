# Error Surface

Mechanical scan scope: all files under `../c_src/include` and `../c_src/src`.
The scan covered error returns, `assert`, `if`, `switch`, range checks, null
checks, enums, and min/max constants. The complete C implementation has no
rejection or error path.

| # | function | trigger (the exact invalid input/condition) | expected C result | tested |
|---|---|---|---|---|
| - | - | No rejection branches exist | - | N/A |

The only public API is `void driver(int x, int y)`. Both arguments are
by-value C integers, so pointer, length, and enum boundary cases do not apply.
