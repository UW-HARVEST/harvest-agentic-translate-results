# CONFIGS.md — Phase A.3: configuration-surface table

## Mechanical derivation of the axes

### Public entry points (complete set)

`nm -D` on the C `.so` exports exactly one function, and the header declares
exactly one function:

| entry point | signature | is it a convenience wrapper? |
|---|---|---|
| `max_size_frame` | `tflac_u32 (tflac_u32 blocksize, tflac_u32 channels, tflac_u32 bitdepth)` | **No.** It is simultaneously the highest- and lowest-level entry point. It calls nothing (leaf function), has no `static` helpers, no init/teardown, and no hidden state. |

There is therefore no "call hierarchy" to walk and no lower-level API hiding
behind a one-shot wrapper — the single export *is* the low-level API. Confirmed
by `SYMBOLS.md`'s completeness audit of the 15 total lines of C.

### Build-time configuration axes

* `Cargo.toml` has **no `[features]` table** ⇒ the only valid feature
  combination is the empty set. Enumerated and verified by
  `./check_all_features.sh` (1 combination, plus the `default` and
  `--all-features` invocations, which are identical to it here).
* `c_src/CMakeLists.txt` defines **no** `option()`, `add_definitions`,
  `target_compile_definitions`, or conditional sources — it unconditionally
  compiles `src/lib.c`. `grep -nE '#if'` over the C source has **no matches**,
  so there is no `#ifdef`-selected code either.

⇒ **One build configuration total.** All rows below are runtime-input rows.

### Runtime axes the C code actually branches on

Derived from the three predicates and the arithmetic in `src/lib.c:4-9`:

```c
return 18U + channels +
       (((blocksize * bitdepth * (channels * (channels != 2))) +   /* T1 */
         (blocksize * bitdepth * (channels == 2)) +                /* T2 */
         (blocksize * (bitdepth + (bitdepth != 32)) * (channels == 2)) + /* T3 */
         +7) / 8);
```

| axis | values the code distinguishes | why (source evidence) |
|---|---|---|
| **X1 — stereo predicate** | `channels == 2` vs `channels != 2` | `(channels != 2)` gates `T1`; `(channels == 2)` gates `T2` and `T3`. The two branches are mutually exclusive, so they are genuinely different code paths. |
| **X2 — zero-channel degeneracy** | `channels == 0` vs `channels > 0` | `channels * (channels != 2)` is `0` when `channels == 0`, zeroing `T1` even on the non-stereo path. |
| **X3 — bit-depth predicate** | `bitdepth == 32` vs `bitdepth != 32` | `(bitdepth != 32)` adds `1` to `bitdepth` inside `T3`. Only observable when `channels == 2`, so X3 must be crossed with X1. |
| **X4 — blocksize magnitude** | `0`, `1`, small, realistic, `65535`, `2^24`, `UINT32_MAX` | Multiplicand of all three terms; `0` zeroes the numerator, large values drive 32-bit wrap. |
| **X5 — bitdepth magnitude** | `0`, `1..31`, `32`, `33..64`, huge, `UINT32_MAX` | `bitdepth == 0` makes `T3 == blocksize`; `UINT32_MAX` makes `bitdepth + 1` wrap to `0`. |
| **X6 — 32-bit wrap of the products** | no wrap vs wrap | Unsigned `*` is mod 2^32; wrapping changes the numerator and hence the result. |
| **X7 — ceiling-divide residue** | `numerator mod 8 ∈ {0,…,7}` | The `+7 ... / 8` idiom rounds up; each of the 8 residues is a distinct rounding outcome. |
| **X8 — wrap of `numerator + 7`** | `sum + 7` fits vs overflows | When `sum` is within 7 of `UINT32_MAX` the `+7` wraps, so the divide sees a *tiny* numerator instead of a huge one. |
| **X9 — wrap of the outer `18U + channels`** | fits vs overflows | `channels` near `UINT32_MAX` wraps the final sum. |

Axes that are **N/A** for this library (recorded so the pruning is auditable):
element type / width (every parameter and the return are `uint32_t`), byte order
and format (nothing is serialized, no memory is read or written), counts and
"empty / one / many" collections (there is no array, buffer, or length
parameter), and runtime option/mode/flag state (there is no context struct,
no setter, and no global).

## Configuration-surface table

Every row is exercised with **many randomized inputs** drawn inside that row's
constraint (fixed seed `0x5EED_C0FF_EE00_0000` + per-row salt, splitmix64 PRNG in
`tests/common/mod.rs`), not one hand-picked value. Rows 22–24 are *exhaustive*
over their stated domain. All rows live in `tests/differential.rs` and call both
`.so` files through `libloading`.

| # | entry point(s) | configuration (options set + input shape) | axes | [ ] |
|---|----------------|-------------------------------------------|------|-----|
| 1 | `max_size_frame` | `channels == 2`, `bitdepth == 32` (the equality-guarded value), `blocksize` random `1..=65535` | X1,X3 | [x] |
| 2 | `max_size_frame` | `channels == 2`, `bitdepth == 16` (≠32), `blocksize` random `1..=65535` | X1,X3 | [x] |
| 3 | `max_size_frame` | `channels == 2`, `bitdepth` random `1..=31`, `blocksize` random `1..=65535` | X1,X3,X5 | [x] |
| 4 | `max_size_frame` | `channels == 2`, `bitdepth` random `33..=64`, `blocksize` random `1..=65535` | X1,X3,X5 | [x] |
| 5 | `max_size_frame` | `channels == 2`, `bitdepth == 0`, `blocksize` random full 32-bit (`T3` collapses to `blocksize`) | X1,X5,X6 | [x] |
| 6 | `max_size_frame` | `channels == 2`, `bitdepth == UINT32_MAX` (inner `+1` wraps to `0`), `blocksize` random full range | X1,X5,X6 | [x] |
| 7 | `max_size_frame` | `channels == 2`, `bitdepth` **and** `blocksize` random full 32-bit (stereo path with heavy wrapping) | X1,X5,X6,X8 | [x] |
| 8 | `max_size_frame` | `channels == 0`, `bitdepth`/`blocksize` random full range (all terms vanish) | X2 | [x] |
| 9 | `max_size_frame` | `channels == 1` (mono), `bitdepth ∈ {4,8,12,16,20,24,32}`, `blocksize` realistic FLAC sizes | X1,X2 | [x] |
| 10 | `max_size_frame` | `channels == 3`, `bitdepth` random `1..=32`, `blocksize` random `1..=65535` (just past the stereo special case) | X1,X3 | [x] |
| 11 | `max_size_frame` | `channels` random `4..=8`, `bitdepth ∈ {4,8,12,16,20,24,32}`, `blocksize` realistic | X1 | [x] |
| 12 | `max_size_frame` | `channels` random `9..=255`, `bitdepth` random `1..=64`, `blocksize` random `1..=8192` | X1 | [x] |
| 13 | `max_size_frame` | `channels` random `2^16..2^32` (≠2) with non-zero `blocksize`/`bitdepth` — forces `T1` wrap **and** outer wrap | X1,X6,X9 | [x] |
| 14 | `max_size_frame` | `channels == UINT32_MAX`, `blocksize`/`bitdepth` random full range | X6,X9 | [x] |
| 15 | `max_size_frame` | `blocksize == 0`, `channels`/`bitdepth` random full range (result must be `18 + channels`) | X4,X9 | [x] |
| 16 | `max_size_frame` | `blocksize == 1`, `channels`/`bitdepth` random full range | X4,X6 | [x] |
| 17 | `max_size_frame` | `blocksize == UINT32_MAX`, `channels` random `0..=8`, `bitdepth` random `0..=64` | X4,X6 | [x] |
| 18 | `max_size_frame` | `blocksize` = every power of two `2^0..2^31`, crossed with `channels ∈ {1,2,3}` and `bitdepth ∈ {16,24,32}` (exhaustive) | X4,X6 | [x] |
| 19 | `max_size_frame` | Non-stereo rounding sweep: `channels == 1`, `bitdepth == 1`, `blocksize` chosen to hit **every** residue `numerator mod 8 == 0..7` | X7 | [x] |
| 20 | `max_size_frame` | Stereo rounding sweep: `channels == 2`, `bitdepth == 1` (`sum == 3*blocksize`), `blocksize` chosen to hit every residue | X1,X7 | [x] |
| 21 | `max_size_frame` | Numerator engineered into `[UINT32_MAX-6, UINT32_MAX]` so `sum + 7` itself wraps. Solved **constructively** via the multiplicative inverse of the odd multiplier `M` mod 2^32 (140 000 verified wrap cases); random sampling cannot reach this window (p ~ 1.6e-9) | X8 | [x] |
| 22 | `max_size_frame` | **Exhaustive** small cube: `blocksize 0..=48` × `channels 0..=8` × `bitdepth 0..=40` (18 081 calls) | X1,X2,X3,X4,X5,X7 | [x] |
| 23 | `max_size_frame` | **Exhaustive** single-axis sweeps: `channels 0..=1024` at realistic `blocksize`/`bitdepth`; `bitdepth 0..=64`; `blocksize 0..=4096` | X1,X2,X3,X4,X5 | [x] |
| 24 | `max_size_frame` | **Exhaustive** FLAC-realistic matrix: `blocksize ∈ {192,576,1152,2304,4608,256,512,1024,2048,4096,8192,16384,32768,65535}` × `channels 1..=8` × `bitdepth ∈ {4,8,12,16,20,24,32}` | X1,X3,X4,X5 | [x] |
| 25 | `max_size_frame` | Uniform random full-range triples, 200 000 iterations (unconstrained fuzz) | all | [x] |
| 26 | `max_size_frame` | Mixed boundary/random: each argument independently drawn from the boundary set `{0,1,2,3,4,7,8,31,32,33,64,4096,65535,65536,2^31-1,2^31,2^32-2,2^32-1}` ∪ uniform random — covers every boundary *interaction* | all | [x] |

**Total rows: 26.** Each is checked off only after passing across all of its
randomized inputs.

## Feature-combination coverage

Because there is exactly one valid feature combination (no `[features]` table),
running the suite once covers "every feature combination". `run_all_tests.sh`
nevertheless drives the full suite through `--no-default-features`, `default`,
and `--all-features` so the claim is machine-checked rather than argued.
