# ERRORS.md — Error / rejection surface table (Phase C)

Derived mechanically from `c_src/src/lib.c`. The library has **no error enum and
no error return codes**: every rejection path is either "return the caller's
`default_val`", "print a warning to `stderr`", "take the state-restore branch",
or "undefined behaviour / crash" (NULL pointer dereference). There is not a
single `assert`, `return -1`, `return NULL`, or explicit range check in the file
— the table below is the exhaustive list of every *early return*, *rejection*,
*NULL check*, *conditional guard* and *magic constant boundary* the C actually
contains, plus the generic FFI boundaries the task requires.

Greps used:

```
grep -n 'return'  c_src/src/lib.c   # 7 return statements total
grep -n 'NULL'    c_src/src/lib.c   # 10 NULL comparisons
grep -n 'assert'  c_src/src/lib.c   # 0 hits
grep -n 'if ('    c_src/src/lib.c   # 15 branches
```

| #  | function | trigger (the exact invalid input/condition) | expected C result | test | ✔ |
|----|----------|---------------------------------------------|-------------------|------|---|
| 1  | `parse_env_numeric` | `getenv(env_name) == NULL` (variable unset) — `lib.c:50` | returns `default_val` verbatim, **no** output | `err_01_unset_var_returns_default` | [x] |
| 2  | `parse_env_numeric` | value contains `','` — `lib.c:54-58` | `fprintf(stderr, "Warning: Invalid character in %s\n", env_name)` then returns `default_val` | `err_02_comma_returns_default_and_warns` | [x] |
| 3  | `parse_env_numeric` | value contains `';'` (and no `','`) — `lib.c:60-64` | `fprintf(stderr, "Warning: Semicolon found in %s\n", env_name)` then returns `default_val` | `err_03_semicolon_returns_default_and_warns` | [x] |
| 4  | `parse_env_numeric` | value contains **both** `','` and `';'` | comma check runs first ⇒ *only* the "Invalid character" warning, returns `default_val` | `err_04_comma_wins_over_semicolon` | [x] |
| 5  | `parse_env_numeric` | value is `','` / `';'` alone, or has the char in first/last position | same as rows 2/3 (`strchr` is position-independent) | `err_05_separator_positions` | [x] |
| 6  | `parse_env_numeric` | value is set but **not numeric** (`"abc"`, `""`, `" "`, `"+-3"`) | `atoi` returns `0` — value is *accepted*, `default_val` is **not** used | `err_06_non_numeric_atoi_zero` | [x] |
| 7  | `parse_env_numeric` | value numerically **out of `int` range** (`"99999999999999"`, `"-99999999999999"`, `"2147483648"`) | `atoi` overflow ⇒ glibc-defined result (UB in ISO C); Rust must match byte-for-byte | `err_07_atoi_overflow` | [x] |
| 8  | `parse_env_numeric` | `env_name` is a `NULL` pointer | `getenv(NULL)` dereferences NULL ⇒ fatal signal; must be the **same** signal in both | `err_08_null_env_name_same_signal` | [x] |
| 9  | `parse_env_numeric` | `env_name` names a variable with an empty name / not present (`""`) | `getenv("")` ⇒ `NULL` ⇒ returns `default_val` | `err_09_empty_name_returns_default` | [x] |
| 10 | `init_config_from_env` | `flags` is a `NULL` pointer | write through NULL ⇒ fatal signal; must match | `err_10_init_null_flags_same_signal` | [x] |
| 11 | `init_config_from_env` | `PROG_VERBOSE` set but does **not** contain `'1'` (`"yes"`, `"0"`, `""`) | `verbose` bit rejected ⇒ `0` | `err_11_verbose_without_one_rejected` | [x] |
| 12 | `init_config_from_env` | `PROG_DEBUG` set but does **not** contain `'1'` | `debug` bit rejected ⇒ `0` | `err_12_debug_without_one_rejected` | [x] |
| 13 | `init_config_from_env` | `PROG_OPTIMIZE` set to the **empty string** | *presence-only* test ⇒ `optimize = 1` (empty is **not** rejected) | `err_13_optimize_empty_is_set` | [x] |
| 14 | `perform_operation` | `flags` is a `NULL` pointer | read through NULL ⇒ fatal signal; must match | `err_14_perform_null_flags_same_signal` | [x] |
| 15 | `perform_operation` | `flags->optimize == 0` **and** `flags->log_level` out of the "expected" value `3` — i.e. any of the 8 values a 3-bit field can hold, incl. `0` and `7` | no check at all: `val1 * log_level + val2/2` is computed with whatever the field holds | `err_15_log_level_full_range` | [x] |
| 16 | `perform_operation` | signed overflow: `val1 + val2` or `val1 * log_level` or `+ val2/2` overflowing `int` (`INT_MAX`, `INT_MIN`) | UB in ISO C; gcc wraps (two's complement). Rust must produce the identical wrapped value | `err_16_perform_signed_overflow` | [x] |
| 17 | `perform_operation` | `INT_MIN / 2` (`val2 == INT_MIN`) | truncate-toward-zero division ⇒ `-1073741824` (no trap: divisor is the constant 2) | `err_17_int_min_div_two` | [x] |
| 18 | `apply_bit_operations` | `flags` is a `NULL` pointer | read through NULL ⇒ fatal signal; must match | `err_18_bitops_null_flags_same_signal` | [x] |
| 19 | `apply_bit_operations` | `verbose == 1` and `value << 1` overflows (`value >= 0x40000000` or negative) | UB in ISO C; gcc emits a plain `shl` ⇒ bit-pattern shift. Rust must match | `err_19_shift_overflow` | [x] |
| 20 | `apply_bit_operations` | `value == INT_MIN`, `verbose == 1` | `INT_MIN << 1 == 0`, then `| 0x0F` ⇒ `15` | `err_20_int_min_shift` | [x] |
| 21 | `envy` | computed `result < 0` — `lib.c:171` | state is `memcpy`-restored from the backup and `result` becomes `state.base_value` (== `param1`) | `err_21_negative_result_restores_base` | [x] |
| 22 | `envy` | `result < 0` **and** `param1 < 0` | the restore branch is **not** re-checked ⇒ `envy` returns a **negative** value (`param1`) | `err_22_restore_can_return_negative` | [x] |
| 23 | `envy` | `param3 == 0` (guard at `lib.c:145`) | the `param3 * multiplier` term is skipped entirely | `err_23_param3_zero_skips_term` | [x] |
| 24 | `envy` | `param4 == 0` (guard at `lib.c:149`) | the `param4 >> 2` term is skipped entirely | `err_24_param4_zero_skips_term` | [x] |
| 25 | `envy` | `param4 < 0` ⇒ `param4 >> 2` on a negative `int` | implementation-defined; gcc = arithmetic shift (rounds toward −∞) | `err_25_negative_param4_arith_shift` | [x] |
| 26 | `envy` | `strchr(buffer, ':') == NULL` — `lib.c:160` | unreachable in practice (`snprintf` always writes `"Result:"`), but the guard must not change behaviour; verified via the full-verbose output comparison | `err_26_colon_guard_always_taken` | [x] |
| 27 | `envy` | `second_colon == NULL` — `lib.c:166` | unreachable in practice; `"Debug: Result string format validated"` is therefore always printed when `debug` is set | `err_27_second_colon_guard` | [x] |
| 28 | `envy` | `snprintf` truncation boundary: `BUFFER_SIZE == 256` vs the longest possible `"Result:-2147483648:Complete"` (28 bytes) | never truncates; return value ignored by C | `err_28_no_snprintf_truncation` | [x] |
| 29 | `envy` | `PROG_BASE_OFFSET` / `PROG_MULTIPLIER` rejected (rows 1–5) | the **octal** defaults `0100 == 64` and `012 == 10` are used | `err_29_octal_defaults_on_rejection` | [x] |
| 30 | `envy` | extreme `param1..param4` (`INT_MIN`/`INT_MAX`) combined with an overflowing `PROG_MULTIPLIER` | every arithmetic step wraps; both libraries must agree exactly | `err_30_envy_extreme_overflow` | [x] |
| 31 | all | out-of-range "enum"/flag values across the FFI boundary: the `ConfigFlags` allocation unit is 4 bytes but only bits 0..7 are declared. Passing a unit with **all 32 bits set** (`0xFFFFFFFF`) or arbitrary garbage in bits 8..31 | only bits 0..7 are consulted; bits 8..31 are ignored and left untouched by `init_config_from_env` | `err_31_garbage_upper_bits_ignored` | [x] |
| 32 | all | zero-length / oversized *lengths* — the API takes no length arguments (grep: no `size_t` parameter anywhere), so the only analogous boundary is the fixed `BUFFER_SIZE` of row 28 | n/a — documented, covered by row 28 | `err_28_no_snprintf_truncation` | [x] |

## Divergences found and fixed (Rust side only; `c_src/` untouched)

| rows | symptom | root cause | fix |
|------|---------|------------|-----|
| 10, 14, 18 | passing a `NULL struct ConfigFlags*` killed the C library with **SIGSEGV** but the Rust library with **SIGABRT** (only in a `debug-assertions` build, so a release-only test run would have missed it) | the translation formed a Rust reference (`&*flags` / `&mut *flags`) from the caller's raw pointer. Reference creation — and even a plain `*p` deref — is instrumented with a null check by rustc when debug assertions are on; the resulting panic escaping an `extern "C"` function is turned into `abort()` | the bit-fields are now read/written through `core::ptr::read_volatile` / `write_volatile` on the allocation unit's byte 0 (`bf_get` / `bf_set` in `src/lib.rs`). No reference is ever formed, no check is inserted, and the single load/store faults exactly like the C's — verified identical (`signal(11)`) under **both** Rust profiles |

Everything else matched byte-for-byte on the first run, including the `atoi`
overflow results, the truncate-toward-zero division, the arithmetic right shift
of a negative `param4`, the wrapped signed overflows and the exact warning text
on `stderr`.
