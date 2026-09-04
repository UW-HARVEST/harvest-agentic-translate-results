# CONFIGS.md — configuration surface table (Phase B)

## Mechanical derivation of the axes

The C library has **no runtime options, flags, modes, or `#ifdef`s** — the grep
in `ERRORS.md` shows zero `if`/`switch`/`#ifdef` branches outside loop headers.
The axes it *does* branch on are therefore the entry points, the one scalar
parameter, the shape of the global state, and the composition count:

**Axis A — entry point (the full public set, lowest level first).**
`nm -D` gives exactly three:

* `A1` `array` — the 0x100000-byte global; read/written directly by a consumer
* `A2` `perform_expensive_operations()` — the low-level kernel pass, `f^100`
  applied element-wise to whatever is in `array` (no wrapper, no seeding)
* `A3` `long_exec(seed)` — the one-shot convenience entry: `srand` + fill with
  `rand()` + 2000 × `A2` + XOR fold + `printf("%d\n")`

**Axis B — `seed` (the only parameter).** The C passes it straight to glibc
`srand`, which special-cases `0`, so the distinct classes are: `0`, `1`, small
odd/even, `INT_MAX`, `2^31` (sign bit set), `UINT_MAX`, and arbitrary 32-bit.

**Axis C — `array` content shape.** The kernel
`x=x*3+7; x^=x>>3; x=x-(x<<1); x=x/2+x%7` distinguishes: sign of `x` (arithmetic
`>>`, truncating `/`, sign-following `%`), overflow of `*3+7` and `<<1`,
multiples of 7, zero, `INT_MIN`/`INT_MAX`, and the `[0, 2^31)` sub-range that
`rand()` actually produces (`A3` can never feed a negative value into the first
pass, but later passes do).

**Axis D — composition count `k`** (how many times `A2` runs before the state is
observed): `0, 1, 2, 3, 20, 82, 100, 2000`. `k = 2000` is what `A3` does. `k ≥ 82`
(`n = k·100 ≥ 8192`) is the point where the Rust crate switches from naive
iteration to its cycle-accelerated path, so it is a real behavioural boundary on
the Rust side that must produce identical bytes.

**Axis E — state interaction / ordering.** `array` is persistent global state:
`A2` before any `A3`; `A3` then `A2`; `A3` twice; caller-poisoned `array` then
`A3`; interleaved C/Rust calls.

**Axis F — cargo features.** `debug-stats` (stderr only) and default. Every row
below is run under both.

**Observable compared in every row (byte-for-byte):** all 262144 `int`s of
`array` after the call (1 MiB `memcmp`), *and* the exact stdout bytes when `A3`
is involved.

**Randomisation:** rows marked *(random)* are run with many independently
generated inputs from a fixed-seed SplitMix64 generator (seed `0x5EED_1234_ABCD`
), not a single hand-picked value, and each round is compared independently.

## Table

| # | entry point(s) | configuration (options set + input shape) | [ ] |
|---|----------------|--------------------------------------------|-----|
| 1 | `A2` | `array` = full-width uniform random `i32` (all four sign/parity classes present), `k = 1` — *(random, 64 rounds)* | [x] |
| 2 | `A2` | `array` = uniform random in `[0, 2^31)` i.e. exactly the `rand()` range, `k = 1` — *(random, 32 rounds)* | [x] |
| 3 | `A2` | `array` = all zeros (`.bss` initial shape), `k = 1` | [x] |
| 4 | `A2` | `array` = all `INT_MAX`, `k = 1` (overflow of `x*3+7` on every element) | [x] |
| 5 | `A2` | `array` = all `INT_MIN`, `k = 1` (overflow of `x<<1` and of the `-x` idiom) | [x] |
| 6 | `A2` | `array` = all `-1`, `k = 1` (negative operand of `>>`, `/`, `%`) | [x] |
| 7 | `A2` | `array` = exhaustive small window `-131072 ..= 131071` laid out over all 262144 slots, `k = 1` (every sign/parity/mod-7 residue combination near zero) | [x] |
| 8 | `A2` | `array` = the 262144 values `INT_MIN + i` (extreme negative band), `k = 1` | [x] |
| 9 | `A2` | `array` = the 262144 values `INT_MAX - i` (extreme positive band), `k = 1` | [x] |
| 10 | `A2` | `array` = multiples of 7 and neighbours (`7i`, `7i±1`) interleaved, `k = 1` (`x % 7 == 0` boundary) | [x] |
| 11 | `A2` | `array` = powers of two and their negations, `±(1<<b)` for all `b`, cycled, `k = 1` (shift/overflow boundaries) | [x] |
| 12 | `A2` | `array` = random, `k = 0` (no call — array must be untouched by both) | [x] |
| 13 | `A2` | `array` = random, `k = 2` (composition; catches drift that a single pass hides) — *(random, 8 rounds)* | [x] |
| 14 | `A2` | `array` = random, `k = 3` — *(random, 4 rounds)* | [x] |
| 15 | `A2` | `array` = random, `k = 20` (`n = 2000`) — *(random, 2 rounds)* | [x] |
| 16 | `A2` | `array` = random, `k = 81` (`n = 8100`, last count *below* the Rust accelerator threshold) | [x] |
| 17 | `A2` | `array` = random, `k = 82` (`n = 8200`, first count *at/above* the accelerator threshold) | [x] |
| 18 | `A2` | `array` = random, `k = 100` (`n = 10000`, well inside the accelerated regime) | [x] |
| 19 | `A2` | `array` = all zeros, `k = 100` (degenerate orbit repeated: single shared cycle, maximal memo coalescing) | [x] |
| 20 | `A1` + `A2` | write only `array[0]` and `array[262143]`, zeros elsewhere, `k = 1` — boundary indices of the exported object | [x] |
| 21 | `A1` | `array` object size/alignment and raw byte read-back without any call | [x] |
| 22 | `A3` | `seed = 0` (glibc `srand(0)` special case), full pipeline `n = 200000`: stdout **and** final 1 MiB `array` | [x] |
| 23 | `A3` | `seed = 1` | [x] |
| 24 | `A3` | `seed = 7` | [x] |
| 25 | `A3` | `seed = 255` | [x] |
| 26 | `A3` | `seed = 65535` | [x] |
| 27 | `A3` | `seed = 42` — stdout **and** final 1 MiB `array` | [x] |
| 28 | `A3` | `seed = 12345` | [x] |
| 29 | `A3` | `seed = 3` | [x] |
| 30 | `A3` | `seed = 100` | [x] |
| 31 | `A3` | `seed = 999983` | [x] |
| 32 | `A3` | `seed = 2147483648` (`2^31`, sign bit set) | [x] |
| 33 | `A3` | `seed = 4294967295` (`UINT_MAX`) — stdout **and** final 1 MiB `array` | [x] |
| 34 | `A3` + `A2` | `A3(seed)` then `A2()` — the low-level pass applied on top of the post-pipeline state (`n = 200100`) | [x] |
| 35 | `A2` + `A3` | `A2()` on poisoned state, then `A3(seed)` — the seeded fill must overwrite every element | [x] |
| 36 | `A3` × 2 | `A3(seed)` twice, same seed — reseeding must make the second call reproduce the first exactly | [x] |
| 37 | `A3` × 2 | `A3(s1)` then `A3(s2)`, `s1 != s2` — no carry-over between calls | [x] |
| 38 | `A2` (interleaved) | random `array`, C and Rust `.so`s driven alternately in the same process (shared glibc `stdout`/`rand` state) | [x] |
| 39 | `A2` | `array` = the *fixed points and short-cycle members* of the kernel discovered by search, `k = 1` and `k = 100` | [x] |
| 40 | `A2` | `array` = random, `k = 1`, run *after* a `debug-stats` build printed to stderr (feature must not perturb stdout or `array`) | [x] |

## Cost note

Rows 22–37 and 34–37 need the C `.so`'s real `long_exec`, which is
~470 s per call (2000 × 262144 × 100 kernel steps at `-O0`). Those C reference
values are therefore captured **once**, out of band, by
`tests/ground_truth/capture.sh` (which dlopens the C `.so` directly and records
both the printed line and the final 1 MiB `array` image); the differential test
then compares the Rust `.so`'s live output against those recorded C bytes. The
capture program calls the C `.so` and nothing else, so the recorded bytes are C
ground truth, not a Rust-derived expectation. Row 34 and rows 36–37 additionally
run their *whole* sequence live against the C `.so` in the `slow_live_c` tests
(`cargo test -- --ignored`), which take ~8–16 min each.

All 40 rows pass under both feature combinations. See `tests/configs.rs`.

## Beyond the table: exhaustive verification of the low-level entry point

The rows above sample the input space. For `perform_expensive_operations` the
space can be closed completely, because the function is `f^100` applied
element-wise and each element's result depends on nothing but that element's
value: there are only 2^32 possible inputs, and 2^32 / 262144 = 16384
array-fulls.

`./exhaustive.sh` drives **every one** of the 2^32 `int` values through both
`.so`s (dlopen both, `memcpy` the chunk into each exported `array`, call each
`perform_expensive_operations`, `memcmp` the 1 MiB results), sharded across
processes:

```
### total chunks=16384 (expected 16384)  total mismatches=0
### EXHAUSTIVE: all 2^32 inputs identical
```

This makes rows 1-11 and 20-21 exhaustive rather than sampled: the kernel and
the exported `array` ABI are proven equivalent for the entire input domain,
under both feature combinations. What the sampled rows still contribute is the
*composition* axis (`k > 1`) and the `long_exec` pipeline, which exhaustive
enumeration cannot reach.

## Test inventory

| file | rows covered | notes |
|---|---|---|
| `tests/common/mod.rs` | -- | harness: dlopen both `.so`s, stdout capture via `dup2`, SplitMix64 |
| `tests/smoke.rs` | -- | harness self-check: symbols resolve, `array` writable at both boundaries |
| `tests/configs.rs` | 1-40 | Phase B |
| `tests/errors.rs` | `ERRORS.md` 1-21 | Phase C |
| `tests/slow_live_c.rs` | 27, 34, 36, 37 live | `#[ignore]`d; runs the real C `long_exec` (~8 min/call) |
| `tests/ground_truth/capture.sh` | -- | records C reference output for the slow pipeline |
| `verify.sh` | all | symbol diff + full suite for every feature combination |
| `exhaustive.sh` | 1-11, 20-21 | all 2^32 inputs to `perform_expensive_operations` |
