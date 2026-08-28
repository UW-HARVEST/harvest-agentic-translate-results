# CONFIGS.md — Phase A: configuration surface table (VALID inputs)

Mechanically derived from `c_src/src/lib.c` + `c_src/include/lib.h`.

## Public entry points (complete set)

`c_src/include/lib.h` declares exactly one function, and `nm -D` on the C `.so`
confirms exactly one dynamic symbol:

| entry point | signature | kind |
|---|---|---|
| `memchra2` | `int memchra2(int a, int b, int c, int d)` | the one and only public entry point — it is simultaneously the highest-level *and* lowest-level exported function |

There are no convenience wrappers vs. low-level variants here: the 8 helpers
(`memchra`, `process_buffer`, `int_to_float_bits`, `process_strings`,
`safe_sum_array`, `interpret_as_int`, `count_occurrences`, `complex_iteration`)
are `static`, so the only way to drive them from outside the `.so` is through
`memchra2`. Each row below therefore names the internal pipeline stage(s) the
configuration is chosen to stress, and every row runs the **full** composed
pipeline end to end (format → count → sum → strncmp table → float pun →
buffer sum → byte reinterpretation → XOR fold).

## Runtime options / modes / flags

There are **no** runtime options, no global state, no setters, no environment
lookups and no `#ifdef` compile-time switches in `lib.c`. The "configuration" of
this library is therefore entirely the *shape and value class* of the four `int`
arguments. The axes below are the ones the C code actually branches on.

The Rust crate declares one cargo feature, `test_internals`, which is **off by
default** and only adds the test-only `harness_*` exports used by Phase C — it
does not change `memchra2`. Every row below is nevertheless re-run under every
feature combination (`--no-default-features`,
`--no-default-features --features test_internals`, `--all-features`) in both the
`dev` and `release` profiles by `run_verification.sh`.

## Axes the C code actually distinguishes

| axis | source line | distinct classes |
|---|---|---|
| **F** — float class of `a`'s bit pattern | `lib.c:151-154` (`int_to_float_bits`, `f > 0.0f && f < 1000.0f`, `(int)f`) | `a < 0` (negative float / −inf / −NaN); `a == 0` (+0.0); `1 ≤ a < 0x3F800000` (subnormal or 0<f<1 → `(int)f == 0`); `0x3F800000 ≤ a < 0x447A0000` (1 ≤ f < 1000 → non-zero contribution); `0x447A0000 ≤ a ≤ 0x7F7FFFFF` (f ≥ 1000); `a == 0x7F800000` (+inf); `a > 0x7F800000` (NaN) |
| **S** — sign pattern of `(a,b,c,d)` | `lib.c:132,134` (`%d` emits `-`, then `count_occurrences(buffer,'-')`) | 16 combinations; `dash_count` ∈ 3..7 |
| **W** — decimal width of each argument | `lib.c:132,156` (`snprintf` field length → `strlen` → `process_buffer` sum) | 1 digit … 10 digits, plus the 11-char `-2147483648` |
| **B** — low byte of `b`, `c`, `d` | `lib.c:161-167` (`bytes[0..2]`, `interpret_as_int`, little-endian) | `0x00` vs non-zero, each of the 3 positions; whole 24-bit space |
| **X** — low byte of `a`,`b`,`c`,`d` | `lib.c:120-123` (`complex_iteration`, `result ^= u & 0xFF`) | XOR fold = 0 vs non-zero |
| **O** — arithmetic overflow of `sum` / `result` | `lib.c:88-90` (`safe_sum_array`), `lib.c:135-171` | no wrap vs wrap (two's-complement) |
| **P** — `buf_sum` magnitude & `buf_sum % 256` | `lib.c:156-159` | small (short buffer) vs large (long buffer); residue 0 vs non-zero |
| **C** — fixed constants exercised on every call | `lib.c:137-149` (`values[4]`, `test_strings[4]`, `target="test"`, `count=4`) | always: `safe_sum_array(size=4)`, `process_strings(count=4)` → 3 matches of 4 |

## Configuration rows

Each row is checked off only after **both** libraries agree byte-for-byte across
the randomized inputs generated for that row (fixed-seed xorshift PRNG,
`tests/differential.rs`).

| # | entry point(s) | configuration (options set + input shape) | [x] |
|---|----------------|--------------------------------------------|-----|
| 1 | `memchra2` | F=`a==0`, S=all non-negative, W=1 digit, X=0: `a=b=c=d=0` (degenerate all-zero shape) | [x] |
| 2 | `memchra2` | F=`a==0`, S=all non-negative, W=1 digit, random small `b,c,d ∈ 0..9` | [x] |
| 3 | `memchra2` | F=subnormal (`a ∈ 1..0xFFFF`), random `b,c,d` full range | [x] |
| 4 | `memchra2` | F=`0<f<1` (`a ∈ 0x00010000..0x3F7FFFFF`), random `b,c,d` full range → `(int)f == 0` | [x] |
| 5 | `memchra2` | F=`1≤f<1000` (`a ∈ 0x3F800000..0x4479FFFF`), random `b,c,d` full range → non-zero float contribution | [x] |
| 6 | `memchra2` | F=`1≤f<1000`, `a` pinned to exact float bit patterns for f = 1.0, 1.5, 2.0, 9.99, 255.5, 999.9375, and the last value below 1000 | [x] |
| 7 | `memchra2` | F=`f≥1000` finite (`a ∈ 0x447A0000..0x7F7FFFFF`), random `b,c,d` | [x] |
| 8 | `memchra2` | F=`f` boundary pair: `a = 0x447A0000` (exactly 1000.0, excluded) and `a = 0x4479FFFF` (largest included) | [x] |
| 9 | `memchra2` | F=+inf (`a = 0x7F800000`) / −inf (`a = 0xFF800000u as i32`), random `b,c,d` | [x] |
| 10 | `memchra2` | F=NaN quiet/signalling, both signs (`0x7FC00000`, `0x7F800001`, `0x7FFFFFFF`, `0xFFC00000`, `0xFFFFFFFF`), random `b,c,d` | [x] |
| 11 | `memchra2` | F=`a<0` (negative float), S=`a` negative + `b,c,d` non-negative → dash_count = 4 | [x] |
| 12 | `memchra2` | S = each of the 16 sign combinations of `(a,b,c,d)`, magnitudes randomized → dash_count sweeps 3..7 | [x] |
| 13 | `memchra2` | W = uniform width sweep: all four args randomized within each decimal width 1..10 (positive) | [x] |
| 14 | `memchra2` | W = uniform width sweep, negative: all four args randomized within each decimal width 1..10 (negative) | [x] |
| 15 | `memchra2` | W = mixed widths: `a` 1-digit, `b` 10-digit, `c` 5-digit negative, `d` 2-digit (maximal `strlen` asymmetry) | [x] |
| 16 | `memchra2` | W = maximal buffer length: all four = `INT_MIN` (`-2147483648`) → 51-byte string, longest `snprintf` output | [x] |
| 17 | `memchra2` | W = `INT_MAX` in all four positions and in each position individually | [x] |
| 18 | `memchra2` | B: low byte of `b` = 0, `c`,`d` non-zero (interpret_as_int LSB zero) | [x] |
| 19 | `memchra2` | B: low bytes of `b`,`c`,`d` all 0 → `interpret_as_int == 0` (XOR identity) | [x] |
| 20 | `memchra2` | B: low bytes of `b`,`c`,`d` all `0xFF` → `interpret_as_int == 0x00FFFFFF` (max 24-bit) | [x] |
| 21 | `memchra2` | B: exhaustive-ish sweep of `(b&0xFF, c&0xFF, d&0xFF)` via randomized high bits with pinned low bytes | [x] |
| 22 | `memchra2` | X: low bytes chosen so `complex_iteration` XOR fold == 0 (e.g. pairs cancel) | [x] |
| 23 | `memchra2` | X: low bytes chosen so XOR fold == 0xFF | [x] |
| 24 | `memchra2` | O: `a+b+c+d` overflows `int` positively (e.g. all near `INT_MAX`) → wraparound in `safe_sum_array` | [x] |
| 25 | `memchra2` | O: `a+b+c+d` overflows `int` negatively (all near `INT_MIN`) → wraparound | [x] |
| 26 | `memchra2` | P: shortest buffer (`a=b=c=d=0`, 11 bytes) vs longest (row 16) → `buf_sum % 256` at both extremes | [x] |
| 27 | `memchra2` | P: arguments tuned so `buf_sum % 256 == 0` (search over randomized inputs) | [x] |
| 28 | `memchra2` | Full unconstrained random: uniform over the whole `int^4` space (100 000 samples, fixed seed) | [x] |
| 29 | `memchra2` | Boundary-value cross-product: each argument independently drawn from {`INT_MIN`, `INT_MIN+1`, `-256`, `-255`, `-1`, `0`, `1`, `255`, `256`, `0x3F800000`, `0x447A0000`, `0x7F800000`, `INT_MAX`} (13^4 = 28 561 combinations, exhaustive) | [x] |
| 30 | `memchra2` | Repeated-call / statelessness check: the same argument tuple called 3× interleaved with other tuples returns the identical value from both `.so`s (no hidden global state) | [x] |
| 31 | `memchra2` | Exhaustive sweep of `a` over all 24 float-exponent boundaries `0x00800000 * k` with `b,c,d` randomized | [x] |
| 32 | `memchra2` | Digit-content sweep: arguments composed only of the digit `9` (`9, 99, 999, …, 999999999`) — maximises `buf_sum` per character | [x] |

## Status — every row is checked off

`tests/differential.rs` contains exactly one test per row
(`cfg_row01_…` … `cfg_row32_…`); all 32 pass in every feature combination and in
both the `dev` and `release` profiles.

Volume actually executed (all comparisons are `C .so` vs `Rust .so` through
`dlsym`):

| suite | calls | notes |
|---|---|---|
| `tests/differential.rs` (rows 1-32) | ≈ 145 000 | includes the exhaustive 13^4 = 28 561 boundary cross-product (row 29) |
| `tests/fuzz.rs` (default) | ≈ 1 900 000 | uniform `int^4`, float window, exponent neighbourhoods, low-byte plane, sign×width cross-product, contiguous `a` sweeps |
| `tests/fuzz.rs -- --ignored` | ≈ 84 000 000 | `heavy_exhaustive_low_bytes` (all 2^24 low-byte triples), `heavy_exhaustive_a_low24` (3 × 2^24 contiguous `a`), `heavy_full_range_a_stride` (4 × 4.2 M strided over the whole `int` range) |
| `tests/c_optimization_levels.rs` | ≈ 50 000 × 6 builds | the same C source at `-O0/-O1/-O2/-O3/-Os` plus the CMake build, all vs Rust |

Zero divergences observed.
