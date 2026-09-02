# CONFIGS.md — Phase B configuration surface table

## How this table was derived

The public API is the full contents of `c_src/include/driver.h`:

```c
void driver(const int *data, int len);
```

plus the second externally-linked symbol found by `nm -D` on the C `.so`, which
the header does not declare:

```c
void fma_array(int *out, const int *mul1, const int *mul2, const int *add, int len);
```

`fma_array` is the **lowest-level entry point**; `driver` is the convenience
one-shot wrapper (it allocates a VLA, `memcpy`s the caller's data into it, then
calls the `static` `inner`, which calls `fma_array` with **all four pointers
aliased to the same buffer** and then `printf`s each element). Both are tested
directly through their `.so` exports.

### Axis 1 — runtime options / modes / flags

**None.** `grep -nE "if *\(|switch|#if" c_src/src/driver.c` matches nothing in
the function bodies. There is no global state, no init call, no option struct,
no mode/flag parameter and no compile-time `#ifdef` that changes behaviour.
The only conditional in the entire translation unit is the loop guard `i < len`
(lines 30 and 37). So this axis contributes exactly one value and the
cross-product below is driven by the remaining axes.

### Axis 2 — entry point

* `fma_array` (low-level, caller-supplied output buffer, 4 independent pointers)
* `driver` (wrapper: internal buffer, full self-aliasing, stdout side effect)

### Axis 3 — `len` (input shape / count)

`0`, `1`, `2`, `3` (odd tail), `8`, `17` (odd, non-power-of-two), `64`
(vectorizable), `1000` (multi-block), and negative (`-1`, `INT_MIN`).
`len` is the only size/count/width knob: element type is fixed `int`, there is
no stride, no byte order and no format selector.

### Axis 4 — pointer aliasing (only meaningful for `fma_array`)

The C signature marks `mul1`/`mul2`/`add` as `const int *` but `inner` passes
the *same* buffer as all four arguments, so aliasing is a first-class input
shape that the code genuinely distinguishes (each element is read and then
written within the same iteration): all-distinct, `out == mul1`, `out == mul2`,
`out == add`, `mul1 == mul2` (square), and full self-alias
`out == mul1 == mul2 == add`. `driver` always exercises the full self-alias.

### Axis 5 — value distribution (boundary values)

* `small` — |v| ≤ 1000, so `m1*m2 + a` never overflows
* `full` — uniform over the whole `i32` range, so overflow happens constantly
* `boundary` — drawn from `{INT_MIN, INT_MIN+1, -2, -1, 0, 1, 2, INT_MAX-1, INT_MAX}`

Every row is driven with **many randomized inputs** (a fixed-seed SplitMix64
PRNG; see `ITER` in `tests/differential.rs`), not one hand-picked value, and
compared byte-for-byte: the full output buffer for `fma_array`, and the exact
captured stdout bytes for `driver`.

## Table

| # | entry point(s) | configuration (options set + input shape) | test | [x] |
|---|----------------|--------------------------------------------|------|-----|
| 1 | `fma_array` | no options; `len=0`; distinct ptrs; `small` — asserts the output buffer is left untouched | `cfg_01` | [x] |
| 2 | `fma_array` | no options; `len=1`; distinct ptrs; `small` | `cfg_02` | [x] |
| 3 | `fma_array` | no options; `len=2`; distinct ptrs; `small` | `cfg_03` | [x] |
| 4 | `fma_array` | no options; `len=3` (odd tail); distinct ptrs; `small` | `cfg_04` | [x] |
| 5 | `fma_array` | no options; `len=8`; distinct ptrs; `small` | `cfg_05` | [x] |
| 6 | `fma_array` | no options; `len=17` (odd, non-power-of-two); distinct ptrs; `small` | `cfg_06` | [x] |
| 7 | `fma_array` | no options; `len=64` (vectorizable); distinct ptrs; `small` | `cfg_07` | [x] |
| 8 | `fma_array` | no options; `len=1000` (multi-block); distinct ptrs; `small` | `cfg_08` | [x] |
| 9 | `fma_array` | no options; `len=64`; distinct ptrs; `full` i32 range (products overflow) | `cfg_09` | [x] |
| 10 | `fma_array` | no options; `len=1000`; distinct ptrs; `full` i32 range | `cfg_10` | [x] |
| 11 | `fma_array` | no options; `len=64`; distinct ptrs; `boundary` values (`INT_MIN`/`INT_MAX`/0/±1/±2) | `cfg_11` | [x] |
| 12 | `fma_array` | no options; `len=1`; distinct ptrs; `boundary` values | `cfg_12` | [x] |
| 13 | `fma_array` | no options; `len=64`; **`out == mul1`** (in-place first multiplicand); `full` | `cfg_13` | [x] |
| 14 | `fma_array` | no options; `len=64`; **`out == mul2`** (in-place second multiplicand); `full` | `cfg_14` | [x] |
| 15 | `fma_array` | no options; `len=64`; **`out == add`** (in-place addend); `full` | `cfg_15` | [x] |
| 16 | `fma_array` | no options; `len=64`; **`mul1 == mul2`** (square + add); `full` | `cfg_16` | [x] |
| 17 | `fma_array` | no options; `len=64`; **full self-alias `out==mul1==mul2==add`** (the pattern `inner` uses); `small` | `cfg_17` | [x] |
| 18 | `fma_array` | no options; `len=1000`; full self-alias; `full` i32 range | `cfg_18` | [x] |
| 19 | `fma_array` | no options; `len=17`; full self-alias; `boundary` values | `cfg_19` | [x] |
| 20 | `fma_array` | no options; `len=-1` and `len=INT_MIN`; distinct ptrs; asserts output untouched | `cfg_20` | [x] |
| 21 | `driver` | no options; `len=0` — asserts empty stdout | `cfg_21` | [x] |
| 22 | `driver` | no options; `len=1`; `small` | `cfg_22` | [x] |
| 23 | `driver` | no options; `len=2`; `small` | `cfg_23` | [x] |
| 24 | `driver` | no options; `len=3` (odd tail); `small` | `cfg_24` | [x] |
| 25 | `driver` | no options; `len=8`; `small` | `cfg_25` | [x] |
| 26 | `driver` | no options; `len=17` (odd, non-power-of-two); `small` | `cfg_26` | [x] |
| 27 | `driver` | no options; `len=64`; `small` | `cfg_27` | [x] |
| 28 | `driver` | no options; `len=1000` (large VLA); `small` | `cfg_28` | [x] |
| 29 | `driver` | no options; `len=64`; `full` i32 range (`d*d+d` overflows, negative results printed) | `cfg_29` | [x] |
| 30 | `driver` | no options; `len=1000`; `full` i32 range | `cfg_30` | [x] |
| 31 | `driver` | no options; `len=64`; `boundary` values | `cfg_31` | [x] |
| 32 | `driver` | no options; `len=1`; `boundary` values (incl. `INT_MIN`, `INT_MAX`) | `cfg_32` | [x] |
| 33 | `fma_array` → `driver` | composed pipeline: `fma_array` result buffer fed as `driver`'s input, `len=64`, `full` — checks the low-level and wrapper paths agree when chained | `cfg_33` | [x] |
