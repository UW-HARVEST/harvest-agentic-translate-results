# ERRORS.md — Phase A: ERROR-SURFACE TABLE

Derived **mechanically** from the C source, not from docs or assumptions.

## Mechanical derivation

The complete C implementation is 9 lines (`c_src/src/lib.c`) plus a 3-line
header (`c_src/include/lib.h`). Grepping the entire C source for every
rejection / error / branch construct:

```sh
grep -rnE 'RETURN_ERROR|return *-|return *NULL|return *0x|assert|abort|exit\(|errno|goto|ERROR|FAIL|_MIN|_MAX|if *\(|switch|while|for *\(|\?|&&|\|\||#if' src include
# -> NO MATCHES
```

```sh
grep -rnE '\*|\[|enum|struct|size_t|len|count|typedef|void' src include
# -> NO MATCHES
```

Both greps return **zero matches**. This establishes, mechanically, that:

* there is **no** error-return macro, `return -1`, `return NULL`, or error enum;
* there is **no** `assert`, `abort`, or `exit`;
* there is **no** explicit range check, null check, or min/max constant;
* there is **no** `if` / `switch` / `?:` / `&&` / `||` — the function is
  entirely **branch-free** straight-line code;
* there is **no** pointer, array, `struct`, `enum`, `size_t`, or length/count
  parameter anywhere in the public API.

## Rejection table

| # | function | trigger (the exact invalid input/condition) | expected C result |
|---|----------|----------------------------------------------|-------------------|
| — | — | *(none — the C code contains zero rejection paths)* | — |

`rev16` is a **total function**: it accepts a single `uint32_t` by value and
every one of the 2^32 bit patterns is a valid input that produces a defined
result. There is no input for which the C code errors, rejects, asserts, traps,
or returns a sentinel. The table is therefore legitimately empty — this is not
an omission, it is the mechanically-derived truth about this library.

## Generic FFI boundary conditions

Because the rejection table is empty, the mandated generic boundaries are
covered instead. Each row below is a real input the C code handles and the Rust
must handle identically; each has a differential test in
`tests/differential.rs`.

| # | condition | applicability | expected C result | test | status |
|---|-----------|---------------|-------------------|------|--------|
| G1 | Null pointer argument | **N/A** — the API has no pointer parameters (grep above finds no `*`). Nothing can be null. | n/a | documented | [x] |
| G2 | Zero length | **N/A** — the API has no length/count/`size_t` parameter. | n/a | documented | [x] |
| G3 | Oversized length | **N/A** — the API has no length/count/`size_t` parameter. | n/a | documented | [x] |
| G4 | Out-of-range enum value across FFI | **N/A** — the API has no `enum` parameter (grep above finds no `enum`). The only parameter is `uint32_t`, for which *every* bit pattern is a valid variant. | n/a | documented | [x] |
| G5 | Zero / minimum input: `a = 0x00000000` | applies | `0x00000000` | `error_g5_zero_input` | [x] |
| G6 | Maximum input: `a = 0xFFFFFFFF` | applies | `0x0000FFFF` (upper 16 bits discarded, low 16 all-ones reverse to all-ones) | `error_g6_max_input` | [x] |
| G7 | One step past the *effective* 16-bit range: `a = 0x00010000` | applies | `0x00000000` — the masks are all 16 bits wide, so the first statement silently discards bit 16 and above. **This truncation is intended C behaviour and is preserved, not "fixed".** | `error_g7_one_past_16bit_range` | [x] |
| G8 | Upper half non-zero in general: `a = (hi << 16) \| lo` for arbitrary `hi != 0` | applies | identical to `rev16(lo)` — result must be independent of `hi` for all 2^16 values of `hi` | `error_g8_upper_half_ignored` | [x] |
| G9 | Return-value width: result must never exceed 16 bits | applies | `rev16(a) <= 0xFFFF` for every input | `error_g9_result_fits_16_bits` | [x] |

## Notes on the truncation semantics (G6–G8)

The C masks (`0xAAAA`, `0x5555`, `0xCCCC`, `0x3333`, `0xF0F0`, `0x0F0F`,
`0xFF00`, `0x00FF`) are all 16-bit. The very first statement

```c
a = ((a & 0xAAAA) >> 1) | ((a & 0x5555) << 1);
```

can only produce bits within `0x0000FFFF`, so the upper 16 bits of the argument
are unconditionally dropped before any other step runs. `rev16` therefore
reverses the bit order of the **low 16 bits only** and always returns a value in
`[0, 0xFFFF]`. The Rust translation reproduces this exactly with the same masks
and the same statement order.

No step can overflow 32 bits: the widest intermediate is
`(a & 0x00FF) << 8 <= 0xFF00`. In C the `int` literals are converted to
`unsigned int` by the usual arithmetic conversions, so all arithmetic is modulo
2^32 and no undefined behaviour (signed overflow / shift) is possible. Plain
Rust `u32` operators are therefore an exact match, and no wrapping helpers are
needed.

## Completion gate (Phase C)

- [x] Every row in the rejection table has a passing differential test
      (vacuous — the table is empty, established mechanically).
- [x] Every applicable generic FFI boundary row (G5–G9) has a passing
      differential test; G1–G4 are documented as structurally inapplicable.
