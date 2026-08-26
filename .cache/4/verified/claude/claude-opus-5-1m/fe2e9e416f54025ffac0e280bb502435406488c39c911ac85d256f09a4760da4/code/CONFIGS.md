# CONFIGS.md — Configuration-surface table (Phase A / Phase B)

Derived mechanically from the branches the C source actually takes.

## Build-time configuration axes: none

`c_src/CMakeLists.txt` contains no `option()`, no `add_definitions`, no
`target_compile_definitions`, and no `CMAKE_BUILD_TYPE` switching; `lib.c` and
`lib.h` contain **zero** `#ifdef` / `#if` / `#ifndef` preprocessor branches.

```
$ grep -cE '^\s*#\s*(if|ifdef|ifndef|else|elif)' c_src/src/lib.c c_src/include/lib.h
c_src/src/lib.c:0
c_src/include/lib.h:0
```

Consequently `Cargo.toml` declares no features, and the complete set of valid
feature combinations is:

| # | feature combination | cargo invocation | verified |
|---|---------------------|------------------|----------|
| F1 | *(empty — the only one)* | `cargo test --no-default-features` | [x] |
| F2 | *(empty, via the default resolution)* | `cargo test` | [x] |

Each is run under **both** the `dev` and the `release` profile, giving four
build configurations in total — all four pass (`./run_all.sh`).

Both spellings are exercised by `./run_all.sh`, together with the `dev` and
`release` profiles (`release` additionally enables `panic = "abort"` and
optimisation, which is a genuinely different code path for the Rust `.so`).

## Public entry points (full set, lowest level included)

`c_src/include/lib.h` declares exactly **one** function, and it is also the
lowest-level primitive — there is no convenience/one-shot wrapper layer to hide
behind:

```c
int bitwriter_add(tflac_bitwriter *bw, tflac_u32 bits, tflac_uint val);
```

The second half of the surface is the **struct ABI** (`struct tflac_bitwriter`:
`val`, `bits`, `pos`, `len`, `tot`, `buffer`), because the caller both supplies
and observes that state. Rows below therefore drive `bitwriter_add` directly
with fully-controlled pre-state and compare the entire 32-byte post-state.

## Runtime axes the C code branches on

| axis | where in `lib.c` | distinct states |
|---|---|---|
| A `bits` magnitude | L8 `val <<= 64 - bits`; L11 guard; L13 `min`; L18 `bits -= b` | `0`; `1..62`; `63`; `64`; `65..127`; `≥128`; wrap-inducing; `0xFFFFFFFF` |
| B `bw->bits` (accumulator width) | L11 guard; L12 `63 - bw->bits`; L14/L21 `val >> bw->bits` | `0`; `1..62`; `63`; `64`; `>64`; `0xFFFFFFFF` |
| C loop taken? | L11 `bw->bits + bits >= 64` — **32-bit wrapping** add | not entered / entered |
| D iteration count | L11 `&& i < 100` | `0`, `1`, `2`, `3..99`, **exactly 100 (cap hit)** |
| E `b > bits` ternary | L13 | takes `bits` / takes `b` |
| F out-of-range shift count (hardware masks to 6 bits) | L8, L14, L17, L21 | `64-bits ≥ 64`; `bw->bits ≥ 64`; `b ≥ 64`; none |
| G `bw->val` pre-state | L14 `|=`, L16 `&= mask`, L21 `|=` | `0`; `0xFFFF…FF`; random; bit-0 set (only bit `mask` clears) |
| H `bw->tot` pre-state | L9 `bw->tot += bits` (wraps mod 2^32) | `0`; near/at `0xFFFFFFFF` |
| I `pos` / `len` / `buffer` | **never read, never written** | must be byte-identical afterwards, incl. `buffer = NULL` and a real pointer |
| J call arity | caller-driven state carry-over | single call / long randomised call sequence |

## The configuration-surface table

One row per combination the C treats differently (cross-product of A–J, pruned
to the distinguishable cases). Every row is driven with **many randomised
inputs** from a fixed-seed SplitMix64 PRNG (not one hand-picked value), and both
`.so`s are compared on `(return value, all 6 struct fields)` byte-for-byte.

| #  | entry point(s) | configuration (options set + input shape) | [x] |
|----|----------------|--------------------------------------------|-----|
| 1  | `bitwriter_add` | **struct ABI**: size 32 / align 8 / offsets `val`0 `bits`8 `pos`12 `len`16 `tot`20 `buffer`24 — probe by writing a 32-byte pattern and diffing which bytes each `.so` mutates | [x] |
| 2  | `bitwriter_add` | A=`1..62` (mid-range), B=`0`, C=loop **not** entered, D=0 iters, F=none — the plain "fits in the accumulator" shape | [x] |
| 3  | `bitwriter_add` | A=`1..62`, B=`1..62` with `bw->bits + bits < 64`, C=not entered, G=random `val` — accumulate into a partially-filled writer | [x] |
| 4  | `bitwriter_add` | A=`1..63`, B chosen so `bw->bits + bits == 63` — boundary one **below** the guard | [x] |
| 5  | `bitwriter_add` | A=`1..63`, B chosen so `bw->bits + bits == 64` exactly — guard boundary, loop entered | [x] |
| 6  | `bitwriter_add` | A=`1..63`, B chosen so `bw->bits + bits == 65` — one **past** the guard boundary | [x] |
| 7  | `bitwriter_add` | A=`0`, B=`0..63`, F=`64-bits == 64` ⇒ **out-of-range left shift masked to 0**; C=not entered (`bw->bits < 64`) | [x] |
| 8  | `bitwriter_add` | A=`0`, B=`≥64`, F=both `64-bits` and `bw->bits` out of range; C=entered with `bits == 0` ⇒ E takes `bits`(=0) ⇒ D hits the 100-cap | [x] |
| 9  | `bitwriter_add` | A=`63`, B=`0`, E=`b`(=63) vs `bits`(=63) tie ⇒ ternary takes `b`; C=entered | [x] |
| 10 | `bitwriter_add` | A=`64` exactly, B=`0`, F=`64-bits == 0` (no mask), C=entered, E takes `b`=63 then second iteration | [x] |
| 11 | `bitwriter_add` | A=`64`, B=`1..63`, C=entered, multiple iterations with `b` shrinking | [x] |
| 12 | `bitwriter_add` | A=`65..127` (past max width), F=`64-bits` masked to `63..1`, C=entered | [x] |
| 13 | `bitwriter_add` | A=`≥128` / exact multiples of 64 (`128`, `192`, `256`) — mask makes `64-bits` land back on 0 | [x] |
| 14 | `bitwriter_add` | A=`0xFFFFFFFF` (`UINT32_MAX`), B=`0` — extreme width, `tot` wraps, loop runs to the cap | [x] |
| 15 | `bitwriter_add` | C=**32-bit wraparound makes the guard false**: B=`64`, A=`0xFFFFFFC0` so `bw->bits + bits == 0x00000000 < 64` ⇒ loop skipped although `bits` is huge | [x] |
| 16 | `bitwriter_add` | D=**exactly 100 iterations (cap)** via B=`63`, A=`1` ⇒ `b = 63-63 = 0`, no progress; verify post-state after the cap | [x] |
| 17 | `bitwriter_add` | D=cap via B=`63`, A=`2..64` — `b = 0` again, but a non-zero `bits` remains for the tail `bw->val |= val >> bw->bits` | [x] |
| 18 | `bitwriter_add` | D=cap via B=`>64` (`64`, `65`, `1000`, `0xFFFFFFFF`) — `63 - bw->bits` **underflows**, E takes `bits`, then stalls at `b = 0` | [x] |
| 19 | `bitwriter_add` | B=`0xFFFFFFFF`, A=`1` ⇒ `bw->bits += b` **wraps mod 2^32**; F=`bw->bits` masked to 63 for the shifts | [x] |
| 20 | `bitwriter_add` | G=`bw->val` pre-set to `0xFFFFFFFFFFFFFFFF` and to values with **bit 0 set** — the only axis where `bw->val &= mask` (clearing bit 0) is observable | [x] |
| 21 | `bitwriter_add` | G=`val` argument `0`, `1`, `u64::MAX`, alternating `0xAAAA…`/`0x5555…`, single-bit-set for every bit `0..63` | [x] |
| 22 | `bitwriter_add` | H=`bw->tot` pre-set to `0xFFFFFFFF` / `0xFFFFFF00` with assorted `bits` ⇒ **`tot` counter overflow wraps** | [x] |
| 23 | `bitwriter_add` | I=`pos`/`len`/`buffer` pre-set to non-zero junk, `len == 0` with `pos > len`, and `buffer` = `NULL` vs a real heap pointer ⇒ must be preserved verbatim | [x] |
| 24 | `bitwriter_add` | J=**long randomised call sequence** (2 000 chained calls per seed) carrying `bw` state forward — catches divergence that only accumulates across the composed pipeline | [x] |
| 25 | `bitwriter_add` | full-range **unconstrained fuzz**: `bits`, `val` and all 6 pre-state fields drawn uniformly at random (1 000 000 cases) — the cross-product safety net for combinations not enumerated above | [x] |
| 26 | `bitwriter_add` | **exhaustive** sweep of the two structural axes: every `bits ∈ 0..=130` × every `bw->bits ∈ 0..=130`, with several `val`/`bw->val` patterns each | [x] |
| 27 | `bitwriter_add` | **near-2^32 `bw->bits` band**, where `63 - bw->bits` is a *small positive* value (`64, 65, … 400`) rather than a huge one, so the ternary takes `b = 63 - bw->bits` **while `bw->bits >= 64`** and `bw->bits += b` wraps to exactly `63`; swept with `bits` below / equal to / above that `b` | [x] |
| 28 | `bitwriter_add` | the **three loop regimes** driven explicitly and separately: (a) guard false ⇒ 0 iterations; (b) guard true but `b == 0` on entry ⇒ 100 idempotent spins; (c) guard true with **exactly one progressing iteration** followed by 99 idempotent spins | [x] |
| 29 | `bitwriter_add` | `b >= 64` inside the loop (`bw->bits >= 64` **and** `bits >= 64`, so `b = bits`), making the in-loop `val <<= b` an out-of-range shift that the hardware masks | [x] |

## Note on loop-iteration structure (established by brute force)

A brute-force sweep over 1 313 316 structured and 40 000 000 random
`(bw->bits, bits)` pairs shows the `while` loop performs **at most one
progressing iteration** (`b != 0`); as soon as `b` becomes `0` the body is
idempotent:

```
bw->val = ((bw->val | (val >> bw->bits)) & mask)   // same operands every spin
bw->bits += 0 ; val <<= 0 ; bits -= 0
```

so spins 2…100 cannot change any observable state. Two consequences, both
recorded in `mutation_check.sh` as *provably equivalent mutants*:

* the exact value of the `i < 100` cap is unobservable for any cap `>= 2`; and
* `b = b > bits ? bits : b` and `b >= bits ? bits : b` are identical, because
  they differ only when `b == bits`, where both yield the same number.

Rows 27–29 exist to pin down the *reachable* loop regimes precisely rather than
relying on the cap itself being observable.
