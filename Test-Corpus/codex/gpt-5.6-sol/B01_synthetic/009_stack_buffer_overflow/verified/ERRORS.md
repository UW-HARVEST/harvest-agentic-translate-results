# Error Surface

Mechanically derived from every `if` rejection arm and null check in
`c_src/src/main.c`. There are no assertions, error-return macros, error enums,
or nonzero error returns.

| # | function | trigger (the exact invalid input/condition) | expected C result | |
|---|---|---|---|---|
| 1 | `printLine` | `line == NULL` | returns `void` and writes no bytes | [x] |
| 2 | `bad` | `fgets(inputBuffer, 14, stdin) == NULL` | writes `fgets() failed.\nERROR: Array index is negative.\n` | [x] |
| 3 | `bad` | parsed `data < 0` | writes `ERROR: Array index is negative.\n` | [x] |
| 4 | `good` (`goodB2G`) | `fgets(inputBuffer, 14, stdin) == NULL` | after the fixed ten-line `goodG2B` output, writes `fgets() failed.\nERROR: Array index is out-of-bounds\n` | [x] |
| 5 | `good` (`goodB2G`) | parsed `data < 0` | after the fixed ten-line `goodG2B` output, writes `ERROR: Array index is out-of-bounds\n` | [x] |
| 6 | `good` (`goodB2G`) | parsed `data >= 10` | after the fixed ten-line `goodG2B` output, writes `ERROR: Array index is out-of-bounds\n` | [x] |

The `bad` sink deliberately has no `data < 10` rejection. Values at or above
10 execute an out-of-bounds C write and are tracked as an unsafe boundary in
`CONFIGS.md`, not as a rejection invented by the translation.
