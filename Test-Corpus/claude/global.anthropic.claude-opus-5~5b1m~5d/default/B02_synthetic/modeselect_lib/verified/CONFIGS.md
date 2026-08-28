# CONFIGS.md — Phase A: configuration surface table (valid inputs)

Mechanically derived from the branches `c_src/src/lib.c` actually takes.

## Axes the C code branches on

| axis | where | distinct values the C distinguishes |
|---|---|---|
| mode string | `classify_mode`, `lib.c:30-37` | `"standard"`, `"enhanced"`, `"turbo"`, `"extreme"`, anything else (5) |
| `level` | `apply_multiplier` `switch`, `lib.c:45-60` | `4`, `3`, `2`, `1`, `0`, other (6 fall-through arms) |
| `base` magnitude | `apply_multiplier` accumulator | small / near `INT_MAX` (overflow) / near `INT_MIN` |
| `factor` magnitude | `convert_time_factor`, `×1e12` then `(int)` | in-range / boundary / out-of-range / non-finite |
| `value` magnitude | `convert_negative_overflow`, `×-1e15` then `(int)` | in-range / boundary / out-of-range / non-finite |
| `offset_days`,`offset_hours` | `get_modified_time`, `int` products | non-overflowing / overflowing, each sign |
| `time_t` byte pattern | `hash_time_value`, per-byte `<<(i%4)*8` | 8 byte lanes, high-bit set vs clear, sign of `t` |
| **wall clock** `time(NULL)` | `get_modified_time`, `lib.c:80-81` | `current >> 29` — sign of the clock (arithmetic vs logical shift), the `2^29` multiples, and the `time_t` extremes.  Driven with an `LD_PRELOAD` interposer, see rows C64–C69. |
| `mode_selector % 4` | `modeselect`, array index | `0`,`1`,`2`,`3` (and negative → Phase C) |
| `complexity % 5` | `modeselect` → `apply_multiplier` | `0`,`1`,`2`,`3`,`4` (and negative → Phase C) |
| `seed % 24` | `modeselect` → `get_modified_time(offset_hours)` | `0`, positive, negative |
| `time_offset` | `modeselect` → `get_modified_time(offset_days)` + `factor2` | `0`, ±small, ±overflowing |
| `seed` | `modeselect` → `factor1` | `0` (→ `result1 == 0`) vs non-zero (→ `INT_MIN`) |
| stdout | 8 `printf` calls in `modeselect` | `%s`, `%X`, `%d`, `%ld`, `%.2e` formatting all compared byte-for-byte |

There are **no compile-time `#ifdef`s, no runtime option/flag setters, no
opaque context object and no global state** in `lib.c`; the only "configuration"
is the argument tuple of each entry point.  All 7 public entry points are
exercised directly below, including the six low-level ones that `modeselect`
composes (`classify_mode`, `apply_multiplier`, `convert_time_factor`,
`convert_negative_overflow`, `get_modified_time`, `hash_time_value`) — not just
the `modeselect` convenience wrapper declared in `include/lib.h`.

Every row is driven with **many randomized inputs** from a fixed-seed
SplitMix64 PRNG (`SEED = 0x9E3779B97F4A7C15`), not a single hand-picked value.

## Table

| #  | entry point(s) | configuration (options set + input shape) | [x] |
|----|----------------|--------------------------------------------|-----|
| C1  | `classify_mode` | exact `"standard"` | [x] |
| C2  | `classify_mode` | exact `"enhanced"` | [x] |
| C3  | `classify_mode` | exact `"turbo"` | [x] |
| C4  | `classify_mode` | exact `"extreme"` | [x] |
| C5  | `classify_mode` | randomized unknown ASCII strings, length 1..64 (2000 samples) | [x] |
| C6  | `classify_mode` | randomized unknown strings over full byte range 0x01..0xFF, length 1..64 (2000 samples) | [x] |
| C7  | `classify_mode` | randomized 1-byte mutations of the 4 valid modes (insert/replace/truncate/extend, 2000 samples) | [x] |
| C8  | `classify_mode` | long strings (256, 1024, 4096 bytes) of random bytes | [x] |
| C9  | `apply_multiplier` | `level = 0`, randomized `base` over the full `i32` range (2000 samples) | [x] |
| C10 | `apply_multiplier` | `level = 1`, randomized `base` over the full `i32` range | [x] |
| C11 | `apply_multiplier` | `level = 2`, randomized `base` over the full `i32` range | [x] |
| C12 | `apply_multiplier` | `level = 3`, randomized `base` over the full `i32` range | [x] |
| C13 | `apply_multiplier` | `level = 4`, randomized `base` over the full `i32` range | [x] |
| C14 | `apply_multiplier` | `base = 0xA0` (the literal `modeselect` uses) × `level ∈ 0..=4` | [x] |
| C15 | `apply_multiplier` | `base ∈ {0, 1, -1, INT_MAX, INT_MIN, INT_MAX-0x300, INT_MIN+0x300}` × `level ∈ 0..=4` (overflow corners) | [x] |
| C16 | `convert_time_factor` | in-range: random `factor` with `|factor| ≤ 2.147e-3` so the `×1e12` product fits `int` (2000 samples) | [x] |
| C17 | `convert_time_factor` | boundary sweep: `factor = k/1e12` for `k` within ±4 ULP of `INT_MAX`/`INT_MIN`, plus `±2147483647e-12`, `±2147483648e-12` | [x] |
| C18 | `convert_time_factor` | `0.0`, `-0.0`, `f64::MIN_POSITIVE`, subnormal (`5e-324`), `1e-300` | [x] |
| C19 | `convert_time_factor` | randomized full-range `f64` bit patterns (finite, 2000 samples) — mixes in-range and out-of-range | [x] |
| C20 | `convert_negative_overflow` | in-range: random `value` with `|value| ≤ 2.147e-6` so the `×-1e15` product fits `int` (2000 samples) | [x] |
| C21 | `convert_negative_overflow` | boundary sweep around `∓2147483647e-15` / `∓2147483648e-15` (note the sign flip from `-1e15`) | [x] |
| C22 | `convert_negative_overflow` | `0.0`, `-0.0`, `f64::MIN_POSITIVE`, subnormal, `1e-300` | [x] |
| C23 | `convert_negative_overflow` | randomized full-range `f64` bit patterns (finite, 2000 samples) | [x] |
| C24 | `get_modified_time` | `(0, 0)` | [x] |
| C25 | `get_modified_time` | randomized non-overflowing `offset_days ∈ ±24855`, `offset_hours ∈ ±596523` such that the sum fits `int` (2000 samples) | [x] |
| C26 | `get_modified_time` | randomized negative-only offsets in the non-overflowing range | [x] |
| C27 | `get_modified_time` | randomized `offset_days` over the full `i32` range, `offset_hours = 0` (product overflow) | [x] |
| C28 | `get_modified_time` | `offset_days = 0`, randomized `offset_hours` over the full `i32` range (product overflow) | [x] |
| C29 | `get_modified_time` | both randomized over the full `i32` range (product **and** sum overflow, 4000 samples) | [x] |
| C30 | `get_modified_time` | corners: `{0, ±1, 24855, 24856, -24855, -24856, INT_MAX, INT_MIN}²` | [x] |
| C31 | `hash_time_value` | `t ∈ {0, 1, -1, 2, -2, i64::MIN, i64::MAX, 0x5A5A5A5A5A5A5A5A}` | [x] |
| C32 | `hash_time_value` | randomized full-range `i64` (4000 samples) — exercises all 8 byte lanes incl. high-bit-set bytes | [x] |
| C33 | `hash_time_value` | single-byte walks: `t = 0xNN << (8*k)` for every byte value `NN ∈ {0x01,0x7F,0x80,0xFF}` and every lane `k ∈ 0..8` | [x] |
| C34 | `hash_time_value` | plausible `time_t` values: `now`, `now>>29`, `now ± random`, `2^29·k` boundaries | [x] |
| C35 | `get_modified_time` → `hash_time_value` | **composed pipeline**: feed the output of C25/C29 straight into `hash_time_value` and compare (2000 samples) | [x] |
| C36 | `classify_mode` ∘ `apply_multiplier` ∘ `get_modified_time` ∘ `hash_time_value` | **hand-composed re-implementation** of `modeselect`'s body from the low-level exports of *one* library, cross-checked against the other library's `modeselect` return value | [x] |
| C37 | `modeselect` | `mode_selector%4 = 0`, `complexity%5 = 0`, randomized `time_offset`, `seed` (return value **and** full stdout) | [x] |
| C38 | `modeselect` | `mode_selector%4 = 0`, `complexity%5 = 1`, randomized `time_offset`, `seed` | [x] |
| C39 | `modeselect` | `mode_selector%4 = 0`, `complexity%5 = 2`, randomized `time_offset`, `seed` | [x] |
| C40 | `modeselect` | `mode_selector%4 = 0`, `complexity%5 = 3`, randomized `time_offset`, `seed` | [x] |
| C41 | `modeselect` | `mode_selector%4 = 0`, `complexity%5 = 4`, randomized `time_offset`, `seed` | [x] |
| C42 | `modeselect` | `mode_selector%4 = 1`, `complexity%5 = 0`, randomized `time_offset`, `seed` | [x] |
| C43 | `modeselect` | `mode_selector%4 = 1`, `complexity%5 = 1`, randomized `time_offset`, `seed` | [x] |
| C44 | `modeselect` | `mode_selector%4 = 1`, `complexity%5 = 2`, randomized `time_offset`, `seed` | [x] |
| C45 | `modeselect` | `mode_selector%4 = 1`, `complexity%5 = 3`, randomized `time_offset`, `seed` | [x] |
| C46 | `modeselect` | `mode_selector%4 = 1`, `complexity%5 = 4`, randomized `time_offset`, `seed` | [x] |
| C47 | `modeselect` | `mode_selector%4 = 2`, `complexity%5 = 0`, randomized `time_offset`, `seed` | [x] |
| C48 | `modeselect` | `mode_selector%4 = 2`, `complexity%5 = 1`, randomized `time_offset`, `seed` | [x] |
| C49 | `modeselect` | `mode_selector%4 = 2`, `complexity%5 = 2`, randomized `time_offset`, `seed` | [x] |
| C50 | `modeselect` | `mode_selector%4 = 2`, `complexity%5 = 3`, randomized `time_offset`, `seed` | [x] |
| C51 | `modeselect` | `mode_selector%4 = 2`, `complexity%5 = 4`, randomized `time_offset`, `seed` | [x] |
| C52 | `modeselect` | `mode_selector%4 = 3`, `complexity%5 = 0`, randomized `time_offset`, `seed` | [x] |
| C53 | `modeselect` | `mode_selector%4 = 3`, `complexity%5 = 1`, randomized `time_offset`, `seed` | [x] |
| C54 | `modeselect` | `mode_selector%4 = 3`, `complexity%5 = 2`, randomized `time_offset`, `seed` | [x] |
| C55 | `modeselect` | `mode_selector%4 = 3`, `complexity%5 = 3`, randomized `time_offset`, `seed` | [x] |
| C56 | `modeselect` | `mode_selector%4 = 3`, `complexity%5 = 4`, randomized `time_offset`, `seed` | [x] |
| C57 | `modeselect` | shape: `seed = 0` (⇒ `factor1 = 0`, `result1 = 0`, `%.2e` prints `0.00e+00`) × all 4 mode indices | [x] |
| C58 | `modeselect` | shape: `time_offset = 0` (⇒ `factor2 = -0.0`, `result2 = 0`, `%.2e` prints `-0.00e+00`) × all 4 mode indices | [x] |
| C59 | `modeselect` | shape: `seed = 0 && time_offset = 0` (both conversions in range) | [x] |
| C60 | `modeselect` | shape: `seed` negative ⇒ `seed % 24` negative ⇒ negative `offset_hours` (2000 samples) | [x] |
| C61 | `modeselect` | shape: `time_offset` chosen to overflow `offset_days * 86400` (|v| > 24855), randomized (2000 samples) | [x] |
| C62 | `modeselect` | shape: all four arguments randomized over the **full `i32` range**, restricted to `mode_selector % 4 >= 0` (10000 samples) | [x] |
| C63 | `modeselect` | corners: `{0, ±1, ±4, ±5, 24856, INT_MAX, INT_MIN}` cross-product on all 4 arguments, filtered to `mode_selector % 4 >= 0` | [x] |

### The `time(NULL)` axis (`tests/phase_b_faketime.rs`)

`get_modified_time` reads the wall clock, and with the real clock `time(NULL) >> 29`
is the constant `3` until 2038.  That hides an entire axis, so the rows below
re-run the suite in a child process with an `LD_PRELOAD` `time()` interposer
(`examples/faketime.rs`) forcing 25 different clock values: `0, ±1, ±2,
2^29−1, 2^29, 2^29+1, −(2^29−1), −2^29, −(2^29+1), 1756000000, ±2^31, 2^32,
i64::MIN, i64::MIN+1, i64::MAX` and 6 random `i64`s.

| #  | entry point(s) | configuration (options set + input shape) | [x] |
|----|----------------|--------------------------------------------|-----|
| C64 | `get_modified_time` | 25 forced clock values × 2015 offset pairs each (15 corner pairs + 1500 full-range random + 500 realistic) — asserts the C's exact `base + (int)(d*86400 + h*3600)` wraparound | [x] |
| C65 | `get_modified_time` → `hash_time_value` | composed, for every (clock, offset) pair of C64 | [x] |
| C66 | `modeselect` | 25 forced clocks × full 4×5 cross product of mode index and complexity level × 6 `(time_offset, seed)` shapes, return value **and** stdout | [x] |
| C67 | `modeselect` | the `Modified time: %ld` field for **negative** clock bases (`%ld` of a negative `time_t`) | [x] |
| C68 | `modeselect` | negative `mode_selector` multiples of 4 (`-4`, `-8`, `INT_MIN`) under each forced clock | [x] |
| C69 | `hash_time_value` | the exact `time_t` values each forced clock produces, ±4, plus `clock` itself and `2·(clock>>29)` | [x] |

This axis is what makes the arithmetic `>> 29` observable: with clock `-536870913`
the base is `-2`, whereas a logical shift would give `34359738366`.  Both
`current >>= 29` mutants (logical shift, and shift-by-28) are detected only by
these rows.

Legend: `[x]` = passes byte-for-byte against the C `.so` across all randomized
inputs for that row.  Row → test mapping: `tests/phase_b_valid.rs` (C1–C63),
`tests/phase_b_faketime.rs` (C64–C69).

## Anti-vacuity evidence

The suite was mutation-tested: 24 single-edit mutants were injected into
`src/lib.rs`, rebuilt, and run.  **22 of 24 were detected.**  The two survivors
are *provably equivalent* mutants, not coverage gaps:

| mutant | why it cannot be detected |
|---|---|
| `result1 & 0xFF` → `& 0x1FF` | inside `modeselect`, `result1` is only ever `0` or `INT_MIN` (E27); `0x80000000 & 0x1FF == 0x80000000 & 0xFF == 0` |
| `result2 & 0xFF00` → `& 0xFFFF` | likewise `result2 ∈ {0, INT_MIN}` (E28); `0x80000000 & 0xFFFF == 0` |

Detected mutants included: saturating instead of x86 `cvttsd2si` `double`→`int`
conversion, `d2i` boundary off-by-one, NaN→0, a removed `switch` fall-through
edge, wrong `default:` sentinel, `hash *= 0x1F`→`0x1E`, `& 0x7FFFFFFF`→`& 0xFFFFFFFF`,
wrong hash seed, `(i%4)*8`→`(i%8)*8`, `i64` instead of wrapping `i32` offset
arithmetic, logical instead of arithmetic `>>29`, `>>29`→`>>28`, `86400`→`86399`,
`% 4`→`& 3` for the mode index, `% 5`→`% 4`, `% 24`→`% 23`, `% 0x1000`→`% 0x800`,
`0xBEEF`→`0xBEEE`, `1e12`→`1e11`, `-1e15`→`1e15`, `1e8`→`1e7`, a swapped
`classify_mode` branch, a changed return sentinel, a `printf` argument change,
and two format-string changes (`%.2e`→`%.3e`, `%ld`→`%d`, dropped `\n`).
