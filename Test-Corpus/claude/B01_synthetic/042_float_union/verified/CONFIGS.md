# CONFIGS.md — configuration-surface table (Phase B)

## How this table was derived

The library has **no runtime options**: there is no argv handling, no environment
variable, no `setlocale` call (so the locale is always `"C"`: decimal point `.`,
no thousands grouping), and no compile-time `#ifdef` in `c_src/src/main.c`. The
axes the C code actually branches on are therefore

1. **the two public entry points** — `driver(double)` (the lowest-level one) and
   `main()` (the composed pipeline `scanf` → `driver`); both are exported by the
   `.so` and both are driven directly, plus the real executable via stdin/stdout;
2. **the input *shape* fed to `scanf("%lf")`** — every branch glibc's `%f`
   conversion takes: notation (decimal / hex / `inf` / `infinity` / `nan`), sign,
   leading white space, letter case, digit layout, exponent form, and the
   magnitude classes `strtod` special-cases (zero, subnormal, normal, overflow,
   underflow, rounding ties);
3. **the `double` *value class* handed to `driver`** — every branch of glibc's
   `%llx` / `%a` / `%.4f` formatting: sign bit, exponent field `== 0`, `== 0x7ff`
   and in between, mantissa `== 0` (which suppresses the `.` in `%a`), mantissa
   with trailing zero nibbles (trimmed by `%a`), and the `%.4f` cases of exact
   round-half-to-even ties, sub-`0.00005` magnitudes and 300-digit expansions.

Feature combinations: `Cargo.toml` declares **no `[features]`** and
`c_src/CMakeLists.txt` declares no options, so there is exactly **one** build
configuration (`--no-default-features` and the default build are identical).
See "Feature combinations" at the bottom.

Every row is checked with **many randomized inputs** (fixed seed `0x9E3779B97F4A7C15`,
`common::Rng`), not a single hand-picked value.

| # | entry point(s) | configuration (options set + input shape) | [ ] |
|---|----------------|-------------------------------------------|-----|
| 1 | `driver` (.so, dlsym) | `±0.0` — bit patterns `0x0000000000000000`, `0x8000000000000000`; `%a` takes the `exponent==0 && zero_mantissa` branch | [x] |
| 2 | `driver` | `±inf` — exponent field `0x7ff`, mantissa 0 | [x] |
| 3 | `driver` | default quiet NaN, both signs (`0x7ff8…`, `0xfff8…`) | [x] |
| 4 | `driver` | NaN with arbitrary payloads incl. **signalling** NaNs (exp `0x7ff`, mantissa `1…0xfffffffffffff`), both signs — randomized | [x] |
| 5 | `driver` | subnormals: min (`0x1`), max (`0xfffffffffffff`), random mantissas, both signs — `%a` leading digit `0`, `p-1022` | [x] |
| 6 | `driver` | normals over the whole exponent range, random mantissas, both signs — randomized full 64-bit patterns | [x] |
| 7 | `driver` | mantissa `== 0` normals (exact powers of two) for every exponent field `1…2046` — `%a` emits **no** `.` | [x] |
| 8 | `driver` | mantissas with 1…12 trailing zero nibbles — exercises `%a` trailing-zero trimming at every length | [x] |
| 9 | `driver` | mantissa `0xfffffffffffff` (all 13 nibbles significant, nothing trimmed) | [x] |
| 10 | `driver` | exponent-field boundaries `1` (`p-1022`), `1022` (`p-1`), `1023` (`p+0`), `1024` (`p+1`), `2046` (`p+1023`) | [x] |
| 11 | `driver` | `%.4f` exact round-half-to-even ties: dyadic values with exactly 5 decimals ending in `5` (`0.15625`, `0.09375`, `±`, and generated `k/2^n`) | [x] |
| 12 | `driver` | `%.4f` just below / just above `0.00005` and `0.00015`, both signs (`0.0000` vs `0.0001`, `-0.0000`) | [x] |
| 13 | `driver` | `%.4f` huge magnitudes `1e300 … DBL_MAX` — 300+ digit exact expansions, both signs | [x] |
| 14 | `driver` | `%.4f` of subnormals and of `1e-320`-scale values → `±0.0000` | [x] |
| 15 | `driver` | exactly-representable integers `0 … 2^53`, powers of ten, both signs | [x] |
| 16 | `main` (.so, dlsym, stdin redirected) | decimal, no sign, integer digits only, no exponent — randomized lengths 1…20 | [x] |
| 17 | `main` | decimal with `+`/`-` sign, integer **and** fraction digits, no exponent — randomized | [x] |
| 18 | `main` | decimal with fraction only (`.5`, `-.25`), and integer-then-dot (`5.`) | [x] |
| 19 | `main` | decimal with `e`/`E` exponent, optional exponent sign, 1…3 exponent digits — randomized | [x] |
| 20 | `main` | decimal with extreme exponents: `±300…±400`, `±999`, `e+1000000000`, 400-digit exponent, leading-zero exponents | [x] |
| 21 | `main` | 17-digit mantissas at the representability boundary (`DBL_MAX`/`DBL_MIN`/subnormal edges, exact halfway values) — randomized | [x] |
| 22 | `main` | very long digit strings (100…800 digits) with the dot at a random position — exercises `strtod`'s slow path | [x] |
| 23 | `main` | every white-space byte (`' '`, `\t`, `\n`, `\v`, `\f`, `\r`) and mixed/long (4096-byte) runs before the number | [x] |
| 24 | `main` | hexadecimal `0x`/`0X` with integer hex digits only, no `p` exponent, mixed digit case — randomized | [x] |
| 25 | `main` | hexadecimal with a hex fraction and no `p` exponent (`0x1.8`, `0x.8`, `0xa.`) — randomized | [x] |
| 26 | `main` | hexadecimal with `p`/`P` exponent, signed and unsigned, landing in the normal range — randomized | [x] |
| 27 | `main` | hexadecimal with **> 14 significant hex digits** (rounding + sticky bit), incl. exact ties on the 53rd bit — randomized, up to 40 digits | [x] |
| 28 | `main` | hexadecimal whose `p` exponent lands in the **subnormal** range (`p-1022 … p-1080`) or **overflows** (`p+1020 … p+1030`) — every boundary | [x] |
| 29 | `main` | hexadecimal with huge/long `p` exponents (`p+99999999999`, `p-99999999999`, 40-digit exponents) | [x] |
| 30 | `main` | `inf`, `infinity`, `nan` in **all** case permutations, with `+`/`-`, and with an `nan(chars)` suffix | [x] |
| 31 | `main` | dangling exponent characters — `1e`, `1e+`, `1E-`, `0x1p`, `0x1p+`, `0x1p-`, and a hex digit immediately after `p` (`0x1pa`, `0x1p2f3`) | [x] |
| 32 | `main` | multiple/late decimal points and separator characters — `1.2.3`, `1..2`, `1,000`, `1'0`, `0x1.8.8` | [x] |
| 33 | `main` | a valid number followed by arbitrary trailing garbage / a second number (must be ignored) — randomized | [x] |
| 34 | `main` | random byte soup from a numeric-biased alphabet, lengths 0…12, plus fully random bytes — randomized (large volume) | [x] |
| 35 | `driver` + `main` | the low-level and the composed entry point in the **same** process, called repeatedly (state/buffering must not leak between calls) | [x] |
| 36 | executables (`c_src/build/driver` vs `target/<profile>/driver`, stdin → stdout) | a sample of every category above through the real process boundary, comparing stdout, stderr **and** exit status | [x] |

## Feature combinations

`Cargo.toml` has no `[features]` table, and `c_src/CMakeLists.txt` has no options,
`option()`, `add_definitions`, or `target_compile_definitions`; `main.c` contains no
`#ifdef`. The complete set of valid build configurations is therefore:

| # | configuration | command | status |
|---|---------------|---------|--------|
| 1 | default (= no features) | `cargo check --no-default-features` / `cargo test --no-default-features` | [x] |
| 2 | default (implicit) | `cargo check` / `cargo test` | [x] |
| 3 | release (`panic = "abort"` profile, the profile the shipped binary uses) | `cargo test --release --no-default-features` | [x] |

Rows 1 and 2 are the same configuration; row 3 is included because the crate
overrides `[profile.release]`, which is the only build-time switch that exists.

## Row → test mapping (Phase B evidence)

Every row is a `harness = false` case, run sequentially so the differential
capture of fd 0/1 is deterministic. "inputs" is the number of distinct inputs the
row feeds to **both** implementations.

| # | test | inputs compared (C vs Rust, byte-for-byte) |
|---|------|--------------------------------------------|
| 1 | `ffi_driver::row_01_signed_zeros` | 2 |
| 2 | `ffi_driver::row_02_infinities` | 2 |
| 3 | `ffi_driver::row_03_default_nan` | 2 |
| 4 | `ffi_driver::row_04_nan_payloads` | 2 009 |
| 5 | `ffi_driver::row_05_subnormals` | 2 111 |
| 6 | `ffi_driver::row_06_random_bit_patterns` | 20 000 |
| 7 | `ffi_driver::row_07_powers_of_two` | 4 092 |
| 8 | `ffi_driver::row_08_trailing_zero_nibbles` | 1 155 |
| 9 | `ffi_driver::row_09_full_mantissa` | 34 |
| 10 | `ffi_driver::row_10_exponent_boundaries` | 120 |
| 11 | `ffi_driver::row_11_rounding_ties` | 4 128 |
| 12 | `ffi_driver::row_12_near_threshold` | 1 014 |
| 13 | `ffi_driver::row_13_huge_magnitudes` | 1 646 |
| 14 | `ffi_driver::row_14_tiny_magnitudes` | 1 508 |
| 15 | `ffi_driver::row_15_integers_and_powers_of_ten` | 4 220 |
| 16 | `ffi_main::row_16_decimal_integer_only` | 3 000 |
| 17 | `ffi_main::row_17_decimal_with_fraction` | 3 000 |
| 18 | `ffi_main::row_18_fraction_only_and_trailing_dot` | 3 016 |
| 19 | `ffi_main::row_19_decimal_exponent` | 3 000 |
| 20 | `ffi_main::row_20_extreme_exponents` | 3 019 |
| 21 | `ffi_main::row_21_boundary_mantissas` | 4 025 |
| 22 | `ffi_main::row_22_long_digit_strings` | 600 |
| 23 | `ffi_main::row_23_leading_whitespace` | 1 521 |
| 24 | `ffi_main::row_24_hex_integer_only` | 3 000 |
| 25 | `ffi_main::row_25_hex_fraction` | 3 009 |
| 26 | `ffi_main::row_26_hex_p_exponent` | 3 000 |
| 27 | `ffi_main::row_27_hex_rounding` | 4 014 |
| 28 | `ffi_main::row_28_hex_subnormal_and_overflow` | 4 528 |
| 29 | `ffi_main::row_29_hex_huge_exponents` | 812 |
| 30 | `ffi_main::row_30_specials` | 4 096 |
| 31 | `ffi_main::row_31_dangling_exponents` | 637 |
| 32 | `ffi_main::row_32_dots_and_separators` | 822 |
| 33 | `ffi_main::row_33_trailing_input` | 2 000 |
| 34 | `ffi_main::row_34_random_soup` | 30 000 |
| 35 | `ffi_main::row_35_interleaved_entry_points` | 320 `main` + 320 `driver` + 5 multi-number |
| 36 | `cli_diff::*` (7 cases) | 889 process pairs, incl. six 200-300 KiB inputs |

**Total: 132 075 differential comparisons through the two shared objects plus 889 through the two executables** (measured with `DIFF_COUNTS=1 cargo test`).

## Negative controls

The harness was validated by deliberately breaking the Rust and confirming the
suites fail (then reverting):

| injected bug | detected by |
|--------------|-------------|
| `%a` prefix `"0x"` → `"0X"` | `ffi_driver::row_01_signed_zeros` (and every other `driver` row) |
| dropping the sign from the bare-hex-prefix rejection check (`w.len() == 2 + got_sign` → `w.len() == 2`) | `errors::row_19_signed_bare_hex_prefix` and `ffi_main::row_34_random_soup` |

In addition `symbols::harness_capture_self_check` and `cli_diff::cli_self_check`
pin concrete expected bytes, so a harness that captured nothing could not pass.
