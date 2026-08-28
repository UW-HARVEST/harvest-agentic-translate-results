# CONFIGS.md — Configuration-surface table (valid inputs)

## How this was derived

There are **no** compile-time or runtime option flags in this library:

```sh
grep -nE '#ifdef|#if |#define|enum|extern .*global|static .*=' c_src/src/lib.c   # -> nothing
grep -nE '^\[features\]|feature' translation/Cargo.toml                          # -> nothing
grep -nE 'cfg\(|feature =' translation/src/lib.rs                                # -> nothing
```

So there is exactly **one feature combination** (the default; `--all-features`
and `--no-default-features` are identical to it). The configuration surface is
therefore driven entirely by **input shape**, plus the one *build* axis that
genuinely changes generated code (debug ⇒ Rust integer-overflow checks on,
release ⇒ off + `panic="abort"`).

### Axes the C actually branches on

| axis | values the C source distinguishes | evidence |
|------|-----------------------------------|----------|
| **E. entry point** | `stbds_hash_bytes` (low-level, called directly), `siphash` (driver that calls the hash 64× for `len = 0..63` and `printf`s) | `src/lib.c:110`, `src/lib.c:114` |
| **N. whole 8-byte blocks** | 0, 1, 2, many | loop `for (i; i + sizeof(size_t) <= len; ...)` `src/lib.c:18` |
| **R. tail residue `len - i`** | 0,1,2,3,4,5,6,7 — 8 distinct fall-through arms | `switch (len - i)` `src/lib.c:48-65` |
| **B. byte values / high bit** | `d[3] >= 0x80` and/or `d[7] >= 0x80` trigger the `int`-overflow → `cltq` **sign-extension** that floods the upper 32 bits of `data`; also the tail `case 4` `d[3] << 24`. All-zero / all-0xFF / random. | `src/lib.c:20-22`, `src/lib.c:56`; `cltq` at `.text+0x123d`, `+0x128a`, `+0x13c5` |
| **S. seed** | `0`, `SIZE_MAX` (so `~seed == 0`), arbitrary; used both as `seed` and `~seed` | `src/lib.c:10-17` |
| **A. pointer alignment** | any (all loads are single-byte `movzbl`), offsets 0..7 | `src/lib.c:7,20` |
| **I. `siphash` init** | any `int`; sets `mem[i] = (unsigned char)(init+i)`, so it selects which high-bit pattern the 64 hashes see, and wraps at `INT_MAX` | `src/lib.c:117-118` |
| **P. build profile** | Rust debug (overflow checks) vs release | `Cargo.toml [profile.release] panic="abort"` |

Rows below are the cross-product of these axes, pruned to the combinations the
code actually treats differently. Every row is exercised with **many
randomized inputs** (fixed seed `0x5150_5CA1_AB1E_D00D`, ChaCha-free
xorshift64* PRNG defined in the test harness) — not a single hand-picked value.

## Table

| # | entry point(s) | configuration (options set + input shape) | [x] |
|---|----------------|-------------------------------------------|-----|
| 1 | `stbds_hash_bytes` | N=0, R=0 → `len == 0`; seed swept (0, 1, SIZE_MAX, random) | [x] |
| 2 | `stbds_hash_bytes` | N=0, R=1 → `len == 1`; random bytes, random seeds | [x] |
| 3 | `stbds_hash_bytes` | N=0, R=2 → `len == 2`; random bytes, random seeds | [x] |
| 4 | `stbds_hash_bytes` | N=0, R=3 → `len == 3`; random bytes, random seeds | [x] |
| 5 | `stbds_hash_bytes` | N=0, R=4 → `len == 4`; random bytes (hits tail `case 4` `d[3]<<24` **sign-extension** for half the samples) | [x] |
| 6 | `stbds_hash_bytes` | N=0, R=4 → `len == 4` with `d[3]` **forced `>= 0x80`** (tail sign-extension guaranteed) | [x] |
| 7 | `stbds_hash_bytes` | N=0, R=4 → `len == 4` with `d[3]` **forced `< 0x80`** (no sign-extension) | [x] |
| 8 | `stbds_hash_bytes` | N=0, R=5 → `len == 5`; random, plus `d[3]>=0x80` forced | [x] |
| 9 | `stbds_hash_bytes` | N=0, R=6 → `len == 6`; random, plus `d[3]>=0x80` forced | [x] |
| 10 | `stbds_hash_bytes` | N=0, R=7 → `len == 7`; random, plus `d[3]>=0x80` forced (all four tail `case 7..4` arms fall through) | [x] |
| 11 | `stbds_hash_bytes` | N=1, R=0 → `len == 8`; random bytes, random seeds | [x] |
| 12 | `stbds_hash_bytes` | N=1, R=0, `len == 8`, block `d[3]` **forced `>= 0x80`** → main-loop low-half `cltq` sign-extension | [x] |
| 13 | `stbds_hash_bytes` | N=1, R=0, `len == 8`, block `d[7]` **forced `>= 0x80`** → main-loop high-half `cltq` sign-extension | [x] |
| 14 | `stbds_hash_bytes` | N=1, R=0, `len == 8`, **both** `d[3]` and `d[7] >= 0x80` | [x] |
| 15 | `stbds_hash_bytes` | N=1, R=0, `len == 8`, **neither** `d[3]` nor `d[7] >= 0x80` | [x] |
| 16 | `stbds_hash_bytes` | N=1, R=1..7 → `len == 9..15`, random (block + every tail arm) | [x] |
| 17 | `stbds_hash_bytes` | N=2, R=0 → `len == 16`; random | [x] |
| 18 | `stbds_hash_bytes` | N=2, R=1..7 → `len == 17..23`; random | [x] |
| 19 | `stbds_hash_bytes` | N=many, R=0..7 → `len` swept `0..=264` exhaustively, random bytes/seed per len | [x] |
| 20 | `stbds_hash_bytes` | all bytes `0x00`, `len` swept `0..=64`, seed 0 and SIZE_MAX | [x] |
| 21 | `stbds_hash_bytes` | all bytes `0xFF`, `len` swept `0..=64` (every `d[3]`/`d[7]` sign-extension path maximally set) | [x] |
| 22 | `stbds_hash_bytes` | bytes alternating `0x7F`/`0x80` (straddles the high-bit boundary at every offset), `len` swept `0..=64` | [x] |
| 23 | `stbds_hash_bytes` | S: seed `0` fixed, random bytes, `len` swept `0..=72` | [x] |
| 24 | `stbds_hash_bytes` | S: seed `SIZE_MAX` fixed (`~seed == 0`), random bytes, `len` swept `0..=72` | [x] |
| 25 | `stbds_hash_bytes` | S: seed random full 64-bit, random bytes, random `len` — 20 000 property samples | [x] |
| 26 | `stbds_hash_bytes` | A: misaligned `p` at byte offsets 1..7 into the buffer × `len` `0..=32` | [x] |
| 27 | `stbds_hash_bytes` | oversized buffer: `len` = 1 MiB of random bytes, several seeds | [x] |
| 28 | `stbds_hash_bytes` | determinism/purity: same args called repeatedly and interleaved between the two `.so`s give identical results (no hidden mutable state) | [x] |
| 29 | `siphash` (stdout differential) | `init == 0` — the canonical stb_ds table-dump invocation | [x] |
| 30 | `siphash` (stdout differential) | `init` positive small: 1, 2, 7, 63, 64, 127 | [x] |
| 31 | `siphash` (stdout differential) | `init` crossing the high-bit boundary: 0x80-63 … 0x80, 0xFF, 0x100 (so `mem` straddles `0x7f→0x80`) | [x] |
| 32 | `siphash` (stdout differential) | `init` negative: -1, -64, -128, -255 (`(unsigned char)` truncation of negative `int`) | [x] |
| 33 | `siphash` (stdout differential) | `init == INT_MAX` (the `z++` **overflow-wrap** to `INT_MIN` mid-loop) and `init == INT_MIN` | [x] |
| 34 | `siphash` (stdout differential) | `init` = 64 randomized full-range `i32` values | [x] |
| 35 | `siphash` + `stbds_hash_bytes` | composed pipeline: reconstruct `siphash`'s printed table from direct low-level `stbds_hash_bytes(mem, len, 0)` calls and require C-print == Rust-print == recomputed table | [x] |
| 36 | both | P: every row above re-run under **debug** profile (Rust overflow checks ON) and **release** profile | [x] |
| 37 | both | P: every row above re-run under `--no-default-features` and `--all-features` (degenerate — no features exist — asserted by script) | [x] |

---

## Note on the seed axis (rows 23-25)

The `seed` parameter **provably cancels out** in the C (it is XORed into every
state word twice — see the "Finding" section of `ERRORS.md`), so
`stbds_hash_bytes` is seed-independent. Rows 23-25 remain valuable as
byte/length coverage, but they do **not** discriminate seed handling, and no
differential test can: the C ignores the seed, so any Rust that also ignores it
matches for every seed.

That blind spot is closed explicitly, not by sweeping, in
`quirk_seed_cancels_out_identically_in_both_libraries`
(`tests/phase_c_errors.rs`), which asserts that the C is seed-independent, that
the Rust is seed-independent, and that both agree — so a future change making
the Rust honour the seed fails loudly.

## Where rows 29-35 live

`siphash` writes only to stdout, and capturing stdout means `dup2`-ing over the
process-wide fd 1. Any other thread printing during that window (including
libtest's own `test foo ... ok` progress lines) would be captured and misread as
library output — this actually produced 6 spurious failures during development.

Rows 29-35 are therefore implemented as sub-cases of the single `#[test]` in
`tests/phase_bc_siphash_stdout.rs` (`siphash_stdout_differential_all_rows`).
Cargo runs test binaries one at a time and that file contains exactly one test,
so no sibling thread can interleave. Each row is tagged (`cfg29 init=0`,
`cfg30 small-positive`, …) and all failures are collected and reported
individually.

## Verification status

`./run_all_tests.sh` runs the whole suite for the cross-product of
{debug, release} x {default, `--no-default-features`, `--all-features`}.
There is no `[features]` table, so the three feature flags are provably
equivalent (asserted by the script) and the real axis is the build profile:
debug enables Rust integer-overflow checks, release disables them and sets
`panic = "abort"`. Both pass.

Last run: **6/6 configurations passed, 47 tests each, symbol parity 0 missing.**

## Detection power (mutation sweep)

Row coverage alone does not prove the tests can *see* a divergence, so
`./mutation_check.sh` injects 42 targeted faults into `src/lib.rs`, rebuilds the
cdylib, and requires the suite to fail for each:

```
caught-as-expected: 39   equivalent-as-expected: 3   no-op: 0   skipped: 0   GAPS: 0
```

The 3 survivors are provably semantics-preserving (seed cancellation; a
sign-extension whose bits are shifted out by a total shift of 32; a `rem == 7`
guard where `rem <= 7` is invariant) and each is asserted as a property test
rather than assumed. The script fails if a mutation tagged `CAUGHT` survives
**or** if one tagged `EQUIVALENT` is caught, so misclassifications surface too.
