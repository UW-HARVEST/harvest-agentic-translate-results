# ERRORS.md — Phase C error / rejection surface table

Mechanically derived from every early `return`, every conditional rejection,
every implicit-failure path and every boundary constant in
`c_src/src/lib.c`. The C source contains **no** `assert`, no error enum, no
`RETURN_ERROR`-style macro and no function that returns a pointer, so the
complete rejection vocabulary is:

* `parse_env_numeric` returning `default_val` instead of a parsed value
  (3 distinct triggers, 2 of them also emitting a `stderr` diagnostic),
* `atoi` silently yielding `0` / a truncated value for unparseable or
  out-of-range text,
* `envy`'s `if (result < 0)` roll-back path (the function's only recovery
  branch),
* dereferencing a caller-supplied pointer without any null check
  (4 sites — the C code validates *nothing*).

Grep evidence:

```
$ grep -n 'return\|if (\|assert' c_src/src/lib.c   # 3 early returns in parse_env_numeric,
                                                   # 1 roll-back branch in envy, 0 asserts
$ grep -n 'NULL' c_src/src/lib.c                   # only *inputs from getenv/strchr* are
                                                   # null-checked, never the parameters
```

| #  | function | trigger (the exact invalid input/condition) | expected C result | test | ✔ |
|----|----------|----------------------------------------------|-------------------|------|---|
| 1  | `parse_env_numeric` | `getenv(env_name) == NULL` — variable not present in the environment (`lib.c:50`) | returns `default_val` verbatim; **no** output on stdout/stderr | `err_01_missing_env_returns_default` | [x] |
| 2  | `parse_env_numeric` | `env_name` names an **empty** variable (`""`): `getenv` returns a non-NULL empty string, `strchr` finds nothing, `atoi("")` | returns `0` (not `default_val`); no output | `err_02_empty_env_value_is_atoi_zero` | [x] |
| 3  | `parse_env_numeric` | value contains `,` anywhere (`strchr(env_value, ',') != NULL`, `lib.c:54`) | returns `default_val` **and** writes `"Warning: Invalid character in <name>\n"` to `stderr` | `err_03_comma_rejected_with_warning` | [x] |
| 4  | `parse_env_numeric` | value contains `;` and no `,` (`strchr(env_value, ';') != NULL`, `lib.c:60`) | returns `default_val` **and** writes `"Warning: Semicolon found in <name>\n"` to `stderr` | `err_04_semicolon_rejected_with_warning` | [x] |
| 5  | `parse_env_numeric` | value contains **both** `,` and `;` — the comma check runs first and short-circuits | `default_val` + only the *"Invalid character"* warning (never the semicolon one) | `err_05_comma_wins_over_semicolon` | [x] |
| 6  | `parse_env_numeric` | value is non-numeric text (`"abc"`, `"+"`, `"-"`, `" "`, `"0x10"`, `"1 2"`) — `atoi` has no error channel | `atoi`'s value (`0`, or the leading run of digits); `default_val` is *not* used | `err_06_unparseable_value_falls_through_to_atoi` | [x] |
| 7  | `parse_env_numeric` | value overflows `int` (`"2147483648"`, `"-2147483649"`, `"99999999999999999999"`) — `atoi` overflow is UB, glibc saturates via `strtol` then truncates | whatever glibc `atoi` returns; Rust must delegate to the **same** `atoi` | `err_07_int_overflow_in_atoi` | [x] |
| 8  | `parse_env_numeric` | `env_name` is the empty string `""` — `getenv("")` finds nothing | returns `default_val` | `err_08_empty_env_name` | [x] |
| 9  | `parse_env_numeric` | `default_val` at the extremes (`INT_MIN`, `INT_MAX`, `0`, `-1`) combined with triggers 1/3/4 | the extreme value is returned unchanged (no clamping anywhere in the C) | `err_09_extreme_default_val_passthrough` | [x] |
| 10 | `parse_env_numeric` | `env_name == NULL` | glibc `getenv(NULL)` dereferences → **SIGSEGV**. The C performs no null check, so this is UB in C and in Rust | `err_10_null_env_name_crashes_identically` (run in a forked child; both sides must die on the same signal) | [x] |
| 11 | `init_config_from_env` | `flags == NULL` — written through with no null check (`lib.c:74`) | **SIGSEGV** | `err_11_null_flags_init_crashes_identically` (forked child) | [x] |
| 12 | `perform_operation` | `flags == NULL` — read through with no null check (`lib.c:87`) | **SIGSEGV** | `err_12_null_flags_perform_crashes_identically` (forked child) | [x] |
| 13 | `apply_bit_operations` | `flags == NULL` — read through with no null check (`lib.c:104`) | **SIGSEGV** | `err_13_null_flags_apply_crashes_identically` (forked child) | [x] |
| 14 | `envy` | `result < 0` after all adjustments (`lib.c:171`) | roll-back: `memcpy` the backup over `state`, then `result = state.base_value`, i.e. the returned value becomes **`param1`** | `err_14_negative_result_rolls_back_to_param1` | [x] |
| 15 | `envy` | `result < 0` **and** `param1` itself negative — the roll-back does *not* re-check, so a negative value is returned | returns the negative `param1` unchanged | `err_15_rollback_can_return_negative` | [x] |
| 16 | `envy` | `param3 == 0` — the multiplier contribution is skipped entirely (`lib.c:145`), which is *not* the same as multiplying by 0 when `multiplier` is garbage | `result` unchanged by that block | `err_16_param3_zero_skips_block` | [x] |
| 17 | `envy` | `param4 == 0` — the shift contribution is skipped (`lib.c:149`) | `result` unchanged by that block | `err_17_param4_zero_skips_block` | [x] |
| 18 | `envy` | signed-integer overflow in `val1 * log_level`, `param3 * multiplier`, `result + param3*mult`, `result + base_offset`, `adjusted << 1` (all UB in C; gcc at `-O0` wraps two's-complement) | wrapped two's-complement result — Rust must use `wrapping_*`, never panic | `err_18_signed_overflow_wraps_everywhere` | [x] |
| 19 | `envy` | `param4 < 0` → `param4 >> 2` is an *arithmetic* (sign-propagating) shift on gcc, not logical | floor-division-by-4 semantics, e.g. `-1 >> 2 == -1` | `err_19_negative_right_shift_is_arithmetic` | [x] |
| 20 | `envy` | `param2 == INT_MIN` → `val2 / 2` in the non-optimize branch | `-1073741824` (truncation toward zero, no `INT_MIN / -1` trap) | `err_20_int_min_division` | [x] |
| 21 | `envy` | `PROG_BASE_OFFSET` / `PROG_MULTIPLIER` rejected (comma/semicolon) while `PROG_VERBOSE` is on | the `stderr` warnings and the stdout verbose lines must interleave exactly as C does, and the octal defaults `0100`=64 / `012`=10 must be used | `err_21_rejected_env_with_verbose_output` | [x] |
| 22 | `envy` | `PROG_MULTIPLIER` set to `INT_MIN`/`INT_MAX` so `param3 * multiplier` overflows hard | wrapped result | `err_22_extreme_multiplier_overflow` | [x] |
| 23 | all four `flags`-taking entry points | `struct ConfigFlags` bytes 1..3 contain garbage (only byte 0 holds the six bit-fields on x86-64 SysV) | the garbage must be **ignored** on read and **preserved** on write; `init_config_from_env` must not clobber bytes 1..3 | `err_23_padding_bytes_ignored_and_preserved` | [x] |
| 24 | `perform_operation`, `apply_bit_operations` | out-of-range "enum"/flag value: a `ConfigFlags` byte 0 with *every* one of the 256 possible bit patterns, including `log_level` values `4..7` that `init_config_from_env` can never produce and `reserved == 1` | value is masked to the declared bit widths; behaviour must match for all 256 patterns | `err_24_all_256_flag_bit_patterns` | [x] |
| 25 | `envy` | `PROG_OPTIMIZE` set to the **empty string** — the C tests only `!= NULL`, so `optimize` becomes 1 even though the value is empty/falsy | takes the `val1 + val2` branch | `err_25_empty_prog_optimize_still_enables` | [x] |
| 26 | `envy` | `PROG_VERBOSE` / `PROG_DEBUG` set but *without* the character `'1'` (`"0"`, `"true"`, `"yes"`, `""`) — the C uses `strchr(v,'1')`, not a truth test | flag stays 0, no output | `err_26_verbose_debug_require_literal_one` | [x] |
| 27 | `envy` | `PROG_VERBOSE` set to a value where `'1'` appears in an unexpected place (`"x1"`, `"31337"`, `"0001"`) | flag becomes 1 (substring match, not equality) | `err_27_one_anywhere_enables_flag` | [x] |

## Status

**27/27 rows have a passing differential test.** Row *N* is covered by the test
named `err_NN_…` in `tests/error_paths.rs`, plus two extra generic sweeps
(`generic_scalar_boundary_sweep`, `generic_misaligned_flags_pointer`) for the
boundaries every C API has. The row↔test mapping is checked mechanically:

```
ERRORS.md rows: 27  ->  err_ tests in tests/error_paths.rs: 27   MATCH
```

Each test asserts the C and Rust libraries reject the input the *same specific
way* — the same returned sentinel **and** the same diagnostic bytes on stderr —
and then additionally pins the actual C-defined value, so a future edit that made
both sides wrong in the same way would still fail. Rows 10–13 (the pointer rows,
where the C validates nothing) are compared by running the call in a forked child
and requiring death by the same signal.

Rows 11–13 initially **FAILED**: C died with `SIGSEGV` while Rust died with
`SIGABRT`. That was a genuine translation divergence; see `VERIFICATION.md` for
the root cause and fix.
