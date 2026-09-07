# CONFIGS.md — Phase B configuration-surface table

## Axes mechanically derived from the C source

The C library has **one** public entry point and **no** runtime options,
modes, flags, `#ifdef`s, `switch`es, or `if`s. `grep -c 'if\|switch\|#if' c_src/src/lib.c` → 0.
So the configuration surface is entirely the **input shape** of the single
in/out parameter, `cn_rnd_t { uint64_t state[2] }`.

Axes that the C code's arithmetic actually distinguishes:

| axis | values the code branches on / is sensitive to |
|---|---|
| **A. entry point** | `next_double` (the only export); `cn_rnd_next` reached *through* it (`static`, not directly callable) |
| **B. `state[0]` (`x`) shape** | `0`; `1`; `u64::MAX`; only low 17 bits set (`x >> 17 == 0`); only high 23 bits set (`x << 23 == 0`); single-bit walks; random |
| **C. `state[1]` (`y`) shape** | `0`; `1`; `u64::MAX`; only low 26 bits set (`y >> 26 == 0`); high bits only; single-bit walks; random |
| **D. call count / sequence length** | 1 (single step); 2 (first state rotation visible); many (1..=1000, long stream) |
| **E. observed output** | return `double` **raw bits** (`to_bits`), *and* the mutated `state[0]`, `state[1]` after the call — state mutation is an output, not just the return |
| **F. mantissa shape of `value`** | `mantissa == 0` → exactly `0.0`; `mantissa == all ones` → `1.0-2^-52`; low 12 bits of `value` vary but `mantissa` fixed → identical `double`, different state |
| **G. `x + y` overflow** | no wrap; wraps modulo 2^64 |
| **H. instance multiplicity** | one generator; several independent generators interleaved (no hidden global state) |
| **I. memory layout of the arg** | stack struct; heap `Box`; element of an array of `cn_rnd_t` (neighbour untouched); misaligned pointer |
| **J. driver binary** | **none** — `c_src/CMakeLists.txt` builds only `add_library(... SHARED src/lib.c)`; there is no `add_executable`, and `translation/Cargo.toml` declares only `[lib] crate-type=["cdylib"]` with no `[[bin]]` and no `src/main.rs`. No stdout comparison is applicable. |

Every row below is checked by calling **both** `.so`s through `libloading`
(never Rust directly) and comparing the return value's raw `f64` bits **and**
the post-call `state[0]`/`state[1]` byte-for-byte. Rows marked "randomized"
use `N` pseudo-random inputs from a fixed-seed SplitMix64 (seed `0x2545F491_4F6CDD1D`)
for reproducibility.

## Table

| # | entry point(s) | configuration (options set + input shape) | [x] |
|---|----------------|--------------------------------------------|-----|
| 1 | `next_double` | `state = {0, 0}` (degenerate fixed point), 1 call | [x] |
| 2 | `next_double` | `state = {0, 0}`, 1000 sequential calls (must stay pinned) | [x] |
| 3 | `next_double` | `state = {u64::MAX, u64::MAX}`, 1 call | [x] |
| 4 | `next_double` | `state = {u64::MAX, u64::MAX}`, 1000 sequential calls | [x] |
| 5 | `next_double` | `state = {1, 0}` and `{0, 1}` (minimal non-zero seeds), 1 call each | [x] |
| 6 | `next_double` | single-bit walk: `state = {1<<i, 0}` for all `i` in 0..64, 1 call | [x] |
| 7 | `next_double` | single-bit walk: `state = {0, 1<<i}` for all `i` in 0..64, 1 call | [x] |
| 8 | `next_double` | single-bit walk on both: `state = {1<<i, 1<<j}` for all 64x64 pairs, 1 call | [x] |
| 9 | `next_double` | randomized `state`, 1 call — 20 000 inputs (axis B x C random) | [x] |
| 10 | `next_double` | randomized `state`, 2 calls (state rotation `state[0] <- y` observable) — 5 000 inputs | [x] |
| 11 | `next_double` | randomized `state`, 1000-call stream, compare all 1000 returns + final state — 200 seeds | [x] |
| 12 | `next_double` | `x >> 17 == 0`: `state[0]` restricted to low 17 bits, random `state[1]` — 5 000 inputs | [x] |
| 13 | `next_double` | `x << 23 == 0`: `state[0]` restricted to high 23 bits, random `state[1]` — 5 000 inputs | [x] |
| 14 | `next_double` | `y >> 26 == 0`: `state[1]` restricted to low 26 bits, random `state[0]` — 5 000 inputs | [x] |
| 15 | `next_double` | `y` restricted to high 38 bits (`y >> 26` fully populated), random `state[0]` — 5 000 inputs | [x] |
| 16 | `next_double` | `x + y` wraps modulo 2^64 — seeds solved so the final sum overflows — 2 000 inputs | [x] |
| 17 | `next_double` | `x + y` does **not** wrap — seeds solved so the final sum fits — 2 000 inputs | [x] |
| 18 | `next_double` | `mantissa == 0` → return must be exactly `+0.0` (bits `0x0000000000000000`) | [x] |
| 19 | `next_double` | `mantissa == 0xF_FFFF_FFFF_FFFF` → return must be exactly `1.0 - 2^-52` (bits `0x3FEFFFFFFFFFFFFE`) | [x] |
| 20 | `next_double` | `value` differing only in its low 12 bits → identical `double`, **different** state — 2 000 pairs | [x] |
| 21 | `next_double` | output-range invariant on random seeds: `0.0 <= r < 1.0`, never NaN/Inf, both libs — 20 000 inputs | [x] |
| 22 | `next_double` | H: 8 independent `cn_rnd_t` instances interleaved round-robin, 500 rounds (no shared global state) | [x] |
| 23 | `next_double` | I: arg is an element of `[cn_rnd_t; 4]`; assert neighbouring elements are byte-identical after the call (no OOB write) — 2 000 inputs | [x] |
| 24 | `next_double` | I: arg on the heap (`Box<cn_rnd_t>`) vs on the stack — same seed must give same result in both libs — 2 000 inputs | [x] |
| 25 | `next_double` | I: misaligned `cn_rnd_t*` (odd byte offset in a `[u8]` buffer) — 1 000 inputs | [x] |
| 26 | `next_double` | E: state mutation contract — after 1 call, `state[0]` must equal the pre-call `state[1]` in both libs — 5 000 inputs | [x] |
| 27 | `next_double` | cross-check: C-then-Rust vs Rust-then-C on a *shared* struct — a Rust step must be able to continue a C-produced state and vice versa, 200 alternating rounds x 50 seeds | [x] |
| 28 | `cn_rnd_next` (via `next_double`) | low-level generator exercised through 100 000 consecutive steps from one seed, comparing every raw `f64` bit and both state words (long-run drift detection) | [x] |
| 29 | driver binary | **N/A** — no `add_executable` in CMakeLists.txt, no `[[bin]]`/`src/main.rs` in the crate. Asserted by a test that both facts hold. | [x] |
| 30 | feature combos | default == `--no-default-features` == `--all-features` (no `[features]` table); all rows re-run under each | [x] |

## Phase B result

All 30 rows pass. Executed via `run_verification.sh`, which builds both `.so`s
and runs the full suite under every feature combination x profile:

```
  <default> <dev>                    43 tests ok
  <default> --release                43 tests ok
  --no-default-features <dev>        43 tests ok
  --no-default-features --release    43 tests ok
  --all-features <dev>               43 tests ok
  --all-features --release           43 tests ok
```

Randomized rows use a fixed-seed SplitMix64 (`0x2545F4914F6CDD1D`, per-row
salted), so every run is reproducible. Total differential calls per profile:
roughly 1.1 M `next_double` invocations per library.

Every comparison checks BOTH observable outputs:
* the returned `double` as **raw bits** (`f64::to_bits`), which distinguishes
  `+0.0` from `-0.0` and every NaN payload — plain `f64 ==` would not;
* the **mutated 16-byte `cn_rnd_t`** after the call.

Row 25 (misaligned pointer) is where divergence **D1** was found — see
`ERRORS.md`. Row 29's premise (no driver binary) is asserted by a test rather
than assumed, so it fails loudly if an `add_executable`/`[[bin]]` is ever added
and a stdout comparison becomes required.

Rows 16/17/18/19/20 need seeds that land on a specific arithmetic outcome
(wraparound, `mantissa == 0`, `mantissa == all ones`). These are constructed by
inverting the generator (`seed_for_value`, using the invertibility of
`x ^= x << 23` and `x ^= x >> 17`) rather than by rejection sampling, which
could never hit a 1-in-2^52 target. `meta_seed_for_value_is_exact` validates the
inversion itself over 5 000 random cases. The inverted model is used ONLY to
*choose* inputs — the C `.so` remains the sole oracle for expected output.
