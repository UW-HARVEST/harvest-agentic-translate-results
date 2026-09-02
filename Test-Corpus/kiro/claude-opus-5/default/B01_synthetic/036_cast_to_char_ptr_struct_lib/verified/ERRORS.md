# ERRORS.md — Phase C error-surface table

## How this table was derived

Mechanical grep of the entire C source for every rejection / error construct:

```sh
grep -n -E 'return|assert|NULL|errno|exit|abort|if *\(|switch|else|#if|<|>|<=|>=|==|!=|&&|\|\|' \
     c_src/src/driver.c c_src/include/driver.h
```

The **only** match in `driver.c` is the loop condition on line 35:

```c
for (int i = 0; i < len; i++) {
```

Findings, stated exactly as the source supports them:

* `RETURN_ERROR`-style macros: **none**.
* `return <error>` statements: **none**. Both functions are `void`; neither
  contains any `return` statement at all.
* `return NULL`: **none** — no function returns a pointer.
* error enums / status codes: **none** — the public header declares no enum,
  no typedef, and no status type.
* `assert` / `abort` / `exit` / `errno`: **none**.
* explicit range checks: **none**.
* null-pointer checks: **none**.
* min/max constants: **none**.
* `#ifdef`-gated behaviour: only the `DRIVER_H_` include guard, which has no
  runtime effect.

So the count of *distinct rejection branches in the C* is **zero**: `driver`
accepts the entire `int` domain, has no failure mode, and returns `void`, so it
has no channel through which to report an error. Every 32-bit value of `floors`
is a valid input.

Because a table of zero rows cannot be tested, the rows below are the
**boundary / degenerate-input surface** that the protocol mandates for any C
API even when the source contains no explicit check. Each row states the
condition, and the expected C result derived from reading the code — *not* from
assuming an error occurs. "Expected C result" for a total function means
"produces its normal 33-byte output and does not fail", and the differential
test asserts C and Rust agree on exactly that.

## Error / boundary surface table

| # | function | trigger (the exact invalid input/condition) | expected C result | test |
|---|----------|----------------------------------------------|-------------------|------|
| 1 | `driver` | `floors == 0` (zero / degenerate value) | no rejection; prints `00000000` `03000000` `0000000000000040` + `\n` | `err_01_zero` |
| 2 | `driver` | `floors == INT_MIN` (`-2147483648`, one step past the negative end of the signed range) | no rejection; prints `00000080…` | `err_02_int_min` |
| 3 | `driver` | `floors == INT_MAX` (`2147483647`, the positive extreme) | no rejection; prints `ffffff7f…` | `err_03_int_max` |
| 4 | `driver` | `floors == -1` (all-bits-set; would be a sentinel `-1` in an API that had one) | no rejection; prints `ffffffff…` | `err_04_minus_one` |
| 5 | `driver` | out-of-range "enum" value: an `int` bit pattern that would be an invalid variant if the parameter were a C enum (`0x7FFFFFFF`, `0x80000000`, `0xDEADBEEF`, `0xFFFFFFFF` reinterpreted as `i32`) — C enums accept any `int`, so these are real inputs | no rejection; the raw 4 bytes are printed verbatim, no validation | `err_05_out_of_range_enum_values` |
| 6 | `driver` | one step past each side of every "plausible range" boundary a caller might assume (`-1/0/1`, `0x7f/0x80/0x81`, `0x7fff/0x8000/0x8001`, `0x7fffff/0x800000/0x800001`, `0x7ffffffe/0x7fffffff`, `-0x7fffffff/-0x80000000`) | no rejection at any boundary; each prints its own little-endian bytes | `err_06_range_boundaries` |
| 7 | `driver` | oversized / zero "length": the internal `len` argument to `print_hex` is hard-coded to `sizeof(house_t)`, so `len <= 0` and `len > 16` are **unreachable** from the public ABI. The public API exposes no length parameter to corrupt. | unreachable by construction; output length is invariantly 33 bytes for every input | `err_07_output_length_invariant` |
| 8 | `driver` | null pointer arguments: the public ABI takes **no** pointer parameter, so there is no null to pass. `print_hex`'s pointer is always `&house`, an automatic object. | unreachable by construction | documented, not testable — asserted indirectly by row 7 |
| 9 | `driver` | repeated / back-to-back invocation, and C-then-Rust interleaving in one process (stale state, shared `stdout` buffering) | each call is independent; `house` is a fresh automatic object every call, so output depends only on `floors` | `err_09_no_residual_state` |
| 10 | `driver` | full-domain sweep of the low byte (`0x00..=0xFF`) — exercises `%02x` zero-padding, the one formatting decision in the code | values `< 0x10` print as two chars with a leading `0`, never one char | `err_10_hex_zero_padding` |

## Row status

| row | status |
|-----|--------|
| 1 | [x] passing |
| 2 | [x] passing |
| 3 | [x] passing |
| 4 | [x] passing |
| 5 | [x] passing |
| 6 | [x] passing |
| 7 | [x] passing |
| 8 | [x] unreachable by construction (documented; covered indirectly by row 7) |
| 9 | [x] passing |
| 10 | [x] passing |
