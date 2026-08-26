# CONFIGS.md — configuration-surface table (Phase A / Phase B)

## §0 Build-time configuration surface

* `Cargo.toml [features]`: `default = []` — **no optional features exist**, so
  the complete set of feature combinations is:

  | # | feature combination | command |
  |---|---------------------|---------|
  | 1 | *(none)* / `default` (empty) | `cargo check --no-default-features` == `cargo check` == `cargo check --all-features` |

* `c_src/CMakeLists.txt`: no `option()`, no `add_definitions`, no
  `target_compile_definitions`, a single source file (`src/lib.c`) — **no
  build-time configuration**.
* `c_src/src/lib.c` / `include/lib.h`: **no** `#if`/`#ifdef`/`#define`
  conditional compilation (verified by grep, see `ERRORS.md`).

⇒ exactly one build configuration; every row below is verified under it, which
is simultaneously "every feature combination".

## §1 Runtime configuration surface

The public API is one pure function with no options, no state, no
initialisation, no context object and no global setters:

```c
float pow43(int x);          /* c_src/include/lib.h — the entire header */
```

There is therefore **no runtime option/flag/mode axis**. The whole
configuration surface is the *shape of the input value*, i.e. the branches the
C code actually takes on `x`:

```c
if (x < 129)  return g_pow43[16 + x];          /* branch A: direct table read  */
if (x < 1024) { mult = 16; x <<= 3; }          /* branch B: scaled-up input    */
                                               /* branch C: x >= 1024, mult=256*/
sign = 2 * x & 64;                             /* sign ∈ {0, 64}               */
frac = (float)((x & 63) - sign) / ((x & ~63) + sign);
return g_pow43[16 + ((x + sign) >> 6)] * (1.f + frac*((4.f/3) + frac*(2.f/9))) * mult;
```

Axes derived from that code:

* **branch**: A (`x < 129`), B (`129 ≤ x < 1024`, `mult = 16`, `x <<= 3`),
  C (`x ≥ 1024`, `mult = 256`).
* **`sign`**: `0` vs `64` (`sign = (x & 32) << 1` after the optional shift).
* **`frac`**: `== 0` exactly, `> 0` (`sign == 0`), `< 0` (`sign == 64`).
* **table index** `16 + …`: first entry (`0`), the negative-mirror half
  (`0..15`), the zero entry (`16`), the positive half (`17..144`), last entry
  (`144`).
* **`mult`**: `16` (branch B) vs `256` (branch C).
* **input magnitude**: minimum defined (`-16`), maximum defined (`8223`), and
  each branch/`sign`/`frac` cross-product in between.

Defined domain (index inside `g_pow43[0..=144]`): **`x ∈ [-16, 8223]`**
(8240 values) — see `ERRORS.md` rows 4–9 for the undefined remainder.

## §2 Configuration rows (each verified with randomized inputs, fixed seed)

Entry point is `pow43` for every row (it is the *only* — and therefore also the
lowest-level — exported entry point; there is no convenience wrapper layer to
hide behind). All rows call **both** `.so` images through `dlopen`/`dlsym` and
compare the returned `float` **bit-for-bit**.

| # | entry point(s) | configuration (options set + input shape) | test | [x] |
|---|----------------|-------------------------------------------|------|-----|
| 1 | `pow43` | branch A, negative-mirror half: `x ∈ [-16, -1]` → index `0..15` (exhaustive + randomized) | `phase_b_row01_branch_a_negative_half` | [x] |
| 2 | `pow43` | branch A, zero entry: `x == 0` → index `16` | `phase_b_row02_branch_a_zero` | [x] |
| 3 | `pow43` | branch A, small positives: `x ∈ [1, 15]` → index `17..31` | `phase_b_row03_branch_a_small_positive` | [x] |
| 4 | `pow43` | branch A, main positive half: `x ∈ [16, 128]` → index `32..144` (randomized) | `phase_b_row04_branch_a_positive_half` | [x] |
| 5 | `pow43` | branch A boundaries: `x ∈ {-16, -15, -2, -1, 0, 1, 15, 16, 127, 128}` | `phase_b_row05_branch_a_boundaries` | [x] |
| 6 | `pow43` | branch B (`mult = 16`, `x <<= 3`), `sign == 0`, `frac > 0`: `x ∈ [129, 1023]` with `(8x & 63) ∈ {8,16,24}` | `phase_b_row06_branch_b_sign0_frac_pos` | [x] |
| 7 | `pow43` | branch B, `sign == 0`, `frac == 0` exactly: `x ∈ [129,1023]`, `x % 8 == 0` | `phase_b_row07_branch_b_frac_zero` | [x] |
| 8 | `pow43` | branch B, `sign == 64`, `frac < 0`: `x ∈ [129,1023]` with bit 2 set (`8x & 32 != 0`) | `phase_b_row08_branch_b_sign64_frac_neg` | [x] |
| 9 | `pow43` | branch B boundaries: `x ∈ {129,130,131,132,135,136,1016,1020,1021,1022,1023}` (lowest/highest of the branch, both `sign` values) | `phase_b_row09_branch_b_boundaries` | [x] |
| 10 | `pow43` | branch C (`mult = 256`), `sign == 0`, `frac > 0`: `x ∈ [1024, 8223]` with `(x & 63) ∈ [1, 31]` | `phase_b_row10_branch_c_sign0_frac_pos` | [x] |
| 11 | `pow43` | branch C, `sign == 0`, `frac == 0` exactly: `x ∈ [1024, 8223]`, `x % 64 == 0` | `phase_b_row11_branch_c_frac_zero` | [x] |
| 12 | `pow43` | branch C, `sign == 64`, `frac < 0`: `x ∈ [1024, 8223]` with `(x & 63) ∈ [32, 63]` | `phase_b_row12_branch_c_sign64_frac_neg` | [x] |
| 13 | `pow43` | branch C, extreme table indices: index `32` (`x ∈ [1024, 1055]`) and index `144` (`x ∈ {8192..8223}` and `x ∈ {8128..8191}` with `sign == 64`) | `phase_b_row13_branch_c_index_extremes` | [x] |
| 14 | `pow43` | branch C boundaries: `x ∈ {1024,1025,1055,1056,1087,1088,8191,8192,8222,8223}` | `phase_b_row14_branch_c_boundaries` | [x] |
| 15 | `pow43` | branch transitions / `mult` switch: the adjacent pairs `(128,129)` (A→B) and `(1023,1024)` (B→C, `mult` 16→256) | `phase_b_row15_branch_transitions` | [x] |
| 16 | `pow43` | **exhaustive**: every `x` in the defined domain `[-16, 8223]`, all 8240 values | `phase_b_row16_exhaustive_domain_sweep` | [x] |
| 17 | `pow43` | randomized whole-domain property test, 20 000 samples, fixed seed, mixed branches in one run | `phase_b_row17_randomized_whole_domain` | [x] |
| 18 | `pow43` | statelessness / call-order independence: a random permutation of domain inputs, each called repeatedly and with C/Rust calls interleaved (catches hidden lazy-init or cached state) | `phase_b_row18_call_order_and_repetition` | [x] |
| 19 | `pow43` | concurrency: 8 threads calling both libraries simultaneously over disjoint random input sets | `phase_b_row19_concurrent_calls` | [x] |
| 20 | `pow43` | fresh `dlopen` of both libraries (second, independent handle) then re-run of the boundary set — verifies no load-time/one-shot state | `phase_b_row20_reload_libraries` | [x] |
| 21 | `pow43` | result *classification* over the whole domain: sign of zero (`+0.0` vs `-0.0`), finiteness, and monotonic growth of the table region — compared between C and Rust, not against hand-written expectations | `phase_b_row21_result_classification` | [x] |

Row 16 (exhaustive) subsumes rows 1–15 by construction; rows 1–15 are kept
because they pin down *which* configuration a failure belongs to, and rows
17–21 add axes (ordering, repetition, threading, re-loading) that a single
sweep does not cover.
