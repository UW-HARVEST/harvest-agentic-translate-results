# CONFIGS.md — Phase B configuration-surface table

Mechanically derived from the branches `c_src/src/lib.c` actually takes. The
public API is a single entry point,

```c
int memchra2(int a, int b, int c, int d);
```

with no options struct, no global flags, no `#ifdef`s and no runtime modes.
Therefore the configuration surface is entirely the **input shape of the four
`int` arguments**, and the axes are the value classes the code branches on.

## Axes the C code actually distinguishes

| axis | where it branches in the C | distinct classes |
|---|---|---|
| **A1** — sign of each of `a,b,c,d` as printed by `snprintf("test%d-%d-%d-%d")` | `count_occurrences(buffer, '-')` counts `'-'`; each NEGATIVE argument emits an extra `'-'`, so `dash_count = 3 + (#negatives)` and `result += dash_count*10` | 0, 1, 2, 3, 4 negatives (⇒ `+30 .. +70`) |
| **A2** — bit pattern of `a` viewed as `float` (`int_to_float_bits` union pun) | `if (f > 0.0f && f < 1000.0f) result += (int)f;` | (i) `f` negative (`a < 0`); (ii) `f == +0.0` (`a == 0`); (iii) `f == -0.0` (`a == INT_MIN`); (iv) subnormal `0<f<2^-126` ⇒ `(int)f == 0`; (v) normal `0<f<1` ⇒ `(int)f == 0`; (vi) `1 <= f < 1000` ⇒ `(int)f` non-zero (the only branch that changes `result`); (vii) `f >= 1000`; (viii) `f == +inf` (`a == 0x7F800000`); (ix) `f` NaN (`a` in `0x7F800001..0x7FFFFFFF` or negative NaN) |
| **A3** — decimal WIDTH of each argument (digit count) | drives `strlen(buffer)`, hence `memchra`'s `n`, `process_buffer`'s `len` and the loop trip counts | 1 digit … 10 digits, plus `INT_MIN` (11 chars incl. sign) |
| **A4** — digit CONTENT of the formatted buffer | `process_buffer` sums `(int)(char)` of every byte; `result += buf_sum % 256` | value-dependent; covered by randomization over the whole `int` range |
| **A5** — low byte of `b`, `c`, `d` | `interpret_as_int` reads `{b&0xFF, c&0xFF, d&0xFF, 0}` as a little-endian `int`; `result ^= interpreted` | all-zero low bytes; `0xFF` low bytes (sign/high-bit interactions); mixed |
| **A6** — low bytes of all four values | `complex_iteration` computes `result ^= (int)((unsigned)v & 0xFF)` folded over `a,b,c,d` | XOR-cancelling (equal values) vs. distinct |
| **A7** — signed overflow of the accumulations | `safe_sum_array` (`sum += *i`) and the `result +=` chain | sum fits; sum overflows positive; sum overflows negative |
| **A8** — `process_strings` prefix matching against the fixed literal table | `strncmp(*i, "test", 4)`: `"test1"`,`"test2"`,`"testing"` match, `"other"` does not ⇒ `matches == 3`, `result += 15` | input-independent constant, validated on every call |

Axes **A1–A3** are the ones the control flow keys on; **A4–A7** are
value-dependent data paths. `process_strings`, `safe_sum_array` array length,
`interpret_as_int` length and `complex_iteration` count are all fixed literals
in `memchra2`, so they contribute no additional *valid* configuration axis (their
guard branches are enumerated in `ERRORS.md`).

## Configuration rows (pruned cross-product of the axes above)

Every row is exercised with MANY randomized inputs (fixed seed, deterministic
xorshift PRNG in the test harness) plus the named boundary values, calling BOTH
the C `.so` and the Rust `.so` `memchra2` export via `libloading` and asserting
the returned `int` is bit-identical.

| # | entry point(s) | configuration (options set + input shape) | [ ] |
|---|----------------|--------------------------------------------|-----|
| 1 | `memchra2` | all four args small non-negative (`0..=9`, 1 digit) — exhaustive over `10^4` tuples; A1=0 negatives, A2=(ii)/(iv), A3=1 digit | [x] |
| 2 | `memchra2` | all four args small negative (`-9..=-1`) — exhaustive; A1=4 negatives, A2=(i) | [x] |
| 3 | `memchra2` | mixed signs, exhaustive over `{-2,-1,0,1,2}^4`; A1 = 0,1,2,3,4 negatives all hit | [x] |
| 4 | `memchra2` | `a` chosen so the float pun lands in `[1.0, 1000.0)` (`a` in `0x3F800000..0x447A0000`), `b,c,d` random — A2=(vi), the only branch that adds `(int)f` | [x] |
| 5 | `memchra2` | `a` subnormal float pattern (`a` in `1..=0x007FFFFF`) — A2=(iv), `f > 0` true but `(int)f == 0` | [x] |
| 6 | `memchra2` | `a` normal float in `(0.0, 1.0)` (`a` in `0x00800000..0x3F800000`) — A2=(v) | [x] |
| 7 | `memchra2` | `a` float `>= 1000.0` but finite (`a` in `0x447A0000..0x7F7FFFFF`) — A2=(vii) | [x] |
| 8 | `memchra2` | `a == 0x7F800000` (+inf) and `a == 0xFF800000`/`INT_MIN\|0x7F800000` (-inf) — A2=(viii) | [x] |
| 9 | `memchra2` | `a` a quiet/signalling NaN pattern (`0x7F800001`, `0x7FC00000`, `0x7FFFFFFF`, and negative NaNs) — A2=(ix), all comparisons false | [x] |
| 10 | `memchra2` | `a == 0` (`+0.0`) and `a == INT_MIN` (`-0.0`) — A2=(ii)/(iii) | [x] |
| 11 | `memchra2` | each argument at every decimal width boundary: `±1, ±9, ±10, ±99, ±100, ±999, ±10^3..10^9, ±(10^k - 1)`, `INT_MAX`, `INT_MIN`, `INT_MIN+1` — A3 = 1..11 chars, sweeping one position at a time with the others fixed and randomized | [x] |
| 12 | `memchra2` | longest possible formatted buffer: all four args `INT_MIN` / `INT_MAX` (51-byte and 44-byte `strlen`) — A3 max, probes the `snprintf` size-64 boundary | [x] |
| 13 | `memchra2` | `b,c,d` low bytes all `0x00` (e.g. multiples of 256) — A5 makes `interpret_as_int` return 0 (`result ^= 0`) | [x] |
| 14 | `memchra2` | `b,c,d` low bytes all `0xFF` — A5 gives `interpret_as_int == 0x00FFFFFF`, largest magnitude | [x] |
| 15 | `memchra2` | `b,c,d` low bytes mixed/random while high bytes randomized independently — A5, checks only the LOW byte is used | [x] |
| 16 | `memchra2` | `a == b == c == d` (A6: `complex_iteration` XOR fold cancels to 0) | [x] |
| 17 | `memchra2` | `a,b,c,d` with pairwise-equal low bytes but different high bytes (A6 partial cancellation, A5 interaction) | [x] |
| 18 | `memchra2` | arguments summing to signed-overflow-positive (e.g. all near `INT_MAX`) — A7 | [x] |
| 19 | `memchra2` | arguments summing to signed-overflow-negative (e.g. all near `INT_MIN`) — A7 | [x] |
| 20 | `memchra2` | fully random over the entire `int32` range, 200 000 tuples, fixed seed — A4 value-dependent `process_buffer` digit sums, and the full cross-product by sampling | [x] |
| 21 | `memchra2` | fully random restricted to `int8`-magnitude values (`-128..=127`) in all four positions, exhaustive-ish sampling — dense coverage of A1×A5×A6 together | [x] |
| 22 | `memchra2` | one argument extreme (`INT_MIN`/`INT_MAX`/`0`) while the other three are random — sweeps all 4 positions × 3 extremes, catching position-specific handling (`a` feeds the float pun, `b,c,d` feed `interpret_as_int`) | [x] |
| 23 | `memchra2` | `a` extreme × `b,c,d` low-byte extremes simultaneously (A2 × A5 interaction, full `3 × 3` grid, randomized fill) | [x] |
| 24 | `memchra2` | powers of two and powers-of-two-minus-one in all four positions (`±2^k`, `±(2^k-1)` for `k = 0..31`) — exercises A2 exponent boundaries and A5/A6 byte boundaries at once | [x] |

## Feature combinations

`translation/Cargo.toml` has no `[features]` table, so the only combination is
the default build. Rows 1–24 are additionally re-run against the
`--release`-profile `.so` (the artifact under test) and the crate is
`cargo check`ed with `--no-default-features` and `--all-features` for parity.

---

## Results

All 24 rows PASS: C and Rust `memchra2` returned bit-identical `int`s on every
input, under all 3 feature invocations × both profiles (release and debug — the
debug cdylib has overflow checks ON and unwinding panics, a genuinely different
codegen path).

Beyond the 24 rows, `tests/phase_e_stress.rs` and `tests/phase_f_exhaustive.rs`
add ~140 million further differential calls, including these EXHAUSTIVE sweeps:

| sweep | size |
|---|---|
| entire float-accept window `1.0 <= f < 1000.0` (`a` = `0x3F800000..=0x4479FFFF`) | 16,383,999 |
| all low 24 bits of `a` | 16,777,216 |
| all low 24 bits of `b`, of `c`, of `d`, and of `-b` | 4 × 16,777,216 |
| top 2^24 and bottom 2^24 of the `i32` range for `a` | 2 × 16,777,217 |
| all 2^24 combinations of the low bytes of `(b,c,d)` — the exact bytes `interpret_as_int` reinterprets | 16,777,216 |
| every IEEE-754 exponent field of `a` × 9 mantissas × both signs × 4 arg sets | 18,432 |
| uniform random over the full `int32^4` space | 20,000,000 |
