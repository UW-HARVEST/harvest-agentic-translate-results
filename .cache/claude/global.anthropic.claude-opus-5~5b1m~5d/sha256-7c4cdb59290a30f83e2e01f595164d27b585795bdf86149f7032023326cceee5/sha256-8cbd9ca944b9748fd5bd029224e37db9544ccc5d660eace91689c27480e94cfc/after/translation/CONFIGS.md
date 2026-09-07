# CONFIGS.md — Phase A: configuration-surface table

## Mechanical derivation of the axes

The public header exposes exactly one entry point and it is also the
lowest-level entry point — there is no convenience wrapper / one-shot layer to
skip past:

```c
void driver(double f);          // include/driver.h — the entire public API
```

`c_src/src/driver.c` has **no** `if`, `switch`, `?:`, `#ifdef` (beyond the
include guard), no global/static mutable state, no init/teardown, and no
runtime option, mode or flag. So:

| axis category | axes found in the C |
|---|---|
| runtime options / modes / flags | **none** (no setters, no globals, no env reads) |
| public entry points | **1**: `driver` |
| compile-time `#ifdef` config | **none** |

Consequently the *only* configuration axis is the **shape of the single
`double` argument**, and the branching on it is not in `driver`'s own code but
in the three `printf` conversions it delegates to:

```c
printf("%llx %a %.4f\n", u.x, f, f);
```

Each conversion special-cases the operand differently, which is what generates
distinct code paths:

| conversion | operand | C paths it distinguishes |
|---|---|---|
| `%llx` | `u.x` — the raw 64-bit pun of the `double` | value only; leading zeros suppressed, so bit patterns with a zero top nibble/word take a shorter path |
| `%a` | `f` as `double` | normal (`0x1.…p±d`) vs subnormal (`0x0.…p-1022`) vs zero (`0x0p+0`) vs `inf`/`nan`; trailing-mantissa-zero trimming; sign; exponent sign & width |
| `%.4f` | `f` as `double` | sign incl. `-0.0`; exact decimal expansion of the binary value; round-half-to-even at a tie; integer-part digit count from 1 to ~309; `inf`/`nan` spelling instead of digits |

The rows below are the cross-product of the IEEE-754 **classes** (the shapes
`%a` and `%.4f` genuinely treat differently) with the **magnitude / mantissa /
sign / tie** sub-shapes, pruned to the combinations the code actually
distinguishes. Also included is the union-pun axis (`raw_double_t`), whose only
shape distinction is "which bytes are set", covered by the raw-bit-pattern rows.

Every row is exercised **through both `.so` exports loaded with `libloading`**,
with stdout captured per call and compared byte-for-byte, and with **many
randomized inputs per row** (seeded, reproducible PRNG) rather than one
hand-picked value.

## The configuration-surface table

| # | entry point(s) | configuration (options set + input shape) | [ ] |
|---|----------------|--------------------------------------------|-----|
| 1 | `driver` | no options (none exist) + `+0.0` and `-0.0` — the two zero encodings; exercises `%a`→`0x0p+0`, `%.4f`→`0.0000`/`-0.0000`, `%llx`→`0`/`8000000000000000` | [x] |
| 2 | `driver` | small positive integers, exactly representable, `1.0…1024.0` (randomized) — shortest `%a` mantissa (trailing zeros fully trimmed), `%.4f` exact `.0000` | [x] |
| 3 | `driver` | negative counterparts of row 2 — sign path in all three conversions | [x] |
| 4 | `driver` | positive powers of two `2^-1074 … 2^1023` (every exponent, randomized order) — `%a` exponent formatting across its full range incl. the normal/subnormal transition | [x] |
| 5 | `driver` | negative powers of two, same exponent sweep | [x] |
| 6 | `driver` | normal doubles with **full 52-bit random mantissas**, exponent field random in `[1, 2046]`, random sign — the general normal path; no `%a` trailing-zero trimming | [x] |
| 7 | `driver` | normal doubles with mantissas having **random trailing-zero runs** (mantissa `= r << k`, `k` random in `0..52`) — drives glibc's `%a` trailing-hex-digit trimming at every truncation length | [x] |
| 8 | `driver` | **subnormals**: exponent field `0`, random non-zero 52-bit mantissa, both signs — `%a`'s `0x0.…p-1022` leading-digit path and `%.4f`'s deep-underflow-to-`0.0000` path | [x] |
| 9 | `driver` | subnormal **boundary** values: min subnormal `2^-1074`, max subnormal, `DBL_MIN`, and `nextafter` neighbours of each, both signs | [x] |
| 10 | `driver` | **huge magnitudes**: `DBL_MAX`, `1e308`, random normals with exponent field in `[2000, 2046]`, both signs — `%.4f` emits a ~310-character integer part; longest output path | [x] |
| 11 | `driver` | **tiny magnitudes**: random normals with exponent field in `[1, 60]`, both signs — `%.4f` rounds to `0.0000`/`-0.0000`, `%a` uses large negative exponents | [x] |
| 12 | `driver` | `%.4f` **tie / half-way** shapes: values near `k/2 * 10^-4` (`0.00005`, `0.00015`, `x.xxxx5`), plus each one's `nextafter` neighbours in both directions — round-half-to-even vs half-away divergence | [x] |
| 13 | `driver` | `%.4f` **carry-propagation** shapes: values just below a rounding carry that ripples through the fractional and into the integer part (`0.99995`, `9.99995`, `999999.99995`, randomized `10^n - eps`) | [x] |
| 14 | `driver` | **infinities**: `+inf`, `-inf` — `%a` and `%.4f` print `inf`/`-inf`, `%llx` prints the raw exponent-all-ones pattern | [x] |
| 15 | `driver` | **NaNs**: quiet ±, signalling ± (`0x7ff0000000000001`), random non-zero payloads, all-ones patterns — `nan`/`-nan` spelling driven by the sign bit while the payload only shows up in `%llx` | [x] |
| 16 | `driver` | **uniformly random raw `u64` bit patterns** reinterpreted as `double` (`f64::from_bits`) — hits every class in its natural frequency, incl. patterns unreachable from decimal literals; the strongest single row | [x] |
| 17 | `driver` | random doubles drawn from **wide log-uniform magnitudes** (exponent uniform, mantissa uniform) spanning `10^-320 … 10^308`, both signs — decorrelates exponent and mantissa coverage | [x] |
| 18 | `driver` | random doubles in the **`[0,1)` unit interval** and in `[-1,0)` — the densest region of the format, `%.4f` truncates ~48 mantissa bits away | [x] |
| 19 | `driver` | values with a **zero high word / zero low word** (`u64` patterns like `0x0000_0000_xxxx_xxxx`, `0xxxxx_xxxx_0000_0000`) — the `%llx` leading-zero-suppression and union-pun byte-coverage path | [x] |
| 20 | `driver` | **decimal round-trip** shapes: doubles parsed from random short decimal strings (`d.dddde±dd`) — the shapes a real consumer actually feeds in, where `%a`'s exact binary form is "ugly" | [x] |
| 21 | `driver` | **sequential / stateful use**: the same `.so` handle called thousands of times in a row (rows 1–20 replayed back-to-back without reloading), and interleaved with the test's own libc `printf` — proves there is no hidden per-call state, no stdout-buffering divergence, and identical stream interleaving | [x] |
| 22 | `driver` | **exhaustive exponent-field sweep**: all 2048 exponent-field values × a fixed set of representative mantissas × both signs, driven through raw bits — deterministic full coverage of every `%a` exponent and every IEEE class transition | [x] |

## Feature combinations

`Cargo.toml` has no `[features]` table, so the feature cross-product is the
single empty set. Rows 1–22 are re-run verbatim under `--no-default-features`
and `--all-features` (see `run_all.sh`) to discharge the "every feature
combination" gate.
