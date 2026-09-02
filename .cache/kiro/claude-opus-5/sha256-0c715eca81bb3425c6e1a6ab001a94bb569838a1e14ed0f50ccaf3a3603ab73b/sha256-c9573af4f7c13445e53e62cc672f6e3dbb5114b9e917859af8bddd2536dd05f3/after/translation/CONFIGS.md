# CONFIGS.md — Configuration-surface table (Phase A, gates Phase B)

Derived **mechanically** from the C source, the same way `ERRORS.md` is.

## Axis enumeration (what the C actually branches on / distinguishes)

### Runtime options / modes / flags

```
$ grep -nE 'if *\(|switch|#if|#ifdef|static [^u]|extern|global' -r c_src/include c_src/src
(no option/flag/branch found)
```

**None.** The public header exposes no setter, no flags word, no mode enum, no
init function, and `src/lib.c` contains no `#ifdef`, no `if`, no `switch`, and
no mutable global. The library has exactly one runtime "option": the 128-bit
state the caller puts in `cn_rnd_t` — which is data, and is enumerated below.

### Public entry points (full set, incl. the lowest level)

| entry point | linkage | reachable how |
|-------------|---------|---------------|
| `next_double(cn_rnd_t*)` | exported (`T`) | directly by any consumer |
| `cn_rnd_next(cn_rnd_t*)` | `static` — **not** exported by the C `.so` | only transitively, via `next_double` |

`cn_rnd_next` is the lowest-level routine. Because it is `static` it cannot be
called through the FFI boundary in either library, so it is exercised
*indirectly but exhaustively*: `next_double` is a bijection-preserving wrapper
over it, and rows 1–14 below are chosen specifically to drive each individual
bit-operation inside `cn_rnd_next` (`<<23`, `>>17`, `>>26`, `+`), plus the state
write-back, which is `cn_rnd_next`'s only side effect and is asserted on every
row. There is no convenience-wrapper-only coverage here: `next_double` *is* the
low-level entry point.

### Input-shape axes the code special-cases (data-flow driven)

| axis | statement in `c_src/src/lib.c` | distinguished shapes |
|------|--------------------------------|----------------------|
| S1 | `x ^= x << 23` (line 6) | `x` with bits ≥ 41 (bits shifted out) vs `x < 2^41` (nothing lost) vs `x` whose only bits are ≥ 41 (`x << 23 == 0`) |
| S2 | `x ^= x >> 17` (line 7) | `x < 2^17` (`x >> 17 == 0`) vs larger |
| S3 | `x ^= y ^ (y >> 26)` (line 8) | `y < 2^26` (`y >> 26 == 0`) vs larger; `y == 0` (mixing is a no-op) |
| S4 | `return x + y` (line 11) | sum wraps mod 2^64 vs does not wrap |
| S5 | `mantissa = value >> 12` (line 17) | `value < 2^12` → mantissa `0` → result exactly `+0.0`; mantissa all-ones → largest representable result; generic |
| S6 | low 12 bits of `value` (discarded by `>> 12`) | must not influence the result: two states differing only in those bits give the same `double` |
| S7 | `rnd->state[0] = y` / `rnd->state[1] = x` (lines 5, 10) | the struct is an **out-parameter**: the post-call 16 bytes are part of the observable output on every row |
| S8 | call-sequence length (state is carried) | 1 call, 2 calls, long run (1 000 / 10 000) |
| S9 | instance multiplicity | one struct; two structs interleaved (proves no hidden shared state) |
| S10 | struct storage | stack vs heap (`Box`), both 8-byte aligned |

### Feature combinations

```
$ grep -A20 '^\[features\]' translation/Cargo.toml
(no [features] section)
```

The crate declares **no** Cargo features, so the complete set of feature
configurations is: default (`{}`) and `--no-default-features` (`{}`) — the same
code path. Both are still run explicitly (see `run_all.sh`).

## The table

One row per meaningful combination the C treats differently. Every row asserts,
byte-for-byte between the C `.so` and the Rust `.so`: (a) the returned `double`
compared **by its 64 raw bits**, and (b) the 16 post-call state bytes.
Rows marked *randomized* use a fixed-seed PRNG (SplitMix64, seed
`0x243F6A8885A308D3`) so runs are reproducible.

| # | entry point(s) | configuration (options set + input shape) | [x] |
|---|----------------|-------------------------------------------|-----|
| 1 | `next_double` | no options (none exist); state `(0, 0)` — degenerate all-zero, 1 call | [x] |
| 2 | `next_double` | state `(0, 0)`, 1 000-call sequence (fixed point: S8 × degenerate) | [x] |
| 3 | `next_double` | state `(1, 0)` — S2 `x>>17==0`, S3 `y==0`, S1 nothing shifted out | [x] |
| 4 | `next_double` | state `(0, 1)` — S3 `y>>26==0`, `x==0` | [x] |
| 5 | `next_double` | state `(u64::MAX, u64::MAX)` — S1/S2/S3 all saturated, S4 wraps | [x] |
| 6 | `next_double` | state `(u64::MAX, 0)` | [x] |
| 7 | `next_double` | state `(0, u64::MAX)` | [x] |
| 8 | `next_double` | state `(1<<63, 1<<63)` — only the sign bit set in both words | [x] |
| 9 | `next_double` | state `(1<<41, 0)` — S1 boundary: `x << 23` shifts **every** bit out (`== 0`) | [x] |
| 10 | `next_double` | state `(x < 2^17, y < 2^26)` — S2 **and** S3 both degenerate to `0` simultaneously | [x] |
| 11 | `next_double` | state chosen so `x_out + y` **wraps** mod 2^64 (S4 = wrap), constructed by inverting the round function | [x] |
| 12 | `next_double` | state chosen so `x_out + y` does **not** wrap (S4 = no wrap) | [x] |
| 13 | `next_double` | state chosen (by inverting the round function) so `value == 0` → S5 mantissa `0` → result exactly `+0.0` | [x] |
| 14 | `next_double` | state chosen so `value == 0xFFF` → S5 mantissa still `0`, low-12-bits boundary | [x] |
| 15 | `next_double` | state chosen so `value >> 12` is **all ones** → S5 largest result, closest `double` below `1.0` | [x] |
| 16 | `next_double` | S6: pairs of states whose `value` differs only in the discarded low 12 bits → identical `double`, different state write-back | [x] |
| 17 | `next_double` | *randomized*: 20 000 independent uniform states, 1 call each (S1–S5 uniformly mixed) | [x] |
| 18 | `next_double` | *randomized*: 200 states × 1 000-call sequences, full sequence + final state compared (S8 long run) | [x] |
| 19 | `next_double` | *randomized*: 5 000 states with `state[1] == 0` (S3 mixing disabled) | [x] |
| 20 | `next_double` | *randomized*: 5 000 states with `state[0] == 0` | [x] |
| 21 | `next_double` | *randomized*: 5 000 sparse states (single random bit set in each word) — S1/S2/S3 shift boundaries | [x] |
| 22 | `next_double` | *randomized*: 5 000 states drawn from the 64 shift-boundary values `{0,1,2^k-1,2^k,MAX}` cross-product | [x] |
| 23 | `next_double` | *randomized*: 5 000 low-entropy states (`x,y < 2^12`) — S5 small-`value` neighbourhood | [x] |
| 24 | `next_double` | S9: two `cn_rnd_t` instances, interleaved calls, 2 000 rounds — proves no hidden global state in either library | [x] |
| 25 | `next_double` | S10: identical state in a stack struct vs a heap (`Box`) struct → identical result (ABI/alignment) | [x] |
| 26 | `next_double` | S7 isolation: `#[repr(C)]` layout parity — the same raw 16-byte buffer is handed to both libraries and both must consume/produce the identical byte image (also covers `cn_rnd_t` field order `state[0]` before `state[1]`) | [x] |
| 27 | `next_double` | determinism: replaying the same state through the *same* library twice yields identical output (no time/entropy dependence in either) | [x] |
| 28 | `next_double` | range invariant across all of the above: result always in `[0.0, 1.0)`, never NaN, never `-0.0` | [x] |
| 29 | `next_double` | feature config `default` (no features exist) — all rows above | [x] |
| 30 | `next_double` | feature config `--no-default-features` — all rows above | [x] |

## How to run

```
cd translation && ./run_all.sh
```

The script builds the C `.so`, enumerates the Cargo feature combinations from
`Cargo.toml`, builds and `cargo check`s every combination in both profiles,
diffs `nm -D` for each, and then runs the differential suite for every
(feature combination × test profile × `cdylib` profile) — 8 runs of 36 tests.

## Evidence

All 30 rows pass. Row → test mapping is one-to-one:
`configs::row01_…` … `configs::row28_…` in `tests/differential.rs`, with rows 29
and 30 being those 28 rows re-run under the `default` and `--no-default-features`
configurations by `run_all.sh`.

```
$ ./run_all.sh
=== 2. enumerate Cargo feature combinations ===
no [features] declared -> combinations: {default} and {--no-default-features}
=== 5. differential suite: every combination x profile x cdylib profile ===
  OK    tests (36 passed)  combo=default profile=dev     cdylib=debug
  OK    tests (36 passed)  combo=default profile=dev     cdylib=release
  OK    tests (36 passed)  combo=default profile=release cdylib=debug
  OK    tests (36 passed)  combo=default profile=release cdylib=release
  OK    tests (36 passed)  combo=none    profile=dev     cdylib=debug
  OK    tests (36 passed)  combo=none    profile=dev     cdylib=release
  OK    tests (36 passed)  combo=none    profile=release cdylib=debug
  OK    tests (36 passed)  combo=none    profile=release cdylib=release
ALL CHECKS PASSED
```

Randomized rows use SplitMix64 seeded with `0x243F6A8885A308D3`; total distinct
states driven through both libraries per run is ≈ 290 000 (row 18 alone is
200 × 1 000 sequential calls per library). Every call compares the returned
`double` **by raw bits** and the 16 post-call state bytes.
