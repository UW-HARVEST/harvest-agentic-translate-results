# CONFIGS.md — Phase A configuration-surface table

The mirror of `ERRORS.md`: every **valid** configuration the C code treats
differently.

## Axis enumeration (derived from the C source, not guessed)

### Axis 1 — runtime options / modes / flags

**None.** `c_src/include/lib.h` exposes a single function with a single scalar
argument and no context struct, no setter, no global mode variable, no
environment lookup and no `#ifdef`. There is no state to configure, so this axis
contributes exactly one (empty) setting to every row.

### Axis 2 — public entry points

**One, and it is also the lowest level one.** There is no convenience wrapper
layered over a lower-level core to skip; `half2float` *is* the primitive:

| entry point | signature | notes |
|-------------|-----------|-------|
| `half2float` | `float half2float(uint16_t h)` | the only export; called directly via the `.so` in all tests |

### Axis 3 — input shapes the code actually special-cases

The C has no `if`/`switch`; its branching is *data-driven* through the two
64-entry side tables indexed by `n = h >> 10`. Reading the literal contents of
`m__offset` and `m__exponent` gives the regions the code genuinely distinguishes:

| `n = h >> 10` | `m__offset[n]` | `m__exponent[n]` | meaning of the region |
|---------------|----------------|-------------------|-----------------------|
| `0` | `0x0000` | `0x00000000` | positive zero / positive subnormal (mantissa table used at low half) |
| `1 .. 30` | `0x0400` | `0x00800000 * n` | positive normal |
| `31` | `0x0400` | `0x47800000` | positive infinity / positive NaN (the one "odd" exponent entry) |
| `32` | `0x0000` | `0x80000000` | negative zero / negative subnormal (offset drops back to 0) |
| `33 .. 62` | `0x0400` | `0x80000000 + 0x00800000*(n-32)` | negative normal |
| `63` | `0x0400` | `0xC7800000` | negative infinity / negative NaN |

Crossed with the mantissa field `h & 0x3ff` (boundary values `0x000`, `0x001`,
`0x200`, `0x3FF`, plus randomized values), and with the two distinct `m__offset`
values that select the low vs. high half of `m__mantissa`.

## Configuration-surface table

One row per meaningful combination of the axes above. Every row is driven
through the `.so` export of **both** C and Rust and compared **bit-for-bit**
(`f32::to_bits`, never `==`, so NaN payloads and signed zero are distinguished).
Every row uses many randomized inputs from a fixed seed in addition to its
boundary values.

| # | entry point(s) | configuration (options set + input shape) | [ ] |
|---|----------------|--------------------------------------------|-----|
| 1 | `half2float` | `n = 0`, mantissa `0x000` → exact `+0.0` (must keep sign bit clear) | [x] |
| 2 | `half2float` | `n = 0`, mantissa `0x001` → smallest positive subnormal | [x] |
| 3 | `half2float` | `n = 0`, mantissa `0x3FF` → largest positive subnormal | [x] |
| 4 | `half2float` | `n = 0`, mantissa randomized over `0x000..0x3FF` → positive subnormal region, low half of `m__mantissa` (offset `0x0000`) | [x] |
| 5 | `half2float` | `n = 1` (smallest positive normal exponent), mantissa boundaries + randomized | [x] |
| 6 | `half2float` | `n ∈ 2..=29` randomized, mantissa randomized → interior positive normal, high half of `m__mantissa` (offset `0x0400`) | [x] |
| 7 | `half2float` | `n = 30` (largest finite positive exponent), mantissa `0x3FF` → largest finite positive half (`0x7BFF`) | [x] |
| 8 | `half2float` | `n = 31`, mantissa `0x000` → `+Inf` (`0x7C00`) | [x] |
| 9 | `half2float` | `n = 31`, mantissa `0x001` → positive signalling-NaN pattern (`0x7C01`); payload bits must survive the register return unchanged | [x] |
| 10 | `half2float` | `n = 31`, mantissa `0x200` → positive quiet NaN (`0x7E00`) | [x] |
| 11 | `half2float` | `n = 31`, mantissa `0x3FF` → `0x7FFF`, top of the positive NaN range | [x] |
| 12 | `half2float` | `n = 31`, mantissa randomized → whole positive NaN/Inf region | [x] |
| 13 | `half2float` | `n = 32`, mantissa `0x000` → exact `-0.0` (sign bit set, must not compare equal to `+0.0`) | [x] |
| 14 | `half2float` | `n = 32`, mantissa `0x001` → smallest negative subnormal | [x] |
| 15 | `half2float` | `n = 32`, mantissa `0x3FF` → largest negative subnormal | [x] |
| 16 | `half2float` | `n = 32`, mantissa randomized → negative subnormal region; verifies `m__offset` drops back to `0x0000` at `n = 32` (low half of `m__mantissa` reused with a negative exponent) | [x] |
| 17 | `half2float` | `n = 33` (smallest negative normal exponent), mantissa boundaries + randomized | [x] |
| 18 | `half2float` | `n ∈ 34..=61` randomized, mantissa randomized → interior negative normal | [x] |
| 19 | `half2float` | `n = 62` (largest finite negative exponent), mantissa `0x3FF` → largest-magnitude finite negative half (`0xFBFF`) | [x] |
| 20 | `half2float` | `n = 63`, mantissa `0x000` → `-Inf` (`0xFC00`) | [x] |
| 21 | `half2float` | `n = 63`, mantissa `0x001` → negative signalling-NaN pattern (`0xFC01`) | [x] |
| 22 | `half2float` | `n = 63`, mantissa `0x3FF` → `0xFFFF`, the maximum input value | [x] |
| 23 | `half2float` | `n = 63`, mantissa randomized → whole negative NaN/Inf region; also the region where `m__mantissa + m__exponent` comes closest to `u32` overflow, so it pins down the wrapping-add semantics | [x] |
| 24 | `half2float` | both `m__offset` values exercised back-to-back in one run (`n = 0` then `n = 1`, `n = 32` then `n = 33`) to catch a mis-shared index base | [x] |
| 25 | `half2float` | fully randomized `h` over the whole `0x0000..=0xFFFF` domain, fixed seed, many iterations (unconstrained cross-product of both axes) | [x] |
| 26 | `half2float` | **exhaustive**: all 65 536 inputs, bit-for-bit — the complete cross-product of Axis 2 × Axis 3, which subsumes rows 1–25 | [x] |
| 27 | `half2float` | repeated / interleaved calls in a single loaded-library session (both `.so`s stay loaded), confirming the function is stateless and order-independent in both implementations | [x] |

## Feature combinations

`Cargo.toml` has no `[features]` table, so the complete set of build
configurations is:

| # | cargo invocation | [ ] |
|---|------------------|-----|
| F1 | `cargo test` (default features — the empty set) | [x] |
| F2 | `cargo test --no-default-features` | [x] |
| F3 | `cargo test --all-features` | [x] |

All three are identical builds here, but all three are run so the claim is
measured rather than assumed.
