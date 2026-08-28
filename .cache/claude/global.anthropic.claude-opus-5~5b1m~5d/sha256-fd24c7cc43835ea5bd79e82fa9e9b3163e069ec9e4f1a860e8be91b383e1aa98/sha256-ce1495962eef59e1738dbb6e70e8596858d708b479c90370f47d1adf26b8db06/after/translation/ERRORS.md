# ERRORS.md — Error-surface table (Phase A / gate for Phase C)

Every row below was derived mechanically from `c_src/src/lib.c` by grepping for
each distinct rejection site: every `return NULL` / `return 0` / `return -1`
guard, every `== NULL` check, every `default:` arm, every `strcmp(...) == 0`
gate, and every allocation whose failure is checked. There are no `assert`s and
no error enums in the C source.

`stdout` text is part of the observable result and is compared byte-for-byte in
every error test (the C `.so` reaches `stdout` via GCC's `printf`→`puts`
rewrite, the Rust `.so` via `printf`; both go through the same libc and must
produce identical bytes).

| # | function | trigger (the exact invalid input/condition) | expected C result | test | [x] |
|---|----------|----------------------------------------------|-------------------|------|-----|
| E1 | `create_result_string` | `malloc(64)` returns `NULL` (`lib.c:40`) | returns `NULL`; prints nothing | `err_e1_e3_e10_alloc_failure` (OOM subprocess) | [x] |
| E2 | `safe_add` | `perms` missing `READ_PERM` (0400) and/or `WRITE_PERM` (0200), i.e. `check_permissions(perms, 0600) == 0` (`lib.c:52`) | prints `Insufficient permissions for addition\n`; returns `0` (NOT `a+b`) | `err_e2_safe_add_insufficient_perms` | [x] |
| E3 | `multiply_with_log` | `*log_msg == NULL` after `create_result_string` failed its `malloc` (`lib.c:61`) | returns `0` (NOT `a*b`); leaves `*log_msg == NULL` | `err_e1_e3_e10_alloc_failure` (OOM subprocess) | [x] |
| E4 | `multiply_with_log` | `log_msg == NULL`: there is **no** null check, `*log_msg = ...` dereferences it (`lib.c:60`) | undefined behaviour — SIGSEGV on Linux/x86-64 | `err_e4_multiply_with_log_null_outparam` (crash-signal subprocess) | [x] |
| E5 | `copy_and_sum` | `src == NULL` (`lib.c:68`) | prints `Source pointer is NULL\n`; returns `-1`. Checked *before* `count`, so it wins even for `count == 0` | `err_e5_copy_and_sum_null_src` | [x] |
| E6 | `copy_and_sum` | `malloc(count * sizeof(int))` returns `NULL` (`lib.c:74`). Reached for any **negative** `count`: the `int` is converted to `size_t`, so `-1` becomes `0xFFFF_FFFF_FFFF_FFFF` and `* 4` wraps to `0xFFFF_FFFF_FFFF_FFFC` — an unsatisfiable request | prints `Memory allocation failed\n`; returns `-1` | `err_e6_copy_and_sum_alloc_failure` | [x] |
| E7 | `compare_operations` | `op1 == NULL`, `op2` valid (`lib.c:91`, left disjunct) | prints `One or both operation strings are NULL\n`; returns `-1` | `err_e7_e8_e9_compare_operations_nulls` | [x] |
| E8 | `compare_operations` | `op1` valid, `op2 == NULL` (`lib.c:91`, right disjunct) | prints `One or both operation strings are NULL\n`; returns `-1` | `err_e7_e8_e9_compare_operations_nulls` | [x] |
| E9 | `compare_operations` | both `op1 == NULL` and `op2 == NULL` | prints `One or both operation strings are NULL\n`; returns `-1` (single message, not two) | `err_e7_e8_e9_compare_operations_nulls` | [x] |
| E10 | `complexmode` | `malloc(sizeof(Result))` (40 bytes) returns `NULL` (`lib.c:106`) | prints `Failed to allocate result tracker\n`; returns `-1`; **no** `Operation performed:` line (early return) | `err_e1_e3_e10_alloc_failure` (OOM subprocess) | [x] |
| E11 | `complexmode` | `mode` is not one of `1,2,3,4` → `default:` arm (`lib.c:166`) | prints `Invalid mode\n`; returns `-1`; `operation` is still `"none"` so `strcmp(...,"none") == 0` and the `Operation performed:` line is suppressed (`lib.c:173`) | `err_e11_e15_complexmode_invalid_mode` | [x] |
| E12 | `complexmode` mode 2 | `log_message == NULL \|\| strcmp(log_message, "") == 0` (`lib.c:131`) | prints `Log message creation failed\n` instead of `Mode 2: <msg>\n`, and does **not** `free(log_message)`; the return value is whatever `multiply_with_log` produced | `err_e12_complexmode_mode2_log_failure` (OOM subprocess) | [x] |
| E13 | `copy_and_sum` | `count == 0` with non-`NULL` `src` — boundary, *not* rejected: glibc `malloc(0)` yields a non-`NULL` pointer and the `for` body never executes (`lib.c:82`) | returns `0`; prints nothing | `err_e13_copy_and_sum_zero_count` | [x] |
| E14 | `check_permissions` | `required == 0` — boundary, *never* rejects, since `(perms & 0) == 0` holds for every `perms` (`lib.c:48`) | returns `1` for all `perms`, including `0` and negative | `err_e14_check_permissions_zero_required` | [x] |
| E15 | `complexmode` | out-of-range values for the `mode` "enum" crossing the FFI boundary: `0`, `-1`, `5`, `INT_MIN`, `INT_MAX`, and randomized values outside `1..=4` | all fall through to `default:` → `Invalid mode\n`, `-1` | `err_e11_e15_complexmode_invalid_mode` | [x] |
| E16 | `create_result_string` | `op == NULL` — there is **no** null check; the pointer goes straight into `snprintf("%s")` (`lib.c:43`) | glibc substitutes the literal `(null)`: buffer becomes `Operation: (null), Value: <val>`; returns non-`NULL` | `err_e16_create_result_string_null_op` | [x] |
| E17 | `create_result_string` | `op` long enough that `"Operation: %s, Value: %d"` exceeds the 64-byte buffer → `snprintf` truncates (`lib.c:43`) | exactly 63 bytes + `NUL`, no overflow, returns non-`NULL` | `err_e17_create_result_string_truncation` | [x] |
| E18 | `safe_add` | `perms == 0` and `perms == -1` (all bits set) — the two extremes of the permission check | `0` → reject path (E2); `-1` → accept path, returns `a+b` | `err_e2_safe_add_insufficient_perms` | [x] |
| E19 | `copy_and_sum` | `count == INT_MIN` — the most negative `count`; `(size_t)INT_MIN * 4 == 0xFFFF_FFFE_0000_0000` | prints `Memory allocation failed\n`; returns `-1` | `err_e6_copy_and_sum_alloc_failure` | [x] |
| E20 | `compare_operations` | non-`NULL` but degenerate strings: `""` vs `""`, `""` vs non-empty, and bytes `>= 0x80` (`strcmp` must compare as *unsigned* char) | the exact `strcmp` value from the shared libc, sign included | `err_e20_compare_operations_degenerate` | [x] |

All 20 rows are checked off — see `tests/phase_c_errors.rs`.
