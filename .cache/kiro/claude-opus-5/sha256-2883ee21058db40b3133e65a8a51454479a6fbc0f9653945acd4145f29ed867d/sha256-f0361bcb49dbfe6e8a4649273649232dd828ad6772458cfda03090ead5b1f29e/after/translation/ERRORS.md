# ERRORS.md — error-surface table (Phase A → gates Phase C)

Derived mechanically from `c_src/src/driver.c` by grepping every rejection
construct: `grep -n 'return|goto|assert|Error|fail|!=|==|NULL|-1'`.

The C source contains **no** `assert`, **no** `return NULL`, **no** `return -1`,
**no** error enum, **no** pointer parameters and therefore **no** null checks,
and **no** min/max constants. Every rejection is one of the three `goto fail`
guards in `static int multi_stage(int x, int z)` (driver.c:33, :39, :45), each
of which sets a distinct `result` code and falls into the shared `fail:` label
(driver.c:54) that prints `"Operation failed\n"`.

`void driver(int, int, int)` returns `void`, so the error code is observable
**only** through stdout: `multi_stage`'s return value is printed by
`printf("Result: %d\n", result)` (driver.c:61). "Same error/rejection" is
therefore asserted as byte-identical stdout, which includes the exact numeric
sentinel (`Result: 1` / `2` / `3`).

Note the guards are checked in the fixed order x → y → z and short-circuit, so
an earlier-failing input **masks** later invalid values. That masking is part of
the contract and is asserted below.

| # | function | trigger (the exact invalid input/condition) | expected C result | test |
|---|----------|---------------------------------------------|-------------------|------|
| 1 | `multi_stage` via `driver` | `x != 1` (driver.c:33), any `local_y`, any `z` | stdout `Error: x != 1\nOperation failed\nResult: 1\n`; `multi_stage` returns 1 | `err_row1_x_not_1` |
| 2 | `multi_stage` via `driver` | `x == 1` **and** `y != 2`, where `y` is the static set from `local_y` (driver.c:39), any `z` | stdout `Error: x == 1 but y != 2\nOperation failed\nResult: 2\n`; returns 2 | `err_row2_y_not_2` |
| 3 | `multi_stage` via `driver` | `x == 1` **and** `y == 2` **and** `z != 3` (driver.c:45) | stdout `Error: x == 1 and y == 2, but z != 3\nOperation failed\nResult: 3\n`; returns 3 | `err_row3_z_not_3` |
| 4 | `driver` | guard **ordering / masking**: `x != 1` while `local_y != 2` and `z != 3` too — only row 1's message may appear (`fail:` is shared, so exactly 3 lines are printed, never 4+) | row-1 output only; `Result: 1` | `err_row4_masking_order` |
| 5 | `driver` | guard ordering: `x == 1`, `local_y != 2`, `z != 3` — the `z` guard must be unreachable | row-2 output only; `Result: 2` | `err_row5_masking_y_before_z` |
| 6 | `driver` | boundary/extreme ints one step past and far past the single valid value of `x`: `x ∈ {0, 2, -1, INT_MIN, INT_MAX}` | all take row 1; `Result: 1` | `err_row6_x_boundaries` |
| 7 | `driver` | boundary/extreme ints for `local_y`: `x == 1`, `local_y ∈ {1, 3, 0, -1, INT_MIN, INT_MAX}` | all take row 2; `Result: 2` | `err_row7_y_boundaries` |
| 8 | `driver` | boundary/extreme ints for `z`: `x == 1`, `local_y == 2`, `z ∈ {2, 4, 0, -1, INT_MIN, INT_MAX}` | all take row 3; `Result: 3` | `err_row8_z_boundaries` |
| 9 | `driver` | "out-of-range enum" analogue across the FFI boundary: the C API takes plain `int`s with exactly one accepted value each, so *any* `int` with no valid meaning is an out-of-range input. Sweep arbitrary/garbage 32-bit values (incl. `0x8000_0000`, `0x7FFF_FFFF`, `0xFFFF_FFFF`, `0xDEAD_BEEF` reinterpreted as `i32`) in all three positions. | identical rejection path & `Result:` code in both libs; never a crash | `err_row9_out_of_range_ints` |
| 10 | `driver` | the initial value `static int y = 123` (driver.c:29) is **never** observable as an error trigger, because `driver` unconditionally assigns `y = local_y` (driver.c:59) before `multi_stage` reads it. Asserted by making the first-ever call to each freshly `dlopen`ed library a `(1, 2, 3)` success — if the initializer leaked, this would print row 2. | `Ok!\nResult: 0\n` on the very first call after load | `err_row10_initial_y_never_observable` |
| 11 | `driver` | no-crash / no-abort contract on the rejection paths: `driver` returns `void` and must return normally (not `abort`/panic) for every rejection above, including after repeated failing calls. | normal return, process alive, stdout as tabulated | `err_row11_rejection_paths_return_normally` |

All 11 rows are checked off in the Phase C section of
`tests/differential.rs` (see the `# ERRORS.md row N` comments) and pass.
