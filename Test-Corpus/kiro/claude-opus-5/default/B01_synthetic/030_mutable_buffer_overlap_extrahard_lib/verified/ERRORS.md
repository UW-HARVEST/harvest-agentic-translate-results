# ERRORS.md — Phase C error / rejection surface table

## How this table was derived

Mechanical grep of the whole C source for every rejection construct:

```sh
grep -nE "return|assert|NULL|errno|error|ERROR|if *\(|switch|#if" \
    c_src/src/driver.c c_src/include/driver.h
```

The **only** hit is the `#ifndef DRIVER_H_` include guard in the header.

`c_src/src/driver.c` therefore contains:

* **0** `return` statements with a value (both public functions are `void`)
* **0** `assert` / `NULL` checks / range checks
* **0** error enums, error codes, sentinel returns or `errno` writes
* **0** `if` / `switch` / `#ifdef` branches
* **0** min/max constants

There is no explicit error surface. Every "rejection" this API has is
**implicit**: it is produced by the loop bound `i < len`, by the C conversion
rules on `len * sizeof(int)`, or it is undefined behaviour that manifests as a
fault. The rows below are therefore the complete set of *implicit* rejections
and boundaries derived from the four executable statements of the source
(lines 29-46).

Legend for the `mode` column:

* `in-proc` — safe to call directly in the test process.
* `forked`  — the condition faults (UB); the differential test runs each call
  in its own `fork()`ed child and asserts the C child and the Rust child are
  terminated **the same way** (same exit code, or same signal), so "same
  rejection" is compared, not merely "both failed somehow".

## Table

| # | function | trigger (the exact invalid input/condition) | expected C result | mode | test | [x] |
|---|----------|----------------------------------------------|-------------------|------|------|-----|
| 1 | `fma_array` | `len == 0` (loop guard `0 < 0` false on first evaluation, line 30) | no-op: zero iterations, no load, no store, returns void | in-proc | `err_01_fma_len_zero_no_writes` | [x] |
| 2 | `fma_array` | `len == -1` (negative length; `0 < -1` false) | no-op: zero iterations, no memory touched | in-proc | `err_02_fma_len_negative` | [x] |
| 3 | `fma_array` | `len == INT_MIN` (most negative length, one past the negative range) | no-op: zero iterations, no memory touched | in-proc | `err_03_fma_len_int_min` | [x] |
| 4 | `fma_array` | all four pointers `NULL` **and** `len == 0` (null ptr never dereferenced because loop body is unreachable) | no-op, no fault | in-proc | `err_04_fma_all_null_len_zero` | [x] |
| 5 | `fma_array` | all four pointers `NULL` **and** `len < 0` | no-op, no fault | in-proc | `err_05_fma_all_null_len_negative` | [x] |
| 6 | `fma_array` | `out == NULL`, `len > 0` (store through null on line 31) | fault — `SIGSEGV` | forked | `err_06_fma_null_out_len_positive` | [x] |
| 7 | `fma_array` | `mul1 == NULL`, `len > 0` (load through null on line 31) | fault — `SIGSEGV` | forked | `err_07_fma_null_mul1_len_positive` | [x] |
| 8 | `fma_array` | `mul2 == NULL`, `len > 0` | fault — `SIGSEGV` | forked | `err_08_fma_null_mul2_len_positive` | [x] |
| 9 | `fma_array` | `add == NULL`, `len > 0` | fault — `SIGSEGV` | forked | `err_09_fma_null_add_len_positive` | [x] |
| 10 | `fma_array` | `len` greater than the real allocation (oversized length ⇒ out-of-range index read/write far past the buffer) | fault — `SIGSEGV` | forked | `err_10_fma_len_oversized` | [x] |
| 11 | `fma_array` | signed integer overflow of `mul1[i] * mul2[i]` (e.g. `INT_MAX * INT_MAX`) — UB in C, no check present | wraps modulo 2^32 (x86-64 `imul`) | in-proc | `err_11_fma_mul_overflow_wraps` | [x] |
| 12 | `fma_array` | signed integer overflow of `… + add[i]` (e.g. product `INT_MAX`, `add == 1`) — UB in C, no check present | wraps modulo 2^32 (x86-64 `add`) | in-proc | `err_12_fma_add_overflow_wraps` | [x] |
| 13 | `fma_array` | `INT_MIN * -1` (the one product with no two's-complement representation) | wraps to `INT_MIN` | in-proc | `err_13_fma_int_min_times_minus_one` | [x] |
| 14 | `driver` | `len == 0` — zero-length VLA `int out[0]` (line 43) plus `memcpy(out, data, 0)` | no fault, prints nothing | in-proc | `err_14_driver_len_zero_no_output` | [x] |
| 15 | `driver` | `data == NULL` **and** `len == 0` (`memcpy` with null source and zero count) | no fault, prints nothing | in-proc | `err_15_driver_null_data_len_zero` | [x] |
| 16 | `driver` | `data == NULL` **and** `len > 0` (`memcpy` reads through null) | fault — `SIGSEGV` | forked | `err_16_driver_null_data_len_positive` | [x] |
| 17 | `driver` | `len == -1`: VLA of negative size, and `len * sizeof(int)` converts `int -1` to `size_t` ⇒ `0xFFFFFFFFFFFFFFFC` byte copy | fault — `SIGSEGV` | forked | `err_17_driver_len_minus_one` | [x] |
| 18 | `driver` | `len == INT_MIN` (one step past the negative range): `size_t` byte count `0xFFFFFFFF_00000000` | fault — `SIGSEGV` | forked | `err_18_driver_len_int_min` | [x] |
| 19 | `driver` | `len` larger than the source buffer (oversized length ⇒ `memcpy` reads past the end) | fault — `SIGSEGV` | forked | `err_19_driver_len_oversized` | [x] |
| 20 | `driver` | `len` so large the VLA exceeds the stack (`len == 1 << 28`, 1 GiB) | fault — `SIGSEGV` (stack overflow) | forked | `err_20_driver_vla_stack_overflow` | [x] |

## Generic FFI boundaries required by Phase C

| boundary | applies here? | covered by |
|---|---|---|
| null pointers | yes — every pointer parameter, with both harmless (`len <= 0`) and faulting (`len > 0`) lengths | rows 4-9, 15, 16 |
| zero length | yes | rows 1, 4, 14, 15 |
| oversized length | yes | rows 10, 19, 20 |
| one step past a valid range | yes — `INT_MIN` length, `INT_MIN * -1`, `INT_MAX + 1` | rows 3, 12, 13, 18 |
| **out-of-range enum values across FFI** | **not applicable** — the public API (`c_src/include/driver.h`) declares no `enum`, and neither `driver` nor `fma_array` takes an enum, flag, mode or `int` tag parameter. The only non-pointer parameter is `len`, whose entire `int` range (negative, zero, positive, `INT_MIN`, oversized) is covered by rows 1-3, 10, 17-20. | rows 1-3, 10, 17-20 |

## Divergences found and fixed (Rust changed, C untouched)

Both were rejection-path bugs invisible to the happy path; both were found by
this table's rows, not by the valid-path tests.

1. **`driver` with an over-long `len` aborted instead of faulting.**
   Found by the `len == INT_MAX` case of `boundary_sweep_len_domain_faulting`.
   The C `int out[len]` is a stack VLA, so a `len` whose array cannot fit in the
   remaining stack dies with `SIGSEGV` (signal 11). The translation used
   `vec![0; len]`, so the same input hit a *heap* allocation failure and Rust's
   allocation-error handler aborted with `SIGABRT` (signal 6), after printing
   `memory allocation of 8589934588 bytes failed`.
   Fix: `driver` now calls `vla_stack_probe(n_bytes)` before allocating. It
   reads the current thread's stack extent (`pthread_getattr_np` /
   `pthread_attr_getstack`) and, when the array would not fit, performs the same
   below-the-stack access the C `memcpy` performs, producing an identical
   `SIGSEGV`. Sizes under 64 KiB and byte counts that wrap the address space
   (negative `len`) skip the probe, because the C faults inside `memcpy` in those
   cases and the translation already does too (rows 17-18).

2. **`fma_array` with a NULL pointer aborted instead of faulting — in debug
   builds only.** Found by running rows 6-9 against the *debug* `.so` as well as
   the release one. With `debug-assertions` on, rustc inserts a UB precondition
   check on raw-pointer dereferences, so the null deref panicked and, because
   unwinding cannot cross `extern "C"`, aborted with `SIGABRT` (6) where the C
   dies with `SIGSEGV` (11). The release `.so` had no such check and matched.
   Fix: `debug-assertions` and `overflow-checks` are disabled for **both**
   profiles in `Cargo.toml`, since the C reference performs no null, alignment
   or overflow checking; the loop counters were also switched to explicit
   `wrapping_add` so the source does not depend on that profile setting.

## Harness sensitivity check

To confirm these tests can actually fail (rather than passing vacuously or
accidentally loading the C library twice), three mutations were injected into
`src/lib.rs`, each caught, then reverted:

| mutation | caught by |
|---|---|
| `wrapping_add` → `saturating_add` in `fma_array` | `cfg_09`, `cfg_10`, `cfg_11`, … (all `Full`/`Boundary` rows) |
| loop bound `i < len` → `i < len - 1` | 20+ `cfg_*` rows across both entry points |
| `vla_stack_probe` call removed | `boundary_sweep_len_domain_faulting` (signal 11 vs 6) |
