# ERRORS.md — Error-surface table (Phase A / gate for Phase C)

Derived mechanically from the C source. Grep used:

```sh
grep -nEi 'return|assert|NULL|-1|if|switch|#if|else|error|max|min' \
    c_src/src/staticloop.c c_src/include/staticloop.h
```

Non-comment hits (the complete set):

```
src/staticloop.c:31:  return sum;      # normal value return, not an error return
src/staticloop.c:42:  return;          # bare return at end of void driver()
include/staticloop.h:24:#ifndef STATICLOOP_H_   # header include guard only
include/staticloop.h:30:#endif //STATICLOOP_H_  # header include guard only
```

## Findings

The library has **NO error surface**:

* no error-return macro (`RETURN_ERROR`-style) exists anywhere;
* no `return -1`, no sentinel return, no error enum, no `errno` use;
* no `assert` / `NDEBUG` / abort path;
* no `NULL` check — **no function takes a pointer**, so no null-pointer input is
  representable across the FFI boundary;
* no explicit range check, no min/max constant, no clamping;
* no `if` / `switch` / `#ifdef` branch in the implementation at all (the only
  control flow is `driver`'s fixed `for (int i = 0; i < 10; i++)` loop, whose
  bounds are compile-time constants and not input-dependent);
* no enum parameter, so there is no out-of-range-enum input class;
* no length/size/count parameter, so there is no zero-length or oversized-length
  input class.

Both entry points are total functions of a single `int`: **every** `int` value is
a valid input, and neither can reject anything. Therefore the error-surface table
has **zero rows** for library-defined rejections.

| # | function | trigger (exact invalid input/condition) | expected C result |
|---|----------|------------------------------------------|-------------------|
| — | — | *(none: the C source contains no rejection, error return, assert, range check, null check, or min/max constant)* | — |

## Generic-boundary rows tested anyway (Phase C)

Because "no error path" is itself a claim that must be verified differentially,
the generic boundaries every C API has are still tested — for these signatures
the only representable extremes are the `int` domain boundaries and the
undefined-behaviour-adjacent arithmetic edges. Each row below asserts that C and
Rust return the *same* value (and, for `driver`, the same stdout), i.e. that both
accept the input identically rather than one of them rejecting/trapping.

| # | function | boundary condition constructed | expected C result | test | status |
|---|----------|--------------------------------|-------------------|------|--------|
| E1 | `static_sum` | `update = 0` (identity / zero-length analogue) | returns accumulator unchanged; no rejection | `err_e1_static_sum_zero` | [x] |
| E2 | `static_sum` | `update = INT_MAX` (max representable) | wraps `sum + INT_MAX` two's-complement; no rejection | `err_e2_static_sum_int_max` | [x] |
| E3 | `static_sum` | `update = INT_MIN` (min representable, one past `-INT_MAX`) | wraps `sum + INT_MIN` two's-complement; no rejection | `err_e3_static_sum_int_min` | [x] |
| E4 | `static_sum` | positive signed-overflow of the accumulator (`sum` driven past `INT_MAX`) | wraps to negative; no rejection/trap | `err_e4_static_sum_overflow_positive` | [x] |
| E5 | `static_sum` | negative signed-overflow of the accumulator (`sum` driven past `INT_MIN`) | wraps to positive; no rejection/trap | `err_e5_static_sum_overflow_negative` | [x] |
| E6 | `driver` | `stride = 0` (degenerate stride) | prints 10 lines, accumulator unchanged; returns `void` | `err_e6_driver_zero_stride` | [x] |
| E7 | `driver` | `stride = INT_MAX` (max representable ⇒ `i * stride` overflows for `i >= 2`) | prints 10 wrapped values; no rejection/trap | `err_e7_driver_int_max` | [x] |
| E8 | `driver` | `stride = INT_MIN` (min representable ⇒ `i * stride` overflows for `i >= 2`) | prints 10 wrapped values; no rejection/trap | `err_e8_driver_int_min` | [x] |
| E9 | `driver` | `stride` just past the largest non-overflowing stride (`INT_MAX/9 + 1`) — one step past the "valid" range | prints 10 values, last one wrapped; no rejection | `err_e9_driver_one_past_nonoverflow_range` | [x] |
| E10 | both | out-of-range "enum" analogue: arbitrary bit patterns reinterpreted as `int` (`0x80000000`, `0xFFFFFFFF`, `0x7FFFFFFF`) passed across FFI | accepted as ordinary `int`s; identical results | `err_e10_arbitrary_bit_patterns` | [x] |
| E11 | both | wrong-arity / garbage upper 32 bits: value passed in the 64-bit register with dirty high half (`i64` cast to `i32` at the boundary) | only low 32 bits significant; identical results | `err_e11_dirty_high_bits` | [x] |

All rows above pass. Tests live in `tests/phase_c_errors.rs` (plus the extra
`err_generic_boundary_sweep`, which sweeps every value one step around each
interesting point for both entry points). Run with:

```sh
cargo test --test phase_c_errors -- --test-threads=1
```
