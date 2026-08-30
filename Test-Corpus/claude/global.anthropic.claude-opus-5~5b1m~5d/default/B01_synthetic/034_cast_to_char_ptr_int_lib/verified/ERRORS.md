# ERRORS.md — Phase C error-surface table

Derived mechanically from `c_src/src/driver.c` and `c_src/include/driver.h`.

## Mechanical grep of every rejection construct

```
grep -nE 'return|RETURN_ERROR|NULL|assert|errno|exit|abort|goto|\?:' c_src/src/driver.c
```

Findings, exhaustively:

* `return` statements: **none** (both functions are `void` and fall off the end).
* error-return macros (`RETURN_ERROR`, `CHECK`, …): **none**.
* `NULL` / null-pointer checks: **none**.
* `assert` / `static_assert`: **none**.
* `errno`, `exit`, `abort`, `longjmp`: **none**.
* explicit range / min / max checks or named limit constants: **none**.
* error enums or sentinel values: **none**; the only public function returns
  `void`, so there is no channel through which an error could be reported.
* the only conditional in the whole library is the loop bound `i < len` in
  `print_hex`, and `len` is not caller-controlled: `driver` always passes the
  compile-time constant `sizeof(x)` (`4`).

**Conclusion: the C library has an empty error surface.** `void driver(int x)`
accepts every one of the 2^32 possible `int` bit patterns, reports nothing, and
cannot fail. There is no invalid input to construct. Inventing rows here (e.g.
"null pointer rejected") would contradict the C, which is the ground truth.

## Error-surface table

| # | function | trigger (the exact invalid input/condition) | expected C result | test | status |
|---|----------|---------------------------------------------|-------------------|------|--------|
| E1 | `driver` | *(no rejection construct exists anywhere in the library)* | n/a — unreachable, nothing to assert | n/a | n/a (vacuous) |

Because the table has no real rows, Phase C is discharged by testing the
*generic* FFI boundary conditions that every C API has, which for this ABI
(`void driver(int)`) are the extremes and unusual bit patterns of the single
`int` parameter. These ARE the "one step past a documented valid range" and
"out-of-range enum value" cases for this signature: an `int` parameter has no
invalid values, so every such probe must be accepted identically by both
libraries rather than rejected.

| # | boundary probed | rationale | test | status |
|---|-----------------|-----------|------|--------|
| B1 | `INT_MIN` (`-2147483648`) | one step past the negative end of the range | `boundary_extremes` | [x] pass |
| B2 | `INT_MAX` (`2147483647`) | one step past the positive end of the range | `boundary_extremes` | [x] pass |
| B3 | `INT_MIN + 1`, `INT_MAX - 1` | one step inside each extreme | `boundary_extremes` | [x] pass |
| B4 | `0` | all-zero bit pattern; every `%02x` group is `00` | `boundary_extremes` | [x] pass |
| B5 | `-1` (`0xffffffff`) | all-ones bit pattern; sign-extension trap for `%02x` | `boundary_extremes` | [x] pass |
| B6 | `0x80000000` as unsigned reinterpreted to `int` | sign bit only; UB-adjacent conversion a caller can still make | `boundary_extremes` | [x] pass |
| B7 | out-of-range "enum-like" ints (`-2`, `256`, `65536`, `0x7fffffff`, `12345678`) passed where a C enum would be | C enums accept any `int`; no variant check exists, so both must print the raw bytes | `out_of_range_enum_values` | [x] pass |
| B8 | values whose bytes contain `0x00` and `0x0a` (`10`, `2560`, `0x0a000a00`, `0x000a0000`) | embedded NUL / newline bytes must not truncate or add framing | `embedded_nul_and_newline_bytes` | [x] pass |
| B9 | values with a single byte in `0x80..0xff` at each of the 4 positions | catches `char`-vs-`unsigned char` sign-extension in `%02x` (would print `ffffffXX`) | `high_bit_per_byte_position` | [x] pass |
| B10 | 4096 exhaustive-ish + randomized `int` values (fixed seed) | value-dependent formatting over the whole 32-bit range | `randomized_full_range` | [x] pass |
| B11 | repeated / interleaved calls (C then Rust then C, 1000x) | shared `FILE *stdout` state, no cross-call state corruption | `interleaved_calls_share_stdout` | [x] pass |

Note on null pointers and lengths: `driver`'s only parameter is a by-value
`int`, and the pointer/length pair (`print_hex`'s `p` and `len`) is *not*
reachable from outside the `.so` — `print_hex` is `static` and unexported in C,
so a null pointer or a zero/oversized length is not an input an external caller
can supply. Both libraries hard-code `&x` and `sizeof(int)`. This is verified
structurally by the symbol diff in `SYMBOLS.md` (no `print_hex` export in
either `.so`) and asserted in the test `print_hex_is_not_exported_by_either`.
