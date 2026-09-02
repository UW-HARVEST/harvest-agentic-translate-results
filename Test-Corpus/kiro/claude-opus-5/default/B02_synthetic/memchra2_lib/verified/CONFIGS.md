# CONFIGS.md — Configuration-surface table (Phase A / Phase B)

The mirror of `ERRORS.md` for **valid** inputs. Axes are derived mechanically
from the `if` / comparison branches the C code actually takes, not from a guess
about which inputs "matter".

## Public entry points

`c_src/include/lib.h` declares exactly one:

```c
int memchra2(int a, int b, int c, int d);
```

There are no convenience-vs-low-level tiers to choose between: `memchra2` *is*
the lowest-level public entry point, and it is the only one. The 8 helpers
(`memchra`, `process_buffer`, `int_to_float_bits`, `process_strings`,
`safe_sum_array`, `interpret_as_int`, `count_occurrences`,
`complex_iteration`) are `static`, so they are not part of the linkable
surface — they are reachable only through `memchra2`, and every row below
drives the whole composed pipeline end to end through it.

## Runtime options / modes / flags

Grep census: the C source contains **no** `#ifdef`, **no** `#if`, **no**
`switch`, no global/`static` mutable state, no setter functions, no
environment-variable reads, and no configuration struct.

```
$ grep -c '#ifdef\|#if \|switch\|setenv\|getenv\|static [a-z_]* [a-z_]* =' c_src/src/lib.c
0
```

`memchra2` is therefore a pure function of its four `int` arguments, and the
"configuration" axes are exactly the input **shapes** the body branches on.
There is likewise no `[features]` table in `translation/Cargo.toml`, so there is
one build configuration.

## Axes the C actually branches on

**Axis 1 — IEEE-754 class of `a`** (from `int_to_float_bits(a)` and the
`if (f > 0.0f && f < 1000.0f)` test). Seven classes the branch distinguishes:

| class | `a` (as `uint32`) | `f` | branch | contribution |
|-------|-------------------|-----|--------|--------------|
| `Zero` | `0x00000000` | `+0.0` | not taken | 0 |
| `PosSubnormal` | `0x00000001` … `0x007FFFFF` | `0 < f < 2^-126` | taken | `(int)f == 0` |
| `PosNormLtOne` | `0x00800000` … `0x3F7FFFFF` | `2^-126 <= f < 1.0` | taken | `(int)f == 0` |
| `PosNormInRange` | `0x3F800000` … `0x4479FFFF` | `1.0 <= f < 1000.0` | taken | `(int)f` ∈ `[1, 999]` |
| `PosGeThousand` | `0x447A0000` … `0x7F7FFFFF` | `f >= 1000.0` | not taken | 0 |
| `PosInfNan` | `0x7F800000` … `0x7FFFFFFF` | `+inf` / `+NaN` | not taken | 0 |
| `Negative` | `0x80000000` … `0xFFFFFFFF` | `-0.0`, negative, `-inf`, `-NaN` | not taken | 0 |

**Axis 2 — sign pattern of `(b, c, d)`** (8 combinations). Each negative value
makes `snprintf` emit an extra `'-'`, which changes `count_occurrences(buffer,
'-')` (the `dash_count * 10` term), the buffer length, and hence the
`process_buffer` byte sum. Note `a`'s sign is already pinned by Axis 1, and
also contributes a `'-'`; the total dash count ranges over `3..=7`.

**Axis 3 — low-byte shape of `(b, c, d)`** (the `x & 0xFF` extractions feeding
`interpret_as_int`'s little-endian load and `complex_iteration`'s XOR fold).
Distinguished sub-shapes: all-zero low bytes, all-`0xFF` low bytes, low bytes
that XOR-cancel in pairs, low bytes crossing the `char` sign boundary
(`0x7F`/`0x80`), and unconstrained/random.

**Axis 4 — magnitude shape** (decimal width of the `%d` conversions, driving
buffer length ∈ `11..=51`, and `int` overflow of `a+b+c+d` in
`safe_sum_array`): single-digit values, 10-digit values, `INT_MIN`/`INT_MAX`
mixes, and sum-overflowing tuples.

Rows below are the cross product of Axes 1 × 2 (56 rows — the code takes a
genuinely different path for each), pruned of nothing, plus 12 targeted rows
covering Axes 3 and 4 which are value-dependent rather than branch-dependent.

Each row is exercised with **400 randomized inputs** drawn from that row's
class (fixed seed `0x5EED_0000 + row`, so runs are reproducible) **plus** the
row's exhaustive boundary representatives. A row is checked off only after all
of its inputs match byte-for-byte between the C `.so` and the Rust `.so`.

## Rows 1–56 — Axis 1 × Axis 2 (IEEE-754 class of `a` × sign pattern of `b,c,d`)

| # | entry point(s) | configuration (options set + input shape) | [ ] |
|---|----------------|--------------------------------------------|-----|
| 1 | `memchra2` | `Zero`: a == 0x00000000 (f=+0.0, float branch NOT taken); sign(b,c,d)=`+++` → dash_count=3, dash term=30 | [x] |
| 2 | `memchra2` | `Zero`: a == 0x00000000 (f=+0.0, float branch NOT taken); sign(b,c,d)=`++-` → dash_count=4, dash term=40 | [x] |
| 3 | `memchra2` | `Zero`: a == 0x00000000 (f=+0.0, float branch NOT taken); sign(b,c,d)=`+-+` → dash_count=4, dash term=40 | [x] |
| 4 | `memchra2` | `Zero`: a == 0x00000000 (f=+0.0, float branch NOT taken); sign(b,c,d)=`+--` → dash_count=5, dash term=50 | [x] |
| 5 | `memchra2` | `Zero`: a == 0x00000000 (f=+0.0, float branch NOT taken); sign(b,c,d)=`-++` → dash_count=4, dash term=40 | [x] |
| 6 | `memchra2` | `Zero`: a == 0x00000000 (f=+0.0, float branch NOT taken); sign(b,c,d)=`-+-` → dash_count=5, dash term=50 | [x] |
| 7 | `memchra2` | `Zero`: a == 0x00000000 (f=+0.0, float branch NOT taken); sign(b,c,d)=`--+` → dash_count=5, dash term=50 | [x] |
| 8 | `memchra2` | `Zero`: a == 0x00000000 (f=+0.0, float branch NOT taken); sign(b,c,d)=`---` → dash_count=6, dash term=60 | [x] |
| 9 | `memchra2` | `PosSubnormal`: a in [0x00000001,0x007FFFFF] (positive subnormal, branch taken, (int)f=0); sign(b,c,d)=`+++` → dash_count=3, dash term=30 | [x] |
| 10 | `memchra2` | `PosSubnormal`: a in [0x00000001,0x007FFFFF] (positive subnormal, branch taken, (int)f=0); sign(b,c,d)=`++-` → dash_count=4, dash term=40 | [x] |
| 11 | `memchra2` | `PosSubnormal`: a in [0x00000001,0x007FFFFF] (positive subnormal, branch taken, (int)f=0); sign(b,c,d)=`+-+` → dash_count=4, dash term=40 | [x] |
| 12 | `memchra2` | `PosSubnormal`: a in [0x00000001,0x007FFFFF] (positive subnormal, branch taken, (int)f=0); sign(b,c,d)=`+--` → dash_count=5, dash term=50 | [x] |
| 13 | `memchra2` | `PosSubnormal`: a in [0x00000001,0x007FFFFF] (positive subnormal, branch taken, (int)f=0); sign(b,c,d)=`-++` → dash_count=4, dash term=40 | [x] |
| 14 | `memchra2` | `PosSubnormal`: a in [0x00000001,0x007FFFFF] (positive subnormal, branch taken, (int)f=0); sign(b,c,d)=`-+-` → dash_count=5, dash term=50 | [x] |
| 15 | `memchra2` | `PosSubnormal`: a in [0x00000001,0x007FFFFF] (positive subnormal, branch taken, (int)f=0); sign(b,c,d)=`--+` → dash_count=5, dash term=50 | [x] |
| 16 | `memchra2` | `PosSubnormal`: a in [0x00000001,0x007FFFFF] (positive subnormal, branch taken, (int)f=0); sign(b,c,d)=`---` → dash_count=6, dash term=60 | [x] |
| 17 | `memchra2` | `PosNormLtOne`: a in [0x00800000,0x3F7FFFFF] (0<f<1.0, branch taken, (int)f=0); sign(b,c,d)=`+++` → dash_count=3, dash term=30 | [x] |
| 18 | `memchra2` | `PosNormLtOne`: a in [0x00800000,0x3F7FFFFF] (0<f<1.0, branch taken, (int)f=0); sign(b,c,d)=`++-` → dash_count=4, dash term=40 | [x] |
| 19 | `memchra2` | `PosNormLtOne`: a in [0x00800000,0x3F7FFFFF] (0<f<1.0, branch taken, (int)f=0); sign(b,c,d)=`+-+` → dash_count=4, dash term=40 | [x] |
| 20 | `memchra2` | `PosNormLtOne`: a in [0x00800000,0x3F7FFFFF] (0<f<1.0, branch taken, (int)f=0); sign(b,c,d)=`+--` → dash_count=5, dash term=50 | [x] |
| 21 | `memchra2` | `PosNormLtOne`: a in [0x00800000,0x3F7FFFFF] (0<f<1.0, branch taken, (int)f=0); sign(b,c,d)=`-++` → dash_count=4, dash term=40 | [x] |
| 22 | `memchra2` | `PosNormLtOne`: a in [0x00800000,0x3F7FFFFF] (0<f<1.0, branch taken, (int)f=0); sign(b,c,d)=`-+-` → dash_count=5, dash term=50 | [x] |
| 23 | `memchra2` | `PosNormLtOne`: a in [0x00800000,0x3F7FFFFF] (0<f<1.0, branch taken, (int)f=0); sign(b,c,d)=`--+` → dash_count=5, dash term=50 | [x] |
| 24 | `memchra2` | `PosNormLtOne`: a in [0x00800000,0x3F7FFFFF] (0<f<1.0, branch taken, (int)f=0); sign(b,c,d)=`---` → dash_count=6, dash term=60 | [x] |
| 25 | `memchra2` | `PosNormInRange`: a in [0x3F800000,0x4479FFFF] (1.0<=f<1000.0, branch taken, (int)f in [1,999]); sign(b,c,d)=`+++` → dash_count=3, dash term=30 | [x] |
| 26 | `memchra2` | `PosNormInRange`: a in [0x3F800000,0x4479FFFF] (1.0<=f<1000.0, branch taken, (int)f in [1,999]); sign(b,c,d)=`++-` → dash_count=4, dash term=40 | [x] |
| 27 | `memchra2` | `PosNormInRange`: a in [0x3F800000,0x4479FFFF] (1.0<=f<1000.0, branch taken, (int)f in [1,999]); sign(b,c,d)=`+-+` → dash_count=4, dash term=40 | [x] |
| 28 | `memchra2` | `PosNormInRange`: a in [0x3F800000,0x4479FFFF] (1.0<=f<1000.0, branch taken, (int)f in [1,999]); sign(b,c,d)=`+--` → dash_count=5, dash term=50 | [x] |
| 29 | `memchra2` | `PosNormInRange`: a in [0x3F800000,0x4479FFFF] (1.0<=f<1000.0, branch taken, (int)f in [1,999]); sign(b,c,d)=`-++` → dash_count=4, dash term=40 | [x] |
| 30 | `memchra2` | `PosNormInRange`: a in [0x3F800000,0x4479FFFF] (1.0<=f<1000.0, branch taken, (int)f in [1,999]); sign(b,c,d)=`-+-` → dash_count=5, dash term=50 | [x] |
| 31 | `memchra2` | `PosNormInRange`: a in [0x3F800000,0x4479FFFF] (1.0<=f<1000.0, branch taken, (int)f in [1,999]); sign(b,c,d)=`--+` → dash_count=5, dash term=50 | [x] |
| 32 | `memchra2` | `PosNormInRange`: a in [0x3F800000,0x4479FFFF] (1.0<=f<1000.0, branch taken, (int)f in [1,999]); sign(b,c,d)=`---` → dash_count=6, dash term=60 | [x] |
| 33 | `memchra2` | `PosGeThousand`: a in [0x447A0000,0x7F7FFFFF] (f>=1000.0, branch NOT taken); sign(b,c,d)=`+++` → dash_count=3, dash term=30 | [x] |
| 34 | `memchra2` | `PosGeThousand`: a in [0x447A0000,0x7F7FFFFF] (f>=1000.0, branch NOT taken); sign(b,c,d)=`++-` → dash_count=4, dash term=40 | [x] |
| 35 | `memchra2` | `PosGeThousand`: a in [0x447A0000,0x7F7FFFFF] (f>=1000.0, branch NOT taken); sign(b,c,d)=`+-+` → dash_count=4, dash term=40 | [x] |
| 36 | `memchra2` | `PosGeThousand`: a in [0x447A0000,0x7F7FFFFF] (f>=1000.0, branch NOT taken); sign(b,c,d)=`+--` → dash_count=5, dash term=50 | [x] |
| 37 | `memchra2` | `PosGeThousand`: a in [0x447A0000,0x7F7FFFFF] (f>=1000.0, branch NOT taken); sign(b,c,d)=`-++` → dash_count=4, dash term=40 | [x] |
| 38 | `memchra2` | `PosGeThousand`: a in [0x447A0000,0x7F7FFFFF] (f>=1000.0, branch NOT taken); sign(b,c,d)=`-+-` → dash_count=5, dash term=50 | [x] |
| 39 | `memchra2` | `PosGeThousand`: a in [0x447A0000,0x7F7FFFFF] (f>=1000.0, branch NOT taken); sign(b,c,d)=`--+` → dash_count=5, dash term=50 | [x] |
| 40 | `memchra2` | `PosGeThousand`: a in [0x447A0000,0x7F7FFFFF] (f>=1000.0, branch NOT taken); sign(b,c,d)=`---` → dash_count=6, dash term=60 | [x] |
| 41 | `memchra2` | `PosInfNan`: a in [0x7F800000,0x7FFFFFFF] (+inf/+NaN, branch NOT taken); sign(b,c,d)=`+++` → dash_count=3, dash term=30 | [x] |
| 42 | `memchra2` | `PosInfNan`: a in [0x7F800000,0x7FFFFFFF] (+inf/+NaN, branch NOT taken); sign(b,c,d)=`++-` → dash_count=4, dash term=40 | [x] |
| 43 | `memchra2` | `PosInfNan`: a in [0x7F800000,0x7FFFFFFF] (+inf/+NaN, branch NOT taken); sign(b,c,d)=`+-+` → dash_count=4, dash term=40 | [x] |
| 44 | `memchra2` | `PosInfNan`: a in [0x7F800000,0x7FFFFFFF] (+inf/+NaN, branch NOT taken); sign(b,c,d)=`+--` → dash_count=5, dash term=50 | [x] |
| 45 | `memchra2` | `PosInfNan`: a in [0x7F800000,0x7FFFFFFF] (+inf/+NaN, branch NOT taken); sign(b,c,d)=`-++` → dash_count=4, dash term=40 | [x] |
| 46 | `memchra2` | `PosInfNan`: a in [0x7F800000,0x7FFFFFFF] (+inf/+NaN, branch NOT taken); sign(b,c,d)=`-+-` → dash_count=5, dash term=50 | [x] |
| 47 | `memchra2` | `PosInfNan`: a in [0x7F800000,0x7FFFFFFF] (+inf/+NaN, branch NOT taken); sign(b,c,d)=`--+` → dash_count=5, dash term=50 | [x] |
| 48 | `memchra2` | `PosInfNan`: a in [0x7F800000,0x7FFFFFFF] (+inf/+NaN, branch NOT taken); sign(b,c,d)=`---` → dash_count=6, dash term=60 | [x] |
| 49 | `memchra2` | `Negative`: a in [0x80000000,0xFFFFFFFF] (negative/-0.0/-inf/-NaN, branch NOT taken); sign(b,c,d)=`+++` → dash_count=4, dash term=40 | [x] |
| 50 | `memchra2` | `Negative`: a in [0x80000000,0xFFFFFFFF] (negative/-0.0/-inf/-NaN, branch NOT taken); sign(b,c,d)=`++-` → dash_count=5, dash term=50 | [x] |
| 51 | `memchra2` | `Negative`: a in [0x80000000,0xFFFFFFFF] (negative/-0.0/-inf/-NaN, branch NOT taken); sign(b,c,d)=`+-+` → dash_count=5, dash term=50 | [x] |
| 52 | `memchra2` | `Negative`: a in [0x80000000,0xFFFFFFFF] (negative/-0.0/-inf/-NaN, branch NOT taken); sign(b,c,d)=`+--` → dash_count=6, dash term=60 | [x] |
| 53 | `memchra2` | `Negative`: a in [0x80000000,0xFFFFFFFF] (negative/-0.0/-inf/-NaN, branch NOT taken); sign(b,c,d)=`-++` → dash_count=5, dash term=50 | [x] |
| 54 | `memchra2` | `Negative`: a in [0x80000000,0xFFFFFFFF] (negative/-0.0/-inf/-NaN, branch NOT taken); sign(b,c,d)=`-+-` → dash_count=6, dash term=60 | [x] |
| 55 | `memchra2` | `Negative`: a in [0x80000000,0xFFFFFFFF] (negative/-0.0/-inf/-NaN, branch NOT taken); sign(b,c,d)=`--+` → dash_count=6, dash term=60 | [x] |
| 56 | `memchra2` | `Negative`: a in [0x80000000,0xFFFFFFFF] (negative/-0.0/-inf/-NaN, branch NOT taken); sign(b,c,d)=`---` → dash_count=7, dash term=70 | [x] |

## Rows 57–68 — Axis 3 (low-byte shape) and Axis 4 (magnitude shape)

| # | entry point(s) | configuration (options set + input shape) | [ ] |
|---|----------------|--------------------------------------------|-----|
| 57 | `memchra2` | Axis 3: low bytes of b,c,d all `0x00` → `interpret_as_int` loads `0x00000000`, `complex_iteration` XOR-folds `a&0xFF` only | [x] |
| 58 | `memchra2` | Axis 3: low bytes of b,c,d all `0xFF` → `interpret_as_int` loads `0x00FFFFFF` | [x] |
| 59 | `memchra2` | Axis 3: low bytes of b,c,d pairwise equal (XOR cancellation in `complex_iteration`) | [x] |
| 60 | `memchra2` | Axis 3: low bytes of b,c,d at the `char` sign boundary `0x7F`/`0x80` (signed-vs-unsigned `(char)c` behaviour) | [x] |
| 61 | `memchra2` | Axis 3: low byte of `a` sweeps all 256 values with b,c,d fixed (isolates `complex_iteration`'s `a` term) | [x] |
| 62 | `memchra2` | Axis 3: low bytes of b,c,d sweep all 256 values each (16.7M-pruned: 256 diagonal + 4096 random pairs) | [x] |
| 63 | `memchra2` | Axis 4: all four args single-digit `0..9` → shortest buffer (11 bytes), exhaustive 10^4 cross product | [x] |
| 64 | `memchra2` | Axis 4: all four args = `INT_MIN` → longest buffer (51 bytes), max dash count 7 | [x] |
| 65 | `memchra2` | Axis 4: all four args = `INT_MAX` → 10-digit widths, dash count 3 | [x] |
| 66 | `memchra2` | Axis 4: `a+b+c+d` overflows `int` positively (wrap-around in `safe_sum_array`) | [x] |
| 67 | `memchra2` | Axis 4: `a+b+c+d` overflows `int` negatively (wrap-around in `safe_sum_array`) | [x] |
| 68 | `memchra2` | Axis 1+4: `a` chosen so `(int)f` hits every integer in `[1,999]` (all 999 values, 3 ulp probes each) | [x] |

**68 / 68 rows pass** across their randomized inputs and boundary
representatives; see `tests/configs.rs`.

## Robustness: C optimization level

`c_src/CMakeLists.txt` sets no `CMAKE_BUILD_TYPE`, so the reference `.so` is
built at `-O0`. This library relies on signed-integer overflow wrapping
(`a+b+c+d` in `safe_sum_array`) and on type punning through a union
(`int_to_float_bits`), both of which a compiler is entitled to treat
differently at higher optimization levels. To confirm the Rust matches the C's
*semantics* and not merely one build of it, the whole suite was re-run against
`c_src/src/lib.c` compiled out-of-tree (nothing in `c_src/` was modified) at
`-O0`, `-O1`, `-O2`, `-O3`, and `-Os`:

| C optimization | tests passed | failed |
|----------------|--------------|--------|
| `-O0` (the CMake default, i.e. the reference build) | 46 | 0 |
| `-O1` | 46 | 0 |
| `-O2` | 46 | 0 |
| `-O3` | 46 | 0 |
| `-Os` | 46 | 0 |

Reproduce with `MEMCHRA2_C_SO=<path to alternative .so> cargo test --release`.
