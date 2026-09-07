# CONFIGS.md — Configuration surface table (valid inputs)

Mechanically derived from the branches the C actually takes.

## Axes the C code branches on

**Runtime options / modes.** The library exposes **no** option struct, no
setter, no global flag, no `#ifdef`-selected mode, and no environment lookup.
`grep -n "#if\|#ifdef\|static \|extern \|getenv" c_src/src/lib.c` → only the
`#include`s. Therefore the *only* configuration axes are the **input shapes** of
the five entry points.

**Public entry points (all five, lowest-level first — not just the `lib.h`
wrapper `overunder`):**

1. `safe_double_to_int(double)` — leaf
2. `process_with_fallthrough(int, int)` — leaf
3. `copy_data_block(DataBlock*, const DataBlock*)` — leaf
4. `handle_pointer_operations(int)` — leaf
5. `overunder(int, int, int, int)` — composes 1, 2, 3, 4 and does all the I/O

**Input-shape axes the code distinguishes:**

| axis | distinct values the C treats differently | source |
|---|---|---|
| `safe_double_to_int` range class | `> INT_MAX` / `< INT_MIN` / `NaN` / in-range | `lib.c:40-47` |
| `safe_double_to_int` magnitude class | zero, ±0.0, subnormal, `<1` fractional, small, large, exact boundary, ±inf | cast at `lib.c:47` |
| `process_with_fallthrough` `code` | `5`, `4`, `3`, `2`, `1` (each a distinct fall-through depth), `0`, `default` — **7** distinct paths | `lib.c:54-72` |
| `process_with_fallthrough` `base_value` | negative / zero / positive / near `INT_MIN` / near `INT_MAX` (overflow in `+=`) | `lib.c:56-64` |
| `DataBlock` content | `id` sign, `value` normal/NaN/inf, `label` NUL-terminated / unterminated / all-`0xFF`, padding bytes | `lib.c:33-37,78` |
| `handle_pointer_operations` `value` | negative / zero / positive / `*2` overflowing / `+100` overflowing | `lib.c:81-90` |
| `overunder` `a` residue `a % 6` | `0,1,2,3,4,5` and the negative residues `-1,-2,-3,-4,-5` → selects which of the 7 switch paths runs | `lib.c:115` |
| `overunder` `a,b,c,d` magnitude | small / large-positive / large-negative / `INT_MIN` / `INT_MAX` / `0` — drives clamping and `d*d+a*a` overflow | `lib.c:103-106` |
| `overunder` `d*d + a*a` sign | non-overflowing positive vs. overflow-to-negative → `sqrt(neg) = NaN` | `lib.c:106` |
| `overunder` stdout | 6 `printf` sites + a 5-iteration loop, incl. `%.2f` of `a*1.5` and `%s` of the copied label | `lib.c:100-154` |

Every row below is compared on **both** the `int` return value **and** the
byte-exact stdout captured from fd 1 (rows for `overunder`), with **many
randomized inputs per row** (fixed seed `0x5EED_...`, SplitMix64) rather than
one hand-picked value.

## Configuration table

| # | entry point(s) | configuration (options set + input shape) | ✔ |
|---|---|---|---|
| 1 | `safe_double_to_int` | in-range positive integral values, randomized in `[0, INT_MAX]` | [x] |
| 2 | `safe_double_to_int` | in-range negative integral values, randomized in `[INT_MIN, 0]` | [x] |
| 3 | `safe_double_to_int` | in-range fractional values (truncation toward zero), randomized `±[0,1)` and `±[1, 1e9)` | [x] |
| 4 | `safe_double_to_int` | magnitude `< 1`: `±0.0`, subnormals, `5e-324`, `f64::MIN_POSITIVE`, `0.999…` | [x] |
| 5 | `safe_double_to_int` | exact boundaries `±(double)INT_MAX`, `(double)INT_MIN`, and ±1 ULP either side | [x] |
| 6 | `safe_double_to_int` | out-of-range high: randomized in `(INT_MAX, 1e300]`, plus `+inf` | [x] |
| 7 | `safe_double_to_int` | out-of-range low: randomized in `[-1e300, INT_MIN)`, plus `-inf` | [x] |
| 8 | `safe_double_to_int` | `NaN` in all bit patterns: quiet, negative-quiet, signalling, payload-carrying (randomized mantissas) | [x] |
| 9 | `safe_double_to_int` | fully randomized `f64` from random 64-bit patterns (hits every class incl. inf/NaN/subnormal simultaneously) | [x] |
| 10 | `process_with_fallthrough` | `code = 5` (deepest fall-through, +150), randomized `base_value` incl. extremes | [x] |
| 11 | `process_with_fallthrough` | `code = 4` (+130), randomized `base_value` | [x] |
| 12 | `process_with_fallthrough` | `code = 3` (+90), randomized `base_value` | [x] |
| 13 | `process_with_fallthrough` | `code = 2` (+30), randomized `base_value` | [x] |
| 14 | `process_with_fallthrough` | `code = 1` (+10, `break`), randomized `base_value` | [x] |
| 15 | `process_with_fallthrough` | `code = 0` (`result = 0`, discards `base_value`), randomized `base_value` | [x] |
| 16 | `process_with_fallthrough` | `code` in `default` (randomized over all of `int` excluding `0..=5`), randomized `base_value` | [x] |
| 17 | `process_with_fallthrough` | full cross-product `code ∈ {-2..8} ∪ {INT_MIN, INT_MAX}` × `base_value ∈ {INT_MIN, -1, 0, 1, INT_MAX, INT_MAX-149, INT_MIN+149}` | [x] |
| 18 | `copy_data_block` | randomized well-formed blocks: random `id`, random finite `value`, NUL-terminated `label`; compare all 40 bytes of `dest` | [x] |
| 19 | `copy_data_block` | `value` = `NaN` / `±inf` / `±0.0` / subnormal, `id` at `INT_MIN`/`INT_MAX` | [x] |
| 20 | `copy_data_block` | `label` fully occupied with no NUL (20 non-zero bytes), plus all-`0xFF` and all-`0x00` blocks | [x] |
| 21 | `copy_data_block` | source built from a fully random 40-byte pattern (padding bytes non-zero) — verifies `memcpy` copies padding verbatim | [x] |
| 22 | `copy_data_block` | pre-filled destination (`0xAA` everywhere) to prove the copy overwrites all 40 bytes, not just the named fields | [x] |
| 23 | `handle_pointer_operations` | randomized small `value` (no overflow) | [x] |
| 24 | `handle_pointer_operations` | randomized full-range `int` `value` (`*2` and `+100` overflow paths) | [x] |
| 25 | `handle_pointer_operations` | boundary `value ∈ {INT_MIN, INT_MIN+1, -1, 0, 1, INT_MAX/2, INT_MAX/2+1, INT_MAX}` | [x] |
| 26 | `overunder` | `a % 6 == 5` (switch path 5) — randomized `b,c,d` in a small range; return + stdout | [x] |
| 27 | `overunder` | `a % 6 == 4` — randomized `b,c,d`; return + stdout | [x] |
| 28 | `overunder` | `a % 6 == 3` — randomized `b,c,d`; return + stdout | [x] |
| 29 | `overunder` | `a % 6 == 2` — randomized `b,c,d`; return + stdout | [x] |
| 30 | `overunder` | `a % 6 == 1` — randomized `b,c,d`; return + stdout | [x] |
| 31 | `overunder` | `a % 6 == 0` (switch `result = 0`) — randomized `b,c,d`; return + stdout | [x] |
| 32 | `overunder` | `a % 6` negative (`a < 0`, non-multiple of 6) → `default`, `switch_result == -1`; randomized | [x] |
| 33 | `overunder` | all-zero input `(0,0,0,0)` — the degenerate shape (`sqrt(0)`, `0/3.3`, `%.2f` of `0.00`) | [x] |
| 34 | `overunder` | small magnitudes only: randomized `a,b,c,d ∈ [-100, 100]` (no overflow anywhere) | [x] |
| 35 | `overunder` | medium magnitudes: randomized `a,b,c,d ∈ [-46340, 46340]` so `d*d+a*a` stays non-negative → real `sqrt` | [x] |
| 36 | `overunder` | large magnitudes: randomized `a,b,c,d ∈ [-2³¹, 2³¹)` → `d*d+a*a` overflow, `sqrt(NaN)`, `conv1/conv2` clamping, `total` wrap | [x] |
| 37 | `overunder` | `a` extreme, `b,c,d` random (`a ∈ {INT_MIN, INT_MAX}`) — drives `conv1`, `conv4`, `a%6`, `array1[4] = a+b` wrap simultaneously | [x] |
| 38 | `overunder` | `b` extreme (`b ∈ {INT_MIN, INT_MAX}`), rest random — drives `conv2` clamp and `a+b` wrap | [x] |
| 39 | `overunder` | `c` extreme (`c ∈ {INT_MIN, INT_MAX}`), rest random — drives `conv3` (`c/3.3`) and `handle_pointer_operations(c)` overflow | [x] |
| 40 | `overunder` | `d` extreme (`d ∈ {INT_MIN, INT_MAX}`), rest random — drives `d*d` overflow only | [x] |
| 41 | `overunder` | full corner cross-product `{INT_MIN, -1, 0, 1, INT_MAX}⁴` = 625 cases; return + stdout each | [x] |
| 42 | `overunder` | powers of two and their negations `{±2ᵏ}` for `k ∈ 0..31` swept through each of the four parameters | [x] |
| 43 | `overunder` | values chosen so `a*1.5` and `b*2.7` land exactly on `.5` / `.0` boundaries (rounding-sensitive `%.2f` and truncation) | [x] |
| 44 | `overunder` | high-volume randomized fuzz: 20 000 fully random `(a,b,c,d)` quadruples, return value compared (stdout compared for a sampled subset) | [x] |
| 45 | composed pipeline | `overunder`'s own results cross-checked against direct low-level calls to `safe_double_to_int` / `process_with_fallthrough` / `handle_pointer_operations` on **both** libraries, so the composition — not just each wrapper — is verified | [x] |

## Feature combinations

`translation/Cargo.toml` has **no `[features]` table**, so the default build is
the only build; `--no-default-features` is byte-identical to it. The test suite
is nevertheless run under both invocations by `run_all.sh`.
