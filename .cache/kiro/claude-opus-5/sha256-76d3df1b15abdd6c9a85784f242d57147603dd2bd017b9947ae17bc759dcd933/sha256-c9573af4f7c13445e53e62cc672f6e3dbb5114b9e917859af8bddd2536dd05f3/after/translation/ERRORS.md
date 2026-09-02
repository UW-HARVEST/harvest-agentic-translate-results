# ERRORS.md — error / rejection surface of `c_src/src/driver.c`

Derived by reading every line of the only C translation unit
(`c_src/src/driver.c`, 63 lines) and enumerating each construct that rejects,
truncates, or short-circuits on its input.

## Mechanical inventory of the C source

What the grep-style sweep actually finds:

| construct searched for | occurrences in `c_src/` |
|------------------------|--------------------------|
| `RETURN_ERROR` / error macros | 0 — none exist |
| `return -1` / `return NULL` | 0 |
| error enums / status codes | 0 — no enum or typedef in the header |
| `assert` / `<assert.h>` | 0 |
| `errno` inspection | 0 |
| explicit null checks (`if (p == NULL)`, `if (!p)`) | 0 |
| explicit range checks | 1 — `if (len == 0)` in `call_fma` |
| early `break` on failed input | 1 — `if (sscanf(...) != 1) break;` in `driver` |
| loop-bound / min-max constants | 2 — `i < 100` in `driver` (and the `int data[100]` it guards), `i < len` in `fma_array` |

So the library has **no error-code channel at all**: `fma_array` and `driver`
return `void`, and `call_fma` returns an `int` that is a *data value*, not a
status. Every rejection is therefore either a silent no-op, a truncation, a
`0` return, or undefined behaviour. The table below has one row per distinct
rejection, including the undefined-behaviour ones (marked UB) because a caller
can reach them across the FFI boundary.

## Error-surface table

| #  | function | trigger (the exact invalid input/condition) | expected C result | test |
|----|----------|----------------------------------------------|-------------------|------|
| E1 | `call_fma` | `len == 0` — hits `if (len == 0) return 0;` before any VLA is created or `data` is dereferenced | returns `0`; `data` is never read, so even a null/dangling `data` is safe | `e1_call_fma_len_zero` |
| E2 | `call_fma` | `len == 0` **and** `data == NULL` — the guard above means the null is never dereferenced | returns `0`, no fault | `e2_call_fma_len_zero_null_data` |
| E3 | `fma_array` | `len == 0` — loop condition `i < len` is false on the first test | returns without writing any element of `out`; `out` keeps its prior contents | `e3_fma_array_len_zero` |
| E4 | `fma_array` | `len < 0` (`-1`, `-100`, `INT_MIN`) — `0 < len` is false, so the loop body never runs | returns without writing anything; **not** a crash and **not** a wrap-around to a huge count | `e4_fma_array_negative_len` |
| E5 | `fma_array` | `len == 0` or `len < 0` with **all four pointers null** — no dereference happens because the body never executes | returns, no fault | `e5_fma_array_null_ptrs_nonpositive_len` |
| E6 | `driver` | `sscanf` returns `EOF` (`-1`) — input failure: the string is empty or contains only whitespace, so `%d` never sees a character. Hits `if (... != 1) break;` on iteration 0 | breaks with `i == 0`, so `call_fma(data, 0)` returns `0` via E1 and `"0\n"` is printed | `e6_driver_sscanf_eof_input_failure` |
| E7 | `driver` | `sscanf` returns `0` — matching failure: the first non-whitespace character cannot begin an `int` (`"abc"`, `"-"`, `"+"`, `".5"`, `"--5"`, `","`). Same `break`, different `sscanf` return value | breaks with `i == 0`; prints `"0\n"` | `e7_driver_sscanf_zero_matching_failure` |
| E8 | `driver` | `sscanf` succeeds `k` times then fails (`1 <= k < 100`) — trailing garbage after `k` integers | breaks with `i == k`; prints `data[k-1]`, i.e. the last successfully parsed integer, **not** an error | `e8_driver_partial_parse_then_failure` |
| E9 | `driver` | more than 100 parsable integers — the `i < 100` loop bound truncates; this is the `int data[100]` capacity limit and prevents the overflow | stops after exactly 100 conversions; prints the **100th** integer and ignores the rest of the string | `e9_driver_more_than_100_truncates` |
| E10 | `driver` | exactly 100 integers — the boundary itself: the loop runs 100 times and then exits on the bound rather than on a `sscanf` failure | prints the 100th integer | `e9_driver_more_than_100_truncates` |
| E11 | `driver` | `%d` value out of `int` range (`"2147483648"`, `"-2147483649"`, `"99999999999999999999"`) — the C code does **not** range-check; glibc's `%d` saturates the intermediate `long` and truncates to `int` | `sscanf` still returns `1`, so this is accepted, not rejected; the saturated/truncated value is stored and can be printed | `e11_driver_int_overflow_accepted` |
| E12 | `call_fma` | `len < 0` — declares `int out[len]` etc., i.e. **negative-size VLAs**: undefined behaviour, then reads `out[len-1]`, off the front of the object | **UB, empirically nondeterministic**: the value tracks the process's own stack layout, so it changes with ASLR from one `execve` to the next (`290775954`, `-2082466926`, `-857812078`, `605361042`, ... on successive fresh runs) while being perfectly stable inside a single address space. No fixed value exists to match, so this is documented and *proven*, not asserted equal; the Rust returns a deterministic `0` instead of reading out of bounds. Recorded here so the gap is explicit rather than overlooked. | `e12_call_fma_negative_len_is_ub` (+ `e12_probe_child`) |
| E13 | `call_fma` | `len == INT_MAX` — VLA of 3 x 8 GiB on the stack | UB, guaranteed stack exhaustion / `SIGSEGV` in C. Present as an `#[ignore]`d test rather than a live one: matching it would mean asking the Rust for ~24 GiB and thrashing the host to learn nothing | `e13_call_fma_int_max_len` (`#[ignore]`) |
| E14 | `fma_array` | `len > 0` with a null pointer in each of the four argument positions in turn | UB, `SIGSEGV` — the C has no null check and dereferences on iteration 0. **Differentially tested**: the call runs in a forked child, so the fatal signal is captured as data and the C's and Rust's signal numbers are compared. The *safe* null cases are E5 | `e14_fma_array_faulting_nulls` |
| E15 | `call_fma` | `len > 0` with `data == NULL` | UB, `SIGSEGV` — `fma_array` dereferences `mul2 == NULL`. Differentially tested the same way, for `len` in `{1, 2, 8, 100}` | `e15_call_fma_faulting_null_data` |
| E16 | `driver` | `in == NULL` | UB — glibc `sscanf` faults on a null input string; the C has no null check. Differentially tested via the forked-child mechanism | `e16_driver_faulting_null_input` |
| E17 | `fma_array` | signed-overflow inputs, e.g. `mul1[i] * mul2[i] + add[i]` exceeding `INT_MAX` (`INT_MAX * INT_MAX`, `INT_MIN * -1`, `INT_MAX + 1`) | signed overflow is UB per the standard, but the C library is built with no optimisation flags (`CMakeLists.txt` sets no `CMAKE_BUILD_TYPE`), so gcc emits plain two's-complement `imul`/`add` and the result wraps. The Rust uses `wrapping_mul`/`wrapping_add` to match. Differentially tested. | `e17_fma_array_signed_overflow_wraps` |

### Out-of-range enum values

There are **no enum or flag parameters anywhere in this API** — the header
declares a single function taking `const char *`, and the two other exported
functions take only pointers and `int len`. The "C enums accept any int"
class of bug therefore degenerates to out-of-range `int len`, which is
covered by rows E1, E3, E4, E12 and E13, and to the full `int` value range of
the array contents, covered by E17 and the randomised Phase B rows.

### Generic FFI boundaries

| boundary | where covered |
|----------|---------------|
| null pointers, safe (no dereference) | E2, E5 |
| null pointers, faulting | E14, E15, E16 — run in a forked child so the `SIGSEGV` is compared as data |
| zero length | E1, E3 |
| negative length | E4 (defined: no-op), E12 (UB) |
| oversized length | E9, E10 (`driver`'s 100 cap), E13 (`INT_MAX`, UB) |
| one past a valid range | E10 (100th vs 101st integer), E11 (`INT_MAX + 1` as text) |
| value-range extremes | E11, E17 |
| arbitrary raw bytes incl. 0x80..0xFF | `CONFIGS.md` C34 |

## Where the tests live

| file | rows |
|------|------|
| `tests/phase_c_errors.rs` | E1..E5, E12..E17, plus `e6_e7_sscanf_failure_modes_are_distinct` |
| `tests/phase_c_driver.rs` | E6..E11 — these need fd 1 redirected, so they run as plain functions inside a single `#[test]` in their own binary (same reasoning as `phase_b_driver.rs`) |

`driver` has no error return, so for E6..E11 the "same error code" being asserted
is the exact byte string printed: `"0\n"` for the total-rejection rows E6 and E7,
and the specific surviving integer for the partial-parse rows E8..E11. That is a
concrete sentinel, not "both failed somehow".

E6 and E7 are kept as separate rows even though they share one `if (sscanf(...)
!= 1) break;` site, because `sscanf` fails there in two different ways — `-1`
(EOF / input failure) and `0` (matching failure).
`e6_e7_sscanf_failure_modes_are_distinct` calls the shared libc directly to
confirm the split is real rather than cosmetic.

## Checklist

- [x] E1  `call_fma` len == 0
- [x] E2  `call_fma` len == 0 with null `data`
- [x] E3  `fma_array` len == 0
- [x] E4  `fma_array` len < 0
- [x] E5  `fma_array` null pointers with non-positive len
- [x] E6  `driver` sscanf EOF (input failure)
- [x] E7  `driver` sscanf 0 (matching failure)
- [x] E8  `driver` partial parse then failure
- [x] E9  `driver` more than 100 integers truncates
- [x] E10 `driver` exactly 100 integers
- [x] E11 `driver` `%d` out-of-range text accepted
- [x] E12 `call_fma` len < 0 — UB, nondeterminism proven across fresh processes
- [x] E13 `call_fma` len == INT_MAX — UB, `#[ignore]`d (would thrash the host)
- [x] E14 `fma_array` faulting nulls — both fault with the same signal
- [x] E15 `call_fma` faulting null `data` — both fault with the same signal
- [x] E16 `driver` null `in` — both fault with the same signal
- [x] E17 `fma_array` signed overflow wraps
