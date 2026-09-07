# ERRORS.md — Phase C error-surface table

Mechanically derived from the complete C source (`c_src/src/driver.c`,
`c_src/include/driver.h`). Greps performed over the whole of `c_src/`:

```
grep -n 'return\|assert\|NULL\|errno\|exit(\|abort(\|RETURN_ERROR\|if *(' c_src/src/driver.c c_src/include/driver.h
```

Findings:

* `return` — **0 occurrences.** Both functions are `void`.
* `assert` — 0 occurrences (`<assert.h>` is not included).
* `NULL` / null checks — 0 occurrences.
* error enums / error codes / `errno` / `exit` / `abort` — 0 occurrences.
* explicit range checks / min-max constants — 0 occurrences. The only `if`-like
  construct is the `for (int i = 0; i < len; i++)` loop bound in `print_hex`,
  whose `len` is always the compile-time constant `sizeof(float)` (= 4), because
  the sole caller is `driver` passing `sizeof(raw)`.
* pointer parameters on the public API — none. `driver` takes a single `float`
  **by value**, so a "null pointer" argument is not representable.

## Error-surface table

| # | function | trigger (the exact invalid input/condition) | expected C result |
|---|----------|----------------------------------------------|-------------------|
| 1 | `driver` | *(no rejection path exists)* — the function is `void`, has no parameter validation, no pointer arguments, no assertions, and no error returns. **Every** 32-bit pattern is a valid `float` argument. | Always prints the 4 bytes of the argument's object representation as 8 lowercase hex digits + `"\n"`, then returns. Never errors. |

The error surface of this library is therefore **empty**: there is no input for
which the C code returns an error, sets an error code, or rejects the call.

## Consequence for Phase C

Because there is no rejection path to reproduce, Phase C is satisfied by
differential-testing the *generic* boundaries an FFI surface always has, and
asserting that C and Rust behave **identically** (both succeed and emit the same
bytes) rather than "both fail":

| # | boundary class | concrete inputs tested | test |
|---|----------------|------------------------|------|
| E1 | `+0.0` / `-0.0` (signed-zero distinction) | `0x00000000`, `0x80000000` | `err_signed_zeros` |
| E2 | smallest/largest subnormal (one step past the normal range) | `0x00000001`, `0x007fffff` | `err_subnormal_boundaries` |
| E3 | smallest normal, largest finite (`FLT_MIN`, `FLT_MAX`) and one step past `FLT_MAX` (→ `inf`) | `0x00800000`, `0x7f7fffff`, `0x7f800000` | `err_normal_boundaries` |
| E4 | infinities | `0x7f800000`, `0xff800000` | `err_infinities` |
| E5 | quiet NaN, signalling NaN, NaN with arbitrary payload, negative NaN — bit patterns with no "valid value", the float analogue of an out-of-range enum | `0x7fc00000`, `0x7fa00000`, `0x7f800001`, `0x7fbfffff`, `0xffc00000`, `0xffa00000`, `0xff800001`, `0xffffffff` | `err_nan_payloads` |
| E6 | exhaustive sweep of every **byte** value in every one of the 4 byte positions (catches any `%02x`/sign-extension/`unsigned char` mistake, e.g. bytes ≥ 0x80) | all 4 × 256 patterns | `err_all_byte_values_all_positions` |
| E7 | out-of-range "enum-like" argument: the full 32-bit pattern space is the argument domain, so this is covered by randomized full-`u32` sweeps | 200 000 random `u32` bit patterns, fixed seed | `valid_random_full_u32_space` |
| E8 | repeated / interleaved calls (no hidden state, output not lost to buffering) | 1 000 interleaved C/Rust calls in one capture | `err_repeated_interleaved_calls` |

All rows checked off in `tests/differential.rs`; see the test names above.
