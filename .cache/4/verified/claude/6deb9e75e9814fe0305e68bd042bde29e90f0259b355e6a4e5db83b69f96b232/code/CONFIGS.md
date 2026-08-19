# CONFIGS.md — Configuration-surface table (Phase A / Phase B)

## How this table was derived

### Build-time configuration axes

| source | axes found |
|--------|-----------|
| `Cargo.toml` | **no `[features]` section** -> exactly one Rust configuration. `--no-default-features` is therefore identical to the default build. |
| `c_src/CMakeLists.txt` | no `option()`, no `add_definitions`, no `target_compile_definitions`, no conditional sources; one unconditional target from `src/driver.c`. |
| C preprocessor | `grep -rn "ifdef\|ifndef\|if defined\|#else" c_src/` -> only the `DRIVER_H_` header guard. No variant code. |

**=> 1 build configuration total.** (Phase D's "repeat for every feature
combination" collapses to this single combination; it is still run explicitly
via `--no-default-features`.)

### Runtime configuration axes

`grep` for globals / setters / modes in `c_src/`: **none**. There is no `static`
or file-scope variable, no init/config/setter function, no mode flag, no
context struct. `include/driver.h` declares exactly one entry point:

```c
void driver(double f);
```

So there is **one** public entry point and it is simultaneously the
lowest-level one — there is no convenience wrapper layered over an inner API,
and no state to set up between calls.

### Input-shape axes

With no options and no pointers, the entire input space is the value of a
single `double`. The C body applies three *different* conversions to it, and
each one takes different paths depending on the IEEE-754 class and magnitude:

```c
raw_double_t u = {.f = f};
printf("%llx %a %.4f\n", u.x, f, f);
```

| axis | distinct shapes the code path actually distinguishes |
|------|------------------------------------------------------|
| A. union type-pun -> `%llx` | the raw 64-bit pattern: sign bit, 11-bit exponent field, 52-bit mantissa. Distinguishes nothing else — but must reproduce **every** bit, including non-canonical NaN payloads. Also exercises the `uint64_t` -> `%llx` (`unsigned long long`) varargs promotion. |
| B. `%a` hex-float | glibc branches on: zero (`0x0p+0`), subnormal (leading hex digit `0`), normal (leading hex digit `1`), infinity (`inf`), NaN (`nan`); plus mantissa trailing-zero trimming, exponent sign (`p+`/`p-`), and the sign of the value. |
| C. `%.4f` fixed | glibc branches on: sign (incl. `-0.0` -> `-0.0000`), magnitude below the 4-decimal cliff (-> `0.0000`), round-half-even at the 4th decimal, integer-digit count from 1 to ~309 digits (large values overflow glibc's stack buffer and take the malloc path), and non-finite (`inf`/`nan`). |

Rows below are the cross-product of {IEEE-754 class} x {magnitude regime} x
{sign}, pruned to the combinations these three conversions treat differently.
Every row is driven with **many randomized inputs** (SplitMix64, fixed seed
`0x9E3779B97F4A7C15`) except where the row names an exact singleton value.

## Configuration-surface table

Entry point is `driver` for every row (it is the only public symbol, and the
lowest-level one).

| # | entry point(s) | configuration (options set + input shape) | test | [x] |
|---|----------------|--------------------------------------------|------|-----|
| C1 | `driver` | no options (none exist); `f = +0.0` — zero class, `%a` -> `0x0p+0`, `%.4f` -> `0.0000` | `config_c1_positive_zero` | [x] |
| C2 | `driver` | `f = -0.0` — sign bit set with zero payload; `%.4f` must emit `-0.0000` | `config_c2_negative_zero` | [x] |
| C3 | `driver` | `f = 1.0`, `-1.0`, `2.0`, `0.5` — exact binary fractions, single integer digit, `%a` mantissa fully trimmed | `config_c3_exact_small_binary_fractions` | [x] |
| C4 | `driver` | exact powers of two `2^e` sweeping the **full** exponent range `e = -1074..=1023`, both signs — walks every `%a` exponent, incl. the `p-`/`p+` switch and subnormal powers | `config_c4_all_powers_of_two_both_signs` | [x] |
| C5 | `driver` | every biased exponent field `0..=2047` combined with randomized 52-bit mantissas, both signs — systematic sweep of the `%a` normal/subnormal/inf/nan branch selector | `config_c5_exponent_sweep_random_mantissa` | [x] |
| C6 | `driver` | smallest positive subnormal (`bits == 1`) and largest subnormal (`bits == 0x000FFFFFFFFFFFFF`), both signs — `%a` leading digit `0` | `config_c6_subnormal_extremes` | [x] |
| C7 | `driver` | randomized subnormals: exponent field `0`, random non-zero 52-bit mantissa, both signs — `%a` leading `0x0.` path with varied trailing-zero trimming | `config_c7_random_subnormals` | [x] |
| C8 | `driver` | subnormal/normal boundary: `bits == 0x000FFFFFFFFFFFFF` vs `0x0010000000000000` (one step apart) | `config_c8_subnormal_normal_boundary` | [x] |
| C9 | `driver` | smallest normal (`2.2250738585072014e-308`) and largest finite (`f64::MAX`), both signs — `%.4f` at 1 digit vs ~309 digits | `config_c9_normal_extremes` | [x] |
| C10 | `driver` | randomized normals in `[-1, 1)` — the common case; dense `%.4f` rounding and long `%a` mantissas | `config_c10_random_unit_interval` | [x] |
| C11 | `driver` | randomized normals scaled by `10^k` for `k = -320..=308` — sweeps `%.4f` integer-digit count across the whole decimal range, incl. the >100-digit malloc path | `config_c11_random_scaled_decades` | [x] |
| C12 | `driver` | randomized normals scaled by `2^k` for `k = -1080..=1020` — same sweep on binary boundaries rather than decimal | `config_c12_random_scaled_binary_decades` | [x] |
| C13 | `driver` | `%.4f` rounding ties: values of the form `n + 0.00005` and `n - 0.00005`, plus `x.xxxx5` half-way cases, both signs — round-half-even vs round-half-away | `config_c13_rounding_ties_4th_decimal` | [x] |
| C14 | `driver` | `%.4f` cliff: magnitudes just below / at / just above `0.00005`, incl. `nextafter` neighbours, both signs — decides `0.0000` vs `0.0001` | `config_c14_rounding_cliff_neighbourhood` | [x] |
| C15 | `driver` | `%.4f` carry propagation: `0.99995`, `9.99995`, `99.99995`, `0.9999999`, `-0.99995` — rounding that carries into the integer part / adds a digit | `config_c15_rounding_carry_propagation` | [x] |
| C16 | `driver` | exact integers of increasing width: `1`, `10`, ... `1e22` (exactly representable) and `2^53`, `2^53+2` — `%.4f` integer path, `%a` trimmed mantissa | `config_c16_exact_integers_increasing_width` | [x] |
| C17 | `driver` | values whose `%a` mantissa needs full 13 hex digits vs 1 (mantissa `0xFFFFFFFFFFFFF` vs `0x0000000000000`) — `%a` trailing-zero trimming boundary | `config_c17_mantissa_trimming_extremes` | [x] |
| C18 | `driver` | `±inf` — non-finite class in all three conversions | `config_c18_infinities` | [x] |
| C19 | `driver` | `±NaN` quiet, default payload — non-finite, sign-carrying (`nan` / `-nan`) | `config_c19_quiet_nans` | [x] |
| C20 | `driver` | NaN payload sweep: signaling (mantissa MSB clear) and quiet (MSB set), payloads `1`, `0x7FFFFFFFFFFFF`, and randomized, both signs — `%llx` must reproduce the exact payload while `%a`/`%.4f` collapse to `nan` | `config_c20_nan_payload_sweep` | [x] |
| C21 | `driver` | **fully randomized 64-bit patterns** via `f64::from_bits(rng)` — unbiased coverage of every class simultaneously (≈ half NaN/inf by construction), the property-style catch-all | `config_c21_random_bit_patterns` | [x] |
| C22 | `driver` | `nextafter` neighbourhoods around each of `0`, `1`, `-1`, `2`, `0.1`, `f64::MIN_POSITIVE`, `f64::MAX` — ±few ULP, where `%a`/`%.4f` outputs change by one digit | `config_c22_ulp_neighbourhoods` | [x] |
| C23 | `driver` | decimal-looking values that are not exactly representable: `0.1`, `0.2`, `0.3`, `1/3`, `2/3`, `1e-5`, `1e-4` and their negations — `%a` shows the true binary value, `%.4f` shows the rounded decimal | `config_c23_inexact_decimals` | [x] |
| C24 | `driver` | many consecutive calls in a single stdout capture (4096 mixed values, long + short lines interleaved) — composed pipeline / buffering behaviour rather than one call per capture | `config_c24_bulk_sequential_calls` | [x] |
| C25 | `driver` | build-configuration axis: the single (only) build combination, exercised with `--no-default-features` — see the build-time table above | whole suite, run under `--no-default-features` | [x] |
| C26 | `driver` | **process state axis:** each of the 4 FPU rounding directions (`FE_TONEAREST`, `FE_DOWNWARD`, `FE_UPWARD`, `FE_TOWARDZERO`) x ~2100 randomized + boundary values. glibc honours the rounding direction when converting a double to decimal, so this axis really does change the output (verified: 5e-5 prints `0.0001` under `FE_TONEAREST` but `0.0000` under `FE_DOWNWARD`) | `config_c26_rounding_*` (4 tests) + `config_c26_rounding_actually_changes_c_output` | [x] |
| C27 | `driver` | **process state axis:** an `LC_NUMERIC` locale with a comma radix character (`de_DE.utf8`, falling back through `fr_FR`/`ru_RU`, row skipped if none installed) — both `%a` and `%.4f` take the radix from the locale | `config_c27_locale_with_comma_radix` + `config_c27_locale_actually_changes_c_output` | [x] |
| C28 | `driver` | C26 x C27 cross-product: comma-radix locale **and** each of the 4 rounding directions simultaneously | `config_c26_c27_rounding_and_locale_combined` | [x] |

### Why C26–C28 matter

`driver` itself has no options, but `%a` and `%.4f` are evaluated by `printf`,
whose behaviour depends on process state that any caller may change. A
translation that formatted the number *in Rust* (e.g. with `format!("{:.4}")`)
would agree with C in the default environment and diverge silently under a
non-default rounding direction or locale. Rows C26–C28 are what make that class
of bug visible; each is paired with a guard test asserting the axis still
changes the C output, so the row cannot quietly become vacuous.
