# ERRORS.md — Error-surface table

## Mechanical grep of the C source for rejection paths

```
$ grep -nE 'return|assert|RETURN_ERROR|NULL|errno|exit|abort|<[[:space:]]*0|>[[:space:]]*[A-Z_]*MAX' c_src/src/driver.c c_src/include/driver.h
(no matches other than the loop conditions `i < len`)
```

Findings:

- **Zero** `return` statements with a value. All three functions return `void`.
- **Zero** `assert` / `RETURN_ERROR` / error enums / error codes / sentinels.
- **Zero** null-pointer checks.
- **Zero** explicit range checks, and **zero** min/max constants.
- The only conditionals in the whole library are the two identical loop
  guards `i < len` in `fma_array` and `inner`.

The library therefore has **no error-reporting surface at all**: it cannot
reject any input. Every input is either processed or is undefined behaviour.
Consequently the rows below are the *implicit* rejection/boundary conditions —
the places where `len` or the pointers steer control flow or trip UB — and the
"expected C result" column records the observable behaviour of the compiled C,
which is the ground truth the Rust must reproduce.

## Error / boundary surface

| # | function | trigger (the exact invalid input/condition) | expected C result |
|---|----------|---------------------------------------------|-------------------|
| 1 | `fma_array` | `len == 0` — loop guard `0 < 0` is false | no iterations; no store through `out`; returns normally. `out` untouched. Legal even with all-NULL pointers, since no pointer is ever dereferenced. |
| 2 | `fma_array` | `len < 0` (e.g. `-1`, `INT_MIN`) — loop guard `0 < len` is false | no iterations; no store through `out`; returns normally. Negative `len` is *not* an error: it degenerates to the same no-op as `len == 0`. |
| 3 | `fma_array` | `out == NULL, mul1 == NULL, mul2 == NULL, add == NULL` **with `len <= 0`** | returns normally, no dereference, no crash (guard short-circuits before any load/store). |
| 4 | `fma_array` | `len == 1` (minimal non-empty; boundary one step past the no-op range) | exactly one store: `out[0] = mul1[0]*mul2[0] + add[0]`. |
| 5 | `fma_array` | signed integer overflow in `mul1[i] * mul2[i]` (e.g. `0x7fffffff * 0x7fffffff`) | signed overflow is UB in C; the emitted code is a two's-complement `imul`, so the observable result is the wrapped low 32 bits. Rust must use `wrapping_mul`. |
| 6 | `fma_array` | signed integer overflow in `... + add[i]` (e.g. product `INT_MAX` plus `add[i] = 1`) | same as #5: wrapped low 32 bits (`imul` then `add`). Rust must use `wrapping_add`. |
| 7 | `fma_array` | fully aliased arguments `out == mul1 == mul2 == add` (exactly how `inner` calls it) | no `restrict` qualifiers, so aliasing is legal; each iteration touches only index `i`, so the result is `out[i] = out[i]*out[i] + out[i]` for every `i`, independent of iteration order. |
| 8 | `fma_array` | partially aliased / offset arguments (`mul1 == out + 1`, etc.) | legal (no `restrict`); the result depends on the ascending `i = 0..len` iteration order, which the Rust must match exactly. |
| 9 | `driver` | `len == 0` | `int out[0]` is a zero-length VLA (UB per the standard, accepted by GCC/Clang as a zero-size alloca); `memcpy(out, data, 0)` copies nothing; `inner` runs `fma_array` with `len == 0` (no-op) and the print loop with `len == 0` (no output). Observable result: **returns normally, prints nothing**. |
| 10 | `driver` | `len < 0` (e.g. `-1`, `-2`, `-100`, `INT_MIN`) | `int out[len]` with negative `len` is UB, and `len * sizeof(int)` converts the negative `int` to `size_t`, yielding a huge count (`(size_t)(-1) * 4 == 0xFFFFFFFFFFFFFFFC`), so `memcpy` runs off the stack. **Measured: C dies with SIGSEGV.** There is no error code — the C has no check. Test `e10`. |
| 11 | `driver` | `data == NULL` with `len > 0` (1, 2, 8, 1024) | `memcpy` from NULL. **Measured: C dies with SIGSEGV; Rust dies with SIGSEGV — identical.** Test `e11`. |
| 12 | `driver` | `data == NULL` with `len == 0` | `memcpy(out, NULL, 0)` — glibc's `memcpy` never dereferences with a zero count. **Measured: both return normally (exit 0) and print nothing.** Test `e12`. |
| 13 | `driver` | `len == 1` (boundary one step past the no-op range) | prints exactly one line: `data[0]*data[0] + data[0]` (wrapped) followed by `\n`. Verified for every value in the boundary set plus 200 randoms. Test `e13`. |
| 14 | `driver` | `len` large enough that the VLA `int out[len]` exceeds the caller's stack | The threshold depends on the CALLER's stack, so the test pins the calling thread's stack to exactly 8 MiB (`common::WORKER_STACK_BYTES`). **Measured at that stack size: `len <= 2^20` (VLA <= 4 MiB) returns normally in both, byte-identically; `len >= 2^21` (VLA >= 8 MiB) kills the C with SIGSEGV or SIGABRT** (SIGABRT when the stack-overflow guard page catches it first). Tests `e14`, `e14b`. |
| 15 | `driver` | `len == INT_MAX`, `INT_MAX - 1`, `INT_MAX / 2` | `len * sizeof(int)` overflows to `0x1FFFFFFFC` and the VLA is multiple GiB. **Measured: C dies (SIGSEGV). Rust also fails hard (SIGABRT from a failed multi-GiB allocation) or does not finish; it never falsely succeeds.** Test `e15`. |
| 16 | *(FFI enum boundary)* | the library declares **no enums** and takes **no mode/flag parameters** | Nothing to test: `grep -c 'enum' c_src/src/driver.c c_src/include/driver.h` -> `0` and `0`. There is no out-of-range-enum class of bug in this API. Test `e16` asserts this against the C source itself, so the claim fails loudly if the C ever gains an enum, an `assert`, or a value-returning statement. The nearest analogue — an out-of-range `int len` such as `INT_MIN` — is covered by rows 2, 3 and 10. |
| 17 | `fma_array` | each pointer argument NULL **individually**, and every one of the 16 null-mask combinations, with `len <= 0` | No dereference occurs, so **measured: both return normally (exit 0) for all 16 masks x `len` in {0, -1, INT_MIN}.** Test `e17`. |
| 18 | `fma_array` | each pointer argument NULL individually with `len > 0` | The NULL argument is dereferenced on the first iteration. **Measured: C and Rust both die with SIGSEGV for `out`, `mul1`, `mul2`, `add` and all-four NULL — identical.** Test `e17`. |
| 19 | `fma_array` | `len` far larger than the buffer the caller allocated (`len = 2^24` against an 8-element buffer) | Reads and writes past the end of the caller's allocation. **Measured: C and Rust both die with SIGSEGV — identical.** Test `e19`. |

## Row status

All rows have a passing differential test in `tests/phase_c_errors.rs`.

| row | test | status |
|-----|------|--------|
| 1 | `e01_fma_len_zero_is_a_noop` | [x] |
| 2 | `e02_fma_negative_len_is_a_noop` | [x] |
| 3 | `e03_fma_all_null_pointers_with_nonpositive_len` | [x] |
| 4 | `e04_fma_len_one_single_store` | [x] |
| 5 | `e05_fma_multiply_overflow_wraps_identically` | [x] |
| 6 | `e06_fma_add_overflow_wraps_identically` | [x] |
| 7 | `e07_fma_full_aliasing_is_legal_and_matches` | [x] |
| 8 | `e08_fma_offset_aliasing_pins_iteration_order` | [x] |
| 9 | `e09_driver_len_zero_prints_nothing_and_returns` | [x] |
| 10 | `e10_driver_negative_len_faults_in_c` | [x] |
| 11 | `e11_driver_null_data_with_positive_len_faults_in_both` | [x] |
| 12 | `e12_driver_null_data_with_zero_len_is_fine` | [x] |
| 13 | `e13_driver_len_one_prints_exactly_one_line` | [x] |
| 14 | `e14_driver_vla_within_the_stack_matches_exactly`, `e14b_driver_vla_larger_than_the_stack_faults_in_c` | [x] |
| 15 | `e15_driver_len_int_max_faults_in_c` | [x] |
| 16 | `e16_c_source_declares_no_enums_or_flags` | [x] |
| 17 | `e17_fma_individual_null_arguments` | [x] |
| 18 | `e17_fma_individual_null_arguments` | [x] |
| 19 | `e19_zero_and_oversized_lengths_against_a_small_buffer` | [x] |
| — | `e18_len_one_step_past_each_boundary` (generic one-past-the-boundary sweep) | [x] |

## How the crashing rows are tested

Rows 10, 11, 14, 15, 18 and 19 are hard faults in the C (UB with no error
return). Each is run in a **fresh subprocess** (`common::run_crash_case`
re-execs the test binary in worker mode, rather than `fork()`ing the
multi-threaded test process), the call is performed on a thread with an
explicitly pinned 8 MiB stack so the VLA threshold is deterministic, and the
termination signal of C and Rust is compared. For rows 11, 18 and 19 the two
agree exactly (same signal). For rows 10, 14 and 15 the C dies without
producing any value, so there is no result to match.

## Residual, UB-only divergences

`fma_array` matches the C on **every** input tested, including all NULL
combinations, `INT_MIN`/`INT_MAX` lengths, oversized lengths, and every
overflow. `driver` matches the C on every input where the C has defined
behaviour. The complete set of remaining differences is confined to inputs on
which the C invokes undefined behaviour and terminates without returning:

| input | C | Rust | why |
|-------|---|------|-----|
| `driver` with `len < 0` | SIGSEGV (`memcpy` of `(size_t)len * 4` bytes) | returns normally, prints nothing | the Rust clamps the VLA length to 0 rather than reproducing a wild `memcpy` |
| `driver` with `2^21 <= len <= INT_MAX` (8 MiB stack) | SIGSEGV / SIGABRT (VLA overflows the stack) | succeeds (or fails to allocate, for multi-GiB sizes) | the C's `int out[len]` is an `alloca` on the caller's stack; the Rust uses a heap `Vec`, which has no such limit |

Both are cases where the C has no defined result. Deliberately reproducing a
segmentation fault in the Rust was rejected: it would add no verifiable
behaviour (a crash has no return value or output to compare) while making the
library unsafe for every caller. The C's behaviour on these inputs is measured,
asserted and documented instead.
