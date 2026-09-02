# CONFIGS.md — Phase A configuration-surface table

Derived mechanically from the branches and arithmetic the C code actually
performs, cross-checked against the `-O0` disassembly of the built `.so`.

## Public entry points

`include/lib.h` exposes exactly one function; it *is* the lowest-level entry
point (there are no convenience or one-shot wrappers to hide behind):

```c
int bitwriter_add(tflac_bitwriter *bw, tflac_u32 bits, tflac_uint val);
```

State is carried entirely in the caller-owned `struct tflac_bitwriter`, so the
"configuration" of a call is the tuple *(incoming struct state, `bits`, `val`)*.
Realistic use is a *sequence* of calls that accumulates state, so the table
includes multi-call pipeline rows as well as single-call rows.

## Runtime options / flags

None. There is no options struct, no mode enum, no flag field, and no
`#ifdef` / `#if` / `switch` anywhere in `src/lib.c` or `include/lib.h`
(`grep -nE '#if|#ifdef|switch|case '` → no matches). The crate declares no
Cargo `[features]`. So the axes below are purely *input shape* and
*incoming state*, which is what the C branches on.

## Axes the C actually branches on

Numbered so table rows can cite them.

* **X1 — `bits` class**, driving `val <<= (64 - bits)` (shift count masked to 6
  bits by the emitted `shlq %cl`) and the loop trip count:
  `0`, `1`, mid `2..62`, `63`, `64`, `65`, `>64` small (`100`, `127`, `128`),
  huge (`0x80000000`, `0xFFFFFFFF`), and values chosen so `bw->bits + bits`
  wraps `u32`.
* **X2 — `bw->bits` (incoming) class**, driving `b = (u32)(63 - bw->bits)`
  (32-bit underflow when `> 63`), the `val >> bw->bits` shift-count masking,
  and the loop condition: `0`, `1`, mid `2..62`, `63` (→ `b == 0`), `64`, `65`,
  `>64`, `0xFFFFFFFF`.
* **X3 — ternary `b = b > bits ? bits : b`** (`cmovbe`, unsigned): the
  `b <= bits` arm vs the `b > bits` arm.
* **X4 — loop trip count**: 0 iterations, exactly 1, 2..99, and exactly the
  `i < 100` cap.
* **X5 — `val` class**: `0`, `1`, all-ones `0xFFFF...F`, only-low-bit-set,
  only-high-bit-set, alternating `0xAAAA.../0x5555...`, random.
  Bit 0 matters specifically because the loop body applies
  `bw->val &= 0xFFFFFFFFFFFFFFFE`, clearing it every iteration.
* **X6 — `bw->val` (incoming)**: `0`, all-ones, random — it is OR-accumulated,
  never reset.
* **X7 — `bw->tot` (incoming)**: far from overflow vs near `0xFFFFFFFF`
  (unsigned wrap).
* **X8 — untouched fields `pos`, `len`, `buffer`**: zeroed vs random/garbage
  (incl. null and non-null `buffer`); must come back byte-identical.
* **X9 — call-sequence shape**: single call vs many chained calls on one
  accumulating struct (the composed pipeline).

## Configuration-surface table

Every row is exercised with **many randomized inputs** (fixed-seed splitmix64)
on the free axes, and both `.so`s are compared byte-for-byte over the full
32-byte struct plus the `int` return value.

| # | entry point(s) | configuration (options set + input shape) | [x] |
|---|----------------|-------------------------------------------|-----|
| C1 | `bitwriter_add` | X1=`0`, X2 swept `0..=64`+huge, X5/X6 randomized — zero-length add, loop entered only via `bw->bits` alone | [x] |
| C2 | `bitwriter_add` | X1=`1`, X2=`0`, X5/X6 randomized — single-bit add into empty writer, 0 loop iterations (X4=0) | [x] |
| C3 | `bitwriter_add` | X1=`1`, X2=`63` — `b==0`, X3 = `b<=bits` arm, X4 = 100-iteration cap | [x] |
| C4 | `bitwriter_add` | X1 mid `2..62` randomized, X2=`0`, X5/X6 randomized — common no-loop path | [x] |
| C5 | `bitwriter_add` | X1 mid randomized, X2 mid `1..62` randomized with `bw->bits+bits < 64` — X4=0, accumulate-only | [x] |
| C6 | `bitwriter_add` | X1 mid randomized, X2 mid randomized with `64 <= bw->bits+bits < 128` — X4=1 single loop iteration, X3 both arms | [x] |
| C7 | `bitwriter_add` | X1=`63`, X2 swept `0..=64` — near-word-width add | [x] |
| C8 | `bitwriter_add` | X1=`64`, X2=`0` — X1 at word width, shift-by-0 on entry, loop entered | [x] |
| C9 | `bitwriter_add` | X1=`64`, X2 swept `0..=64`+`0xFFFFFFFF` — full-word add against every incoming `bits` class | [x] |
| C10 | `bitwriter_add` | X1=`65` (one past range), X2 swept `0..=64` — out-of-range shift `64-65` | [x] |
| C11 | `bitwriter_add` | X1 in `{100,127,128,255,256,1000}`, X2 swept — X4 large / capped, repeated `bits -= b` | [x] |
| C12 | `bitwriter_add` | X1 huge (`0x80000000`, `0xFFFFFFFF`, random ≥ 2^31), X2=`0` — X3 = `b > bits` arm impossible→`b=63`, X4 capped at 100 | [x] |
| C13 | `bitwriter_add` | X2=`64` exactly (out-of-range incoming state), X1 swept `0..=65` — `b=(u32)(63-64)=0xFFFFFFFF`, then clamps to `bits` | [x] |
| C14 | `bitwriter_add` | X2 `>64` (`65`,`100`,`0xFF`,`0x10000`), X1 randomized — `b` underflow + masked `val >> bw->bits` | [x] |
| C15 | `bitwriter_add` | X2=`0xFFFFFFFF`, X1 chosen so `(u32)(bw->bits+bits)` **wraps below 64** — loop skipped entirely despite huge operands | [x] |
| C16 | `bitwriter_add` | X2=`0xFFFFFFFF`, X1 chosen so the wrapped sum is **≥ 64** — loop entered from wrapped state | [x] |
| C17 | `bitwriter_add` | X5 = boundary vals `{0, 1, 2, 0x8000000000000000, 0xFFFFFFFFFFFFFFFF, 0xAAAA…, 0x5555…}` × X1 in `{0,1,32,63,64,65}` — exercises the `&= ~1` mask interaction with bit 0 | [x] |
| C18 | `bitwriter_add` | X6 = `bw->val` all-ones / random, X1/X2 randomized — OR-accumulation into a dirty word, mask clearing bit 0 | [x] |
| C19 | `bitwriter_add` | X7 = `bw->tot` near `0xFFFFFFFF` × X1 huge — `tot` unsigned wrap | [x] |
| C20 | `bitwriter_add` | X8 = `pos`/`len`/`buffer` filled with random garbage (incl. `pos > len`, non-null bogus `buffer`) × X1/X2 randomized — must be returned byte-identical | [x] |
| C21 | `bitwriter_add` | X9 = chained pipeline: 64 sequential calls on one struct, all `bits` in `1..=32` (realistic bit-packing), state carried across calls | [x] |
| C22 | `bitwriter_add` | X9 = chained pipeline: 64 sequential calls with fully random `bits` in `0..=0xFFFFFFFF` and random `val` — state drifts into out-of-range `bw->bits` and stays there | [x] |
| C23 | `bitwriter_add` | X9 = chained pipeline: alternating `bits=63`/`bits=1` calls, forcing repeated 100-iteration-cap rows back to back | [x] |
| C24 | `bitwriter_add` | Fully unconstrained fuzz: all 32 struct bytes random + random `bits` + random `val`, 200k cases — cross-product catch-all for combinations not individually named | [x] |

## Phase B result

All 24 rows pass: `cargo test --test phase_b_configs` → **24 passed, 0 failed**,
against both the release and debug cdylibs, and against the C `.so` rebuilt at
`-O0/-O1/-O2/-O3/-Os` (the C code performs out-of-range shifts, so
optimization-invariance is worth confirming rather than assuming — it holds).

Each row drives many randomized inputs from a fixed-seed splitmix64 stream, and
compares the **full 32-byte struct image plus the `int` return**, so a change to
any field — or to padding — fails the test.

## Test-suite sensitivity

Passing rows only mean something if the suite can detect a wrong translation.
`tests/phase_e_mutation_control.rs` applies nine plausible mistranslations and
requires each to be caught by these same generators:

| mutant | detected on |
|--------|-------------|
| forget `bw->val &= mask` | 26575/50000 inputs |
| loop guard computed in 64-bit instead of wrapping `u32` | 23/50000 |
| clamp `b` at 0 instead of `u32` underflow of `63 - bw->bits` | 8237/50000 |
| treat out-of-range shift as yielding 0 instead of masking to 6 bits | 39007/50000 |
| saturating instead of wrapping `bw->tot += bits` | 9722/50000 |
| saturating instead of wrapping `bw->bits += bits` | 46/50000 |
| signed comparison in `b > bits ? bits : b` | 4785/50000 |
| skip the loop body entirely (cap 0) | detected |
| reorder the tail `|=` and `bw->bits += bits` | 4799/50000 |

Two further findings recorded there:

* `n2_faithful_model_agrees_with_c` anchors the above — the unmutated model
  matches the C `.so` on all 50000 inputs, so each detection is attributable to
  the mutation alone.
* `n4_iteration_cap_at_least_one_is_unobservable` proves that **every loop
  iteration after the first is a no-op**, so caps of 1, 101 and 10000 are all
  behaviourally identical to the real 100. Reason: `b = (tflac_u32)(63 -
  bw->bits)` means iteration 1 either leaves `bw->bits == 63` exactly (including
  the `bw->bits > 63` case, where the `u32` underflow makes `bw->bits + b` wrap
  back to 63) or clamps `b` to `bits` and drives `bits` to 0. From iteration 2
  on `b == 0`, so `bw->bits`, `bits` and `val` stop changing and the only new
  contribution is `bw->val |= (val >> 63)` — at most bit 0 — which the next
  `bw->val &= mask` clears again. Rows C3/C11/C12/C23 still cover the capped
  path; the cap value itself is simply not observable, which is why the
  cap-mutants at 1/101/10000 are equivalent mutants rather than coverage gaps.
  `n5_iteration_cap_is_actually_reached` confirms the cap is live code (the
  `bw->bits=63, bits=1` case runs exactly 100 iterations).
