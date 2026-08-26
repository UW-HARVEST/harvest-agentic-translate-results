# ERRORS.md — Error-surface table (Phase A, gate for Phase C)

Derived **mechanically** from `c_src/src/driver.c` and `c_src/include/driver.h`.

## Mechanical grep evidence

```
$ grep -nE 'return|assert|NULL|errno|RETURN_ERROR|exit\(|abort|if *\(|< *0|> *0|<=|>=|== *0|!=|switch|goto|#ifdef|#if |#ifndef' \
      c_src/src/driver.c c_src/include/driver.h
c_src/include/driver.h:24:#ifndef DRIVER_H_        <- include guard only
```

```
$ grep -nE 'for *\(|while *\(|\?|&&|\|\|' c_src/src/driver.c
36:    for (int i = 0; i < len; i++) {             <- loop bound, not a rejection
```

The entire library is:

```c
static void print_hex(unsigned char *p, int len) {
    for (int i = 0; i < len; i++) { printf("%02x", p[i]); }
    printf("\n");
}

void driver(int floors) {
    house_t house = {0};
    house.floors = floors;
    house.bedrooms = 3;
    house.bathrooms = 2.;
    char raw[sizeof(house)];
    memcpy(raw, &house, sizeof(house));
    print_hex((unsigned char *)&raw, sizeof(raw));
}
```

## Result of the mechanical enumeration

**There are ZERO rejection paths in this library.** Concretely, the C source
contains:

* 0 `return` statements that yield a value (the only two functions return `void`)
* 0 error-return macros (`RETURN_ERROR`, `goto fail`, …)
* 0 `assert` / `abort` / `exit` calls
* 0 error enums or status codes
* 0 explicit range checks, null checks, or min/max constants
* 0 pointer parameters on the public API (`driver` takes a by-value `int`)
* 0 `#ifdef`-guarded behaviour (only the header include guard)

Therefore the *entire* domain of `int` is valid input and the expected C result
is always the same: 33 bytes on stdout (32 lowercase hex digits + `'\n'`), and
no error signalling of any kind. The rows below record that as the contract, and
additionally cover the generic FFI boundaries the task mandates. Every row has a
differential test asserting C and Rust agree **exactly** (identical stdout bytes,
identical absence of any abort/trap, identical return of nothing).

## Error-surface table

| # | function | trigger (the exact invalid input/condition) | expected C result | test | ✔ |
|---|----------|----------------------------------------------|-------------------|------|---|
| E1 | `driver` | any `int` value whatsoever — there is no validation, so nothing is rejected | no error; always writes exactly 33 bytes (`32` hex digits + `\n`); returns void | `err_e1_no_input_is_rejected` | [x] |
| E2 | `driver` | `floors == INT_MIN` (`-2147483648`, `0x80000000`) — one step past the negative end of the range | no error; prints `00000080` for the `floors` word | `err_e2_int_min` | [x] |
| E3 | `driver` | `floors == INT_MAX` (`2147483647`, `0x7fffffff`) — one step past the positive end of the range | no error; prints `ffffff7f` for the `floors` word | `err_e3_int_max` | [x] |
| E4 | `driver` | `floors == -1` (all-ones bit pattern; would be a `-1` error sentinel in most C APIs, here it is ordinary data) | no error; prints `ffffffff` for the `floors` word — **not** treated as a sentinel | `err_e4_minus_one_is_not_a_sentinel` | [x] |
| E5 | `driver` | `floors == 0` (the "null-ish"/zero-length analogue for this API's only parameter) | no error; prints `00000000` for the `floors` word; still 33 bytes | `err_e5_zero` | [x] |
| E6 | `driver` | out-of-range *enum-style* integers passed across the FFI boundary: values that would have no valid variant in any enum (`-2`, `255`, `256`, `1000`, `0x7fffffff`, `0x80000000`, `0xdeadbeef as i32`) | no error; each is accepted as plain data and hex-printed verbatim | `err_e6_out_of_range_enum_values` | [x] |
| E7 | `driver` | `floors` bit pattern containing an embedded NUL byte (`0x00ff00ff`, `0x00000001`, …) — the classic "C string truncation" trigger, because the C copies through `char raw[]` | no error and **no truncation**: all 16 bytes are emitted because `print_hex` is length-driven, not NUL-driven | `err_e7_embedded_nul_no_truncation` | [x] |
| E8 | `driver` | `floors` bit pattern whose bytes have the high bit set (`0x80`…`0xff`), i.e. negative values of the *signed* `char raw[]` elements | no error; bytes are reinterpreted through `(unsigned char *)` and print as `80`..`ff`, never as sign-extended `ffffff80` | `err_e8_high_bit_bytes_unsigned` | [x] |
| E9 | `driver` | `floors` bit pattern containing byte `0x0a` (`'\n'`) or `0x25` (`'%'`) — payload bytes that could corrupt the output framing or be mistaken for a format string | no error; emitted as the literal hex text `0a` / `25`; output still exactly one trailing `\n` at the very end | `err_e9_newline_and_percent_payload_bytes` | [x] |
| E10 | `driver` | internal loop bound: `print_hex` with `len` derived from `sizeof(raw)`; a shorter/longer `len` is unreachable, and `len <= 0` would print only `\n`. Verify the reachable bound is never over- or under-run | no error; exactly `2 * 16` hex digits, so no out-of-bounds read of `raw` and no early stop | `err_e10_loop_bound_exact` | [x] |
| E11 | `driver` | repeated / stateful abuse: calling `driver` many times in a row, and interleaving the C and Rust libraries in the same process | no error and no residual state: each call independently re-zeroes its local `house`, so output depends only on the current argument | `err_e11_no_residual_state` | [x] |

### Boundaries that are structurally inapplicable

Recorded so the omission is deliberate, not a blind spot:

| boundary | why inapplicable |
|---|---|
| null pointer arguments | `driver(int)` has no pointer parameters; `print_hex` is `static` and is only ever called with `&raw`, the address of a live stack array. There is no way for a caller to pass a pointer. |
| zero / oversized lengths | no length parameter is exposed; `len` is always `sizeof(house_t)` == 16. |
| buffer/output pointers | none — output goes to `stdout` via `printf`. |
| error codes / `errno` | the API returns `void` and never sets `errno`. |

## Verification status

All 11 rows pass in both debug and release under the single valid feature
combination. `tests/phase_c_errors.rs` has exactly 11 `#[test]`s, one per row.

Because the C library has no rejection paths, each test asserts the *same
non-rejection* in both implementations rather than merely "both failed somehow":
identical stdout bytes, a full 33-byte record, the trailing `\n` present exactly
once, and agreement with the independent byte model. A translation that
erroneously *added* validation (returning early, truncating, or special-casing
`-1`/`0`) would produce a short or absent record and fail.
