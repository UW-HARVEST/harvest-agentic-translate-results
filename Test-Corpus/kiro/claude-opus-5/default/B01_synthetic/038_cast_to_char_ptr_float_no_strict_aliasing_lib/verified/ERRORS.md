# ERRORS.md — error-surface table

Derived mechanically from `c_src/src/driver.c` and `c_src/include/driver.h`.

## Mechanical grep for every rejection construct

```
$ grep -nE 'return|assert|NULL|errno|error|ERROR|exit|abort|<|>|==|!=' \
      c_src/src/driver.c c_src/include/driver.h
src/driver.c:26:#include <stdio.h>
src/driver.c:27:#include <string.h>
src/driver.c:30:    for (int i = 0; i < len; i++) {
```

The only hits are the two `#include` lines (matched on `<`/`>`) and the loop
condition `i < len`. Concretely, the C source contains:

* **0** `return` statements (both functions are `void`; `driver` and
  `print_hex` fall off the end).
* **0** `assert` / `static_assert`.
* **0** `NULL` checks — `driver` takes a `float` by value, so there is no
  pointer parameter to validate. `print_hex` is `static` and is only ever
  called with `&raw[0]`, a valid non-null stack address.
* **0** error enums, error codes, sentinel returns or `errno` inspection.
* **0** explicit range checks, min/max constants, or size validation.
* **0** `exit` / `abort` / longjmp paths.
* **0** `#ifdef` / conditional-compilation branches.

## Error-surface rows

| # | function | trigger (the exact invalid input/condition) | expected C result |
|---|----------|---------------------------------------------|-------------------|
| — | — | *(none: the C library defines no rejection, error return, assertion or validation path)* | — |

**The table is empty by construction, not by omission.** `void driver(float x)`
accepts the entire 2^32-value domain of `float` — every bit pattern, including
all NaN payloads, both infinities, both zeros and all subnormals — and has no
input for which it reports failure. There is no return value through which an
error could be signalled and no out-parameter or global that could carry one.

## Boundary conditions still exercised in Phase C

Even with an empty rejection table, `tests/differential.rs` covers the generic
boundaries the task requires, in the only forms this API can express them:

| id | boundary | why it applies / how it is covered |
|----|----------|-------------------------------------|
| B1 | null pointer | Not expressible: `driver`'s only parameter is a by-value `float`. Documented as N/A; the internal `print_hex` pointer is never caller-controlled. `test_no_pointer_parameter_documented` records this. |
| B2 | zero length | `print_hex` is always called with `len == sizeof(float) == 4`; `len` is not caller-controlled. The zero/negative-`len` path (`0..len` yields no iterations, then a bare newline) is unreachable from the public API. Verified reachable-behaviour equivalence instead: exactly 4 hex bytes + `\n` from both `.so`s for every input (`test_output_shape_is_always_nine_bytes`). |
| B3 | oversized length | Same as B2 — `len` is a compile-time constant `4`, not attacker-controlled. |
| B4 | out-of-range enum value across FFI | Not expressible: there is no enum, `int` flag or mode parameter anywhere in the public API. The nearest analogue for a `float` parameter is a bit pattern with no "valid" interpretation, i.e. NaN (quiet and signalling, all payloads) and the padding-free trap-free extremes. Covered exhaustively-by-sampling in `test_nan_payloads_bit_exact`, `test_signalling_nan_bit_exact` and `test_all_exponent_boundaries`. |
| B5 | one step past a documented valid range | The documented range is "any `float`", so the boundaries are the encoding extremes: `±0.0`, `±MIN_POSITIVE`, largest subnormal, smallest normal, `±MAX`, `±INFINITY`, and the first/last NaN encodings. Covered in `test_float_boundary_values` and `test_all_exponent_boundaries`. |
| B6 | value-dependent formatting | `%02x` on an `unsigned char` promoted to `int`: bytes `0x00`-`0x0f` must zero-pad, `0x80`-`0xff` must NOT sign-extend to `ffffff80`. Covered by exhaustive per-byte-value tests (`config_row_11_every_byte_value_in_every_position`, all 4 x 256 combinations) and by the randomized sweeps. |
| B7 | output-stream identity | The Rust translation must write through libc `stdout`, not Rust's own `std::io::stdout` buffer, or output interleaves differently in a host process that also uses stdio. `errors_b7_interleaved_calls_preserve_ordering` alternates the two `.so`s inside one capture. |
| B8 | **argument register class** | `float` is passed in `%xmm0`; an integer parameter would be passed in `%edi`. A translation typed `extern "C" fn(c_int)` compiles, exports `driver`, and passes `nm -D` parity while reading the wrong register. `errors_b8_float_abi_register_class` pins the exact expected bytes for six inputs so this fails deterministically. **This defect was present and was fixed — see the note below.** |

## Defect found and fixed during verification

`translation/src/lib.rs` exported

```rust
pub extern "C" fn driver(x: c_int)          // WRONG
```

against the C ground truth `void driver(float x)`. Both produce a `driver`
symbol, so `nm -D` parity was clean, but the ABIs differ: the C function reads
its argument from the vector register `%xmm0` (`movss %xmm0,-0x14(%rbp)`), while
the Rust function read `%edi` (`mov %edi,%ebx`). Every caller passing a `float`
therefore received unrelated bytes from the Rust library — for `driver(1.0f)`
the C printed `0000803f` and the Rust printed whatever happened to be in `%edi`.

Fixed by restoring the parameter type to `f32`; the recompiled export now begins
`movd %xmm0,%ebx`. This is exactly the class of defect that symbol-parity checks
and single-value happy-path tests cannot see.
