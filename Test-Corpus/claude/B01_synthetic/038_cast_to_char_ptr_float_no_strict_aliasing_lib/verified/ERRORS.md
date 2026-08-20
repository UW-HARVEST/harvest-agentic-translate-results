# ERRORS.md — Phase C error-surface table

Derived **mechanically** from `c_src/src/driver.c` and `c_src/include/driver.h`,
not from assumptions. The exhaustive grep for rejection constructs is:

```
$ grep -nE "return|assert|NULL|errno|exit|abort|RETURN_ERROR|-1|if *\(|switch|case|#if" \
      src/driver.c include/driver.h
include/driver.h:24:#ifndef DRIVER_H_      <- include guard only
src/driver.c:30:    for (int i = 0; i < len; i++) {   <- the only conditional in the library
```

Result of that grep: the library contains

* **0** `return` statements (both functions are `void`),
* **0** `assert`, `abort`, `exit`, `errno` uses,
* **0** `NULL`/pointer-validity checks,
* **0** error enums, error codes, sentinel values, or `RETURN_ERROR`-style macros,
* **0** explicit range / min / max checks,
* **1** conditional in total: the loop guard `i < len` in `print_hex`, where
  `len` is always the compile-time constant `sizeof(float) == 4` supplied by
  `driver`.

So `driver` has **no failure mode**: it is total over its input domain. Every
one of the 2^32 bit patterns a `float` argument can carry is a *valid* input
that must produce identical bytes. The "error surface" of this library is
therefore made up of (a) the degenerate/boundary values of that domain, which is
where a bit-preservation bug would hide, and (b) the ABI/symbol-level rejections.
Each row below is a distinct rejection-or-boundary condition with its own
differential test.

| # | function | trigger (the exact invalid input/condition) | expected C result | test | status |
|---|----------|---------------------------------------------|-------------------|------|--------|
| 1 | `driver` | No error path exists: `void` return, no `return`/`assert`/error code anywhere. Called with an ordinary finite value. | Returns normally (no value); writes exactly 9 bytes `"%02x%02x%02x%02x\n"`. Never signals failure. | `err_01_no_error_path_total_function` | [x] |
| 2 | `driver` | Quiet NaN, positive sign (`0x7fc00000`) — value with no "numeric" meaning; a naive `f32` round-trip may canonicalize it. | Prints the *stored* bit pattern `7fc00000` byte-reversed (`0000c07f`); no rejection. | `err_02_quiet_nan_positive` | [x] |
| 3 | `driver` | Quiet NaN, negative sign (`0xffc00000`) — sign bit on a non-number. | Prints `0000c0ff`; no rejection. | `err_03_quiet_nan_negative` | [x] |
| 4 | `driver` | **Signalling** NaN (`0x7f800001`, `0xff800001`) and all sNaN payloads. C `memcpy` cannot canonicalize; Rust must not either. | Prints the exact stored bits; no trap, no rejection. | `err_04_signalling_nan` | [x] |
| 5 | `driver` | NaN with every distinct mantissa payload, incl. payload `1` and payload `0x3fffff` (all-ones), both signs. | Exact bits printed; no rejection. | `err_05_nan_all_payloads` | [x] |
| 6 | `driver` | ±Infinity (`0x7f800000`, `0xff800000`) — the exponent-overflow boundary. | Prints `0000807f` / `000080ff`; no rejection. | `err_06_infinities` | [x] |
| 7 | `driver` | Negative zero (`0x80000000`) — compares `== 0.0` yet must not print as `+0.0`. | Prints `00000080`, *not* `00000000`; no rejection. | `err_07_negative_zero` | [x] |
| 8 | `print_hex` | Symbol-surface rejection: `print_hex` is `static`, so `dlsym(handle, "print_hex")` is a rejected lookup. | `dlsym` fails / returns NULL in the C `.so`; must fail identically in the Rust `.so`. | `err_08_static_helper_not_exported` | [x] |
| 9 | (loader) | Non-existent symbol lookup (`driver_init`, `print_hex_impl`, `driver2`, ...) — the generic "unknown entry point" rejection. | `dlsym` fails for both libraries, with the same set of names failing. | `err_09_unknown_symbols_rejected` | [x] |
| 10 | `driver` | One step past the boundary of each `float` class: `nextafter` of 0, of `FLT_MIN` (largest subnormal `0x007fffff` / smallest normal `0x00800000`), of `FLT_MAX` (`0x7f7fffff` -> `0x7f800000` = Inf). | Exact bits printed for each; no clamping, no rejection. | `err_10_one_past_class_boundaries` | [x] |
| 11 | `driver` | Zero/oversized *length* analogue: `print_hex`'s `len` guard `i < len`. `len` is hard-wired to `sizeof(float) == 4` by `driver`, so exactly 4 bytes must be emitted — never 0, never > 4 — for every input. | Output length is invariably 9 bytes (8 hex digits + `'\n'`). A 0-byte or 5+-byte dump is the divergence being ruled out. | `err_11_output_length_invariant` | [x] |
| 12 | `driver` | Byte values requiring zero padding by `%02x` (any byte `< 0x10`, e.g. bits `0x00000001`, `0x01010101`, `0x0f0f0f0f`) — the classic width-specifier divergence. | Each byte is *two* lowercase hex digits with a leading zero (`01`, not `1`); total still 9 bytes. | `err_12_zero_padding_low_bytes` | [x] |
| 13 | `driver` | Byte values `>= 0x80` (e.g. bits `0xffffffff`, `0x80808080`) — `p[i]` is `unsigned char` promoted to `int`, so it must **not** sign-extend to `ffffffff`. | Prints `ff`/`80` per byte (2 digits), never `ffffffff`. | `err_13_no_sign_extension_high_bytes` | [x] |
| 14 | `driver` | Repeated / interleaved invocation (calling C then Rust then C ... on one shared glibc `stdout`), i.e. no hidden per-call state or one-shot init that could make a second call diverge. | Every call is independent; the Nth call's 9 bytes equal the 1st call's for the same input. | `err_14_repeated_and_interleaved_calls` | [x] |
| 15 | `driver` | **ABI-level out-of-range argument** (the `enum`-with-no-valid-variant analogue): the symbol is called through an `extern "C" fn(f64)` pointer, so the upper 96 bits of `xmm0` carry caller junk (`0xffffffff`, `0xdeadbeef`, 200 random words) that the declared `float` parameter does not cover. | Only the low 32 bits are read; the junk never reaches the output. Same 9 bytes as the equivalent `float` call. | `err_15_abi_upper_lane_garbage_ignored` | [x] |
| 16 | `driver` (=> `print_hex`) | **Output-stream write failure**: fd 1 redirected to `/dev/full`, so every write fails with `ENOSPC`. The C ignores `printf`/`putchar` return values, so the Rust must ignore them too (no panic, no unwrap, no early exit). | Both complete all 4 byte writes and return normally; `fflush` returns `-1` and `ferror(stdout)` is set identically for both. | `err_16_output_stream_write_failure` | [x] |

Notes on generic boundaries that the checklist asks about but that this API
cannot express — recorded so the omission is explicit rather than an oversight:

* **Null pointers:** the public API is `void driver(float x)`. It takes no
  pointer, no buffer, and no length; there is no pointer argument that could be
  NULL. The library's only pointer (`print_hex`'s `p`) is internal and always
  receives the address of a live 4-byte stack array. Row 8 covers the only
  pointer-related surface reachable by an external caller (its symbol lookup).
* **Out-of-range enum values across FFI:** the library declares no `enum` and no
  `typedef` (grep for `enum|typedef` over both C files returns nothing), and its
  single parameter is a `float`, whose every bit pattern is in range. The two
  analogues of "a value with no valid variant" are covered: the NaN/sNaN and
  ±Inf space (rows 2–6 and 10, over all payloads) and an argument *wider than the
  declared parameter* passed across the FFI boundary (row 15).
* **Zero / oversized lengths:** there is no caller-supplied length; row 11
  pins the internal one down as an invariant.
