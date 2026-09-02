# ERRORS.md — error-surface table

Derived **mechanically** from `c_src/src/lib.c` and `c_src/include/lib.h`.

## Mechanical grep results

```
$ grep -nE 'RETURN_ERROR|return|assert|NULL|errno|goto|error|ERROR|_MIN|_MAX|abort|exit' \
        c_src/src/lib.c c_src/include/lib.h
src/lib.c:38:        return g_pow43[16 + x];      # value return, not an error return
src/lib.c:46:    return g_pow43[16 + ...] * ...;  # value return, not an error return
```

```
$ grep -cE 'assert|RETURN_ERROR|return *-1|return *NULL|enum' c_src/src/lib.c
0
```

**Finding: the C library has NO error-return surface at all.**

* There is no error enum, no sentinel return value, no `errno` use, no `assert`,
  no `abort`, no `goto` error label, no `NULL` check, and no explicit range
  check that rejects input.
* `pow43` takes a single `int` by value — there are **no pointer parameters**,
  therefore **no null-pointer rejection path exists** in the C.
* There are **no enum parameters**, therefore no out-of-range-enum path exists.
* There are **no length/size parameters**, therefore no zero/oversized-length
  path exists.
* The return type is `float`; every `int` input maps to some `float`. The
  function never signals failure to its caller.

The only input-dependent control flow is two *branch selectors* (not
rejections), and the only way to drive the function outside its defined
behaviour is an out-of-bounds table read, which C leaves **undefined** rather
than rejecting.

## The table

Every distinct condition under which the C code does something other than
return a well-defined finite `float` for its argument. One row per distinct
condition actually present in the source.

| # | function | trigger (the exact invalid input/condition) | expected C result |
|---|----------|----------------------------------------------|-------------------|
| 1 | `pow43` | `x == -16` — smallest argument whose direct index `16 + x` is still in bounds (index 0). One step below the boundary is row 2. | returns `g_pow43[0]` = `0.0f`. **Not** an error; must match exactly. |
| 2 | `pow43` | `x < -16` (e.g. `-17`, `-64`, `INT_MIN`): `line 38` computes `g_pow43[16 + x]` with a **negative** index ⇒ out-of-bounds read before the table. C: undefined behaviour, in practice loads whatever `.rodata` precedes `g_pow43`. Also `16 + x` overflows for `x < INT_MIN + 16` (signed overflow, UB). | **No error is returned.** C reads adjacent memory and returns garbage; the value is not reproducible across two different shared objects. Rust must mirror the *shape* (no panic, no abort, returns some `f32`) — bit-value equality is not defined by the C. |
| 3 | `pow43` | `x == 8223` — largest argument whose computed index `16 + ((x + sign) >> 6)` is still in bounds (index 144, the last element). | returns the well-defined product; must match exactly. |
| 4 | `pow43` | `x > 8223` (e.g. `8224`, `8255`, `100000`): `line 46` computes index `> 144` ⇒ out-of-bounds read past the end of `g_pow43`. UB. | **No error is returned.** Reads adjacent memory; value not reproducible across two `.so`s. Rust must not panic/abort. |
| 5 | `pow43` | `x` large enough that `x <<= 3` (line 41) would overflow — **unreachable**: the shift is guarded by `129 <= x < 1024`, so `x << 3 <= 8184`. No overflow is possible on this path. | n/a — verified unreachable; no divergence possible. |
| 6 | `pow43` | Division by zero at line 44: `(x & ~63) + sign == 0`. Reachable only when `x >= 129`; on that path `x >= 1024` (post-shift) so `x & ~63 >= 1024 > 0` and `sign ∈ {0, 64}` ⇒ denominator `>= 1024`, **never zero** for `x` in `[129, INT_MAX]`. For `x` near `INT_MAX` the denominator wraps to `INT_MIN` (signed overflow, UB) but is still non-zero. | n/a — division by zero is unreachable. No `inf`/`NaN` is producible from a defined-domain input. Confirmed empirically in `tests/differential.rs::errors_row6_denominator_never_zero`. |
| 7 | `pow43` | Signed-overflow UB in `2 * x` (line 43) for `x > INT_MAX/2`, and in `(x & ~63) + sign` / `(x + sign)` (lines 44, 46) for `x` near `INT_MAX`. | **No error is returned.** gcc at `-O0` wraps two's-complement; Rust uses `wrapping_mul`/`wrapping_add` to mirror. Reachable only together with row 4 (OOB index), so the returned value is UB either way. Rust must not panic in debug. |

### Boundary inputs covered even though they are not rejections

| # | input | why it is tested | expected |
|---|-------|------------------|----------|
| 8  | `x == 128` / `x == 129` | the `x < 129` branch selector, both sides | both defined, must match bit-for-bit |
| 9  | `x == 1023` / `x == 1024` | the `x < 1024` branch selector, both sides | both defined, must match bit-for-bit |
| 10 | `x == 0` | `g_pow43[16]` = `0.0f`; also the natural "zero length" analogue for a scalar API | `+0.0f`, same sign bit |
| 11 | `x == INT_MIN`, `x == INT_MAX` | extreme values one step past every documented range | must not panic/abort in Rust (values are UB, see rows 2/4) |
| 12 | out-of-range "enum" values | **N/A** — `pow43` has no enum, flag, mode or pointer parameter. Every one of the 2^32 `int` bit patterns is a syntactically valid argument; the whole defined sub-range `[-16, 8223]` is tested exhaustively, and the UB remainder is probed for absence of panic. | see rows 2, 4, 11 |

## Defined-behaviour domain (derived, not guessed)

Solving `0 <= 16 + index <= 144` for each of the three paths:

* `x < 129`:            index `= 16 + x`               → in bounds ⟺ `x ∈ [-16, 128]`
* `129 <= x < 1024`:    `y = 8x`, index `= 16 + ((y + ((2y)&64)) >> 6)` → `[32, 144]`, **always** in bounds
* `x >= 1024`:          index `= 16 + ((x + ((2x)&64)) >> 6)` → in bounds ⟺ `x <= 8223`
  (`x = 8224` has bit 5 set ⇒ `sign = 64` ⇒ `(8224+64)>>6 = 129` ⇒ index 145, OOB)

**Defined domain = `x ∈ [-16, 8223]` (8240 values), tested EXHAUSTIVELY.**

## Row check-off status

| # | test | status |
|---|------|--------|
| 1  | `errors_row01_lowest_in_bounds` | [x] |
| 2  | `errors_row02_negative_index_oob_is_not_rejected`, `errors_row02b_index_addition_overflow_is_not_rejected` | [x] |
| 3  | `errors_row03_highest_in_bounds` | [x] |
| 4  | `errors_row04_high_index_oob_is_not_rejected`, `errors_row04b_first_oob_is_8224` | [x] |
| 5  | `errors_row05_shift_overflow_unreachable` | [x] |
| 6  | `errors_row06_denominator_never_zero` | [x] |
| 7  | `errors_row07_signed_overflow_wraps_without_rejecting` | [x] |
| 8  | `errors_row08_selector_129` | [x] |
| 9  | `errors_row09_selector_1024` | [x] |
| 10 | `errors_row10_zero_input` | [x] |
| 11 | `errors_row11_extreme_ints` | [x] |
| 12 | `errors_row12a_defined_domain_exhaustive`, `errors_row12b_undefined_remainder_never_rejected` | [x] |
| — | `errors_generic_one_step_past_every_edge`, `errors_generic_idempotent` | [x] |

## Empirical findings for the UB rows (2, 4, 7, 11)

Measured with `examples/probe.rs`, which `dlopen`s one object and calls `pow43`
in a child process so a faulting read does not kill the test binary:

```
x=-17            C: 0x00000000   Rust: 0x00000168      (both returned; values differ)
x=-1000          C: 0x0f66e0ff   Rust: 0x00038920      (both returned; values differ)
x=-65536         C: SIGSEGV      Rust: SIGSEGV
x=8224           C: 0x4262615d   Rust: 0x00e91400      (both returned; values differ)
x=1048576        C: 0x7f800000   Rust: 0xfc958948      (both returned; values differ)
x=4194304        C: SIGBUS       Rust: returned
x=2147483647     C: SIGSEGV      Rust: SIGSEGV
```

Two conclusions, both of which shape the Phase C assertions:

1. **There is no ground truth to match in the UB region.** The value an OOB read
   yields is whatever bytes neighbour `g_pow43` inside that particular shared
   object, and the two objects have different `.rodata`/`.text` layouts. C
   leaves this undefined, so byte-equality is not a requirement the C
   establishes — asserting it would be asserting a property of the linker, not
   of the translation. Likewise, *whether* the read faults depends on where the
   object's mapping ends, which also differs (`x=4194304` above).
2. **The Rust must never reject an input the C accepts, and it does not.** Over
   ~460 probes spanning both UB regions (curated boundaries plus randomized
   draws across the full `int` space, in release *and* in the debug profile with
   integer-overflow checks enabled), the Rust `.so` produced exit code 101 /
   SIGABRT / a `panicked` message **zero** times. This is the property a
   bounds-checked Rust index or non-`wrapping_*` arithmetic would break, and it
   is what `assert_rust_does_not_reject` enforces for every UB row.

## Optimization invariance of the C ground truth

Because the C is the reference, its own stability was checked: the results are
identical for the CMake build (`-O0`, the specified build) and for a `gcc -O3`
build, and both match the Rust bit-for-bit. Disassembly of `pow43` shows plain
`mulss`/`addss`/`divss` with no `vfmadd*`, so GCC's default
`-ffp-contract=fast` cannot alter the result on the baseline x86-64 target (no
FMA instruction available), and there is no double-rounding difference to
reproduce.
