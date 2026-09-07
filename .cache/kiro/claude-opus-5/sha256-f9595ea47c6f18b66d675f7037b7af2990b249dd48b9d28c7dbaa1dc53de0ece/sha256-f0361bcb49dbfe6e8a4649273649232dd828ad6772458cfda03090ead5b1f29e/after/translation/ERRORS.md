# ERRORS.md — Phase C: error / rejection surface

## Mechanical grep of `c_src/src/lib.c` for rejection constructs

```
grep -n -E 'return|assert|NULL|errno|abort|exit|RETURN_ERROR|goto' c_src/src/lib.c
61:    return (uni);
```

Findings, stated exactly:

* `return` statements: **1** — `return (uni);` (the single, unconditional
  success return at L61).
* `assert` / `static_assert`: **0**
* `NULL` / pointer parameters / pointer dereferences: **0** — every parameter is
  a by-value `int`, so there is **no null-pointer surface at all**.
* Length / size / count parameters: **0** — so there is no zero-length or
  oversized-length surface.
* Error enums, error codes, sentinel returns, `errno` writes: **0**
* `goto` / early-exit paths: **0**
* Range checks that reject: **0**. The three `if` groups (L8, L10, L12) and
  (L31, L37, L43) and (L57, L59) are all *value-selection* branches, not
  rejections; the function returns a value on every path.
* min/max constants, `#define`s, magic limits: **0** in the source. The only
  implicit limits are those of C `int` (`INT_MIN`/`INT_MAX`), reached through
  signed-overflow-wrapping arithmetic.

**Conclusion: `encode_quant` has an empty explicit-rejection surface — it is a
total function over `int^6` and cannot fail.** So there are no
`RETURN_ERROR`-style rows to write. Fabricating rows here would be inventing
behavior the C does not have.

What remains, and what the table below therefore enumerates, is the class of
input the task calls out explicitly: the **generic FFI boundary conditions every
C API has** — out-of-range "enum-like" values, and values one step past a
usable range, i.e. the extremes at which the C's arithmetic wraps (signed
overflow). For each row the "expected C result" is *the value the compiled C
`.so` returns*, and the differential test asserts the Rust `.so` returns the
identical `int`. "Same rejection" for this API means **identical returned
`int`** — there is no other channel.

## Rows

`lsbit` is the one parameter used enum-style by the C (`0`, `4`, and
"odd"/"even" tags). It is declared `int`, so any of the 2^32 values can cross
the FFI boundary; rows 1-10 cover the values with no meaningful "variant".

| # | function | trigger (the exact invalid input/condition) | expected C result |
|---|----------|----------------------------------------------|-------------------|
| 1 | `encode_quant` | `lsbit = 0` — out-of-range enum: neither `4` nor an odd tag; `if (lsbit)` is false so the whole block is skipped | returns candidate chosen from unmodified `uni`; no error |
| 2 | `encode_quant` | `lsbit = 2` — even, non-zero, not `4`: falls to the final `else`, clears bit0 | value with bit0 of all candidates cleared |
| 3 | `encode_quant` | `lsbit = 3` — odd, not `1`: `lsbit & 1` true, sets bit0 | value with bit0 of all candidates set |
| 4 | `encode_quant` | `lsbit = 5,6,7,8,100` — arbitrary out-of-range enum ints | dispatch by `==4` / `&1` / else; no error |
| 5 | `encode_quant` | `lsbit = -1` (negative odd) — `lsbit & 1 == 1` in two's complement, so it takes the *set-bit0* branch | bit0 set (not an error) |
| 6 | `encode_quant` | `lsbit = -2, -4, -6` (negative even; note `-4` is **not** `4`) — final `else` | bit0 cleared |
| 7 | `encode_quant` | `lsbit = INT_MAX` (odd) | set-bit0 branch |
| 8 | `encode_quant` | `lsbit = INT_MIN` (even, non-zero, != 4) | clear-bit0 branch |
| 9 | `encode_quant` | `lsbit = 4` exactly — the only "special variant"; one step past it in either direction (`3`, `5`) takes a different branch | `4` -> bit0-from-`(x>>1)&(x>>2)`; `3`/`5` -> set-bit0 |
| 10 | `encode_quant` | `lsbit` = every value in `-16..=16` (exhaustive walk over the branch-relevant neighbourhood) | branch per the L12-27 chain |
| 11 | `encode_quant` | `uni = INT_MAX` — `uni1 = uni + 1` is signed overflow (UB in C; wraps to `INT_MIN` in the built `.so`) | clamp test `(uni ^ uni1) & ~7` sees a huge xor -> `uni1 = uni` |
| 12 | `encode_quant` | `uni = INT_MIN` — `uni2 = uni - 1` overflows, wraps to `INT_MAX` | clamp test -> `uni2 = uni` |
| 13 | `encode_quant` | `step = INT_MAX` — `(2*(uni&7)+1) * step` overflows for any `uni&7 >= 1` | wrapped product, then `/8` truncating toward zero |
| 14 | `encode_quant` | `step = INT_MIN` — product overflows and the wrapped `diff` can be `INT_MIN`, so `diff = -diff` is itself an overflow | wrapped negation returns `INT_MIN` |
| 15 | `encode_quant` | `step = 0` — degenerate: `diff == 0` for all three candidates, so `d0 == d1 == d2` before the `d3>>5` terms | selection driven purely by the `tgt2` terms |
| 16 | `encode_quant` | `step` negative — `((2*(uni&7)+1)*step)/8` divides a negative number; C truncates toward zero, **not** floor | truncate-toward-zero quotient |
| 17 | `encode_quant` | `pred = INT_MAX` with positive `diff` — `p0 = pred + diff` overflows | wrapped `p0` |
| 18 | `encode_quant` | `pred = INT_MIN` with negative `diff` — `p0 = pred + diff` overflows | wrapped `p0` |
| 19 | `encode_quant` | `tgt = INT_MIN` — `d0 = tgt - p0` overflows, then `d0 ^ (d0 >> 31)` is applied to the wrapped value (note: this is a *one's-complement* abs, so `INT_MIN` maps to `INT_MAX`, and the result of "abs" can still be negative) | wrapped pseudo-abs |
| 20 | `encode_quant` | `tgt = INT_MAX` — `tgt - p0` overflows the other way | wrapped pseudo-abs |
| 21 | `encode_quant` | `tgt2 = INT_MIN` / `INT_MAX` — `d3 = tgt2 - p*` overflows, then `d3 >> 5` (arithmetic shift of a possibly negative value) is added | wrapped `d0/d1/d2` |
| 22 | `encode_quant` | `d0 += d3 >> 5` overflows (`d0` near `INT_MAX`, `d3>>5` positive) | wrapped sum, so `d1 < d0` compares wrapped values |
| 23 | `encode_quant` | all six args simultaneously at `INT_MIN` / `INT_MAX` / `0` / `-1` (full 4^6 = 4096 boundary cross-product) | per-case value from the C `.so` |
| 24 | `encode_quant` | `uni = -1` — all bits set: `uni&7==7` *and* bit3 set, and the `lsbit==4` path shifts a negative value (sign-extending `>>`) | `uni1` clamped, diff negated, bit0 forced to 1 by `(uni>>1)&(uni>>2)&1` |

Total rows: **24**. Every row has a passing differential test in
`translation/tests/differential.rs` (`phase_c_*`); see the checklist in
`VERIFICATION.md`.

## Deliberately-absent generic boundaries

* **Null pointers** — not applicable: the API takes no pointers. Passing a
  pointer-sized garbage value is indistinguishable from passing an `int`, so
  there is nothing extra to test beyond row 23.
* **Zero / oversized lengths** — not applicable: the API takes no length.
* **Uninitialised / freed state** — not applicable: the API is stateless (no
  globals, no `static`, no allocation in `lib.c`).

## Row -> test mapping and checklist

Every row below was checked off only after the named test in
`tests/differential.rs` passed with the Rust `.so` loaded via `libloading`
(`cargo test --release -- --test-threads=1`).

| ERRORS.md row(s) | test | status |
|---|---|---|
| 1 | `phase_c_row01_lsbit_zero` | [x] |
| 2 | `phase_c_row02_lsbit_even_two` | [x] |
| 3 | `phase_c_row03_lsbit_odd_three` | [x] |
| 4 | `phase_c_row04_lsbit_arbitrary_out_of_range` | [x] |
| 5 | `phase_c_row05_lsbit_negative_odd` | [x] |
| 6 | `phase_c_row06_lsbit_negative_even` | [x] |
| 7 | `phase_c_row07_lsbit_int_max` | [x] |
| 8 | `phase_c_row08_lsbit_int_min` | [x] |
| 9 | `phase_c_row09_lsbit_four_and_neighbours` | [x] |
| 10 | `phase_c_row10_lsbit_exhaustive_neighbourhood` | [x] |
| 11, 12 | `phase_c_row11_12_uni_overflow` | [x] |
| 13, 14, 15, 16 | `phase_c_row13_16_step_boundaries` | [x] |
| 17, 18, 19, 20, 21, 22 | `phase_c_row17_22_pred_tgt_overflow` | [x] |
| 23 | `phase_c_row23_boundary_cross_product` (7^6 = 117 649 calls) | [x] |
| 24 | `phase_c_row24_uni_all_bits_set` | [x] |

**24 / 24 rows passing.** Each assertion compares the returned `int`
bit-for-bit, so "same rejection" is enforced as "same exact value", not merely
"both failed".
