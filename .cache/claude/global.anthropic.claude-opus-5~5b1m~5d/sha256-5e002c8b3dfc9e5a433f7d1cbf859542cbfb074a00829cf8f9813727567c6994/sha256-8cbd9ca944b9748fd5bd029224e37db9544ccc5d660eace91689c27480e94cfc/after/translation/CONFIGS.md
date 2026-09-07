# CONFIGS.md — Phase B: configuration-surface table

## How this table was derived

Mechanical enumeration of every branch the C actually takes.

### Runtime options / modes / flags

**None.** The public header is one line:

```c
float pow43(int x);
```

There are no setters, no context/handle struct, no global mutable state, no
init/teardown, no environment lookups, no `#ifdef` / `#if` in the sources, and
no compile-time options in `CMakeLists.txt`. `g_pow43` is `static const`.
The library is a pure function of its single `int` argument, so the
configuration surface is **entirely the shape/value of `x`**.

### Full set of public entry points

| entry point | linkage | notes |
|---|---|---|
| `pow43(int) -> float` | `T` in `.dynsym` | The **only** public entry point; it is simultaneously the highest- and lowest-level one. There are no convenience wrappers to prefer and no internal helpers with external linkage (`g_pow43` is a file-local `static`, verified absent from `.dynsym`). |

### Axes the C branches on

Extracted from the two `if`s and the bit arithmetic:

* **A — dispatch path** (`if (x < 129)`, `if (x < 1024)`), three cases:
  * `A0` **table path**: `x < 129` → plain `g_pow43[16 + x]`, no arithmetic
  * `A1` **mid path**: `129 <= x < 1024` → `mult = 16`, `x <<= 3`, polynomial
  * `A2` **high path**: `x >= 1024` → `mult = 256`, no shift, polynomial
* **B — `mult`** : `256` (default / `A0`-irrelevant / `A2`) vs `16` (`A1` only).
  Fully determined by A.
* **C — `sign = 2 * x & 64`**, two cases (polynomial paths only):
  * `C0` `sign == 0` → `frac = (x&63) / (x&~63)`, `frac ∈ [0, 1)` — *positive*,
    round-**down** interpolation from the lower table entry
  * `C1` `sign == 64` → `frac = ((x&63) - 64) / ((x&~63) + 64)`, `frac < 0` —
    *negative*, round-**up** interpolation from the **next** table entry
    (`(x + sign) >> 6` lands one bucket higher)
  * Selector differs per path: on `A2` `sign == 64 ⟺ (x & 32) != 0`; on `A1`
    the value is shifted first, so `sign == 64 ⟺ (x & 4) != 0`.
* **D — table half** (`A0` only): `x < 0` selects the 16 **negative** entries
  (`g_pow43[0..16]`), `x >= 0` selects the `x^(4/3)` entries
  (`g_pow43[16..145]`). `x = 0` and `x = -16` both map to the stored `+0.0`.
* **E — low 6 bits of the (possibly shifted) `x`**, i.e. position inside the
  interpolation bucket: `x&63 == 0` (exact table hit, `frac == 0`, no
  interpolation), `x&63 == 63` (extreme), and everything between. On `A1` the
  shift forces `x&63` to be a **multiple of 8** — a distinct input shape that
  `A2` never produces.
* **F — boundary values**: `-16` (first valid index), `-1`/`0`/`1` (sign
  crossing), `128`/`129` (A0↔A1), `1023`/`1024` (A1↔A2), `8223` (last valid
  index, `i = 144`).

`mult` is not an independent axis (B is a function of A), so the pruned
cross-product is **A × C × D × E × F**, giving the rows below. Rows are pruned
to combinations the code actually distinguishes: `C`/`E` do not exist on `A0`
(no arithmetic runs), and `D` does not exist on `A1`/`A2` (`x >= 129 > 0`).

All rows are driven **through the `.so` exports** (`libloading`) for both C and
Rust and compared **bit-for-bit** (`f32::to_bits`), with many randomized inputs
per row from a fixed-seed xorshift64* PRNG (seed `0xC0FFEE_...` per row, printed
on failure so any counterexample is reproducible).

## Configuration surface table

| # | entry point(s) | configuration (options set + input shape) | test | [x] |
|---|----------------|--------------------------------------------|------|-----|
| 1 | `pow43` | **A0** table path, **D** negative half, boundary `x = -16` (first valid index, `i = 0`, stored `+0.0`) | `cfg_row01_table_x_minus_16` | [x] |
| 2 | `pow43` | **A0** table path, **D** negative half, `x ∈ [-15, -1]` — all 15 negative table entries, exhaustive + randomized | `cfg_row02_table_negative_half` | [x] |
| 3 | `pow43` | **A0** table path, **D** zero, `x = 0` (stored `+0.0`; checks `+0.0` vs `-0.0` bit pattern) | `cfg_row03_table_x_zero` | [x] |
| 4 | `pow43` | **A0** table path, **D** positive half, `x ∈ [1, 128]` randomized (plain `x^(4/3)` lookups, no interpolation) | `cfg_row04_table_positive_half_random` | [x] |
| 5 | `pow43` | **A0** table path, boundary `x = 128` (**last** input taking the table path, `i = 144`) | `cfg_row05_table_upper_boundary_128` | [x] |
| 6 | `pow43` | **A1** mid path, boundary `x = 129` (**first** input taking the polynomial path; `mult = 16`, `x <<= 3` → `1032`) | `cfg_row06_mid_lower_boundary_129` | [x] |
| 7 | `pow43` | **A1** mid path (`mult = 16`, shifted), **C0** `sign == 0` (`x & 4 == 0`), `x ∈ [129, 1023]` randomized | `cfg_row07_mid_sign0_random` | [x] |
| 8 | `pow43` | **A1** mid path (`mult = 16`, shifted), **C1** `sign == 64` (`x & 4 != 0`) — negative `frac`, bucket rounds **up**, `x ∈ [129, 1023]` randomized | `cfg_row08_mid_sign64_random` | [x] |
| 9 | `pow43` | **A1** mid path, **E** shifted `x` is an exact bucket multiple (`(x<<3) & 63 == 0`, i.e. `x % 8 == 0` → `frac == 0`) | `cfg_row09_mid_exact_bucket` | [x] |
| 10 | `pow43` | **A1** mid path, **E** shifted `x` at the top of its bucket (`(x<<3) & 63 == 56`, the largest value the `<< 3` shape can reach) | `cfg_row10_mid_bucket_top` | [x] |
| 11 | `pow43` | **A1** mid path, boundary `x = 1023` (**last** input with `mult = 16`) | `cfg_row11_mid_upper_boundary_1023` | [x] |
| 12 | `pow43` | **A2** high path, boundary `x = 1024` (**first** input with `mult = 256`; `mult` switch verified against `x = 1023`) | `cfg_row12_high_lower_boundary_1024` | [x] |
| 13 | `pow43` | **A2** high path (`mult = 256`, unshifted), **C0** `sign == 0` (`x & 32 == 0`), `x ∈ [1024, 8223]` randomized | `cfg_row13_high_sign0_random` | [x] |
| 14 | `pow43` | **A2** high path (`mult = 256`, unshifted), **C1** `sign == 64` (`x & 32 != 0`) — negative `frac`, `x ∈ [1024, 8223]` randomized | `cfg_row14_high_sign64_random` | [x] |
| 15 | `pow43` | **A2** high path, **E** `x & 63 == 0` (exact table hit, `frac == 0`, polynomial degenerates to `1.0`) | `cfg_row15_high_exact_bucket` | [x] |
| 16 | `pow43` | **A2** high path, **E** `x & 63 == 63` (max low bits; forces **C1** since `x & 32 != 0`) | `cfg_row16_high_bucket_max_lowbits` | [x] |
| 17 | `pow43` | **A2** high path, **E** `x & 63 == 31` and `x & 63 == 32` — the two values straddling the `sign` flip (`C0` → `C1`) inside one bucket | `cfg_row17_high_sign_flip_straddle` | [x] |
| 18 | `pow43` | **A2** high path, boundary `x = 8223` (**last** well-defined input, `i = 144`, the final table element) | `cfg_row18_high_upper_boundary_8223` | [x] |
| 19 | `pow43` | **all paths**, **exhaustive** sweep of the entire well-defined domain `x ∈ [-16, 8223]` (8240 inputs, every A/C/D/E/F combination the code can reach) | `cfg_row19_exhaustive_defined_domain` | [x] |
| 20 | `pow43` | **all paths**, unbiased randomized fuzz over `[-16, 8223]` (200 000 draws, fixed seed) + repeated-call/statelessness check (same `x` called twice interleaved with other `x` returns identical bits, proving no hidden state) | `cfg_row20_fuzz_and_statelessness` | [x] |
