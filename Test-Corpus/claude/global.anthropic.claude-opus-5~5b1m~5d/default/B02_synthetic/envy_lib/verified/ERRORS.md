# ERRORS.md — Phase C error-surface table

Mechanically derived from `c_src/src/lib.c`. This library has **no error enum,
no `RETURN_ERROR` macro, no `assert`, and no negative sentinel return**: every
public function returns an unconstrained `int` (or `void`). Its entire rejection
surface consists of

* the three early `return default_val` paths in `parse_env_numeric`,
* the `NULL` guards on `strchr` results,
* the `result < 0` recovery/rollback path in `envy`,

plus the generic FFI boundary conditions every C API has (null pointers,
out-of-range "enum"/bit-field values, extreme integers). All of these are
enumerated below; each row has a differential test.

## Table

| # | function | trigger (the exact invalid input/condition) | expected C result | test | [x] |
|---|----------|----------------------------------------------|-------------------|------|-----|
| E1 | `parse_env_numeric` | `getenv(env_name) == NULL` (variable unset) | returns `default_val` verbatim; **no** output written | `err_e1_unset_returns_default` | [x] |
| E2 | `parse_env_numeric` | value contains `','` (`strchr(v, ',') != NULL`) | `fprintf(stderr, "Warning: Invalid character in %s\n", env_name)` then returns `default_val` | `err_e2_comma_rejected` | [x] |
| E3 | `parse_env_numeric` | value contains `';'` and **no** `','` | `fprintf(stderr, "Warning: Semicolon found in %s\n", env_name)` then returns `default_val` | `err_e3_semicolon_rejected` | [x] |
| E4 | `parse_env_numeric` | value contains **both** `','` and `';'` — comma check runs first | only the *comma* warning is emitted (semicolon check unreachable); returns `default_val` | `err_e4_comma_wins_over_semicolon` | [x] |
| E5 | `parse_env_numeric` | value set but empty string `""` | no `,`/`;` found → `atoi("") == 0`; returns **0**, *not* `default_val` | `err_e5_empty_string_is_zero` | [x] |
| E6 | `parse_env_numeric` | value is non-numeric garbage (`"abc"`, `"--3"`, `"+"`, `"0x1f"`) | `atoi` returns its own parse result (`0`, `0`, `0`, `0`) — no rejection | `err_e6_garbage_atoi_passthrough` | [x] |
| E7 | `parse_env_numeric` | value has leading whitespace / sign / trailing junk (`"  -42abc"`) | `atoi` skips space, parses `-42`, ignores junk | `err_e7_atoi_leading_trailing` | [x] |
| E8 | `parse_env_numeric` | value overflows `int` (`"99999999999999999999"`, `"-99999999999999999999"`) | UB in ISO C; both libs call the **same** libc `atoi`, so results must be identical | `err_e8_atoi_overflow` | [x] |
| E9 | `parse_env_numeric` | `default_val` at extremes `INT_MIN` / `INT_MAX` / `0` while var unset | returns that exact value (no clamping) | `err_e9_default_extremes` | [x] |
| E10 | `parse_env_numeric` | `env_name` is an empty string `""` | `getenv("")` returns `NULL` → `default_val` | `err_e10_empty_env_name` | [x] |
| E11 | `parse_env_numeric` | `env_name == NULL` | glibc `getenv(NULL)` dereferences the name → fatal signal. Tested for real: both libraries must die from the **same** signal, checked in a forked child. | `err_e11_null_env_name_faults_identically` | [x] |
| E12 | `init_config_from_env` | `PROG_VERBOSE` set but with **no** `'1'` anywhere (`"true"`, `"yes"`, `"0"`) → `strchr(v,'1') == NULL` | `verbose` bit cleared to 0 | `err_e12_verbose_needs_literal_1` | [x] |
| E13 | `init_config_from_env` | `PROG_DEBUG` set but with no `'1'` → `strchr(v,'1') == NULL` | `debug` bit cleared to 0 | `err_e13_debug_needs_literal_1` | [x] |
| E14 | `init_config_from_env` | `PROG_OPTIMIZE` set to the **empty string** `""` (falsy-looking but non-NULL) | `optimize` bit **set to 1** (only NULL-ness is tested, not content) | `err_e14_optimize_empty_is_true` | [x] |
| E15 | `init_config_from_env` | `flags` points at memory pre-filled `0xFF` (dirty non-zero padding) | byte 0 fully rewritten by byte-sized RMW; bytes 1..3 left as `0xFF` | `err_e15_dirty_padding_preserved` | [x] |
| E16 | `perform_operation` | `flags->optimize == 0` and `flags->log_level == 0` (out-of-"range"-looking zero multiplier) | `result = val1*0 + val2/2 = val2/2` (C truncating division) | `err_e16_log_level_zero` | [x] |
| E17 | `perform_operation` | `flags` bit-field byte 0 = **every** value `0x00..0xFF`, i.e. every out-of-range `log_level` 0..7 incl. values no named constant produces | no validation exists; `log_level` promotes to `int` 0..7 and multiplies | `err_e17_all_256_flag_bytes` | [x] |
| E18 | `perform_operation` | signed overflow: `val1 = INT_MAX`, `log_level = 7` (`val1*7` overflows) | UB in ISO C; must match gcc's wrapping result | `err_e18_signed_overflow_mul` | [x] |
| E19 | `perform_operation` | `val2 = INT_MIN` → `val2 / 2` (negative truncating division rounds toward zero) | `INT_MIN/2 == -1073741824` | `err_e19_negative_division` | [x] |
| E20 | `perform_operation` | `val2 = -1` → `-1/2 == 0` (truncation toward zero, **not** floor `-1`) | `0` | `err_e20_minus_one_div_two` | [x] |
| E21 | `apply_bit_operations` | `verbose == 1` and `value` has bit 30/31 set → `value << 1` overflows a signed `int` | UB in ISO C; must match gcc's wrapping (2's-complement) shift | `err_e21_shift_overflow` | [x] |
| E22 | `apply_bit_operations` | `verbose == 1` and `value < 0` → left shift of a **negative** value | UB in ISO C; must match gcc | `err_e22_shift_negative` | [x] |
| E23 | `apply_bit_operations` | `cache_enabled == 0` (the only way to skip the `\| 0x0F`) | returns `adjusted` **without** the low-nibble OR | `err_e23_cache_disabled` | [x] |
| E24 | `envy` | final `result < 0` → rollback branch: `memcpy(&state,&state_backup,16)`; `result = state.base_value` | returns `param1` (the original base value), discarding all arithmetic | `err_e24_negative_result_rollback` | [x] |
| E25 | `envy` | `param3 == 0` — the guard that *skips* `result += param3 * multiplier` | multiplier contribution omitted (indistinguishable from `*0` but a distinct branch) | `err_e25_param3_zero_branch` | [x] |
| E26 | `envy` | `param4 == 0` — the guard that *skips* `result += param4 >> 2` | shift contribution omitted | `err_e26_param4_zero_branch` | [x] |
| E27 | `envy` | `param4 < 0` → `param4 >> 2` right-shift of a negative `int` (impl-defined) | gcc emits an **arithmetic** shift (sign-extending) | `err_e27_negative_right_shift` | [x] |
| E28 | `envy` | all four params at `INT_MIN` / `INT_MAX` boundaries (overflow in `+`, `*`) | UB in ISO C; must match gcc's wrapping | `err_e28_param_extremes` | [x] |
| E29 | `envy` | `PROG_BASE_OFFSET` / `PROG_MULTIPLIER` rejected (comma/semicolon) → defaults `0100`/`012` used **and** warnings go to stderr | `base_offset == 64`, `multiplier == 10`, 2 stderr warnings | `err_e29_envy_with_rejected_env` | [x] |
| E30 | `envy` | `strchr(buffer, ':')` / `strchr(colon_pos+1, ':')` NULL guards | unreachable in practice: `snprintf` always writes `"Result:%d:Complete"` (max 29 < 256), so both colons always exist and no truncation occurs | `err_e30_colons_always_present` | [x] |
| E31 | `envy` | `result` exactly `0` (boundary of the `result < 0` test — `0` must **not** roll back) | no rollback; returns `0` | `err_e31_result_zero_boundary` | [x] |
| E32 | `envy` | `result` exactly `-1` (one step past the boundary → rolls back) | rollback; returns `param1` | `err_e32_result_minus_one_boundary` | [x] |
| E33 | all | out-of-range "enum" value across FFI: `ConfigFlags` byte 0 given all 256 bit patterns (values no valid named combination produces, e.g. `reserved = 1`) | no validation; bits are used as-is | `err_e33_out_of_range_flag_bits` | [x] |
| E34 | `init_config_from_env`, `perform_operation`, `apply_bit_operations` | `flags == NULL` (all three consumers) | gcc simply performs the load/store → **SIGSEGV**. Tested for real in a forked child: the C and Rust termination signals must be equal. **This row found a genuine bug** — see below. | `err_e34_null_flags_pointer_faults_identically` | [x] |
| E35 | `parse_env_numeric` | value is a very long string (255+ bytes, no `,`/`;`) — no length limit exists in C | no truncation, no rejection; `atoi` on the whole string | `err_e35_very_long_value` | [x] |
| E36 | `envy` | `PROG_BASE_OFFSET` / `PROG_MULTIPLIER` set to `INT_MIN`/`INT_MAX` (oversized offsets) driving `result` overflow | UB in ISO C; must match gcc's wrapping | `err_e36_env_extremes` | [x] |

## Notes on non-rejections (deliberate, do not "fix")

* `parse_env_numeric` rejects `,` and `;` **only**. Every other character —
  including `|`, `&`, newline, NUL-adjacent junk — is passed straight to `atoi`.
* The comma check precedes the semicolon check, so a value containing both
  produces *only* the "Invalid character" warning (row E4).
* An env var set to `""` is **not** treated as unset: `parse_env_numeric`
  returns `0` (E5) and `init_config_from_env` sets `optimize = 1` (E14).
* `init_config_from_env` requires a literal `'1'` *character* for
  verbose/debug, so `PROG_VERBOSE=true` is false but `PROG_VERBOSE=xx1xx` is
  true (E12/E13).
* `log_level` is hard-coded to `03` (== 3) by `init_config_from_env`; it is only
  ever something else when a caller pokes `ConfigFlags` directly (E17/E33).

## Resolution of the two rows originally marked "excluded"

Rows **E11** (`parse_env_numeric(NULL, d)`) and **E34** (`flags == NULL`) were
initially written off as "crashes both". That was too weak, so they are now
covered by *real* differential tests instead: `fork_outcome()` in
`tests/common/mod.rs` runs the call in a forked child and reports whether the
child returned a value or died from a signal, and the tests assert the C and
Rust outcomes are **equal** — the same signal, not merely "both failed".

This immediately paid off. `init_config_from_env`, `perform_operation` and
`apply_bit_operations` originally reached the bit-fields via
`let flags = &mut *flags;` / `&*flags`. Forming a Rust reference from a
caller-supplied pointer attaches a validity guarantee that a C caller does not
provide, and in the **debug** profile (`debug_assertions` on) the null case
aborted with **SIGABRT** where the C library takes **SIGSEGV**. The accessors
were rewritten as raw-pointer read/write helpers (`cf_get`/`cf_set`/…) that
never create a reference; both profiles now fault identically. See
`VERIFICATION.md` bug #2.

`E11`/`E34` are therefore **[x] tested**, not excluded:
`err_e11_null_env_name_faults_identically`,
`err_e34_null_flags_pointer_faults_identically`.
