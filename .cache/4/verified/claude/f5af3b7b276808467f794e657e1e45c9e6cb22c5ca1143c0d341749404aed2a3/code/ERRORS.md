# ERRORS.md — Error / rejection surface table

Mechanically derived from `c_src/src/lib.c`. This library has **no error enum,
no `RETURN_ERROR` macro, no `assert`, no `errno` use and no `NULL` checks**.
Its entire rejection surface consists of:

* the three guard branches in `safe_double_to_int` (clamp / NaN rejection),
* the `default:` arm of the `switch` in `process_with_fallthrough` (sentinel `-1`),
* the unchecked pointer dereference in `copy_data_block`,
* the fixed-size buffer bound in `overunder` (`sizeof(label) - 1`),

plus the implicit boundary conditions every C integer API has (signed overflow,
out-of-range enum-like `int` arguments, extreme operands). Every row below is a
*distinct rejection / boundary branch that the C code actually takes*.

Constants referenced: `INT_MAX = 2147483647`, `INT_MIN = -2147483648`,
`sizeof(DataBlock) = 40`, `sizeof(label) = 20`, `sizeof(array1) = 20`.

| #  | function | trigger (exact invalid input / condition) | expected C result | test | ✔ |
|----|----------|-------------------------------------------|-------------------|------|---|
| E1 | `safe_double_to_int` | `d > (double)INT_MAX` — line 40 (e.g. `2147483648.0`, `1e15`, `1e300`) | returns `INT_MAX` (`2147483647`) | `err_e1_e3_clamp_high` | [x] |
| E2 | `safe_double_to_int` | `d == +INFINITY` (satisfies `d > INT_MAX`) | returns `INT_MAX` | `err_e2_pos_infinity` | [x] |
| E3 | `safe_double_to_int` | `d` one ULP above `(double)INT_MAX`, i.e. `nextafter(2147483647.0, +inf)` | returns `INT_MAX` | `err_e1_e3_clamp_high` | [x] |
| E4 | `safe_double_to_int` | `d < (double)INT_MIN` — line 42 (e.g. `-2147483649.0`, `-1e15`, `-1e300`) | returns `INT_MIN` (`-2147483648`) | `err_e4_e6_clamp_low` | [x] |
| E5 | `safe_double_to_int` | `d == -INFINITY` (satisfies `d < INT_MIN`) | returns `INT_MIN` | `err_e5_neg_infinity` | [x] |
| E6 | `safe_double_to_int` | `d` one ULP below `(double)INT_MIN`, i.e. `nextafter(-2147483648.0, -inf)` | returns `INT_MIN` | `err_e4_e6_clamp_low` | [x] |
| E7 | `safe_double_to_int` | `d` is NaN — line 44. All three relational tests are false for NaN, so the `isnan` arm is reached. Covers `+NaN`, `-NaN`, quiet NaN and signalling-NaN bit patterns. | returns `0` | `err_e7_nan_variants` | [x] |
| E8 | `safe_double_to_int` | in-range boundary **not** rejected: `d == (double)INT_MAX` exactly (`>` is false) | returns `INT_MAX` via `(int)d`, *not* via the clamp | `err_e8_e9_inrange_boundaries` | [x] |
| E9 | `safe_double_to_int` | in-range boundary **not** rejected: `d == (double)INT_MIN` exactly (`<` is false) | returns `INT_MIN` via `(int)d`, *not* via the clamp | `err_e8_e9_inrange_boundaries` | [x] |
| E10 | `process_with_fallthrough` | `code` has no matching `case` and is **negative**: any `code < 0` (`-1`, `-6`, `INT_MIN`) → `default:` line 70 | returns sentinel `-1` (base_value discarded) | `err_e10_negative_code` | [x] |
| E11 | `process_with_fallthrough` | `code` has no matching `case` and is **> 5**: any `code >= 6` (`6`, `7`, `100`, `INT_MAX`) → `default:` line 70 | returns sentinel `-1` (base_value discarded) | `err_e11_code_above_range` | [x] |
| E12 | `process_with_fallthrough` | out-of-range "enum" values across FFI: `code` = `INT_MIN`, `INT_MIN+1`, `-1`, `6`, `INT_MAX-1`, `INT_MAX` (C `switch` on `int` accepts any `int`; there is no valid-variant check) | returns `-1` for every one of them | `err_e12_ffi_out_of_range_enum` | [x] |
| E13 | `process_with_fallthrough` | `code == 0` — the *zero-length / reset* branch, which **discards** `base_value` entirely (line 67) rather than adding to it | returns `0` for every `base_value` | `err_e13_code_zero_discards_base` | [x] |
| E14 | `process_with_fallthrough` | signed-overflow boundary: `code == 5` with `base_value` in `[INT_MAX-149, INT_MAX]` (the fall-through adds `+150`, overflowing `int`) | wraps (two's-complement) — must match C bit-for-bit | `err_e14_fallthrough_overflow` | [x] |
| E15 | `process_with_fallthrough` | signed-underflow boundary: `code == 1` with `base_value == INT_MIN` (adds `+10`, no overflow) and `code == 5`/`base_value == INT_MIN` | must match C bit-for-bit | `err_e14_fallthrough_overflow` | [x] |
| E16 | `copy_data_block` | **no null check** (line 78 dereferences both pointers unconditionally). `dest == NULL` or `src == NULL` ⇒ the C `memcpy` faults (SIGSEGV). Rust must be *equally* unchecked — it must not silently return, and it must not turn the fault into a Rust panic/`unimplemented!()`. Verified by asserting the fault happens in a forked child for **both** libraries, and that neither prints a Rust panic message. | process dies on signal (SIGSEGV) for both C and Rust | `err_e16_null_pointers_fault_identically` | [x] |
| E17 | `copy_data_block` | `dest == src` (aliasing; `memcpy` with identical pointers) | contents unchanged, exactly 40 bytes touched | `err_e17_dest_equals_src` | [x] |
| E18 | `copy_data_block` | over/under-length boundary: exactly `sizeof(DataBlock) == 40` bytes are copied — byte 39 must be copied and byte 40 must **not** be (detected with a 96-byte sentinel-filled arena) | bytes `[0,40)` copied, `[40,96)` untouched | `err_e18_copies_exactly_40_bytes` | [x] |
| E19 | `overunder` | `sqrt` domain error: `d*d + a*a` overflows `int` to a **negative** value, so `sqrt(negative)` returns NaN, which `safe_double_to_int` then maps through the `isnan` arm | `conv4 == 0`; full return value and stdout must match | `err_e19_sqrt_domain_negative` | [x] |
| E20 | `overunder` | `a % 6` is negative (any `a < 0` whose remainder is non-zero), so `process_with_fallthrough` takes the `default:` arm | `switch_result == -1` (printed and summed) | `err_e20_negative_modulo_default` | [x] |
| E21 | `overunder` | `a == INT_MIN`: `a % 6` (`INT_MIN % 6 == -2`) plus `a*a`, `a*1.5` and `a+b` at the extreme | must match C bit-for-bit (return value + stdout) | `err_e21_extreme_int_args` | [x] |
| E22 | `overunder` | clamp reached from inside `overunder`. The exact thresholds (derived from the C, and **asymmetric**): `a*1.5 > INT_MAX` ⟺ `a >= 1431655765`; `a*1.5 < INT_MIN` ⟺ `a <= -1431655766` (note `-1431655765*1.5 == -2147483647.5`, which is *not* `< INT_MIN`, so it truncates to `-2147483647` instead of clamping); `b*2.7 > INT_MAX` ⟺ `b >= 795364314`; `b*2.7 < INT_MIN` ⟺ `b <= -795364315`. Each threshold and one step inside it is tested. | `conv1`/`conv2` clamp to `INT_MAX`/`INT_MIN` on the outer side, truncate on the inner side | `err_e22_internal_clamp` | [x] |
| E23 | `overunder` | `handle_pointer_operations` overflow boundary: `c` such that `c * 2` overflows (`c > INT_MAX/2`) or `c * 2 + 100` overflows | wraps; must match C | `err_e23_ptr_op_overflow` | [x] |
| E24 | `overunder` | total accumulation overflow: `a`,`b`,`c`,`d` chosen so the final `total` sum overflows `int` repeatedly | wraps; must match C | `err_e24_total_overflow` | [x] |
| E25 | `handle_pointer_operations` | `value == INT_MAX` / `INT_MIN` / `INT_MAX/2 + 1` — `value*2` and `+100` overflow with no guard | wraps; must match C | `err_e25_hpo_extremes` | [x] |
| E26 | `overunder` | fixed buffer bound `sizeof(label) - 1 == 19`: `strncpy` writes 6 payload bytes + 13 NUL pad, then line 122 forces `label[19] = '\0'`; `%s` must therefore print exactly `Source` with no trailing garbage regardless of inputs | stdout contains `label=Source` | `err_e26_label_buffer_bound` | [x] |

## Notes on rows deliberately **not** present

* There is no `return NULL`, no negative-`errno` return, and no error enum in
  this translation unit, so no such rows exist.
* `overunder` has exactly one exit (`return total;`) and no rejection branch of
  its own — all of its error behaviour is inherited from the four helpers, which
  is why rows E19–E24 describe *which helper branch* `overunder` drives.
