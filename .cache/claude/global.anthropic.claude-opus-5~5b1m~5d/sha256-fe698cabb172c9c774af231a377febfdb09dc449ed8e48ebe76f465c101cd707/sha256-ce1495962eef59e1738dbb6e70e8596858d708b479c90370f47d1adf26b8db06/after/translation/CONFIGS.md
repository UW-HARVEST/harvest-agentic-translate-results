# CONFIGS.md — Phase A configuration-surface table

## Mechanical derivation of the axes

### Axis 1 — runtime options / modes / flags: **none**

`grep -rnE 'if|else|switch|case|while|for|\?|&&|\|\||#if' c_src/include c_src/src`
matches **0** lines. There is no global, no init/config struct, no flag
parameter, no `#ifdef`, and no compile-time option in `CMakeLists.txt` beyond the
standard `add_library(... SHARED)`. There is exactly one behavioural mode.

### Axis 2 — public entry points: **one, and it is the lowest level**

`c_src/include/lib.h` declares exactly one function. There is no convenience
wrapper / low-level pair to distinguish: `rev16` *is* the low-level entry point
and the only one. `nm -D` confirms it is the only exported symbol.

### Axis 3 — input shapes the code actually special-cases

The four statements are the only thing that branches on data, and they do so
bitwise rather than with control flow. Reading them off literally:

| statement | masks | grouping it distinguishes | side effect |
|-----------|-------|---------------------------|-------------|
| 1 | `0xAAAA` / `0x5555` | odd vs even bit positions within bits 0..15 | **bits 16..31 are discarded here**, because both masks are only 16 bits wide |
| 2 | `0xCCCC` / `0x3333` | adjacent 2-bit pairs | — |
| 3 | `0xF0F0` / `0x0F0F` | adjacent nibbles | — |
| 4 | `0xFF00` / `0x00FF` | the two bytes of the low half | — |

So the shapes the code treats differently are: the **low 16 bits** (a full bit
permutation — every one of the 16 positions takes a distinct route through the
four swaps) and the **high 16 bits** (uniformly discarded). Both halves must be
varied independently, which is what the cross-product below does.

## Configuration table

`options set` is "none (the library has no options)" for every row, per Axis 1 —
the meaningful cross-product is therefore *entry point* × *low-half shape* ×
*high-half shape*. Every row is driven through the `.so` exports of **both**
libraries with **many randomized inputs** (seeded, reproducible), not a single
hand-picked value.

| # | entry point(s) | configuration (options set + input shape) | [ ] |
|---|----------------|--------------------------------------------|-----|
| C1 | `rev16` | no options; low half = `0x0000`, high half = `0x0000` (the empty input) | [x] |
| C2 | `rev16` | no options; low half = single bit `1 << k`, k = 0..=15 exhaustive; high half = `0x0000` | [x] |
| C3 | `rev16` | no options; low half = `0x0000`; high half = single bit `1 << k`, k = 0..=15 exhaustive (i.e. arg `1 << (16+k)`) | [x] |
| C4 | `rev16` | no options; low half = single bit `1 << i`; high half = single bit `1 << j`; full 16x16 exhaustive cross-product | [x] |
| C5 | `rev16` | no options; low half = `0xFFFF` (all ones); high half = `0x0000` | [x] |
| C6 | `rev16` | no options; low half = `0xFFFF`; high half = `0xFFFF` (arg `0xFFFFFFFF`, numeric max) | [x] |
| C7 | `rev16` | no options; low half = each statement-1 mask exactly: `0xAAAA`, `0x5555`; high half = `0x0000` | [x] |
| C8 | `rev16` | no options; low half = each statement-2 mask exactly: `0xCCCC`, `0x3333`; high half = `0x0000` | [x] |
| C9 | `rev16` | no options; low half = each statement-3 mask exactly: `0xF0F0`, `0x0F0F`; high half = `0x0000` | [x] |
| C10 | `rev16` | no options; low half = each statement-4 mask exactly: `0xFF00`, `0x00FF`; high half = `0x0000` | [x] |
| C11 | `rev16` | no options; low half = every one of the 8 masks; high half = the *complement* of that mask, so the discarded half is maximally different from the honoured half | [x] |
| C12 | `rev16` | no options; low half = **exhaustive all 65 536 values**; high half = `0x0000` | [x] |
| C13 | `rev16` | no options; low half = exhaustive all 65 536 values; high half = `0xFFFF` (proves the high half never influences the result) | [x] |
| C14 | `rev16` | no options; low half = exhaustive all 65 536 values; high half = seeded-random per value | [x] |
| C15 | `rev16` | no options; high half = **exhaustive all 65 536 values**; low half held at each of `0x0000`, `0x0001`, `0x8000`, `0xFFFF`, `0xAAAA`, `0x5555` | [x] |
| C16 | `rev16` | no options; low half = byte-aligned shapes `0x00XY` / `0xXY00` for all 256 `XY`; high half = `0x0000` | [x] |
| C17 | `rev16` | no options; low half = nibble-aligned shapes `1 nibble set` at each of 4 nibble slots x all 16 nibble values; high half = random | [x] |
| C18 | `rev16` | no options; palindromic low halves (values that are their own 16-bit bit-reversal), so `rev16(x) == x`; high half = random | [x] |
| C19 | `rev16` | no options; **full-range seeded-random 32-bit arguments**, 4 000 000 samples, both halves random and independent | [x] |
| C20 | `rev16` | no options; adversarial "sparse/dense" random shapes — arguments drawn from low-Hamming-weight and high-Hamming-weight distributions rather than uniform, so rare bit patterns are hit | [x] |
| C21 | `rev16` | no options; composed pipeline — `rev16` applied **twice** (`rev16(rev16(x))`), driven through the `.so` boundary each time, over exhaustive low halves and random high halves; catches divergence that only shows up when the output of one call feeds the next | [x] |
| C22 | `rev16` | no options; composed pipeline — `rev16` chained 8 deep over seeded-random inputs, alternating each intermediate with a random high-half injection so the discard path is re-entered mid-pipeline | [x] |
| C23 | `rev16` | no options; call-order / statefulness — the same argument set replayed in shuffled order and interleaved between the C and Rust libraries, asserting results are order-independent | [x] |
| C24 | `rev16` | no options; concurrent invocation from 8 threads against both `.so`s simultaneously over random inputs | [x] |

## Feature combinations

`translation/Cargo.toml` declares **no `[features]` table**, so the complete set
of feature combinations is:

| combo | cargo invocation |
|-------|------------------|
| default (empty) | `cargo test` |
| no-default-features (identical, since there are no features) | `cargo test --no-default-features` |
| all-features (identical) | `cargo test --all-features` |

All three are executed by `run_all_configs.sh`; there is no `#[cfg(feature)]` in
`src/lib.rs` (`grep -c 'cfg(feature' src/lib.rs` → 0), so they compile the same
code, and all three are run to prove it.

## Row C25 — the exhaustive superset

| # | entry point(s) | configuration (options set + input shape) | [ ] |
|---|----------------|--------------------------------------------|-----|
| C25 | `rev16` | no options; **every one of the 4 294 967 296 `u32` values**, i.e. the complete input domain | [x] |

`rev16` takes a single `uint32_t` by value and has no options and no state, so
the configuration space is exactly the 2^32 argument values. Row C25
(`tests/valid_paths.rs::exhaustive_all_2pow32_arguments`, `#[ignore]`d for
runtime, ~26 s) drives all of them through both `.so` exports and requires
byte-identical results. It therefore **strictly subsumes rows C1–C24 and every
row of `ERRORS.md`**: the rows above are retained because they document the
axes the C branches on and localise a failure to a specific shape, but C25 is
the proof of total equivalence.

```
$ cargo test --offline --test valid_paths -- --ignored --nocapture
[EXHAUSTIVE] verified all 4294967296 u32 arguments identical
test exhaustive_all_2pow32_arguments ... ok
```

## Suite adequacy

Passing tests were not taken at face value. The suite was mutation-tested: six
single-line mutations were injected into `src/lib.rs` and the suite re-run.
All four behaviour-changing mutations turned it red (8–9 failing tests each) and
both behaviour-preserving mutations correctly stayed green — the latter confirmed
equivalent by an independent exhaustive 2^32 check, not by assumption. See
`verification/README.md`.
