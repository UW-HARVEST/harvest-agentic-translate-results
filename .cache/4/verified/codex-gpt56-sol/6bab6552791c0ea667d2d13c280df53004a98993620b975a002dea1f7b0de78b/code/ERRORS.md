# Error Surface

Mechanically derived from the ordered `if` checks and failure returns in
`c_src/src/main.c`.

| # | function | trigger (the exact invalid input/condition) | expected C result | tested |
|---|----------|---------------------------------------------|-------------------|--------|
| 1 | `multi_stage` via `main` | `x != 1` after `scanf("%d %d %d", ...)`; this includes no first conversion because `x` starts at `0` | prints `Error: x != 1\nOperation failed\nResult: 1\n`; `multi_stage` returns `1`, then `main` returns `0` | [x] |
| 2 | `multi_stage` via `main` | `x == 1 && y != 2` after `scanf`; this includes no second conversion when retained `y != 2` | prints `Error: x == 1 but y != 2\nOperation failed\nResult: 2\n`; `multi_stage` returns `2`, then `main` returns `0` | [x] |
| 3 | `multi_stage` via `main` | `x == 1 && y == 2 && z != 3` after `scanf`; this includes no third conversion because `z` starts at `0` | prints `Error: x == 1 and y == 2, but z != 3\nOperation failed\nResult: 3\n`; `multi_stage` returns `3`, then `main` returns `0` | [x] |

There are no pointer, length, range, enum, assertion, error-macro, `NULL`, or
`-1` return surfaces in the C source. The only public entry point takes no
arguments.
