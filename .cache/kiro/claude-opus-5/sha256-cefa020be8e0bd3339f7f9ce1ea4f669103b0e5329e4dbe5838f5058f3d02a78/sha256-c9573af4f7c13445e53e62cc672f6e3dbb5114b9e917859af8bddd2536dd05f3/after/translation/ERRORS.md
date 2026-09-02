# ERRORS.md — Phase C error-surface table

Derived mechanically from `c_src/src/driver.c` + `c_src/include/driver.h`.

## Mechanical grep of the C source

```sh
grep -n "return\|assert\|NULL\|if (\|switch\|ERROR\|errno\|#ifdef\|#if\|sizeof" \
     c_src/src/driver.c c_src/include/driver.h
```

Result (comments excluded):

```
src/driver.c:26:#include <stdio.h>
src/driver.c:29:    for (int i = 0; i < len; i++) {
src/driver.c:36:    print_hex((unsigned char *)&x, sizeof(x));
include/driver.h:24:#ifndef DRIVER_H_
```

The complete non-comment C body is:

```c
static void print_hex(unsigned char *p, int len) {
    for (int i = 0; i < len; i++) {
        printf("%02x", p[i]);
    }
    printf("\n");
}

void driver(float x) {
    print_hex((unsigned char *)&x, sizeof(x));
}
```

Findings of the grep, itemised:

* error-return macros (`RETURN_ERROR`, `RETURN_IF`, …): **none**
* `return` statements of any kind: **none** — both functions are `void`
* `assert` / `static_assert` / `abort` / `exit`: **none**
* `NULL` checks: **none**
* error enums / status codes / `errno` inspection: **none**
* explicit range / bounds / min / max checks: **none**
* `#ifdef` feature branches: **none** (only the `DRIVER_H_` include guard)
* the single conditional in the whole library is the `for` loop guard `i < len`
* the return value of `printf` (which *can* fail) is **discarded** at both call
  sites

So the library has **no error-reporting surface at all**: `driver` returns
`void`, takes its single argument **by value**, and cannot reject any input.
Every `float` bit pattern — including all NaNs — is a valid input that must be
printed, never rejected. The rows below are therefore the genuine, complete set
of "rejection / failure" conditions reachable through the public API, plus the
generic FFI boundary conditions the prompt requires be covered even when absent
from the source.

## Error-surface table

| #  | function                | trigger (the exact invalid input/condition)                                                                            | expected C result                                                                                                     | test | status |
|----|-------------------------|------------------------------------------------------------------------------------------------------------------------|-----------------------------------------------------------------------------------------------------------------------|------|--------|
| E1 | `driver`                | any `float` value whatsoever — there is no validation code path, so no input is rejectable                              | no rejection: always writes exactly 8 lowercase hex digits + `\n`; returns `void`                                      | `e1_no_input_is_ever_rejected`            | [x] |
| E2 | `print_hex` (via `driver`) | loop guard `i < len` with `len == 0`; unreachable from the public API because `driver` hard-codes `sizeof(float) == 4`   | not reachable: `len` is always 4, so the body always runs 4 times; the guard never rejects anything                     | `e2_len_is_always_four_never_zero`        | [x] |
| E3 | `driver`                | signalling NaN (`0x7FA0_0000`) passed across the FFI boundary — the nearest thing IEEE-754 `float` has to a trap representation | no trap, no rejection, no quieting: the exact incoming bit pattern is printed (`0000a07f` little-endian)                | `e3_signalling_nan_not_quieted`           | [x] |
| E4 | `driver`                | non-canonical / "impossible" encodings: NaNs with arbitrary payloads, negative zero, subnormals, both infinities        | all accepted and printed verbatim; no normalisation, no rejection                                                      | `e4_noncanonical_encodings_accepted`      | [x] |
| E5 | `driver`                | every one of the 2^32 raw `u32` bit patterns reinterpreted as the `float` argument (sampled exhaustively per byte and randomly) | every pattern accepted; output is the 4 object-representation bytes in memory order                                    | `e5_all_bit_patterns_accepted`            | [x] |
| E6 | `print_hex` (via `driver`) | byte value `>= 0x80` in any of the 4 positions — `p[i]` is `unsigned char` so it zero-extends to `int`; a signed translation would print `ffffffXX` | exactly two hex digits per byte, e.g. `0x80` prints `80`, never `ffffff80`                                             | `e6_high_bytes_zero_extend`               | [x] |
| E7 | `driver`                | `printf` itself fails (write error): `stdout` redirected to a full device, so the flush returns `ENOSPC`                | return value discarded, so the failure is silently swallowed; `driver` still returns normally and does not crash        | `e7_printf_write_failure_swallowed`       | [x] |
| E8 | `driver`                | `driver` called with `stdout`'s underlying fd closed (`EBADF` on flush)                                                 | same as E7: silently swallowed, returns normally                                                                       | `e7_printf_write_failure_swallowed`       | [x] |

## Boundary conditions required by the prompt but absent from this API

| condition                     | applicable? | why                                                                                                    |
|-------------------------------|-------------|--------------------------------------------------------------------------------------------------------|
| null pointer arguments        | **no**      | the only public function takes one `float` **by value**; the public API exposes no pointer parameter    |
| zero / oversized lengths      | **no**      | no length parameter is exposed; `print_hex`'s `len` is `static`-internal and hard-coded to `sizeof(float)` (row E2) |
| out-of-range enum values      | **no**      | the library declares no enum, and the header declares no type at all                                   |
| one step past a valid range   | covered     | `float` has no valid *range* to exceed — every bit pattern is in range; covered exhaustively by E5      |

## Harness sensitivity (why these rows are not vacuous)

Because the C library has no error returns, every row above asserts an *absence*
of rejection, which is the kind of assertion that can pass while testing nothing.
The suite was therefore validated by mutating `translation/src/lib.rs` and
confirming the tests fail. All four mutants were caught:

| mutation                                                | effect on output            | tests failed |
|---------------------------------------------------------|-----------------------------|--------------|
| read the byte as `i8` instead of `u8` (sign-extension)   | `0x80` prints `ffffff80`    | 16 / 16 Phase B, plus E1/E2/E6 |
| format `"%2x"` instead of `"%02x"` (padding dropped)     | `0x0f` prints ` f`          | 17           |
| format `"%02X"` (uppercase digits)                       | `ae` prints `AE`            | 16           |
| drop the trailing `printf("\n")`                         | no newline, lines merge     | 16           |

`src/lib.rs` was restored byte-identically afterwards (verified with `diff`) and
the suite returned to green. Row E6 is the one that specifically pins the
sign-extension case, which is the highest-risk divergence in a translation of
this function.
