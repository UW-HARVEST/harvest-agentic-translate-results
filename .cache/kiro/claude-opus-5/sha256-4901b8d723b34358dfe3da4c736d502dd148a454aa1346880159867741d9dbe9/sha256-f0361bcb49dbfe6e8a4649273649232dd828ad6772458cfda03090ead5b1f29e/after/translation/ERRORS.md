# ERRORS.md — Phase A error / rejection surface

Derived mechanically from `c_src/src/lib.c`. Grep audit performed:

```sh
grep -n 'return\|assert\|NULL\|== 0\|!= 0\|< 0\|> \|#define\|exit\|abort' c_src/src/lib.c
```

Findings of that audit, stated up front because they shape the table:

* There is **no** `RETURN_ERROR`-style macro, **no** error enum, **no**
  `return -1`, **no** `return NULL`, **no** `assert`, **no** `exit`/`abort`,
  and **no** explicit numeric range check anywhere in `src/lib.c`.
* The only size constant is `#define BUFFER_SIZE 256`, used solely as the
  `snprintf` bound in `envy`.
* Every rejection in this library is therefore one of three shapes:
  (a) a null / sentinel check on a `getenv` or `strchr` result that falls back
  to a **default value**, (b) a `!= 0` / truthiness guard that **skips** a
  computation, or (c) the `result < 0` **recovery** path in `envy` that
  discards the computed result.
* All three functions taking a `struct ConfigFlags*` dereference it with no
  null check, so a null pointer is undefined behaviour in the C (documented in
  rows 15–17 and asserted to be *equally* fatal, not "both failed somehow").

Rejection rows — one per distinct branch that rejects, discards, or falls back:

| #  | function | trigger (the exact invalid input/condition) | expected C result | test | ✔ |
|----|----------|---------------------------------------------|-------------------|------|---|
| 1  | `parse_env_numeric` | `getenv(env_name) == NULL` (variable not present in environment) | returns `default_val`, nothing written to stderr | `err_01_missing_env_returns_default` | [x] |
| 2  | `parse_env_numeric` | value contains a comma, e.g. `"1,2"` — `strchr(env_value, ',') != NULL` | returns `default_val`; writes `Warning: Invalid character in <name>\n` to stderr | `err_02_comma_returns_default` | [x] |
| 3  | `parse_env_numeric` | value contains a semicolon, e.g. `"1;2"` — first `strchr` misses, second `strchr(env_value, ';') != NULL` | returns `default_val`; writes `Warning: Semicolon found in <name>\n` to stderr | `err_03_semicolon_returns_default` | [x] |
| 4  | `parse_env_numeric` | value contains BOTH `,` and `;` (`",;"`, `";,"`) — comma branch is checked first and wins | returns `default_val`; the **comma** warning only | `err_04_comma_and_semicolon_comma_wins` | [x] |
| 5  | `parse_env_numeric` | value is the empty string `""` (non-NULL, no `,`, no `;`) — reaches `atoi("")` | returns `0`, **not** `default_val` | `err_05_empty_value_is_zero_not_default` | [x] |
| 6  | `parse_env_numeric` | value is non-numeric garbage (`"abc"`, `"+"`, `"-"`, `"0x10"`, `" \t"`) — reaches `atoi` | returns whatever `atoi` yields (`0` for these), **not** `default_val` | `err_06_non_numeric_atoi_result` | [x] |
| 7  | `parse_env_numeric` | value overflows `int` (`"99999999999999"`, `"-99999999999999"`, `"2147483648"`) — `atoi` out-of-range | returns the same truncated value glibc's `atoi` produces | `err_07_atoi_overflow` | [x] |
| 8  | `parse_env_numeric` | `default_val` itself is a boundary value (`INT_MIN`, `INT_MAX`, `-1`, `0`) and the variable is absent | returns that exact `default_val` unchanged | `err_08_default_val_boundaries` | [x] |
| 9  | `envy` | computed `result < 0` after `+= base_offset` | discards the computed result: `memcpy` restores `state_backup`, returns `state.base_value` i.e. `param1` | `err_09_negative_result_recovery` | [x] |
| 10 | `envy` | `param3 == 0` | the `param3 * state.multiplier` term is **skipped entirely** (not added as 0 — observable when `multiplier` is such that the product would still be 0, and distinguishes skip vs. add) | `err_10_param3_zero_skips_term` | [x] |
| 11 | `envy` | `param4 == 0` | the `param4 >> 2` term is **skipped entirely** | `err_11_param4_zero_skips_term` | [x] |
| 12 | `envy` | `PROG_BASE_OFFSET` / `PROG_MULTIPLIER` rejected by rows 1–4 | `base_offset` falls back to `0100` = 64, `multiplier` to `012` = 10; `envy` still returns a value (no error propagation) | `err_12_envy_uses_defaults_on_rejected_env` | [x] |
| 13 | `init_config_from_env` | `PROG_VERBOSE` present but contains no `'1'` (e.g. `"0"`, `"yes"`, `""`) — `strchr(...,'1') == NULL` | `verbose` bit cleared to 0 even though the variable is set | `err_13_verbose_present_without_1` | [x] |
| 14 | `init_config_from_env` | `PROG_DEBUG` present but contains no `'1'` | `debug` bit cleared to 0 | `err_14_debug_present_without_1` | [x] |
| 15 | `init_config_from_env` | `flags == NULL` | dereferences and writes through the null pointer: fatal (SIGSEGV). Rust must be **equally** fatal with the same signal | `err_15_null_flags_init` (subprocess, compares exit status/signal) | [x] |
| 16 | `perform_operation` | `flags == NULL` | reads through the null pointer: fatal (SIGSEGV) | `err_16_null_flags_perform` | [x] |
| 17 | `apply_bit_operations` | `flags == NULL` | reads through the null pointer: fatal (SIGSEGV) | `err_17_null_flags_apply` | [x] |
| 18 | `parse_env_numeric` | `env_name == NULL` | passed straight to glibc `getenv(NULL)`: fatal (SIGSEGV) | `err_18_null_env_name` | [x] |
| 19 | `parse_env_numeric` | `env_name` is `""` (zero-length name, never a real environment entry) | `getenv("")` returns NULL → returns `default_val` | `err_19_empty_env_name` | [x] |
| 20 | `parse_env_numeric` | `env_name` is oversized (a 4096-byte name that cannot exist in the environment) | `getenv` returns NULL → returns `default_val` | `err_20_oversized_env_name` | [x] |

## Generic FFI boundaries additionally covered (not distinct C branches)

| #  | boundary | what is asserted | test | ✔ |
|----|----------|------------------|------|---|
| 21 | Out-of-range "enum" values across FFI: `struct ConfigFlags` has no enum, but its 3-bit `log_level` field means the *storage word* accepts any `u32`. All 2^8 low-byte patterns **and** garbage in the 24 padding bits are pushed through `perform_operation` / `apply_bit_operations`. | C and Rust agree for every one of the 256 flag bytes, and neither is affected by the padding bits | `err_21_all_flag_bytes_and_padding_garbage` | [x] |
| 22 | `char` operation field / `state.operation = '+'` is written but never read | no observable difference; covered implicitly by every `envy` row | (implicit) | [x] |
| 23 | `int` extremes on every scalar parameter: `INT_MIN`, `INT_MIN+1`, `-1`, `0`, `1`, `INT_MAX-1`, `INT_MAX` for all four `envy` params and both `perform_operation` values (signed overflow / `<< 1` / `>> 2` on negatives / `INT_MIN / 2`) | byte-identical return values | `err_23_int_extremes` | [x] |
| 24 | `snprintf` truncation against `BUFFER_SIZE` = 256 | `"Result:%d:Complete"` is at most 29 bytes, so truncation is unreachable; the buffer is never read back for the return value | (unreachable — documented) | [x] |

## Gate

- [x] Every row above has a passing error-path differential test asserting the
      **same** value / sentinel / signal from both libraries.

## Result

All 24 rows pass. Tests live in `tests/phase_c_errors.rs` (rows 1–24; row 22 is
implicit and row 24 is an assertion that the truncation path is unreachable),
and every one drives both `.so` files through `libloading`.

### One divergence found and fixed

Rows 15–17 (null `struct ConfigFlags*`) diverged in the **debug** build profile:

```
DIVERGENCE [err15]: C died as Signaled(11), Rust died as Signaled(6)
```

The C dereferences the caller's pointer with no null check, so a null argument
is a hardware fault — SIGSEGV. The Rust used a plain `*(flags as *const u8)`
raw-pointer dereference, which rustc instruments under `debug_assertions` with
a null-pointer check that panics (`null pointer dereference occurred`) and, in
a `cdylib`, aborts — SIGABRT. The release `.so` matched, so this was invisible
until the suite was run against the debug-profile artifact.

Fix (`src/lib.rs`, `read_flags` / `write_flags`): use `core::ptr::read` /
`core::ptr::write` instead of `*ptr` / `*ptr = …`. Those carry no null check in
any profile and fault exactly like the C. Verified empirically that `*p`
aborts under `debug_assertions` while `ptr::read(p)` segfaults, in both
profiles. Reverting the fix reproduces the failure, so the guard is genuinely
tested rather than vacuous.
