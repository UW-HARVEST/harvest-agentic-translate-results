# CONFIGS.md — Phase B configuration-surface table

## Mechanical derivation of the axes

The whole executable surface of the C library is three lines:

```c
int n = h >> 10;
out.num = m__mantissa[(h & 0x3ff) + m__offset[n]] + m__exponent[n];
return out.flt;
```

### Axis 1 — public entry points (full set, including the lowest level)

`c_src/include/lib.h` declares exactly one function, and `nm -D` on the C `.so`
exports exactly one symbol: `half2float`. There is **no** convenience/one-shot
wrapper layered over a lower-level API, and no `_ex`/`_with_options` variant —
`half2float` *is* the lowest-level entry point. The three lookup tables are
`static` (file-local) and unreachable from outside. So this axis has exactly one
value, and testing it is testing the lowest level directly.

### Axis 2 — runtime options / modes / flags

Enumerated by grepping the public header and the C source for settable state:
there is no config struct, no setter, no init function, no global or
thread-local mutable state, no environment-variable read, and no `#ifdef` /
`#if` conditional compilation anywhere in `lib.c`. `half2float` is a pure
function of its single argument. **This axis is empty** — zero flags to combine.

Correspondingly, `translation/Cargo.toml` declares no `[features]` section, so
there is exactly one Rust build configuration (see Phase D).

### Axis 3 — input shapes the code actually special-cases

Derived from the branch-free indexing arithmetic, since the *tables* encode the
case distinctions that a branchy implementation would put in `if`s:

- **`n = h >> 10` ∈ [0, 63]** — 64 distinct table rows, one per value.
- **`m__offset[n]` ∈ {0x0000, 0x0400}** — `0x0000` only for `n = 0` and
  `n = 32`; `0x0400` for the other 62. This selects **which half of
  `m__mantissa` is read**: the low region `[0, 1023]` (subnormal-half decoding,
  each entry a differently-scaled normal float) or the high region
  `[1024, 2047]` (normal-half decoding, entries `0x38000000 + m*0x2000`).
- **`m__exponent[n]` class** — 6 distinct classes:
  `0x00000000` (n=0), regular positive `0x00800000..0x0f000000` (n=1..30),
  special positive `0x47800000` (n=31), sign-only `0x80000000` (n=32),
  regular negative `0x80800000..0x8f000000` (n=33..62),
  special negative `0xc7800000` (n=63).
- **`m = h & 0x3ff` ∈ [0, 1023]** — zero vs. non-zero changes the *kind* of
  result at `n ∈ {0, 32}` (±0 vs. subnormal) and at `n ∈ {31, 63}`
  (±infinity vs. NaN), and the payload elsewhere.
- **sign bit `h >> 15`** — equivalently `n >= 32`; mirrors every positive class
  to a negative one.
- **byte order / element width / count** — not axes here: the API takes one
  scalar `uint16_t` by value and returns one scalar `float`. There is no buffer,
  no length, no count, no stride, and no format parameter, so "empty / one /
  many" does not apply.

## Configuration-surface table

One row per combination the C treats differently (the pruned cross-product of
axes 1–3). Every row is exercised by `tests/differential.rs` against **both**
`.so` files, with many randomized inputs per row drawn from a fixed seed
(`SEED = 0x243F_6A88_85A3_08D3`, splitmix64), comparing **raw result bits**
(`to_bits()`) so NaN payloads and the sign of zero are not lost.

| # | entry point(s) | configuration (options set + input shape) | [x] |
|---|----------------|--------------------------------------------|-----|
| 1 | `half2float` | no options (none exist); `n = 0`, `m = 0` → `h = 0x0000`: mantissa low region idx 0, exponent `0x00000000` ⇒ `+0.0` | [x] |
| 2 | `half2float` | `n = 0`, `m ∈ [1, 1023]` (positive subnormal half): offset `0x0000`, mantissa low region `[1, 1023]`, exponent `0x00000000` | [x] |
| 3 | `half2float` | `n ∈ [1, 14]` (positive normal half, exponent below bias), offset `0x0400`, mantissa high region, regular positive exponent | [x] |
| 4 | `half2float` | `n = 15` (positive, unbiased exponent — values around 1.0), offset `0x0400`, mantissa high region | [x] |
| 5 | `half2float` | `n ∈ [16, 29]` (positive normal half, exponent above bias), offset `0x0400`, mantissa high region | [x] |
| 6 | `half2float` | `n = 30` (largest positive finite half exponent), offset `0x0400`, mantissa high region, exponent `0x0f000000` | [x] |
| 7 | `half2float` | `n = 31`, `m = 0` → `h = 0x7C00`: special exponent `0x47800000` ⇒ `+infinity` | [x] |
| 8 | `half2float` | `n = 31`, `m ∈ [1, 1023]`: special exponent `0x47800000` ⇒ positive **NaN**; payload must match bit-for-bit | [x] |
| 9 | `half2float` | `n = 32`, `m = 0` → `h = 0x8000`: offset `0x0000`, mantissa low region idx 0, sign-only exponent `0x80000000` ⇒ `-0.0` (distinct bits from `+0.0`) | [x] |
| 10 | `half2float` | `n = 32`, `m ∈ [1, 1023]` (negative subnormal half): offset `0x0000`, mantissa low region, exponent `0x80000000` | [x] |
| 11 | `half2float` | `n ∈ [33, 46]` (negative normal half, exponent below bias), offset `0x0400`, mantissa high region, regular negative exponent | [x] |
| 12 | `half2float` | `n = 47` (negative, unbiased exponent — values around −1.0), offset `0x0400`, mantissa high region | [x] |
| 13 | `half2float` | `n ∈ [48, 61]` (negative normal half, exponent above bias), offset `0x0400`, mantissa high region | [x] |
| 14 | `half2float` | `n = 62` (largest negative finite half exponent), offset `0x0400`, mantissa high region, exponent `0x8f000000` | [x] |
| 15 | `half2float` | `n = 63`, `m = 0` → `h = 0xFC00`: special exponent `0xc7800000` ⇒ `−infinity` | [x] |
| 16 | `half2float` | `n = 63`, `m ∈ [1, 1023]`: special exponent `0xc7800000` ⇒ negative **NaN**; payload must match bit-for-bit | [x] |
| 17 | `half2float` | offset-axis isolation: every `n` for which `m__offset[n] == 0x0000` (i.e. `n ∈ {0, 32}`, the low-mantissa-region rows) crossed with `m` at `0`, `1`, `512`, `1023` | [x] |
| 18 | `half2float` | offset-axis isolation: every `n` for which `m__offset[n] == 0x0400` (the other 62 rows) crossed with `m` at `0`, `1`, `512`, `1023` — i.e. the full high-region index span `[1024, 2047]` | [x] |
| 19 | `half2float` | full cross-product of axis 3: **all 64 values of `n` × all 1024 values of `m`** = all 65 536 inputs, exhaustive (the entire domain of the API) | [x] |
| 20 | `half2float` | randomized whole-domain sweep, 200 000 seeded draws over `[0, 0xFFFF]`, verifying repeated/interleaved calls are stateless and order-independent | [x] |
| 21 | `half2float` | ABI shape: argument supplied through a wider (32-bit) parameter slot with non-zero high bits, exercising narrow-argument truncation on both sides | [x] |

## Notes on the arithmetic that the tests must pin down

- The C add `m__mantissa[...] + m__exponent[n]` is `uint32_t` arithmetic and
  therefore **wraps** rather than saturating or trapping. Rust must use
  `wrapping_add`; plain `+` would panic in a debug build. Rows 15–16 and 19
  cover the largest sums (`n = 63`, exponent `0xc7800000`, mantissa up to
  `0x387fe000`, giving `0xFFFFE000` — the closest the domain comes to wrapping).
- The result is produced by **type-punning** the `uint32_t` through a union, not
  by a numeric conversion; Rust must use `f32::from_bits`, and the tests must
  compare `to_bits()`, never `==`.
