# ERRORS.md — error-surface table (Phase C)

Derived mechanically from `c_src/src/lib.c` and `c_src/src/main.c`. Every
`return <negative>` / rejecting branch / bound-check / min-max constant in the C
sources is one row. There are **no** `assert`s, `abort`s, `exit`s or
`return NULL`s anywhere in `c_src` (verified by grep), so every rejection is a
sentinel integer return value.

Mechanical grep basis — the complete set of negative returns in `lib.c`:

```
lib.c:53:        return -1;      /* process_decisions: NULL buffer or length == 0 */
lib.c:59:        if (length < 3) return -2;   /* operation 0 */
lib.c:70:        if (length < 3) return -2;   /* operation 1 */
lib.c:98:        return -3;      /* process_decisions: unknown operation */
lib.c:155:       return -10;     /* apply_permissions: write only            */
lib.c:158:       return -20;     /* apply_permissions: execute only          */
lib.c:236:       return -1;      /* evaluate_conditions: unknown logic_op    */
lib.c:330:       return -10;     /* validate_sequence: rule 1                */
lib.c:335:       return -11;     /* validate_sequence: rule 2                */
lib.c:344:       return -12;     /* validate_sequence: rule 3                */
```

## Library rejections (`process_decisions`)

| # | function | trigger (the exact invalid input/condition) | expected C result | test | ✔ |
|---|----------|----------------------------------------------|-------------------|------|---|
| E1 | `process_decisions` (lib.c:52) | `decision_string == NULL`, any `length > 0`, any `operation`, any `param` | `-1` | `err_e1_null_pointer` | [x] |
| E2 | `process_decisions` (lib.c:52) | `length == 0` with a valid non-NULL buffer, every `operation` in `{-2..5}` | `-1` (checked **before** the operation switch, so it wins over `-3`) | `err_e2_zero_length` | [x] |
| E3 | `process_decisions` (lib.c:52) | `decision_string == NULL` **and** `length == 0` simultaneously | `-1` | `err_e3_null_and_zero_length` | [x] |
| E4 | `process_decisions` (lib.c:59) | `operation == 0` and `1 <= length <= 2` | `-2` | `err_e4_op0_short_length` | [x] |
| E5 | `process_decisions` (lib.c:70) | `operation == 1` and `1 <= length <= 2` | `-2` | `err_e5_op1_short_length` | [x] |
| E6 | `process_decisions` (lib.c:97-98) | `operation` outside `{0,1,2,3}`: `4`, `5`, `-1`, `-3`, `INT_MIN`, `INT_MAX`, and every value one step past the range | `-3` | `err_e6_unknown_operation` | [x] |

## `apply_permissions` rejections (reached via `operation == 0`)

| # | function | trigger (the exact invalid input/condition) | expected C result | test | ✔ |
|---|----------|----------------------------------------------|-------------------|------|---|
| E7 | `apply_permissions` (lib.c:153-155) | write-only permission set: `read == false, write == true, execute == false`, i.e. decision string `"nyX"` for any non-`y`/`Y` third byte | `-10` | `err_e7_write_only` | [x] |
| E8 | `apply_permissions` (lib.c:156-158) | execute-only permission set: `read == false, write == false, execute == true`, i.e. `"nny"` | `-20` | `err_e8_execute_only` | [x] |
| E9 | `apply_permissions` (lib.c:139-143, fallthrough) | `read && write && !execute` but `permission_value != 6` — **dead branch** in C (the value is always exactly 6 there), so the `return 0` at lib.c:162 is unreachable through this path; verified the reachable value is `56` and never `0` | `56` (never `0`) | `err_e9_read_write_no_execute_never_falls_through` | [x] |

## `evaluate_conditions` rejections (reached via `operation == 1`)

| # | function | trigger (the exact invalid input/condition) | expected C result | test | ✔ |
|---|----------|----------------------------------------------|-------------------|------|---|
| E10 | `evaluate_conditions` (lib.c:235-236) | `operation == 1` with `param` (the `logic_op` enum-like selector) outside `{0,1,2,3}`: `4`, `-1`, `INT_MIN`, `INT_MAX`, one past each end. C enums accept any `int`, so these are real inputs. | `-1` | `err_e10_unknown_logic_op` | [x] |

## `validate_sequence` rejections (reached via `operation == 3`)

| # | function | trigger (the exact invalid input/condition) | expected C result | test | ✔ |
|---|----------|----------------------------------------------|-------------------|------|---|
| E11 | `validate_sequence` (lib.c:319) | `len == 0` — early `return 0`. Unreachable through `process_decisions` (caught by E2) but reachable in principle; asserted to stay `-1` at the public boundary. | `0` internally / `-1` at the public entry point | `err_e11_validate_zero_len` | [x] |
| E12 | `validate_sequence` (lib.c:329-330) | Rule 1 violated: first byte does not parse as true, i.e. `sequence[0] not in {'y','Y'}` (includes `'n'`, `'N'`, `'\0'`, any other byte) | `-10` | `err_e12_rule1_must_start_true` | [x] |
| E13 | `validate_sequence` (lib.c:334-335) | Rule 2 violated: `len > 1` and the last byte parses as true, i.e. `sequence[len-1] in {'y','Y'}` | `-11` | `err_e13_rule2_must_end_false` | [x] |
| E14 | `validate_sequence` (lib.c:340-345) | Rule 3 violated: more than 3 consecutive equal parsed values anywhere (run length >= 4), e.g. `"ynnnn"`, `"yyyyn"` | `-12` | `err_e14_rule3_max_consecutive` | [x] |

## Bounds / min-max constants (explicit range checks in the C)

| # | function | trigger (the exact invalid input/condition) | expected C result | test | ✔ |
|---|----------|----------------------------------------------|-------------------|------|---|
| E15 | `process_decisions` (lib.c:82-83) | `operation == 2` with `length > 32`: `count` is clamped by `(length < 32) ? length : 32`, so only the first 32 bytes may influence the result and the local `bool decisions[32]` is never overrun. Bytes 32.. must be ignored entirely. | result identical to the same input truncated to 32 bytes | `err_e15_op2_count_clamped_to_32` | [x] |
| E16 | `configure_flags` (lib.c:249) | Loop guard `i < count && i < 32` — the `1u << i` shift can therefore never reach the UB range `i >= 32`. Exercised at `count == 31, 32` and with `length` up to 1023. | no UB; bit 31 is the highest set | `err_e16_configure_flags_shift_bound` | [x] |
| E17 | `configure_flags` (lib.c:270) | `special_count == count - 1` with `count` a `size_t`: the subtraction is unsigned. Only reachable with `count >= 3`, but the wrap-around behaviour for small counts must agree. | same as C for every `count` in `1..=32` | `err_e17_configure_flags_count_minus_one_unsigned` | [x] |
| E18 | `validate_sequence` (lib.c:363, 373) | `len - 1` and `len - 3` are `size_t` subtractions compared against an `int transitions` (which is converted to `size_t`). Exercised at the `len == 1`, `3/4` and `10/11` branch boundaries. | same as C | `err_e18_validate_size_t_comparisons` | [x] |
| E19 | `process_decisions` | `length` at the largest value `main` can supply (`1023`, from `MAX_INPUT_SIZE - 1`) for every operation | same as C | `err_e19_max_length_1023` | [x] |
| E20 | `parse_bool` (lib.c:108-114) | Any byte that is neither `y`/`Y` nor `n`/`N` — including `0x00`, `0x80..0xFF` (i.e. negative `char` on x86-64), `'0'`, `' '` — silently parses to `false` rather than being rejected | treated as `false`, no error | `err_e20_parse_bool_invalid_bytes` | [x] |

## Executable-level rejections (`main.c`)

| # | function | trigger (the exact invalid input/condition) | expected C result | test | ✔ |
|---|----------|----------------------------------------------|-------------------|------|---|
| E21 | `main` (main.c:43-46) | `fgets` returns NULL for the **operation** line (stdin empty) | prints `Error reading operation\n` to stderr, exit code `1`, no stdout | `err_e21_missing_operation_line` | [x] |
| E22 | `main` (main.c:50-53) | `fgets` returns NULL for the **parameter** line (only one line on stdin) | prints `Error reading parameter\n` to stderr, exit code `1`, no stdout | `err_e22_missing_parameter_line` | [x] |
| E23 | `main` (main.c:57-60) | `fgets` returns NULL for the **decision** line (only two lines on stdin) | prints `Error reading decision string\n` to stderr, exit code `1`, no stdout | `err_e23_missing_decision_line` | [x] |
| E24 | `main` (main.c:34, 43/50/57) | A line longer than `MAX_INPUT_SIZE - 1 == 1023` bytes: `fgets` truncates and the remainder becomes the *next* line | same stdout/stderr/exit as C | `err_e24_line_longer_than_1023` | [x] |
| E25 | `main` (main.c:47, 54) via `atoi` | Non-numeric / empty / whitespace-only / signed / overflowing operation & param lines (`atoi` never reports an error; glibc `atoi` is `(int)strtol(s,NULL,10)`, saturating at `LONG_MIN`/`LONG_MAX` then truncating) | same stdout as C | `err_e25_atoi_edge_cases` | [x] |
| E26 | `main` (main.c:63-67) via `strlen` | An embedded `'\0'` in the decision line truncates `len`; a decision line with no trailing `'\n'` (EOF-terminated) keeps its full length | same stdout as C | `err_e26_embedded_nul_and_no_newline` | [x] |
