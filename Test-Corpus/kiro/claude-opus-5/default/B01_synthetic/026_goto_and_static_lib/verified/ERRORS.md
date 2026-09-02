# ERRORS.md — Phase C error-surface table

Derived mechanically from `c_src/src/driver.c`. Method: grep every rejection
construct in the whole C source.

```
grep -n 'return\|goto\|assert\|if (\|Error' c_src/src/driver.c
```

Full inventory of what the C source contains:

| construct | count | where |
|-----------|-------|-------|
| `goto fail` | 3 | `multi_stage`: `x != 1`, `y != 2`, `z != 3` |
| `return <code>` | 2 | `multi_stage`: success path `return result` (0), fail path `return result` (1/2/3) |
| `assert` | 0 | — none in the source |
| null-pointer check | 0 | — the API takes no pointers; `driver(int, int, int)` |
| explicit range / min / max check | 0 | — no bounds, no clamping, no `INT_MAX`-style constants |
| error enum / `errno` / `-1` sentinel | 0 | — `driver` returns `void`; the only error channel is `stdout` |
| memory allocation that can fail | 0 | — `stdlib.h` is included but never used |

Consequence, and the reason every row below asserts on **stdout bytes**:
`driver` has **no return value and no out-parameters**. The three error
conditions are observable *only* as the exact byte sequence printed to
`stdout`. "Same error/rejection" therefore means: identical error message line,
identical `Operation failed` line, and identical `Result: <code>` code.

## Error-surface table

| # | function | trigger (the exact invalid input/condition) | expected C result | test | ✔ |
|---|----------|---------------------------------------------|-------------------|------|---|
| 1 | `driver` → `multi_stage` | `x != 1` (first guard fails; `y`/`z` never examined) | stdout `"Error: x != 1\nOperation failed\nResult: 1\n"`; internal code 1 | `err_row1_x_not_1` | [x] |
| 2 | `driver` → `multi_stage` | `x == 1` **and** `local_y != 2` (second guard; `z` never examined) | stdout `"Error: x == 1 but y != 2\nOperation failed\nResult: 2\n"`; internal code 2 | `err_row2_y_not_2` | [x] |
| 3 | `driver` → `multi_stage` | `x == 1` **and** `local_y == 2` **and** `z != 3` (third guard) | stdout `"Error: x == 1 and y == 2, but z != 3\nOperation failed\nResult: 3\n"`; internal code 3 | `err_row3_z_not_3` | [x] |
| 4 | `driver` → `multi_stage` | fail-path `goto fail` target reached by ANY of rows 1–3 — must print `Operation failed` (and the success path must NOT) | `"Operation failed\n"` present iff code != 0 | `err_row4_fail_label_only_on_failure` | [x] |
| 5 | `driver` | guard-order precedence: `x != 1` **and** `local_y != 2` **and** `z != 3` simultaneously — only the FIRST message may be emitted | code 1 only; rows 2/3 messages absent | `err_row5_guard_precedence` | [x] |
| 6 | `driver` | guard-order precedence: `x == 1`, `local_y != 2`, `z != 3` — `y` guard wins over `z` guard | code 2 only; row 3 message absent | `err_row5_guard_precedence` | [x] |

## Generic C-API boundary conditions (required even though absent from the table above)

| # | condition | why it is a real input here | expected C result | test | ✔ |
|---|-----------|-----------------------------|-------------------|------|---|
| 7 | `INT_MIN` / `INT_MAX` in each of the three parameters | `int` params accept the full 32-bit range; none equals 1/2/3, so each must take the corresponding failure branch | per rows 1–3 by position | `err_boundary_int_extremes` | [x] |
| 8 | one step past each "valid" value: `x ∈ {0, 2}`, `y ∈ {1, 3}`, `z ∈ {2, 4}` | off-by-one around the only three magic constants in the source | per rows 1–3 | `err_boundary_off_by_one` | [x] |
| 9 | `0` in each parameter (C's default/zero value) | zero is not 1, 2, or 3 → always a failure branch | per rows 1–3 | `err_boundary_off_by_one` | [x] |
| 10 | out-of-range "enum-like" ints passed across FFI: values with no meaningful variant (`-1`, `4`, `0x7fff_ffff`, `0x8000_0000` as `i32`) in each slot | C enums/ints accept any `int`; the ABI is `(i32,i32,i32)` so *every* bit pattern is reachable and must behave identically | identical stdout from both `.so`s | `err_out_of_range_enum_values` | [x] |
| 11 | null pointers | **N/A** — the API surface contains no pointer parameters and no pointer returns (`void driver(int,int,int)`). Documented as intentionally not applicable. | — | — | [x] |
| 12 | zero / oversized lengths | **N/A** — no buffer, length, count, or size parameter exists anywhere in the public API. Documented as intentionally not applicable. | — | — | [x] |
| 13 | fail path leaves `static y` mutated for the next call | `driver` assigns `y = local_y` *before* any guard, so a failing call still commits the write; the following call must observe the same state in C and Rust | identical stdout across a call sequence | `err_state_persists_after_failure` | [x] |
