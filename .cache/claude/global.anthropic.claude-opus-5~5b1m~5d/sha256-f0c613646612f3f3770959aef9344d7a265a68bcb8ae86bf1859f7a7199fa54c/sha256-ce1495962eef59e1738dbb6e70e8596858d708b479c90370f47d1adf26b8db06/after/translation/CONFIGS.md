# CONFIGS.md — Phase A: configuration-surface table

Mechanically derived from the C source, the same way `ERRORS.md` is.

## Step 1 — enumerate the runtime options / modes / flags

```
$ grep -nE 'if *\(|switch|#if|#ifdef|enum |flag|mode|option|_set_|init' \
      c_src/src/lib.c c_src/include/lib.h
(no matches)
```

**There are no runtime options, modes, flags, or `#ifdef`s.** The C is
branch-free straight-line code. There is no `init`/`set_option`/`_free`
function — the caller owns `cn_rnd_t` and seeds it by writing `state` directly.

## Step 2 — enumerate the full set of public entry points

`nm -D --defined-only` on the C `.so` yields exactly one function, and
`lib.h` declares exactly one:

| entry point | signature | level |
|-------------|-----------|-------|
| `next_double` | `double next_double(cn_rnd_t *rnd)` | the **only** public entry point — it is simultaneously the lowest-level and the highest-level API |
| `cn_rnd_next` | `static uint64_t cn_rnd_next(cn_rnd_t *)` | `static`, **not** exported (absent from `nm -D`); only reachable *through* `next_double`, and the Rust keeps it private to match. Its behaviour is therefore covered indirectly, by observing the `state` mutation that `next_double` performs. |

So "exercise the low-level entry points, not only the convenience wrappers"
resolves to: drive `next_double` directly **and** observe the low-level
`cn_rnd_next` state transition (the full 16-byte `cn_rnd_t`) after every call,
not just the returned `double`. Every row below asserts *both*
(returned bit pattern **and** post-call state bytes).

## Step 3 — enumerate the input shapes the code distinguishes

The entire input is the 16-byte `cn_rnd_t` (two `uint64_t`), plus how many
times the caller iterates. The shapes the arithmetic actually distinguishes:

| axis | values the C code treats differently | why (line in `lib.c`) |
|------|--------------------------------------|-----------------------|
| `state[0]` (`x`) bits ≥ 41 | `x << 23` truncates them away vs. not | `x ^= x << 23;` (7) |
| `state[0]` (`x`) bits < 17 after step 1 | `x >> 17` shifts them away vs. not | `x ^= x >> 17;` (8) |
| `state[1]` (`y`) bits ≥ 26 | `y >> 26` is zero vs. non-zero | `x ^= y ^ (y >> 26);` (9) |
| `x + y` | wraps modulo 2^64 vs. not | `return x + y;` (11) |
| low 12 bits of `value` | discarded by `value >> 12` | `mantissa = value >> 12;` (17) |
| `mantissa` | `0` (result is exactly `+0.0`) vs. `0xF…F` (result is the largest double `< 1`) vs. in between | `(exponent << 52) \| mantissa`, `- 1.0` (18-19) |
| degeneracy | all-zero state is a fixed point (stays zero forever) | whole function |
| call count | 1 vs. many (state evolution / sequence divergence) | `state[0]=`, `state[1]=` (6, 10) |
| pointer/placement | stack vs. heap vs. misaligned; two independent instances (no hidden globals) | `rnd->` |
| build profile | `overflow-checks` on/off, `debug-assertions` on/off, `opt-level` 0/3 for the Rust `.so` | `x + y`, the three shifts, and the raw pointer accesses |

## Configuration-surface table

Each row is run against **both** `.so`s (C and Rust), for **all three** Rust
cdylib profiles (`dev`, `release`, `ubcheck` — see `SYMBOLS.md`), with **many
randomized inputs** drawn from a fixed-seed SplitMix64 stream
(`SEED = 0x9E3779B97F4A7C15`), and asserts

* the returned `double`'s **raw 64 bits** are identical, and
* the post-call 16 bytes of `cn_rnd_t` are identical (`memcmp`).

| # | entry point(s) | configuration (options set + input shape) | ✔ |
|---|----------------|-------------------------------------------|---|
| 1  | `next_double` | no options exist; state `{0, 0}`; 1 call — degenerate all-zero seed | [x] |
| 2  | `next_double` | state `{0, 0}`; 4096 consecutive calls — degenerate fixed point must stay stuck | [x] |
| 3  | `next_double` | state `{0, y}` with 512 random non-zero `y`; 1 call | [x] |
| 4  | `next_double` | state `{0, y}` with 64 random non-zero `y`; 1024 consecutive calls each | [x] |
| 5  | `next_double` | state `{x, 0}` with 512 random non-zero `x`; 1 call | [x] |
| 6  | `next_double` | state `{x, 0}` with 64 random non-zero `x`; 1024 consecutive calls each | [x] |
| 7  | `next_double` | state `{u64::MAX, u64::MAX}`; 1 call — every shift saturated, `x + y` overflows | [x] |
| 8  | `next_double` | state `{u64::MAX, u64::MAX}`; 4096 consecutive calls | [x] |
| 9  | `next_double` | state `{1 << 63, 1 << 63}`; 1024 calls — only the top bit set, `x << 23` discards it | [x] |
| 10 | `next_double` | state `{1, 1}`; 1024 calls — minimal non-zero seed, `x >> 17` and `y >> 26` both zero | [x] |
| 11 | `next_double` | `x` restricted to bits `< 41` (so `x << 23` loses nothing), `y` random; 512 seeds × 1 call | [x] |
| 12 | `next_double` | `x` restricted to bits `>= 41` (so `x << 23` is entirely truncated), `y` random; 512 seeds × 1 call | [x] |
| 13 | `next_double` | `y` restricted to bits `< 26` (so `y >> 26 == 0`), `x` random; 512 seeds × 1 call | [x] |
| 14 | `next_double` | `y` restricted to bits `>= 26` (low 26 bits clear, `y >> 26 != 0`), `x` random; 512 seeds × 1 call | [x] |
| 15 | `next_double` | `x` restricted to bits `< 17` (so `x >> 17 == 0` before the xor), `y` random; 512 seeds × 1 call | [x] |
| 16 | `next_double` | fully random `{x, y}`; 20 000 seeds × 1 call — bulk value-dependent coverage | [x] |
| 17 | `next_double` | fully random `{x, y}`; 200 seeds × 1024 consecutive calls — long-sequence state evolution | [x] |
| 18 | `next_double` | seeds searched so the produced `value` has **all** low 12 bits set (max truncation by `>> 12`) | [x] |
| 19 | `next_double` | seeds searched so the produced `value` has **all** low 12 bits clear (no truncation by `>> 12`) | [x] |
| 20 | `next_double` | seeds searched so `x + y` **wraps** (carry out of bit 63) | [x] |
| 21 | `next_double` | seeds searched so `x + y` does **not** wrap | [x] |
| 22 | `next_double` | seeds searched so `mantissa == 0` → return value must be exactly `+0.0` (bits `0x0`, positive zero, not `-0.0`) | [x] |
| 23 | `next_double` | seeds searched so `mantissa == 0x000F_FFFF_FFFF_FFFF` → largest representable value `< 1.0` | [x] |
| 24 | `next_double` | single-bit sweep: `state = {1<<i, 1<<j}` for **all** 64×64 = 4096 combinations | [x] |
| 25 | `next_double` | all-but-one-bit sweep: `state = {!(1<<i), !(1<<j)}` for all 4096 combinations | [x] |
| 26 | `next_double` | `2^k ± 1` boundary sweep on both words (all `k`, both signs, cross product) | [x] |
| 27 | `next_double` | two independent `cn_rnd_t` instances driven interleaved — proves no hidden global/TLS state | [x] |
| 28 | `next_double` | struct on the **stack** vs. **heap** (`Box`) vs. inside a larger buffer at a non-zero offset — same results | [x] |
| 29 | `next_double` | struct inside a canary-guarded buffer; asserts the 16 bytes are the *only* bytes touched | [x] |
| 30 | `next_double` | full 16-byte state re-read and cross-fed: C's post-call state used to seed Rust and vice-versa for 1024 steps (lock-step state equality, catches drift) | [x] |
| 31 | `next_double` | Rust cdylib built with `overflow-checks = on` (`dev` profile): 4096 deliberately-wrapping seeds + shift-saturation sweep — arithmetic must wrap, never panic | [x] |
| 32 | `next_double` | Rust cdylib built `release` (`opt-level = 3`, `overflow-checks = off`, `panic = abort`) — optimisation must not change a single bit | [x] |
| 33 | `next_double` | Rust cdylib built `ubcheck` (`overflow-checks = on` **and** Rust's optional UB checks on): random, `MAX/MAX`, zero-seed, forced-wrapping and misaligned-pointer inputs must all still match the C | [x] |

All 33 rows checked. `[x]` is set only after the row passed for every one of
its randomized inputs, against both `.so`s, in all three cdylib profiles and
both feature combinations.

## Feature-combination sweep

The crate declares **no `[features]` table**, so the complete set of feature
combinations is `{default, --no-default-features}`. `run_all_configs.sh`
extracts this mechanically from `Cargo.toml` (and would automatically expand to
`--features <f>` per feature and `--all-features` if a feature were ever added),
then runs the whole suite for every (feature combination × cdylib profile) pair:

```
declared features: (none) -> combinations are {default, --no-default-features}
-- PASS: cargo check [<default>]
-- PASS: cargo test  [<default>] profiles=all
-- PASS: cargo test [<default>] profile=dev
-- PASS: cargo test [<default>] profile=release
-- PASS: cargo test [<default>] profile=ubcheck
-- PASS: cargo check [--no-default-features]
-- PASS: cargo test  [--no-default-features] profiles=all
-- PASS: cargo test [--no-default-features] profile=dev
-- PASS: cargo test [--no-default-features] profile=release
-- PASS: cargo test [--no-default-features] profile=ubcheck
ALL CONFIGURATIONS PASSED
```

## Divergence found on the valid path

Rows 1-30 all passed on the first run, so the arithmetic translation was
already bit-exact. The two divergences that were found (and fixed in the Rust)
came from the pointer-access strategy and are documented in `ERRORS.md`:
`&mut *rnd` aborted on a misaligned `cn_rnd_t *` that the C accepts, and turned
C's `SIGSEGV` on a NULL pointer into a `SIGABRT`.

One *test* bug was also found and fixed (never the C): row 23 originally
asserted the result equals `nextafter(1.0, 0)`. It does not — with
`mantissa = 2^52-1` the C's `(1.0 + m/2^52) - 1.0` is exactly
`0x3FEF_FFFF_FFFF_FFFE`, one ULP *below* the largest double `< 1.0`. That quirk
of the C is now asserted explicitly rather than "fixed".
