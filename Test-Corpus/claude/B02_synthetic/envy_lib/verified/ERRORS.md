# ERRORS.md — error / rejection surface of `c_src/src/lib.c`

Derived mechanically by grepping the C source for every `return` that is not the
"happy" tail return, every `!= NULL` / `== NULL` test, every explicit range or
value check, every constant limit (`BUFFER_SIZE`, bit-field widths), and every
place where libc can reject the input on the library's behalf (`getenv`,
`atoi`, `strchr`, `snprintf`).

There are **no** `assert`s, no error enums, and no negative error codes in this
library: rejections are expressed as *"fall back to the default / backup value"*
and, in two cases, as a warning line on `stderr`.

Notation: `flags` = `struct ConfigFlags*`; bit layout on x86-64 gcc is
`bit0 verbose, bit1 debug, bit2 optimize, bit3 cache_enabled, bits4-6 log_level,
bit7 reserved, bits8-31 padding`.

| #  | function | trigger (the exact invalid input/condition) | expected C result | test |
|----|----------|----------------------------------------------|-------------------|------|
| 1  | `parse_env_numeric` | `getenv(env_name) == NULL` (variable not present in the environment) | returns `default_val`; nothing printed | `err_01_env_absent_returns_default` |
| 2  | `parse_env_numeric` | env value contains `','` (`strchr(env_value, ',') != NULL`), anywhere in the string incl. first/last char | prints `Warning: Invalid character in <env_name>\n` to **stderr**, returns `default_val` | `err_02_comma_warns_and_returns_default` |
| 3  | `parse_env_numeric` | env value contains `';'` and **no** `','` | prints `Warning: Semicolon found in <env_name>\n` to **stderr**, returns `default_val` | `err_03_semicolon_warns_and_returns_default` |
| 4  | `parse_env_numeric` | env value contains **both** `','` and `';'` | comma check runs first ⇒ only the `Invalid character` warning, returns `default_val` | `err_04_comma_wins_over_semicolon` |
| 5  | `parse_env_numeric` | env value is not a number at all (`"abc"`, `"+"`, `"-"`, `"0x10"`, `" "`) | no warning; `atoi` returns `0` (glibc `atoi` = `(int)strtol(s,NULL,10)`) ⇒ returns `0`, *not* `default_val` | `err_05_non_numeric_atoi_zero` |
| 6  | `parse_env_numeric` | env value is empty string `""` | `getenv` returns a non-NULL empty string ⇒ no warning, `atoi("")==0` ⇒ returns `0` | `err_06_empty_value_is_zero` |
| 7  | `parse_env_numeric` | env value overflows `int`/`long` (`"9999999999"`, `"-9999999999"`, `"99999999999999999999"`) | UB per ISO C, but glibc clamps `strtol` to `LONG_MAX`/`LONG_MIN` then truncates to `int` (`-1` / `0`); the Rust port calls the *same* `atoi`, so both must agree | `err_07_atoi_overflow_truncation` |
| 8  | `parse_env_numeric` | env value has trailing garbage (`"12abc"`, `"12 34"`, `"12."`) | `atoi` stops at first non-digit ⇒ `12` | `err_08_trailing_garbage_prefix_parsed` |
| 9  | `parse_env_numeric` | `env_name` is a valid pointer to `""` (empty name) | `getenv("")` returns `NULL` ⇒ returns `default_val` | `err_09_empty_env_name` |
| 10 | `parse_env_numeric` | `env_name == NULL` | delegated to glibc `getenv(NULL)`, which dereferences the pointer ⇒ `SIGSEGV`; both libraries must fail the same way | `err_10_null_env_name_segv` (child process) |
| 11 | `init_config_from_env` | `flags == NULL` | unconditional write through the pointer ⇒ `SIGSEGV` | `err_11_null_flags_segv` (child process) |
| 12 | `perform_operation` | `flags == NULL` | unconditional read through the pointer ⇒ `SIGSEGV` | `err_12_null_flags_segv` (child process) |
| 13 | `apply_bit_operations` | `flags == NULL` | unconditional read through the pointer ⇒ `SIGSEGV` | `err_13_null_flags_segv` (child process) |
| 14 | `perform_operation` | `flags` holds an **out-of-range / undeclared bit pattern** (all 2^8 low-byte values + garbage in the 24 padding bits, e.g. `0xFFFFFFFF`): `log_level` can be `0..7` although `init_config_from_env` only ever writes `3`, and `reserved`/padding are never validated | no rejection: only `optimize` (bit2), `debug` (bit1) and `log_level` (bits4-6) are consulted; `log_level == 0` makes the non-optimized branch collapse to `val2/2` | `err_14_flag_bit_patterns` |
| 15 | `apply_bit_operations` | same out-of-range bit patterns | no rejection: only `verbose` (bit0) and `cache_enabled` (bit3) are consulted; result is always `| 0x0F` when bit3 set | `err_15_flag_bit_patterns` |
| 16 | `perform_operation` | `flags->optimize == 0` and `val1 * log_level + val2/2` overflows `int` (`val1 = INT_MAX`, `log_level = 7`) | signed overflow is ISO-C UB; gcc `-O0` wraps (two's complement) — the Rust port must wrap identically | `err_16_perform_operation_overflow` |
| 17 | `perform_operation` | `flags->optimize == 1` and `val1 + val2` overflows (`INT_MAX + 1`, `INT_MIN + -1`) | wraps two's complement | `err_17_add_overflow` |
| 18 | `perform_operation` | `val2 == INT_MIN` in `val2 / 2` (division of the most negative value) | `-1073741824` (truncation toward zero, no trap because the divisor is 2) | `err_18_val2_int_min_div` |
| 19 | `apply_bit_operations` | `verbose` set and `value` negative or `>= 2^30` ⇒ `value << 1` shifts into / past the sign bit | UB per ISO C; gcc emits a plain `shl` ⇒ bit-pattern wrap | `err_19_shift_sign_overflow` |
| 20 | `envy` | `param3 != 0` and `param3 * state.multiplier` overflows (`multiplier` is attacker-controlled through `PROG_MULTIPLIER`) | wraps two's complement | `err_20_param3_mul_overflow` |
| 21 | `envy` | `param4 == INT_MIN` (`param4 >> 2` on a negative value) | arithmetic shift ⇒ `-536870912` | `err_21_param4_arithmetic_shift` |
| 22 | `envy` | final `result < 0` (reachable e.g. with a large negative `PROG_BASE_OFFSET`, or by overflowing) | restores `state` from `state_backup` via `memcpy` and returns `state.base_value`, i.e. **`param1`**; extra `Restored state from backup\n` line when `verbose` | `err_22_negative_result_restores_backup` |
| 23 | `envy` | `result < 0` **and** `param1 < 0` | returns the negative `param1` unchanged (the fallback is not re-checked) | `err_23_negative_backup_returned` |
| 24 | `envy` | `PROG_BASE_OFFSET` / `PROG_MULTIPLIER` rejected (rows 1-8) | falls back to `0100`=64 / `012`=10 respectively; the `stderr` warning is emitted from inside `envy` | `err_24_envy_env_rejection_defaults` |
| 25 | `envy` | `strchr(buffer, ':') == NULL` (first colon missing) | dead branch: `snprintf` always writes `"Result:<int>:Complete"`, whose maximum length is 29 < `BUFFER_SIZE` (256), so a colon is always present. Asserted indirectly: the `Found colon at position: 6` line is always printed under `verbose` | `err_25_colon_always_found` |
| 26 | `envy` | `strchr(colon_pos+1, ':') == NULL` (second colon missing / `snprintf` truncation) | dead branch for the same reason (no truncation possible); `Debug: Result string format validated` is therefore always printed under `debug` | `err_26_second_colon_always_found` |
| 27 | `init_config_from_env` | `PROG_VERBOSE` / `PROG_DEBUG` present but containing **no** `'1'` (`""`, `"0"`, `"true"`) | flag cleared (0) — presence alone is *not* enough | `err_27_verbose_debug_need_a_one` |
| 28 | `init_config_from_env` | `PROG_OPTIMIZE` present but empty / `"0"` / `"no"` | flag **set** (1) — only presence is checked, the value is ignored | `err_28_optimize_presence_only` |
| 29 | `init_config_from_env` | `flags` points at memory whose padding bits (8..31) and `reserved` bit are garbage | read-modify-write: bits 0..7 are overwritten (`log_level` ⇐ 3, `reserved` ⇐ 0), bits 8..31 are **preserved** unchanged | `err_29_padding_bits_preserved` |
| 30 | `init_config_from_env`, `perform_operation`, `apply_bit_operations` | `flags` is **misaligned** (a caller casting a `char` buffer: offsets 1, 2, 3) | not rejected: x86-64 permits unaligned 4-byte accesses and that is what gcc emits for the bit-field access, so the call succeeds and touches exactly the 4 bytes at that address | `err_30_misaligned_flags_pointer` |
| 31 | `parse_env_numeric` | env value is not valid UTF-8 / contains high bytes (`"\xff\xfe"`, `"1\xff"`, `"\x80,\x81"`), or is very large (64 KiB, with the `,` only in the last byte) | no rejection beyond the `,`/`;` scan: the value is an opaque C string handed to `strchr`/`atoi` | `err_31_non_utf8_and_huge_values` |

## Generic FFI boundaries also covered

* NULL pointers for every pointer parameter (rows 10-13) — compared by fatal
  signal in a forked child, so "both failed somehow" is not accepted: the exact
  signal number must match (`SIGSEGV`, 11).
* Zero-length / empty inputs: empty env value, empty env name (rows 5, 6, 9).
* Oversized inputs: 64 KiB environment values, with the poison byte in the last
  position (row 31); 38-digit numbers (row 7).
* One step past the documented range: `INT_MAX+1`/`INT_MIN-1` as decimal text
  (row 7), `INT_MAX`/`INT_MIN` parameters, `0x40000000` (rows 16-21).
* Out-of-range "enum"-like values across the FFI boundary: this library has no
  C `enum`, its equivalent is the `struct ConfigFlags` bit-field unit, so **all
  2^8 = 256 low-byte patterns** are passed in (rows 14, 15), including the
  `log_level` values 0,1,2,4,5,6,7 that `init_config_from_env` never produces,
  the `reserved` bit, and 24 bits of garbage padding.
* Misaligned pointers (row 30).

## Result

```
ERRORS.md: 31 rows, 7 033 differential comparisons, 0 failures
```

Verified with the dev-profile cdylib, the release-profile cdylib
(`panic = "abort"`) and against a `-O3` build of the C library.

## Divergences this table found (fixed in the Rust, never in the C)

1. **rows 11-13** — `init_config_from_env` / `perform_operation` /
   `apply_bit_operations` with a NULL `struct ConfigFlags*`: the Rust code
   created a reference (`&mut *flags` / `&*flags`), which trips Rust's
   "null pointer dereference" debug assertion and calls `abort()` → `SIGABRT`
   (6), while the C code simply faults → `SIGSEGV` (11).
2. **row 30** — the first fix (a 4-byte `ptr::read_volatile`) still aborted on a
   *misaligned* `struct ConfigFlags*` ("pointer must be aligned" precondition),
   which the C code accepts (x86-64 unaligned access).

Both are fixed by `flags_load`/`flags_store` in `src/lib.rs`, which perform the
bit-field storage-unit access with four byte-wise volatile reads/writes in
increasing address order: no reference is created, no alignment or null
precondition is checked, and the observable behaviour (value, output, and fatal
signal) is now identical to the C.
