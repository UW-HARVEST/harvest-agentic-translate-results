# ERRORS.md — Phase C error-surface table

## Mechanical derivation

Every error/rejection construct was grepped out of the complete C source
(`c_src/src/driver.c`, `c_src/include/driver.h` — the only two files):

```
$ grep -nE 'return|assert|NULL|errno|exit|abort|<|>|==|!=|#if|#ifdef|ERROR|-1' \
       src/driver.c include/driver.h | grep -v '://'
src/driver.c:26:#include <stdio.h>
src/driver.c:29:    for (int i = 0; i < len; i++) {
include/driver.h:24:#ifndef DRIVER_H_
```

Findings, exhaustively:

* `return` statements: **0** (both functions are `void` and fall off the end).
* `assert` / `abort` / `exit` / `errno` / `NULL` checks: **0**.
* error macros / error enums / sentinel values / min-max constants: **0**.
* explicit range checks: **0**. The only relational operator in the whole
  library is the `i < len` loop bound in `print_hex`, and `len` is never
  attacker-controlled: the single call site hard-codes `sizeof(x)` == 4.
* `#ifdef`: **1**, and it is only the `DRIVER_H_` include guard — no
  configuration-dependent code.
* Public API pointer parameters: **0** (`void driver(float x)` takes a value),
  so there is no null-pointer, zero-length or oversized-length surface at the
  FFI boundary.
* Public API enum parameters: **0**, so there is no out-of-range-enum surface.

**The library therefore has an EMPTY error-return surface: there is no input
for which the C code rejects, errors, or returns a distinguishable failure.**
`driver` accepts every one of the 2^32 possible `float` bit patterns and, for
every one of them, prints 8 lowercase hex digits followed by `\n`.

Because "returns an error code" is not observable here, the rejection table is
recast onto the only observable: **the exact bytes written to stdout**. Each row
below is a *degenerate / hostile / boundary* input that a naive translation is
likely to mangle (by normalising, canonicalising, or trapping on it) even
though the C simply reinterprets its object representation. A row passes only
when C and Rust emit byte-identical stdout AND neither aborts.

## Error / hostile-input surface table

| #  | function | trigger (the exact invalid input/condition) | expected C result |
|----|----------|----------------------------------------------|-------------------|
| 1  | `driver` | no error path exists at all (grep evidence above): every `float` is accepted | never returns an error; always writes exactly 9 bytes (8 hex + `\n`), returns `void` |
| 2  | `driver` | `+0.0f` (`0x00000000`) — all-zero object representation | `00000000\n` |
| 3  | `driver` | `-0.0f` (`0x80000000`) — negative zero must NOT be canonicalised to `+0.0` | `00000080\n` |
| 4  | `driver` | quiet NaN `0x7fc00000` | `0000c07f\n` |
| 5  | `driver` | negative quiet NaN `0xffc00000` | `0000c0ff\n` |
| 6  | `driver` | **signalling** NaN `0x7fa00000` — must not be quieted to `0x7fc00000` in transit | `0000a07f\n` |
| 7  | `driver` | NaN with non-canonical payload `0x7f800001` (smallest sNaN) | `0100807f\n` |
| 8  | `driver` | NaN with all payload bits set `0x7fffffff` | `ffffff7f\n` |
| 9  | `driver` | `+inf` (`0x7f800000`) | `0000807f\n` |
| 10 | `driver` | `-inf` (`0xff800000`) | `000080ff\n` |
| 11 | `driver` | smallest positive subnormal `0x00000001` (must not be flushed to zero) | `01000000\n` |
| 12 | `driver` | largest subnormal `0x007fffff` | `ffff7f00\n` |
| 13 | `driver` | negative subnormal `0x80000001` | `01000080\n` |
| 14 | `driver` | `FLT_MAX` `0x7f7fffff` and `-FLT_MAX` `0xff7fffff` (one step below overflow) | `ffff7f7f\n` / `ffff7fff\n` |
| 15 | `driver` | `FLT_MIN` `0x00800000` (normal/subnormal boundary) | `00008000\n` |
| 16 | `driver` | all-bits-set `0xffffffff` | `ffffffff\n` |
| 17 | `driver` | value `0x0000ff00` — byte with high bit clear next to `00`, catches sign-extension / `%02x` width bugs | `00ff0000\n` |
| 18 | `driver` | any byte >= 0x80 in the representation, e.g. `0x80808080` — `unsigned char` must be zero-extended (not sign-extended) on the variadic promotion to `int`, otherwise `%02x` prints `ffffff80` | `80808080\n` |
| 19 | `driver` | repeated invocation (1000 calls in a row) — no hidden per-call state, no stdout buffer desync | 1000 independent 9-byte lines |
| 20 | `driver` | `print_hex` is `static` in C — it must NOT be dynamically reachable from the Rust `.so` either | `dlsym("print_hex")` fails in BOTH `.so`s |

## Status

All 20 rows are implemented as differential tests in `tests/phase_c_errors.rs`
(`err01_..err20_`), plus one extra test
(`err_generic_boundaries_are_inapplicable_by_construction`) that pins the
generic C-API boundaries (null pointer / zero & oversized length / out-of-range
enum) as inapplicable-by-construction and sweeps the full non-numeric float
space instead.

All rows PASS: `21 passed; 0 failed` under both cargo profiles
(`dev`, `release`) and both feature selections (default, `--no-default-features`).
Verified by `./run_all_configs.sh` => "ALL CONFIGURATIONS PASSED".

Harness sensitivity was proven by mutation testing the Rust source: `%02X`
instead of `%02x`, `%x` instead of `%02x`, `len` 3 instead of 4, dropped
trailing newline, reversed byte order, and `signed char` instead of
`unsigned char` were each caught (16-20 of 22 Phase B tests fail per mutation).

| row | test | status |
|-----|------|--------|
| 1 | `err01_no_error_path_every_input_accepted` | [x] |
| 2 | `err02_positive_zero` | [x] |
| 3 | `err03_negative_zero_not_canonicalised` | [x] |
| 4 | `err04_quiet_nan` | [x] |
| 5 | `err05_negative_quiet_nan` | [x] |
| 6 | `err06_signalling_nan_not_quieted` | [x] |
| 7 | `err07_min_payload_snan` | [x] |
| 8 | `err08_max_payload_nan` | [x] |
| 9 | `err09_positive_infinity` | [x] |
| 10 | `err10_negative_infinity` | [x] |
| 11 | `err11_smallest_subnormal_not_flushed` | [x] |
| 12 | `err12_largest_subnormal` | [x] |
| 13 | `err13_negative_subnormal` | [x] |
| 14 | `err14_flt_max_boundary` | [x] |
| 15 | `err15_flt_min_boundary` | [x] |
| 16 | `err16_all_bits_set` | [x] |
| 17 | `err17_width_padding` | [x] |
| 18 | `err18_unsigned_char_zero_extension` | [x] |
| 19 | `err19_repeated_invocation` | [x] |
| 20 | `err20_static_helper_not_exported` | [x] |
