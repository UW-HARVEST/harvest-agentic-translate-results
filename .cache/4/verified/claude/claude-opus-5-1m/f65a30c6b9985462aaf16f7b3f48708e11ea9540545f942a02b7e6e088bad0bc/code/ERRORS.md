# ERRORS.md — Phase C error-surface table

Mechanically grepped from `c_src/src/driver.c` and `c_src/include/driver.h`
for every rejection site:

```
$ grep -nE 'return|assert|NULL|errno|exit|abort|if *\(|else|switch|[<>]=?|#(if|ifdef|error)' c_src/src/driver.c
34:static void print_hex(unsigned char *p, int len) {
35:    for (int i = 0; i < len; i++) {        <-- loop bound, not a rejection
```

Findings of the mechanical sweep:

* error-return macros (`RETURN_ERROR`, …): **0 occurrences**
* `return <value>` statements: **0** — `driver` and `print_hex` are both `void`
* `return NULL`: **0** — no function returns a pointer
* `assert` / `abort` / `exit`: **0** (`<assert.h>` is not even included)
* error `enum`s / status codes: **0** — no enum, no status type anywhere
* explicit range checks / null checks: **0** — `p` is never compared to `NULL`,
  `len` is never bounds-checked, `floors` is never validated
* min/max constants, `#ifdef` config guards: **0**

**The C library therefore has an EMPTY explicit error surface.** `void
driver(int)` accepts the entire `int` domain unconditionally: every value is
valid and the function always runs to completion with no observable failure
mode. There is no code path that rejects input, so a "matching error code"
means *matching total absence of rejection* — both implementations must accept
the input, print exactly 33 bytes, and return normally.

The rows below are consequently the *generic* C-API boundaries required by
Phase C, each realised as a concrete differential test. "expected C result" is
what the C `.so` actually does when driven this way (observed, not assumed).

| # | function | trigger (the exact invalid input/condition) | expected C result | test |
|---|----------|---------------------------------------------|-------------------|------|
| E1 | `driver` | `INT_MIN` (`-2147483648`) — the negative extreme of the `int` range | no rejection; prints `00000080030000000000000000000040\n` (33 bytes, observed); returns void | `err_e1_int_min` |
| E2 | `driver` | `INT_MAX` (`2147483647`) — the positive extreme | no rejection; prints `ffffff7f030000000000000000000040\n` (33 bytes, observed) | `err_e2_int_max` |
| E3 | `driver` | `-1` (all-ones bit pattern; every byte `0xff`, exercises `%02x` on `0xff`) | no rejection; first 4 bytes `ffffffff` | `err_e3_minus_one` |
| E4 | `driver` | `0` (zero "length"/degenerate value; every byte `0x00`, exercises the `%02x` zero-pad path) | no rejection; first 4 bytes `00000000` | `err_e4_zero` |
| E5 | `driver` | `INT_MIN + 1`, `INT_MAX - 1`, `-2147483647` — one step *inside* each extreme (off-by-one probes around the range ends) | no rejection; byte-exact dump | `err_e5_one_step_inside_range` |
| E6 | `driver` | **out-of-range value passed across the FFI boundary**: the argument register holds a 64-bit value whose upper 32 bits are non-zero / garbage (`0x1_0000_0000`, `0xFFFF_FFFF_FFFF_FFFF`, `0xDEAD_BEEF_0000_0007`), i.e. a value with no valid `int` representation. Both sides must truncate to the low 32 bits identically. | no rejection; output equals `driver((int)(value & 0xffffffff))` | `err_e6_ffi_arg_width_truncation` |
| E7 | `driver` | out-of-range *enum-style* integers: values that would have no valid variant if `int` were an enum (`0x7fffffff`, `0x80000000`, `-99999`, `0xcccccccc` reinterpreted) passed directly as the `int` parameter | no rejection; each is a legal `int` and is dumped verbatim | `err_e7_out_of_range_enum_values` |
| E8 | `driver` | oversized / zero "length" cannot be injected through the public API: `print_hex`'s `len` is hard-wired to `sizeof(house_t)` and its `p` is `&house`, so `p == NULL` and `len <= 0` / `len > 16` are **unreachable**. Verified by symbol table: `print_hex` is `static` and absent from `nm -D` in *both* `.so`s, so no caller can supply a null pointer or a bad length. | n/a — unreachable in C, and equally unreachable in Rust (private `fn`) | `err_e8_print_hex_not_reachable_via_abi` (asserts the symbol is absent from both `.so`s) |
| E9 | `driver` | repeated invocation after every one of the above (no hidden error state / no latching failure) | no rejection; output identical to a fresh call — the function is stateless | `err_e9_no_latched_error_state` |

Rows E1–E9 are all covered by `tests/differential.rs` and all pass against both
`.so`s. Because the C surface has no rejection sites, "same error" is asserted
as "same byte-identical acceptance and same 33-byte output".
