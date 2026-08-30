# CONFIGS.md — Phase A: CONFIGURATION-SURFACE TABLE (valid inputs)

## Axes actually present in the C source

Enumerated mechanically, not guessed:

1. **Runtime options / modes / flags:** **none.** `driver.h` exports exactly one
   declaration, `void driver(float x);` — no setter, no context struct, no
   global, no `#ifdef` in `src/driver.c` (grep: 0 `#if`s other than the header
   guard), no `switch`, no `if`. There is nothing to configure, so the
   configuration cross-product collapses onto the *input-shape* axes below.
2. **Public entry points (FULL set, including the lowest level):**
   - `driver` — the only *exported* symbol (`nm -D` ⇒ 1 symbol).
   - `print_hex` — the lower-level worker. It is `static`, i.e. **not** part of
     the ABI, so it cannot be (and must not be) called across the `.so`
     boundary. It is nevertheless exercised on every `driver` call with
     `len == sizeof(float) == 4`, which is its only reachable configuration
     (see ERRORS.md rows 1–3).
3. **Input shapes the code's behaviour distinguishes** (the value is
   reinterpreted byte-wise, so *every* IEEE-754 binary32 class is a distinct
   shape, and byte order matters):
   - IEEE class: `+0`, `-0`, subnormal, normal, `inf`, quiet NaN, signalling NaN
   - sign bit: `0` / `1`
   - byte-level shapes: bytes `< 0x10` (exercise the `%02x` **zero-padding**),
     bytes `>= 0x80` (exercise `unsigned char` → `int` promotion, i.e. that no
     sign extension leaks in), `0x00` bytes, `0xff` bytes
   - byte order: the loop walks the object representation from the lowest
     address up, so on x86-64 little-endian the LSB prints first — a
     bswap/endianness mistake in the Rust is visible here
   - counts: one call, two calls, many calls (output concatenation / stream
     buffering on the shared libc `stdout`)
4. **Cargo feature axes:** `Cargo.toml` declares **no `[features]` table**, so
   the only feature combination is the default (empty) one. Verified with
   `cargo metadata`; see the feature-combination script in
   `tests/phase_d_symbols.rs` / the report.

## Configuration table (one row per combination the C treats differently)

Every row is driven through the exported `.so` symbol `driver` in **both** the C
and the Rust library, with **many randomized inputs per row** (fixed seed
`0x5EED_1234`, deterministic xorshift64* PRNG) unless the row is a fixed
singleton bit pattern, and stdout compared byte-for-byte.

| # | entry point(s) | configuration (options set + input shape) | [ ] |
|---|----------------|------------------------------------------|-----|
| 1 | `driver` (→ `print_hex`, len=4) | `+0.0f` — all-zero object representation | [x] |
| 2 | `driver` | `-0.0f` — sign bit only | [x] |
| 3 | `driver` | small positive integers as floats: `1,2,3,…,1024` (many values) | [x] |
| 4 | `driver` | small negative integers as floats: `-1,-2,…,-1024` (many values) | [x] |
| 5 | `driver` | randomized normal positives, full exponent range (uniform random bit patterns filtered to normal, sign=0) | [x] |
| 6 | `driver` | randomized normal negatives (uniform random bit patterns filtered to normal, sign=1) | [x] |
| 7 | `driver` | randomized **subnormals**, sign=0 (exp field 0, mantissa≠0) | [x] |
| 8 | `driver` | randomized **subnormals**, sign=1 | [x] |
| 9 | `driver` | `+inf` / `-inf` | [x] |
| 10 | `driver` | randomized **quiet NaNs** with random payloads, both signs | [x] |
| 11 | `driver` | randomized **signalling NaNs** with random payloads, both signs | [x] |
| 12 | `driver` | boundary constants: `FLT_MIN`, `FLT_MAX`, `-FLT_MIN`, `-FLT_MAX`, `FLT_EPSILON`, smallest subnormal, largest subnormal | [x] |
| 13 | `driver` | byte-shape: patterns whose bytes are all `< 0x10` (forces `%02x` zero padding, e.g. `0x0f0e0100`) | [x] |
| 14 | `driver` | byte-shape: patterns whose bytes are all `>= 0x80` (checks `unsigned char`→`int` promotion, no sign extension), e.g. `0xffffffff`, `0x80808080` | [x] |
| 15 | `driver` | byte-shape: mixed `0x00`/`0xff` permutations — all 2^4 = 16 combinations of per-byte `0x00`/`0xff` (endianness/order check) | [x] |
| 16 | `driver` | byte-shape: single-byte walk — `1 << k` for every bit position `k` in `0..32` (exhaustive per-bit endianness/order check) | [x] |
| 17 | `driver` | uniform-random **arbitrary 32-bit patterns** reinterpreted as `float` (large randomized sweep across all IEEE classes at once) | [x] |
| 18 | `driver` | systematic sweep: exhaustive over the top 16 bits with the low 16 bits randomized (covers every sign/exponent/high-mantissa combination) | [x] |
| 19 | `driver` | count = 1 call (single invocation, exactly 9 output bytes) | [x] |
| 20 | `driver` | count = 2 calls (output concatenation, no separator inserted) | [x] |
| 21 | `driver` | count = many (100 000+ calls in one capture; stream buffering / no flush-behaviour divergence) | [x] |
| 22 | `driver` | interleaved C-then-Rust-then-C calls onto the **same** `stdout` stream (shared-libc buffering interaction) | [x] |
| 23 | `driver` | argument passed as a value already living in an xmm register vs. loaded from memory (calling-convention check: value computed at runtime, opaque to the optimiser, so no constant folding hides a mismatch) | [x] |
| 24 | `print_hex` reachable configuration | `len == 4` with every possible byte value `0x00..0xff` appearing in every one of the 4 positions (4 × 256 = 1024 distinct placements) | [x] |
| 25 | all of rows 1–24 | under the **only** feature combination (default / no features — no `[features]` in `Cargo.toml`), and under both the `debug` and `release` (`panic = "abort"`) build profiles of the Rust `.so` | [x] |

Covered by `tests/phase_b_valid.rs` (+ `tests/phase_d_symbols.rs` for row 25's
symbol/profile part).

## Additional deep-coverage rows (opt-in: `cargo test --release -- --ignored`)

| # | entry point(s) | configuration (options set + input shape) | [ ] |
|---|----------------|------------------------------------------|-----|
| 26 | `driver` | strided sweep over the **entire 2^32 input space** (prime stride 4093 ⇒ 1 049 344 distinct bit patterns, walking every sign/exponent/mantissa region) | [x] |
| 27 | `driver` | **exhaustive** over all 2^16 low bits for 8 fixed high halves — one per IEEE class (`0x0000` ±zero/subnormal, `0x8000`, `0x0080` smallest normal, `0x3f80` around 1.0, `0x7f7f` near FLT_MAX, `0x7f80` +inf/+sNaN, `0x7fc0` +qNaN, `0xff80` -inf/-sNaN) ⇒ 524 288 patterns | [x] |

Both were executed and passed byte-for-byte (≈1.57 M differential comparisons).

## Test-suite sensitivity (mutation check)

A configuration table only means something if the tests would notice a wrong
translation. `./mutation_check.sh` injects one deliberate bug at a time into
`src/lib.rs`, rebuilds the `.so`, and checks the suite fails:

| mutant | result |
|--------|--------|
| uppercase hex (`%02X`) | CAUGHT (23 tests) |
| no zero padding (`%2x`) | CAUGHT (24 tests) |
| byte order reversed (big-endian dump) | CAUGHT (24 tests) |
| sign-extended byte (`i8` promotion) | CAUGHT (23 tests) |
| length off by one (3 bytes) | CAUGHT (25 tests) |
| length off by one (5 bytes) | CAUGHT (25 tests) |
| missing trailing newline | CAUGHT (25 tests) |
| newline → `\r\n` | CAUGHT (25 tests) |
| NaN quietened / canonicalised | CAUGHT (11 tests) |
| negative zero flattened | CAUGHT (3 tests) |
| subnormals flushed to zero | CAUGHT (13 tests) |
| dumps `f64` bytes instead of `f32` | CAUGHT (24 tests) |
| loop guard uses **unsigned** compare | *not caught — equivalent mutant* |

The single uncaught mutant is behaviourally **unobservable through the ABI**:
it only changes `print_hex` when `len < 0`, and `driver` always passes the
compile-time constant `sizeof(float)` == 4 (ERRORS.md rows 2–3). No input to the
exported `driver` can distinguish it, so it is an equivalent mutant rather than a
gap in the tests.
