# Error Surface

This table is derived from every input-rejection branch reachable through the
sole public library entry point, `process_decisions`. Negative values produced
by valid permission combinations (`-10` for write-only and `-20` for
execute-only) are normal operation results, not input rejection.

The three `fgets` failures in `c_src/src/main.c` belong to the standalone
executable, not the shared-library FFI surface, and therefore are not listed as
library API rows.

| # | function | trigger (the exact invalid input/condition) | expected C result |
|---|----------|----------------------------------------------|-------------------|
| E01 | `process_decisions` | `decision_string == NULL` (including a nonzero `length`) | [x] `-1` |
| E02 | `process_decisions` | non-null `decision_string` and `length == 0` | [x] `-1` |
| E03 | `process_decisions` / operation 0 | `length < 3` after the global nonzero-length check | [x] `-2` |
| E04 | `process_decisions` / operation 1 | `length < 3` after the global nonzero-length check | [x] `-2` |
| E05 | `process_decisions` | `operation` is not 0, 1, 2, or 3 (out-of-range C enum-like mode) | [x] `-3` |
| E06 | `evaluate_conditions` via operation 1 | `param` is not 0, 1, 2, or 3 (out-of-range logic operator) | [x] `-1` |
| E07 | `validate_sequence` via operation 3 | parsed first element is false: first byte is not `y` or `Y` | [x] `-10` |
| E08 | `validate_sequence` via operation 3 | `length > 1` and parsed last element is true: last byte is `y` or `Y` | [x] `-11` |
| E09 | `validate_sequence` via operation 3 | more than three consecutive parsed boolean values are equal | [x] `-12` |

## Explicit Bounds

| Constant/check | C behavior |
|----------------|------------|
| operation 0/1 minimum length | 3 |
| operation 2 consumed length | `min(length, 32)`; excess bytes are ignored |
| flag bit width | 32 (`uint32_t`, indices 0 through 31) |
| short sequence | `length <= 3` |
| medium sequence | `4 <= length <= 10` |
| long sequence | `length > 10` |
| maximum accepted equal run in operation 3 | 3; a run of 4 returns `-12` |

