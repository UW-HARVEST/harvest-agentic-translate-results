# Configuration Surface

Derived from the public dynamic entry points and every value-dependent
`if`/`while`/`switch` path in `c_src/src/lib.c`. The source has no compile-time
feature branches. Packed flags are enumerated as the full cross-product of the
three one-bit flags and eight three-bit modes. Each flag row includes both the
ordinary counter path (`0..=30`) and the five-bit wrap path (`31 -> 0`).

| # | entry point(s) | configuration (options set + input shape) | verified |
|---|----------------|--------------------------------------------|----------|
| 1 | `create_state`, `destroy_state` | capacity `0`; initial value randomized over zero, signs, and integer boundaries | [x] |
| 2 | `create_state`, `destroy_state` | capacity `1`; only the terminating NUL fits | [x] |
| 3 | `create_state`, `destroy_state` | capacity `2..formatted_len`; formatted text is truncated | [x] |
| 4 | `create_state`, `destroy_state` | capacity `formatted_len + 1`; formatted text exactly fits with NUL | [x] |
| 5 | `create_state`, `destroy_state` | capacity greater than `formatted_len + 1`; formatted text and trailing capacity | [x] |
| 6 | `destroy_state` | ordinary state with non-null buffer | [x] |
| 7 | `process_buffer` | empty NUL-terminated buffer (`remaining == 0`) | [x] |
| 8 | `process_buffer` | nonempty buffer with no target byte (`memchr == NULL`) | [x] |
| 9 | `process_buffer` | nonempty buffer with exactly one target byte | [x] |
| 10 | `process_buffer` | nonempty buffer with multiple target bytes | [x] |
| 11 | `process_buffer` | target is NUL or a high-bit byte, exercising C `char` promotion and `memchr` conversion | [x] |
| 12 | `update_flags` | flags `000`, mode `0`, counter ordinary and wrap | [x] |
| 13 | `update_flags` | flags `001`, mode `0`, counter ordinary and wrap | [x] |
| 14 | `update_flags` | flags `010`, mode `0`, counter ordinary and wrap | [x] |
| 15 | `update_flags` | flags `011`, mode `0`, counter ordinary and wrap | [x] |
| 16 | `update_flags` | flags `100`, mode `0`, counter ordinary and wrap | [x] |
| 17 | `update_flags` | flags `101`, mode `0`, counter ordinary and wrap | [x] |
| 18 | `update_flags` | flags `110`, mode `0`, counter ordinary and wrap | [x] |
| 19 | `update_flags` | flags `111`, mode `0`, counter ordinary and wrap | [x] |
| 20 | `update_flags` | flags `000`, mode `1`, counter ordinary and wrap | [x] |
| 21 | `update_flags` | flags `001`, mode `1`, counter ordinary and wrap | [x] |
| 22 | `update_flags` | flags `010`, mode `1`, counter ordinary and wrap | [x] |
| 23 | `update_flags` | flags `011`, mode `1`, counter ordinary and wrap | [x] |
| 24 | `update_flags` | flags `100`, mode `1`, counter ordinary and wrap | [x] |
| 25 | `update_flags` | flags `101`, mode `1`, counter ordinary and wrap | [x] |
| 26 | `update_flags` | flags `110`, mode `1`, counter ordinary and wrap | [x] |
| 27 | `update_flags` | flags `111`, mode `1`, counter ordinary and wrap | [x] |
| 28 | `update_flags` | flags `000`, mode `2`, counter ordinary and wrap | [x] |
| 29 | `update_flags` | flags `001`, mode `2`, counter ordinary and wrap | [x] |
| 30 | `update_flags` | flags `010`, mode `2`, counter ordinary and wrap | [x] |
| 31 | `update_flags` | flags `011`, mode `2`, counter ordinary and wrap | [x] |
| 32 | `update_flags` | flags `100`, mode `2`, counter ordinary and wrap | [x] |
| 33 | `update_flags` | flags `101`, mode `2`, counter ordinary and wrap | [x] |
| 34 | `update_flags` | flags `110`, mode `2`, counter ordinary and wrap | [x] |
| 35 | `update_flags` | flags `111`, mode `2`, counter ordinary and wrap | [x] |
| 36 | `update_flags` | flags `000`, mode `3`, counter ordinary and wrap | [x] |
| 37 | `update_flags` | flags `001`, mode `3`, counter ordinary and wrap | [x] |
| 38 | `update_flags` | flags `010`, mode `3`, counter ordinary and wrap | [x] |
| 39 | `update_flags` | flags `011`, mode `3`, counter ordinary and wrap | [x] |
| 40 | `update_flags` | flags `100`, mode `3`, counter ordinary and wrap | [x] |
| 41 | `update_flags` | flags `101`, mode `3`, counter ordinary and wrap | [x] |
| 42 | `update_flags` | flags `110`, mode `3`, counter ordinary and wrap | [x] |
| 43 | `update_flags` | flags `111`, mode `3`, counter ordinary and wrap | [x] |
| 44 | `update_flags` | flags `000`, mode `4`, counter ordinary and wrap | [x] |
| 45 | `update_flags` | flags `001`, mode `4`, counter ordinary and wrap | [x] |
| 46 | `update_flags` | flags `010`, mode `4`, counter ordinary and wrap | [x] |
| 47 | `update_flags` | flags `011`, mode `4`, counter ordinary and wrap | [x] |
| 48 | `update_flags` | flags `100`, mode `4`, counter ordinary and wrap | [x] |
| 49 | `update_flags` | flags `101`, mode `4`, counter ordinary and wrap | [x] |
| 50 | `update_flags` | flags `110`, mode `4`, counter ordinary and wrap | [x] |
| 51 | `update_flags` | flags `111`, mode `4`, counter ordinary and wrap | [x] |
| 52 | `update_flags` | flags `000`, mode `5`, counter ordinary and wrap | [x] |
| 53 | `update_flags` | flags `001`, mode `5`, counter ordinary and wrap | [x] |
| 54 | `update_flags` | flags `010`, mode `5`, counter ordinary and wrap | [x] |
| 55 | `update_flags` | flags `011`, mode `5`, counter ordinary and wrap | [x] |
| 56 | `update_flags` | flags `100`, mode `5`, counter ordinary and wrap | [x] |
| 57 | `update_flags` | flags `101`, mode `5`, counter ordinary and wrap | [x] |
| 58 | `update_flags` | flags `110`, mode `5`, counter ordinary and wrap | [x] |
| 59 | `update_flags` | flags `111`, mode `5`, counter ordinary and wrap | [x] |
| 60 | `update_flags` | flags `000`, mode `6`, counter ordinary and wrap | [x] |
| 61 | `update_flags` | flags `001`, mode `6`, counter ordinary and wrap | [x] |
| 62 | `update_flags` | flags `010`, mode `6`, counter ordinary and wrap | [x] |
| 63 | `update_flags` | flags `011`, mode `6`, counter ordinary and wrap | [x] |
| 64 | `update_flags` | flags `100`, mode `6`, counter ordinary and wrap | [x] |
| 65 | `update_flags` | flags `101`, mode `6`, counter ordinary and wrap | [x] |
| 66 | `update_flags` | flags `110`, mode `6`, counter ordinary and wrap | [x] |
| 67 | `update_flags` | flags `111`, mode `6`, counter ordinary and wrap | [x] |
| 68 | `update_flags` | flags `000`, mode `7`, counter ordinary and wrap | [x] |
| 69 | `update_flags` | flags `001`, mode `7`, counter ordinary and wrap | [x] |
| 70 | `update_flags` | flags `010`, mode `7`, counter ordinary and wrap | [x] |
| 71 | `update_flags` | flags `011`, mode `7`, counter ordinary and wrap | [x] |
| 72 | `update_flags` | flags `100`, mode `7`, counter ordinary and wrap | [x] |
| 73 | `update_flags` | flags `101`, mode `7`, counter ordinary and wrap | [x] |
| 74 | `update_flags` | flags `110`, mode `7`, counter ordinary and wrap | [x] |
| 75 | `update_flags` | flags `111`, mode `7`, counter ordinary and wrap | [x] |
| 76 | `confuse_types` | operation `0`; arbitrary initial union bits are replaced by `1078530011` | [x] |
| 77 | `confuse_types` | operation `1`; union bits interpreted as float, including finite, subnormal, infinity, NaN, and C-int overflow conversion shapes | [x] |
| 78 | `confuse_types` | operation `2`; union bits interpreted as unsigned and masked to the low byte | [x] |
| 79 | `confuse_types` | operation `3`; union bits interpreted as four signed `char` values and bytes 0 and 1 are added | [x] |
| 80 | `confuse_types` | operation outside `0..=3`; default switch path | [x] |
| 81 | `confuse_types` | sequence `0 -> 1`; read constant bits as float after mutation | [x] |
| 82 | `confuse_types` | sequence `0 -> 2`; read constant bits as unsigned after mutation | [x] |
| 83 | `confuse_types` | sequence `0 -> 3`; read constant bits as bytes after mutation | [x] |
| 84 | `confusion` | search finds no byte; `param4 % 4 == 0`; all 64 flag/mode settings | [x] |
| 85 | `confusion` | search finds one byte; `param4 % 4 == 0`; all 64 flag/mode settings | [x] |
| 86 | `confusion` | search finds multiple bytes; `param4 % 4 == 0`; all 64 flag/mode settings | [x] |
| 87 | `confusion` | search finds no byte; `param4 % 4 == 1`; all 64 flag/mode settings | [x] |
| 88 | `confusion` | search finds one byte; `param4 % 4 == 1`; all 64 flag/mode settings | [x] |
| 89 | `confusion` | search finds multiple bytes; `param4 % 4 == 1`; all 64 flag/mode settings | [x] |
| 90 | `confusion` | search finds no byte; `param4 % 4 == 2`; all 64 flag/mode settings | [x] |
| 91 | `confusion` | search finds one byte; `param4 % 4 == 2`; all 64 flag/mode settings | [x] |
| 92 | `confusion` | search finds multiple bytes; `param4 % 4 == 2`; all 64 flag/mode settings | [x] |
| 93 | `confusion` | search finds no byte; `param4 % 4 == 3`; all 64 flag/mode settings | [x] |
| 94 | `confusion` | search finds one byte; `param4 % 4 == 3`; all 64 flag/mode settings | [x] |
| 95 | `confusion` | search finds multiple bytes; `param4 % 4 == 3`; all 64 flag/mode settings | [x] |
| 96 | `confusion` | search finds no byte; negative `param4 % 4` takes default switch path; all 64 flag/mode settings | [x] |
| 97 | `confusion` | search finds one byte; negative `param4 % 4` takes default switch path; all 64 flag/mode settings | [x] |
| 98 | `confusion` | search finds multiple bytes; negative `param4 % 4` takes default switch path; all 64 flag/mode settings | [x] |

For wrapper rows, randomized values include `INT_MIN`, `INT_MAX`, zero,
positive and negative decimal strings, all `param3 % 10` results from `-9` to
`9`, irrelevant high bits in `param2`, and congruent `param4` values beyond the
direct switch labels.

Feature combinations from `Cargo.toml`: one (no features are declared).
