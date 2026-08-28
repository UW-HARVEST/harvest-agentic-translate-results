# ERRORS.md — Phase A: error / rejection surface table

Mechanically derived by grepping `c_src/src/lib.c` and `c_src/include/lib.h`
for every rejection mechanism:

```sh
$ grep -nE 'return|assert|RETURN_ERROR|NULL|errno|if *\(|<|>|\?' c_src/src/lib.c
34: float pow43(int x) {
37:     if (x < 129) {
38:         return g_pow43[16 + x];
40:     if (x < 1024) {
46:     return g_pow43[16 + ((x + sign) >> 6)] *
```

**Result of the grep: the C library contains _no_ error-reporting mechanism at
all.**

* no `assert` / `static_assert` / `NDEBUG` use
* no `errno`, no error enum, no error-code `typedef`
* no sentinel returns (`-1`, `NULL`, `NAN`, …) — the return type is `float`
  and *every* `return` statement yields a computed table value
* no pointer parameters, therefore no null checks
* no length/count parameters, therefore no zero-/oversized-length checks
* no `enum` parameters, therefore no out-of-range-enum path
* no range check of any kind on `x` — the two `if`s are *dispatch*, not
  validation

Consequently the "rejection" surface of this API consists purely of
**implicit** failure modes: the unchecked array subscript and the signed
integer arithmetic. Each distinct one gets a row below. `INT_MIN`/`INT_MAX` and
"one step past the valid range" are included even though the C does not check
them, per the Phase C instructions.

Notation: `T = g_pow43`, `N = 145` (`129 + 16` entries), `idx` is the C
subscript expression, `sign = 2*x & 64`, `mult ∈ {16, 256}`.

| #  | function | trigger (the exact invalid input/condition) | expected C result |
|----|----------|---------------------------------------------|-------------------|
| 1  | `pow43` | `x == -16` — lowest `x` for which `idx = 16 + x` is still `>= 0`; **not** an error, it is the exact lower edge of the defined domain | returns `T[0]` = `+0.0f` (`0x00000000`); Rust must return the identical bits |
| 2  | `pow43` | `x == -17` — one step past the lower edge: `idx = -1`, subscript **before** the start of `T` | C has no check: performs an out-of-bounds load of `*(T - 1)` (UB, ISO C 6.5.6p8). Value is whatever the linker placed before `.rodata`, so it is **not** reproducible across the two objects. Requirement: same *index* is computed and the process does not trap — verified table-relatively (see below). |
| 3  | `pow43` | `x <= -18` down to `INT_MIN` — arbitrarily far before `T` | same class as row 2; for `x` far enough below `-16` the address leaves every mapped page and the C **segfaults**. Requirement: identical computed index; only inputs whose target address is inside a readable mapping are actually called. |
| 4  | `pow43` | `x == 8223` — largest `x` for which `idx = 16 + ((x + sign) >> 6) == 144 == N-1`; the exact upper edge of the defined domain (`x = 8192..8223` all give `idx = 144`) | returns `T[144] * poly * 256`; Rust must match bit-for-bit |
| 5  | `pow43` | `x == 8224` — one step past the upper edge: `x & 32 != 0` so `sign = 64`, `(x + 64) >> 6 = 129`, `idx = 145 == N`, i.e. one element **past the end** of `T` | C has no check: out-of-bounds load of `*(T + 145)` (UB). Not reproducible across objects (C reads trailing `.rodata`, Rust reads its own trailing `.rodata`). Requirement: same computed index, same `poly`, same `mult`; verified table-relatively. |
| 6  | `pow43` | `x >= 8225` up to `INT_MAX` — arbitrarily far past `T` | same class as row 5; for large `x` the address leaves every mapped page and the C **segfaults**. Requirement: identical computed index; only inputs landing in a readable mapping are called. |
| 7  | `pow43` | signed overflow of `2 * x` in `sign = 2 * x & 64` — happens for every `x > INT_MAX/2 = 1073741823` (and, symmetrically, would for `x < INT_MIN/2`, unreachable because `x < 129` returns early) | UB per ISO C 6.5p5; every real compiler wraps two's-complement. Only bit 6 of the product is used, and wrapping preserves the low bits, so `sign` is well determined: `sign = 64 ⟺ (x & 32) != 0`. Rust must use wrapping (`wrapping_mul`), *not* a panicking/saturating multiply. |
| 8  | `pow43` | signed overflow of `x << 3` — **unreachable**: guarded by `129 <= x < 1024`, so `x << 3 <= 8184`. Listed to record that the branch was examined and needs no wrap semantics. | no overflow possible; `mult = 16`, `x` becomes `1032..8184` |
| 9  | `pow43` | signed overflow of `x + sign` in `(x + sign) >> 6` — happens for `x > INT_MAX - 64 = 2147483583` when `sign == 64` | UB per ISO C 6.5p5; wraps to a large **negative** value, so `idx` becomes a huge negative subscript. Rust must reproduce with `wrapping_add` and an **arithmetic** `>>`. Address is unmapped ⇒ C segfaults; only the index is compared. |
| 10 | `pow43` | division by zero in `frac = (float)((x & 63) - sign) / ((x & ~63) + sign)` — **unreachable**. Reaching the division requires `x >= 129`, after which `x >= 129` (or `x = 8*x' >= 1032`), so `(x & ~63) >= 64` and `sign >= 0`, hence the denominator is `>= 64`. Verified exhaustively for `x = 129..300000` and by the bit argument for the rest. | never taken; therefore neither `inf` nor `NaN` can be produced by the division. A differential test asserts **no** input in the defined domain yields a non-finite result in either library. |
| 11 | `pow43` | "out-of-range enum value across FFI" — **not applicable**: the sole parameter is `int`, which has no invalid bit pattern. The generic analogue is exercised instead: the full `i32` range is treated as valid input and driven through both objects. | every `int` is an accepted argument; there is no rejection path |
| 12 | `pow43` | null pointer / zero length / oversized length — **not applicable**: the API takes no pointer and no length. Recorded so the generic-boundary checklist is demonstrably complete. | n/a |

## Why rows 2, 3, 5, 6 and 9 cannot be byte-compared, and what is asserted instead

Rows 2/3/5/6/9 are all the *same* defect in the C: `g_pow43[...]` is
subscripted without a bounds check. Outside `x ∈ -16..=8223` the C reads memory
that does not belong to the array. What it finds is decided by the linker's
`.rodata` layout of *that particular shared object* — measured, `x = 8224`
yields `0x4262615d` from the C object and a byte of a Rust string literal from
the Rust object. No translation can make those agree, and demanding it would
mean encoding one compiler's section layout into the Rust source.

So instead of byte-comparing the *value*, the error-path tests verify the thing
that is actually a property of the translation: **both objects compute the same
subscript, `sign`, `frac`, `poly` and `mult`.** Three complementary techniques
are used, in increasing order of reach:

1. **Table-relative oracle** (`tests/errors.rs`) — each object's private
   `g_pow43` is located at run time (`/proc/self/maps` is parsed for the
   object's readable mappings and scanned for the 145-float byte pattern), and
   the object's return value is compared against
   `*(T_that_object + idx) * poly * mult`. Covers every subscript whose address
   happens to be readable, i.e. the bands just outside the table.
2. **Synthetic mapped pages** (`tests/oob_pages.rs`) — for subscripts far
   outside the table the address is unmapped, so nothing can be observed. The
   test therefore `mmap`s a page (`MAP_FIXED_NOREPLACE`) at *exactly* the
   address each object's subscript targets and fills every 4-byte slot with a
   hash of that slot's index **relative to that object's own table base**. Both
   objects then see the same value at the same *relative* index, so equal
   results ⟺ equal subscripts. The call is made in a forked child, so an
   implementation that reads a slot we did not map is reported as
   `Signal(SIGSEGV)` rather than killing the test binary. 491 inputs verified.
3. **Fault parity** (`tests/oob_faults.rs`) — where the address cannot be mapped
   at all, the C's "rejection" *is* a fatal signal, so both objects are called in
   a forked child and asserted to die with the **same signal number** (not
   merely "both failed").

Inputs are only ever called after the harness has confirmed the target address
is dereferenceable. That check is `is_readable()`, which combines a
`/proc/self/maps` snapshot with a `write(2)`-into-a-pipe probe: the kernel
validates the address and returns `EFAULT` instead of faulting us. Both halves
are necessary — a file-backed `.so` mapping is rounded up to a page boundary, so
`/proc/self/maps` reports bytes past EOF as readable even though touching them
raises `SIGBUS`.

## Checklist

All rows verified against both `.so` objects. Test binaries:
`errors.rs` (14 tests), `oob_pages.rs` (1), `oob_faults.rs` (2).

| # | error-path differential test | status |
|---|------------------------------|--------|
| 1  | `edge_lower_minus16_exact_match` | [x] passes |
| 2  | `one_past_lower_edge_minus17_same_index` | [x] passes |
| 3  | `far_below_table_same_index` + `deep_oob_subscript_parity_via_mapped_pages` + `unmappable_subscript_faults_identically_in_both` | [x] passes |
| 4  | `edge_upper_8223_exact_match` | [x] passes |
| 5  | `one_past_upper_edge_8224_same_index` | [x] passes |
| 6  | `far_above_table_same_index` + `deep_oob_subscript_parity_via_mapped_pages` + `unmappable_subscript_faults_identically_in_both` | [x] passes |
| 7  | `overflow_of_2x_wraps_identically` | [x] passes |
| 8  | `shift_by_3_cannot_overflow` | [x] passes |
| 9  | `overflow_of_x_plus_sign_wraps_identically` + `deep_oob_subscript_parity_via_mapped_pages` | [x] passes |
| 10 | `denominator_never_zero_no_inf_or_nan` | [x] passes |
| 11 | `every_int_is_accepted_full_range_sweep` + `row26_full_i32_table_relative_sweep` | [x] passes |
| 12 | `no_pointer_or_length_parameters` | [x] passes (documented n/a, and asserts the header still declares exactly `float pow43(int x);`) |

Supporting tests: `tables_are_locatable_in_both_objects` (proves the run-time
table location works, so the out-of-bounds rows cannot silently degrade into
no-ops) and `fork_harness_reports_normal_returns` (proves a `Signal` outcome is
meaningful by checking that valid inputs are reported as normal returns).
