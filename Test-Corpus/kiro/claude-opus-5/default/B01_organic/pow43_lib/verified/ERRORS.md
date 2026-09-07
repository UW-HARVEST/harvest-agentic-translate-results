# ERRORS.md — Phase C error-surface table

Derived mechanically from the whole C source. Exhaustive grep of every
rejection-capable construct in `c_src/`:

```
$ grep -n "return\|assert\|NULL\|ERROR\|errno\|if \|switch\|#if\|enum" c_src/src/lib.c c_src/include/lib.h
c_src/src/lib.c:37:    if (x < 129) {
c_src/src/lib.c:38:        return g_pow43[16 + x];
c_src/src/lib.c:40:    if (x < 1024) {
c_src/src/lib.c:46:    return g_pow43[16 + ((x + sign) >> 6)] *
```

**Finding:** the library contains **no explicit error surface at all** — no
`RETURN_ERROR` macro, no error enum, no `return -1` / `return NULL`, no
`assert`, no null check, no explicit range check, and no `#ifdef`. `pow43`
returns a `float` unconditionally on both of its two code paths.

The entire rejection surface is therefore *implicit*: the constant
`static const float g_pow43[129 + 16]` (145 elements, valid indices `0..=144`)
is indexed **without a bounds check**, so inputs whose computed index falls
outside `0..=144` are out-of-bounds reads (UB in C). The min/max constants that
define the implicit valid domain, computed from the source:

* path A (`x < 129`): index `16 + x` ⇒ requires `x >= -16`.
* path B (`129 <= x < 1024`): `x <<= 3`, index `16 + ((x+sign)>>6)` ⇒ max index
  144 at `x == 1023`; the whole interval is in range.
* path C (`x >= 1024`): index `16 + ((x+sign)>>6)`, `sign = (2*x)&64` ⇒ in range
  up to and including `x == 8223`; `x == 8224` is the first input whose index is
  145.

Implicit valid domain: **`x ∈ [-16, 8223]`**.

For UB rows the C standard defines no result, so "same error code" is not
available. The observable, comparable contract across the FFI boundary is:
**the Rust `.so` must not panic/abort where the C `.so` returns, and must fault
where the C `.so` faults.** Each row below asserts exactly that (plus value
equality wherever the behaviour is actually defined). This is the divergence
class that matters here, because a bounds-checked Rust index would panic where C
quietly returns.

| # | function | trigger (exact invalid input/condition) | expected C result | test | [x] |
|---|----------|------------------------------------------|-------------------|------|-----|
| 1 | `pow43` | `x = -17` — first input below the table start; index `16 + x = -1` | UB read of the 4 bytes immediately preceding `g_pow43`; **returns normally**, no trap, no error sentinel | `err_row01_x_minus_17_returns_without_trap` | [x] |
| 2 | `pow43` | `x ∈ {-18, -32, -64, -100, -1000}` — index far negative but still inside a mapped page | UB out-of-bounds read; **returns normally**, no trap | `err_row02_moderately_negative_returns_without_trap` | [x] |
| 3 | `pow43` | `x = i32::MIN` (and `i32::MIN + 16`) — `16 + x` overflows signed int, index wildly out of range | UB: signed overflow + unmapped read ⇒ process fault (SIGSEGV) | `err_row03_int_min_faults_in_both` (subprocess, compares exit status) | [x] |
| 4 | `pow43` | `x = 8224` — smallest `x >= 1024` whose index is 145 (`x & 32 != 0` ⇒ `sign = 64`, `(8224+64)>>6 = 129`) | UB read one element past the table end; **returns normally** | `err_row04_x_8224_first_overrun_returns_without_trap` | [x] |
| 5 | `pow43` | `x ∈ {8256, 8320, 9000, 16384, 65536}` — index 145…1040, larger overrun | UB out-of-bounds read; **returns normally** | `err_row05_larger_overruns_return_without_trap` | [x] |
| 6 | `pow43` | `x = i32::MAX` (and `i32::MAX - 63`) — `2 * x` overflows, `(x & ~63) + sign` overflows, index overflows | UB ⇒ process fault (SIGSEGV) | `err_row06_int_max_faults_in_both` (subprocess, compares exit status) | [x] |
| 7 | `pow43` | division by zero: `(x & ~63) + sign == 0` | **unreachable** — the divide is only reached when `x >= 129`, where `x & ~63 >= 1024` (path B shifts `x` to `>= 1032` first). No input in the defined domain divides by zero. | `err_row07_no_division_by_zero_in_domain` (exhaustive scan of `[-16, 8223]`: 0 non-finite results, 0 NaN) | [x] |
| 8 | `pow43` | out-of-range enum value across the FFI boundary | **no enum exists** in the ABI: the signature is `float pow43(int x)`, so every one of the 2^32 `int` values is a syntactically valid argument. The "no valid variant" analogue is an argument outside the implicit domain, covered by rows 1–6. Additionally verified: every value in the defined domain matches bit-for-bit. | `err_row08_full_int_domain_no_enum_surface` (asserts the entire `[-16, 8223]` domain matches and documents the absence of an enum) | [x] |
| 9 | `pow43` | null pointer / zero length / oversized length | **no such surface**: the API takes no pointer, buffer, size or length parameter, and returns by value. Nothing can be null and no length can be zero or oversized. | `err_row09_no_pointer_or_length_surface` (documents; asserts the by-value ABI round-trips for the `0` and boundary inputs) | [x] |
| 10 | `pow43` | one step past each *documented branch* constant (the only literals the C branches on: `129`, `1024`) — `x = 128/129` and `x = 1023/1024` | defined, but a different code path each side of the boundary; values must match exactly | `err_row10_one_past_branch_constants` | [x] |
| 11 | `pow43` | one step past the implicit domain maximum: `x = 8223` (last valid) vs `x = 8224` (first invalid) | `8223` defined and must match bit-exactly; `8224` UB but must still return without panicking | `err_row11_one_past_domain_max` | [x] |
| 12 | `pow43` | one step past the implicit domain minimum: `x = -16` (last valid) vs `x = -17` (first invalid) | `-16` defined and must match bit-exactly; `-17` UB but must still return without panicking | `err_row12_one_past_domain_min` | [x] |

All 12 rows have a passing differential test (see
`translation/tests/differential.rs`).
