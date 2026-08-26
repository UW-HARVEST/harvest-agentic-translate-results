# CONFIGS.md — Phase B configuration-surface table

The mirror of `ERRORS.md`: every *valid* input configuration the C code
distinguishes. Derived mechanically from the C source.

## Axes derived from the C source

### Axis 1 — public entry points (full set, including the lowest level)

`c_src/include/lib.h` declares exactly one function, and `nm -D` on the C `.so`
confirms exactly one exported symbol:

| entry point | linkage | note |
|-------------|---------|------|
| `double next_double(cn_rnd_t *rnd)` | exported (`T`) | the only public entry point; it *is* the lowest level available to a consumer |
| `static uint64_t cn_rnd_next(cn_rnd_t *rnd)` | internal (`static`) | the lower-level primitive. **Not** callable across the FFI boundary, so it is exercised *indirectly but exhaustively*: `next_double` discards only the low 12 bits of its result, so rows 6–8/14–15 pin down the upper 52 bits of every `cn_rnd_next` return, and rows 13/17 pin down **both** state words it writes, bit for bit. The full 128-bit state transition is therefore differentially verified even though the symbol is private. |

### Axis 2 — runtime options / modes / flags

`grep -nE 'if|switch|\?|#if|enum|flag|mode|option' c_src/src/lib.c` → **0 hits**.
The C code is fully straight-line: there are no options, modes, flags, byte-order
switches, element-type switches, or `#ifdef`s. The configuration surface is
therefore entirely a function of the **input shape** (axis 3) — this is derived
from the source, not assumed.

### Axis 3 — input shapes the code is sensitive to

`next_double` reads `state[0]` (call it `x`) and `state[1]` (`y`) and performs:

| C operation (`lib.c`) | shape the operation is sensitive to |
|-----------------------|-------------------------------------|
| `x ^= x << 23` (l.7) | whether the top 23 bits of `x` are set (they are **truncated** — 64-bit wrapping shift) |
| `x ^= x >> 17` (l.8) | whether the low 17 bits matter / logical (not arithmetic) right shift ⇒ sign bit must not sign-extend |
| `x ^= y ^ (y >> 26)` (l.9) | `y`'s high 26 bits; logical shift again |
| `rnd->state[0] = y; rnd->state[1] = x` (l.6, l.10) | the **swap + write-back**: state mutation must match, and only 16 bytes may be written |
| `return x + y` (l.11) | unsigned **wraparound** when the sum exceeds 2^64−1 (carry out) |
| `mantissa = value >> 12` (l.17) | which 52 bits of `value` survive; low 12 bits are discarded |
| `(1023 << 52) \| mantissa` (l.18) | mantissa `0` ⇒ `1.0`; mantissa all-ones ⇒ just under `2.0` |
| `*(double *)&result - 1.0` (l.19) | type-punned bit reinterpretation + exact subtraction ⇒ result in `[0.0, 1.0)`; must be compared **bit-for-bit** (`to_bits`), not with `==`, so `+0.0` vs `-0.0` cannot hide |
| repeated calls | sequential state evolution / no hidden global state |

## Configuration-surface table

One row per meaningful combination of the axes above (cross-product of axis 3
against the single entry point, pruned to what the C actually distinguishes).
Every row is driven with **many randomized inputs, fixed seed (`0x2545F491_4F6CDD1D`)**,
never a single hand-picked value, and both `.so`s are compared on **(a)** the
returned `f64` bit pattern and **(b)** the full 16-byte post-call state.

| # | entry point(s) | configuration (options set + input shape) | [x] |
|---|----------------|-------------------------------------------|-----|
| 1 | `next_double` | degenerate all-zero state `{0, 0}`; single call **and** 64 iterated calls (state must stay `{0,0}`, result must stay exactly `+0.0` — covers `mantissa == 0` / `result == 1.0` boundary) | [x] |
| 2 | `next_double` | `state[0] == 0`, `state[1]` = 1024 randomized non-zero values; 64 sequential calls each | [x] |
| 3 | `next_double` | `state[0]` = 1024 randomized non-zero values, `state[1] == 0`; 64 sequential calls each | [x] |
| 4 | `next_double` | saturated `{u64::MAX, u64::MAX}`; 64 sequential calls (exercises `x << 23` truncation, `y >> 26`, and `x + y` carry simultaneously) | [x] |
| 5 | `next_double` | the four corner states `{0,0}`, `{0,MAX}`, `{MAX,0}`, `{MAX,MAX}`; 256 sequential calls each | [x] |
| 6 | `next_double` | single-bit sweep on `state[0]`: `state[0] = 1u64 << i` for all `i` in `0..64`, `state[1] = 0`; 8 sequential calls each (isolates every bit's path through `<<23`, `>>17`) | [x] |
| 7 | `next_double` | single-bit sweep on `state[1]`: `state[1] = 1u64 << j` for all `j` in `0..64`, `state[0] = 0`; 8 sequential calls each (isolates every bit's path through `>>26`) | [x] |
| 8 | `next_double` | full two-bit cross product: `state = {1<<i, 1<<j}` for all 64×64 = 4096 pairs; 4 sequential calls each | [x] |
| 9 | `next_double` | high-bit-heavy shapes that make `x << 23` truncate: `state[0]` drawn from `!0 << k` and `0xFFFF_FFFF_FFFF_FFFF >> k` masks for all `k` in `0..64`, `state[1]` randomized | [x] |
| 10 | `next_double` | carry/wraparound shapes for `x + y`: states constructed so the pre-return `x + y` overflows (`y == !x`, `y == !x + 1`, `y == x`, `x == y == 2^63`), plus randomized near-boundary pairs | [x] |
| 11 | `next_double` | mantissa-boundary shapes: states whose produced `value >> 12` is `0` (row 1) or maximal, plus a sweep asserting the returned `f64` is bit-identical across the whole `[0,1)` range reached by 1024 random states | [x] |
| 12 | `next_double` | low-12-bit-discard sensitivity: pairs of states differing only in bits that end up in the discarded low 12 bits of `value`, confirming C and Rust discard *the same* bits | [x] |
| 13 | `next_double` | **state-mutation** check: after every single call, the 16-byte `cn_rnd_t` written by C and by Rust are `memcmp`-equal (word swap + write-back), over 1024 randomized states | [x] |
| 14 | `next_double` | property-style bulk: 4096 fully randomized 128-bit states × 32 sequential calls each, comparing every returned bit pattern and every intermediate state | [x] |
| 15 | `next_double` | long single stream: 1,000,000 sequential calls from one fixed seed, comparing every returned bit pattern (catches drift that short runs miss) | [x] |
| 16 | `next_double` | multiple independent `cn_rnd_t` instances driven **interleaved** in round-robin (proves there is no hidden global/`static` state and each pointer is independent) | [x] |
| 17 | `next_double` | struct embedded at an aligned offset inside a larger buffer with 32 guard bytes of random canary before and after — asserts C and Rust write exactly the same 16 bytes and neither touches the canaries | [x] |
| 18 | `next_double` | bit-exact result comparison for every row above via `f64::to_bits` (never `==`), so `+0.0`/`-0.0` and any NaN/subnormal encoding difference would be caught | [x] |

## Verification matrix

Every row above was run in all four cells of the matrix below (Phase B **and**
Phase C), via `./verify.sh`:

| Rust `.so` profile | C `.so` build | result |
|--------------------|---------------|--------|
| debug (`ub_checks` on) | default (`c_src/build`, no `-O`) | 19 + 7 pass |
| debug | `-O2` (`CMAKE_BUILD_TYPE=Release`, same unmodified source) | 19 + 7 pass |
| release (`panic = "abort"`) | default | 19 + 7 pass |
| release | `-O2` | 19 + 7 pass |

The `-O2` C build is included because `return *(double *)&result - 1.0;` is a
type-pun, so an optimizing C compiler could in principle differ from `-O0`; it
does not (`-O2` emits a 16-byte `movups` state store and an `addsd` of `-1.0`,
observably equivalent).

Feature combinations: `Cargo.toml` has no `[features]` table, so the powerset is
the single empty combination — see `SYMBOLS.md`. `verify.sh` derives the powerset
mechanically from `Cargo.toml`, so it stays correct if features are ever added.

Approximate call volume: ~1.2M `next_double` calls per implementation per cell in
the default suite, plus a 10.24M-call soak test
(`cargo test --test phase_b_valid_paths -- --ignored`), which also passes against
both C builds.
