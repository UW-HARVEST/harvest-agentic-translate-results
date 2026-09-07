# CONFIGS.md — Phase B configuration-surface table

Mechanically derived from the branches the C actually takes.

## Axes the C code distinguishes

Full public API (`c_src/include/lib.h`) is a single entry point — there is no
convenience wrapper vs. low-level split, no init/config object, no flags struct:

```c
void normalize(float *dest, const float *src, int size);
```

There are no runtime options, no modes, no global state, no `#ifdef`s in
`c_src/src/lib.c`, and no cargo features in `translation/Cargo.toml`. So the
configuration surface is entirely the **input shape**, over these axes:

* **A. branch on `sum > 0.0f`** (the only real control flow decision):
  `sum` positive-normal / `sum == +0.0` / `sum == NaN` / `sum == +inf` /
  `sum` subnormal.
* **B. branch on `dest != src`** (pointer identity): `dest` separate buffer /
  `dest == src` exactly / `dest` partially overlapping `src` (forward and
  backward offsets — `!=` so it takes the "separate" path but the write loop
  clobbers not-yet-read input).
* **C. `size` shape**: `0`, `1`, `2`, `3`, `4`, `5`, `7`, `8`, `9`, `15`, `16`,
  `17`, `31`, `33`, `64`, `1000` — these straddle the 4/8/16-lane SIMD
  vectorisation and loop-remainder boundaries LLVM/GCC pick for the two loops,
  which is where reassociation/tail-handling divergence would show up.
  Plus negative `size` and `INT_MIN`/`INT_MAX` (see ERRORS.md E3/E4/E15).
* **D. `src` value class** (drives A, and drives bit-exactness of the multiply):
  random normals in [-1,1) / large magnitudes near `FLT_MAX` / tiny denormals /
  mixed signs / `+0.0` / `-0.0` / `±inf` / `NaN` / exact powers of two (so
  `sqrtf` is exact) / values whose sum-of-squares overflows.
* **E. `dest` pre-fill**: pre-poisoned with a recognisable pattern, so that a
  *missing* write (or an over-long write past `size`) is detectable, and so the
  0-length `memset` no-op of E1 is provable.

## Configuration table

Every row is run with **many randomized inputs** (fixed seed `0x5EED_1234`,
deterministic SplitMix64/xorshift PRNG, ≥64 random trials per (row, size) pair,
crossed over the whole `size` list in axis C), comparing the C `.so` and the
Rust `.so` outputs **byte-for-byte** (raw `u32` bit patterns, so `-0.0` vs `+0.0`
and NaN payloads are distinguished), plus the untouched guard bytes on each side
of `dest`.

| # | entry point(s) | configuration (options set + input shape) | test | [x] |
|---|----------------|-------------------------------------------|------|-----|
| C1 | `normalize` | separate `dest`/`src`; random normal floats in [-1,1); all sizes in axis C; `sum` positive-normal ⇒ scale branch | `c1_random_normal_all_sizes` | [x] |
| C2 | `normalize` | separate buffers; random floats over a wide exponent range (2^-60 … 2^60, random signs); all sizes ⇒ exercises rounding of the sequential `f32` accumulation | `c2_random_wide_exponent_all_sizes` | [x] |
| C3 | `normalize` | separate buffers; exact powers of two only, so `sum` is exactly representable and `sqrtf` may be exact; all sizes | `c3_exact_powers_of_two` | [x] |
| C4 | `normalize` | separate buffers; magnitudes near `FLT_MAX` so `sum` overflows to `+inf` ⇒ scale branch with `1/inf == +0.0`; all sizes ≥ 1 | `c4_sum_overflows_to_inf` | [x] |
| C5 | `normalize` | separate buffers; denormal / near-`FLT_MIN` magnitudes so `sum` is subnormal or underflows to `+0.0` ⇒ branch chosen by `> 0.0f`; all sizes ≥ 1 | `c5_denormal_inputs` | [x] |
| C6 | `normalize` | separate buffers; `src` is all `+0.0` ⇒ `sum == 0` ⇒ `memset` branch; all sizes | `c6_all_positive_zero_memset` | [x] |
| C7 | `normalize` | separate buffers; `src` is all `-0.0` (and mixed `±0.0`) ⇒ `sum == +0.0` ⇒ `memset` branch, sign bits destroyed; all sizes | `c7_all_negative_zero_memset` | [x] |
| C8 | `normalize` | separate buffers; `src` contains randomly-placed `NaN`s (quiet and signalling payloads) ⇒ `sum == NaN` ⇒ `memset` branch; all sizes ≥ 1 | `c8_nan_elements_memset` | [x] |
| C9 | `normalize` | separate buffers; `src` contains randomly-placed `±inf` ⇒ `sum == +inf` ⇒ scale branch producing `NaN` for the inf lanes and signed zeros elsewhere; all sizes ≥ 1 | `c9_inf_elements_scale` | [x] |
| C10 | `normalize` | separate buffers; `src` mixes every class (normal, denormal, `±0`, `±inf`, `NaN`, huge) at random positions ⇒ full cross-product of value classes; all sizes | `c10_mixed_value_classes` | [x] |
| C11 | `normalize` | **`dest == src`** (in-place), random normal floats, `sum > 0` ⇒ scale branch writing over its own input as it reads; all sizes | `c11_in_place_scale` | [x] |
| C12 | `normalize` | **`dest == src`**, zero-norm / NaN `src` ⇒ both branches skipped ⇒ strict no-op, original bits (incl. `-0.0`, NaN payload) preserved; all sizes | `c12_in_place_noop_branch` | [x] |
| C13 | `normalize` | **overlapping, `dest = src + k`** (forward overlap, `k` in 1..size) — `dest != src` so the scale branch runs and the write loop clobbers input the read loop has not yet consumed; all sizes | `c13_forward_overlap_scale` | [x] |
| C14 | `normalize` | **overlapping, `src = dest + k`** (backward overlap) — same aliasing hazard in the other direction; all sizes | `c14_backward_overlap_scale` | [x] |
| C15 | `normalize` | overlapping buffers with a zero-norm/NaN `src` so the **`memset`** branch runs on a `dest` that overlaps `src`; all sizes | `c15_overlap_memset_branch` | [x] |
| C16 | `normalize` | `size == 1` specifically, swept over many single random values of every class — the degenerate one-element case, where the result must be exactly `±1.0` for finite non-zero input | `c16_size_one_sweep` | [x] |
| C17 | `normalize` | large `size` (1000, 4096) with random normals — pushes both loops well past any SIMD threshold and accumulates enough rounding to catch reassociation | `c17_large_size` | [x] |
| C18 | `normalize` | `dest` written with a guard/poison pattern before and after the `size`-element window, all branches — proves neither impl writes outside `[0, size)` (and that `memset` uses exactly `size * 4` bytes) | `c18_no_out_of_window_writes` | [x] |
| C19 | `normalize` | misaligned buffers: `dest`/`src` offset to a non-16-byte-aligned start, random normals, all sizes ⇒ exercises the peel/prologue path of any vectorised loop | `c19_misaligned_buffers` | [x] |
| C20 | `normalize` | `size == 0` with separate buffers and with `dest == src`, `dest` poisoned ⇒ strict no-op (see ERRORS.md E1/E2) | `c20_size_zero` | [x] |

No binary/driver executable is produced (`c_src/CMakeLists.txt` builds only
`add_library(... SHARED src/lib.c)`), so the "compare C and Rust stdout" gate is
not applicable.

## Feature combinations

`translation/Cargo.toml` declares **no `[features]` section**, so `default` is the
only configuration; `--no-default-features` is equivalent. Both are run.

---

## Phase B results

All 20 rows pass. Test binary: `translation/tests/phase_b_configs.rs`.

```
running 20 tests
c1_random_normal_all_sizes ...... ok      c11_in_place_scale ............. ok
c2_random_wide_exponent ......... ok      c12_in_place_noop_branch ....... ok
c3_exact_powers_of_two .......... ok      c13_forward_overlap_scale ...... ok
c4_sum_overflows_to_inf ......... ok      c14_backward_overlap_scale ..... ok
c5_denormal_inputs .............. ok      c15_overlap_memset_branch ...... ok
c6_all_positive_zero_memset ..... ok      c16_size_one_sweep ............. ok
c7_all_negative_zero_memset ..... ok      c17_large_size ................. ok
c8_nan_elements_memset .......... ok      c18_no_out_of_window_writes .... ok
c9_inf_elements_scale ........... ok      c19_misaligned_buffers ......... ok
c10_mixed_value_classes ......... ok      c20_size_zero .................. ok
```

Every comparison is over the **entire** backing allocation as raw `u32` words
(not `f32` equality), so `+0.0` vs `-0.0`, every NaN payload, and any write
outside `dest[0..size)` are all caught. `c16_size_one_sweep` additionally walks
every one of the 278 finite `f32` exponents in both signs plus 17 hand-picked
special bit patterns.

## Sensitivity evidence (mutation battery)

Passing tests only mean something if they can fail. `translation/mutation_check.sh`
injects 16 semantic mutations into the Rust translation, rebuilds the `.so`, and
checks the suite catches each:

```
CAUGHT  M1  drop the dest!=src aliasing guard            (3 tests)
CAUGHT  M2  widen the accumulator to f64                 (14 tests)
CAUGHT  M3  branch on sum >= 0.0 instead of > 0.0        (9 tests)
CAUGHT  M4  branch on !(sum <= 0.0)  (NaN -> scale)      (8 tests)
CAUGHT  M5  divide per element instead of *reciprocal    (17 tests)
CAUGHT  M6  compute the scale via powf(-0.5)             (16 tests)
CAUGHT  M7  memset length forgets the *4 element size    (10 tests)
EQUIV   M8  memset length saturates instead of wrapping   -- see below
CAUGHT  M9  negative size clamped to 0 before memset     (2 tests)
CAUGHT  M10 memset writes 0xFF bytes instead of 0        (10 tests)
CAUGHT  M11 scale loop stops one element early           (17 tests)
CAUGHT  M12 sum loop reads one element too many          (17 tests)
CAUGHT  M13 sign flip on the stored element              (17 tests)
CAUGHT  M14 read dest instead of src in the scale loop   (15 tests)
CAUGHT  M15 scale loop iterates in reverse               (3 tests)
EQUIV   M16 compare sum.abs() > 0.0 instead of sum > 0.0  -- see below
```

The two uncaught mutants are **provably equivalent**, not coverage gaps:

* **M16** — `sum` is accumulated from `+0.0f` by adding `v*v` terms. Every `v*v`
  is either `>= +0.0f` or `NaN` (and `(-0.0f)*(-0.0f) == +0.0f`, `(+0.0)+(+0.0)
  == +0.0`), so `sum` can never be negative and never `-0.0f`. `abs()` is the
  identity on `[+0.0, +inf]` and leaves a `NaN` a `NaN`, so
  `sum.abs() > 0.0f` ≡ `sum > 0.0f` for **every** reachable value. No input can
  distinguish them.
* **M8** — `wrapping_mul` and `saturating_mul` differ only when `size < 0`, where
  `size as usize` lies in `[2^63, 2^64)`. `wrapping` yields `0xFFFF…FFFC`,
  `saturating` yields `0xFFFF…FFFF`; both are ~18 exabytes, so `memset` faults in
  the very first unmapped page either way. The E4 fault-parity test confirms this
  empirically: the M8 build still terminates with the same `SIGSEGV(11)` as C.
  The address space cannot express the difference.

## Verification matrix (Phase D)

`translation/verify_all.sh` runs the whole suite across every configuration:

| feature combo | Rust `.so` profile | C `.so` optimisation | result |
|---|---|---|---|
| `<default>` | release, debug | `-O0`, `-O2`, `-O3` | 6/6 ok |
| `--no-default-features` | release, debug | `-O0`, `-O2`, `-O3` | 6/6 ok |
| `--all-features` | release, debug | `-O0`, `-O2`, `-O3` | 6/6 ok |

18/18 cells green, 45 tests each. `Cargo.toml` declares no `[features]`, so
`default`, `--no-default-features` and `--all-features` are the complete set; all
three are exercised anyway. The C optimisation axis is an extra robustness check
(the canonical ground truth is the plain `cmake --build` `-O0` build).
