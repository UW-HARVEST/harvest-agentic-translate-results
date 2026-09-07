# CONFIGS.md — Configuration-surface table (valid / in-range inputs)

Derived mechanically from `c_src/src/lib.c` and `c_src/include/lib.h`.

## Axes the C code actually branches on

There are **no** compile-time options: `grep -nE '#if|#ifdef|#ifndef' src/lib.c
include/lib.h` matches nothing, and `translation/Cargo.toml` has no
`[features]` table. There are no runtime option/mode/flag fields either — the
public API is a single function and a single POD struct.

The branch points in the C, verbatim:

```
src/lib.c:11:    while ((bw->bits + bits >= (8 * sizeof(tflac_uint))) && i < 100) {
src/lib.c:13:        b = b > bits ? bits : b;
```

so the axes are:

| axis | name | values the code distinguishes |
|------|------|-------------------------------|
| **P** | `bits` argument | `0`; `1..=31`; `32..=62`; `63`; `64`; `65..=127`; large (`>= 2^16`); `0x8000_0000`; `UINT32_MAX` |
| **S** | `bw->bits` (accumulator fill state) | `0`; `1..=31`; `32..=62`; `63`; `64`; `65..=127`; large; `UINT32_MAX` |
| **L** | loop entry: `(u32)(bw->bits + bits) >= 64` | not entered / entered (entered ⇒ runs to the `i == 100` cap, because `b = 63 - bw->bits` drives `bw->bits` to `63` and then yields `b == 0` forever) |
| **W** | `bw->bits + bits` wraps `tflac_u32` | no wrap / wrap (changes the sense of the loop test) |
| **C** | ternary `b > bits` | `b <= bits` (use `b`) / `b > bits` (clamp to `bits`, incl. when `63 - bw->bits` wrapped to a huge value) |
| **V** | `val` argument shape | `0`; `UINT64_MAX`; only the low `bits` bits set; only the top bit set; a single random bit; uniform random |
| **A** | `bw->val` accumulator seed | `0`; `UINT64_MAX`; uniform random (tests the `\|=` merge and the `&= mask` clear of bit 0) |
| **T** | `bw->tot` seed | `0`; uniform random; near `UINT32_MAX` (u32 wrap on `tot += bits`) |
| **U** | untouched fields `pos`, `len`, `buffer` | zero / random / wild non-null pointer — must be returned byte-identical |
| **N** | call arity | one-shot single call / composed sequence of many calls against the same `bw` (how a real bit-writer consumer drives it) |

## Entry points

`bitwriter_add` is the **only** public entry point (`nm -D` on the C `.so`
exports exactly one symbol; `include/lib.h` declares exactly one function).
It is simultaneously the lowest-level and the highest-level API — there are no
convenience wrappers to hide behind. Every row below calls it through the
`.so` export.

`c_src/CMakeLists.txt` builds only `add_library(... SHARED src/lib.c)` — there
is **no binary/driver executable**, so the "compare binary stdout" clause of
Phase B does not apply.

## Rows

Every row is executed with **many randomized inputs** (see `ITERS` in
`tests/differential.rs`, seeded `SplitMix64`, fixed seed `0x5EED_1234_ABCD_F00D`),
and asserts that the C and Rust `.so` agree on the return value **and** on all
32 bytes of `tflac_bitwriter` after the call.

| #  | entry point | configuration (options set + input shape) | [x] |
|----|-------------|--------------------------------------------|-----|
| 1  | `bitwriter_add` | S=0, P=0, V/A/T/U random — L not entered, degenerate shift-by-64 | [x] |
| 2  | `bitwriter_add` | S=0, P=`1..=63` random, V/A/T/U random — L not entered, canonical happy path | [x] |
| 3  | `bitwriter_add` | S=0, P=64, V/A/T/U random — L entered, hits `i==100` cap | [x] |
| 4  | `bitwriter_add` | S=0, P=`65..=127` random — L entered, out-of-width shift count | [x] |
| 5  | `bitwriter_add` | S=`1..=31` random, P=`1..=31` random with S+P<64 — L not entered | [x] |
| 6  | `bitwriter_add` | S=`32..=62` random, P=`1..=63` random with S+P<64 — L not entered, high fill | [x] |
| 7  | `bitwriter_add` | S=`1..=62` random, P=`1..=63` random with S+P>=64 — L entered, C=`b<=bits` | [x] |
| 8  | `bitwriter_add` | S=63, P=0 — boundary S, L not entered | [x] |
| 9  | `bitwriter_add` | S=63, P=1 — sum exactly 64, L entered with `b == 0` immediately | [x] |
| 10 | `bitwriter_add` | S=63, P=`2..=63` random — L entered, `b == 0`, C clamps | [x] |
| 11 | `bitwriter_add` | S=64, P=0 — `b = 63-64` wraps to `0xFFFFFFFF`, C clamps to `0` | [x] |
| 12 | `bitwriter_add` | S=64, P=`1..=64` random — wrapped `b` clamped to `bits` | [x] |
| 13 | `bitwriter_add` | S=`65..=127` random, P=0 — L entered, `>> (S & 63)` | [x] |
| 14 | `bitwriter_add` | S=`65..=127` random, P=`1..=127` random — wrapped `b`, wrapped shifts | [x] |
| 15 | `bitwriter_add` | S=`UINT32_MAX`, P=1 — W wrap makes the sum `0`, L **not** entered | [x] |
| 16 | `bitwriter_add` | S=`UINT32_MAX`, P=0 — sum `0xFFFFFFFF`, L entered, wrapped `b` clamped to `0` | [x] |
| 17 | `bitwriter_add` | S=`0x8000_0000`, P=`0x8000_0000` — W wrap to `0`, L not entered | [x] |
| 18 | `bitwriter_add` | S=0, P=`UINT32_MAX` — max-width argument | [x] |
| 19 | `bitwriter_add` | S random full `u32`, P=`UINT32_MAX` | [x] |
| 20 | `bitwriter_add` | S random full `u32`, P random full `u32` — unconstrained fuzz over both | [x] |
| 21 | `bitwriter_add` | S/P in `0..=64`, V=`0` | [x] |
| 22 | `bitwriter_add` | S/P in `0..=64`, V=`UINT64_MAX` | [x] |
| 23 | `bitwriter_add` | S/P in `0..=64`, V = only the low `P` bits set (the realistic call shape) | [x] |
| 24 | `bitwriter_add` | S/P in `0..=64`, V = only bit 63 set | [x] |
| 25 | `bitwriter_add` | S/P in `0..=64`, V = a single random bit set | [x] |
| 26 | `bitwriter_add` | S/P in `0..=64`, A=`0` (empty accumulator) | [x] |
| 27 | `bitwriter_add` | S/P in `0..=64`, A=`UINT64_MAX` (exercises `&= mask` clearing bit 0) | [x] |
| 28 | `bitwriter_add` | S/P in `0..=64`, T = `UINT32_MAX - (0..=64)` — `tot` u32 wrap | [x] |
| 29 | `bitwriter_add` | S/P in `0..=64`, U = wild non-null `buffer`, random `pos`/`len` — must be untouched | [x] |
| 30 | `bitwriter_add` | N: composed sequence of 64 calls on one `bw`, P in `1..=32` (realistic bit-packing pipeline), state compared after **every** call | [x] |
| 31 | `bitwriter_add` | N: composed sequence of 64 calls, P in `0..=64` (incl. degenerate widths) | [x] |
| 32 | `bitwriter_add` | N: composed sequence of 64 calls, P random full `u32` — worst-case composed pipeline | [x] |
| 33 | `bitwriter_add` | N: composed sequence of 64 calls starting from a fully random `bw` seed | [x] |
