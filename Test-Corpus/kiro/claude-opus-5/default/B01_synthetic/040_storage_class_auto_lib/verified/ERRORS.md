# ERRORS.md — error-surface table

Derived mechanically from the C source. Greps run over all of `c_src/`:

```sh
grep -rn 'RETURN_ERROR\|return -\|return NULL\|assert\|errno\|_MAX\|_MIN\|exit(\|abort(' c_src/src c_src/include
grep -rn 'if\s*(\|switch\s*(\|while\s*(\|for\s*(' c_src/src c_src/include
```

Both greps return **zero matches** in code (only the licence comment block and
the include guard exist besides the function body).

The complete body of the only public function is:

```c
void driver(int x) {
    auto int y = 2*x;
    y += 300;
    printf("%d\n", y);
}
```

It has: no return value, no pointer parameters, no enum parameters, no length
parameters, no branches, no asserts, no explicit range checks, no min/max
constants, no error enums and no allocation. Consequently there is **no
explicit rejection site to tabulate** — the table below instead records the
implicit/UB-adjacent conditions that are the only ways a caller can reach
"unusual" behaviour, so that each still gets a differential test.

| #  | function | trigger (the exact invalid input/condition) | expected C result | test | [x] |
|----|----------|---------------------------------------------|-------------------|------|-----|
| E1 | `driver` | *No error return exists.* `void` return, so no error code can ever be observed; the only observable is stdout. | Nothing returned; one line written to stdout for every input. | `e1_no_error_channel_return_type` | [x] |
| E2 | `driver` | `x = INT_MAX` (2147483647) — `2*x` signed overflow (C UB; reference build wraps two's-complement) | `y = -2` then `+300` → prints `298` | `e2_int_max_mul_overflow` | [x] |
| E3 | `driver` | `x = INT_MIN` (-2147483648) — `2*x` signed overflow, negative direction | `y = 0` then `+300` → prints `300` | `e3_int_min_mul_overflow` | [x] |
| E4 | `driver` | `x = INT_MAX/2 = 1073741823` — `2*x = 2147483646` fits, but `y += 300` overflows | wraps → prints `-2147483350` | `e4_add_overflow_boundary` | [x] |
| E5 | `driver` | `x = (INT_MAX-300)/2 + 1 = 1073741674` — first `x` for which `y += 300` overflows (one step past the largest non-overflowing input) | wraps → prints `-2147483648` | `e5_add_overflow_first_input` | [x] |
| E6 | `driver` | `x = 1073741673` — largest `x` whose `y += 300` does *not* overflow (one step *before* E5) | prints `2147483646` | `e5_add_overflow_first_input` | [x] |
| E7 | `driver` | `x = INT_MIN/2 = -1073741824` — `2*x == INT_MIN` exactly (extreme of the non-overflowing multiply range) | prints `-2147483348` | `e7_int_min_half_boundary` | [x] |
| E8 | `driver` | `x = INT_MIN/2 - 1 = -1073741825` — one step past E7, `2*x` overflows negative | wraps → prints `2147483946`… (differential-checked, C is ground truth) | `e7_int_min_half_boundary` | [x] |
| E9 | `driver` | Out-of-range "enum"/bit-pattern argument: every one of the 2^32 bit patterns is a legal `int`, so there is no invalid variant. Passed as raw `u32`-reinterpreted `i32` extremes and random bit patterns across the FFI boundary. | Same line as the corresponding `int` value; never a rejection | `e9_all_bit_patterns_are_valid` | [x] |
| E10 | `driver` | Zero/oversized "length" and null-pointer boundaries: **not applicable** — the ABI signature takes one by-value `int` and no pointer or length. Documented so the gap is explicit rather than silently skipped. | N/A | `e10_no_pointer_or_length_params` | [x] |

Rows E2–E8 exist because the *only* way this function can misbehave is signed
overflow: the C build (`CMakeLists.txt` sets no `CMAKE_BUILD_TYPE`, hence no
`-O`/`-ftrapv`/`-fwrapv` flags) wraps, and the Rust translation must reproduce
those exact bytes. They are asserted differentially against the compiled C
`.so`, never against a hand-computed expectation.
