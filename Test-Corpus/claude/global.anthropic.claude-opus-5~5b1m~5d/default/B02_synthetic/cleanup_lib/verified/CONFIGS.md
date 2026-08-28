# CONFIGS.md — Phase A: configuration surface table (valid inputs)

## Axis inventory (derived from the C source, not from assumptions)

**Runtime options / modes / flags: none.**
`grep -nE '#if|#ifdef|#else|static|extern' c_src/src/lib.c c_src/include/lib.h`
matches nothing but the two object-like macros `STRINGIZE`/`TO_STRING`. The
library has no globals, no init/setter functions, no environment lookups and no
conditional compilation, so there is no option state to cross-product. The
entire configuration surface is therefore the **input shape** of the three
entry points.

**Public entry points — all three, none of them a wrapper.** `lib.h` declares
only `cleanup`, but `lib.c` gives external linkage to `print_result` and
`cleanup_resources` as well, and both are exported by the `.so` (see
`SYMBOLS.md`). `cleanup_resources` is the lowest-level entry point (it is what
`cleanup` calls internally on both its exit paths), `print_result` is an
independent leaf, and `cleanup` is the composed pipeline
(validate → switch loop → allocate → format → print → release). All three are
driven directly, and additionally in the composed order a real consumer uses:
`r = cleanup(a,b,c,d); print_result(label, r);` with a caller-owned buffer
handed to `cleanup_resources`.

**Shape axes the C actually branches on:**

| axis | values the code distinguishes | source |
|------|-------------------------------|--------|
| A. `switch` class of each of the 4 args | `10` (falls through to 20), `20`, `30` (falls through to 40), `40`, `default` | lib.c:48-62 |
| B. arity/position | exactly 4 slots, `numbers[] = {a,b,c,d}`, loop `i < 4` | lib.c:36,47 |
| C. magnitude class of a `default` arg | `0`, small `+`, small `-`, near-case (`9,11,19,21,29,31,39,41`), negated case (`-10,-20,-30,-40`), large, `INT_MAX`, `INT_MIN` | lib.c:60 |
| D. accumulator overflow | sum stays in range / wraps positive / wraps negative | lib.c:50-60 |
| E. `label` shape (`print_result`) | normal, empty, `NULL`, contains `%` specifiers, contains newlines, non-UTF-8 bytes, 64 KiB, 1 MiB | lib.c:80 |
| F. `result` shape (`print_result`) | `0`, `+`, `-`, `INT_MAX`, `INT_MIN`, value produced by `cleanup` | lib.c:80 |
| G. pointer shape (`cleanup_resources`) | `NULL`, `malloc(0)`, `malloc(1)`, `malloc(50)`, `malloc(1<<20)` | lib.c:84 |
| H. observable channel | returned `int`, stdout bytes, allocator state | all |
| I. build profile | `debug` and `release` (`release` enables the `printf`→`puts` rewrite and constant folding) | Cargo.toml |

Axis A × B is the pruned cross-product that matters: **5⁴ = 625** distinct
class-combinations, all of which are enumerated exhaustively by row 4 below.

## Rows

Every row is run against **both** `.so`s through `libloading` and compared on
both channels (return value **and** exact stdout bytes). Rows marked
*randomised* use ≥ 4 096 seeded inputs (`seed = 0x5EED_C0FFEE`, SplitMix64).

| # | entry point(s) | configuration (options set + input shape) | ✅ |
|---|----------------|-------------------------------------------|-----|
| 1 | `cleanup` | all four args `default`-class, small positives (`1,2,3,4`) — baseline happy path; also asserts the `"Processed numbers: numbers\n"` line and the `snprintf` stringisation | [x] |
| 2 | `cleanup` | each `case` label alone in slot 0, rest neutral `0`: `(10,0,0,0)`, `(20,0,0,0)`, `(30,0,0,0)`, `(40,0,0,0)` → +30/+20/+70/+40 | [x] |
| 3 | `cleanup` | each `case` label alone in **every** slot (4 labels × 4 positions, rest `0`) — proves position-independence of the fall-through | [x] |
| 4 | `cleanup` | **exhaustive** cross-product of the 5 switch classes over all 4 slots (5⁴ = 625), `default` represented by a rotating set of non-case values | [x] |
| 5 | `cleanup` | all four args the same `case` label: `(10,10,10,10)`, `(20,…)`, `(30,…)`, `(40,…)` → 120/80/280/160 | [x] |
| 6 | `cleanup` | mixed `case` labels, all 4! orderings of `{10,20,30,40}` (24 permutations) → always 30+20+70+40 = 160 | [x] |
| 7 | `cleanup` | near-case boundary values only: exhaustive over `{9,11,19,21,29,31,39,41}⁴` (4 096 combos) — one step either side of every label | [x] |
| 8 | `cleanup` | negated case labels `{-10,-20,-30,-40}` in all slots (256 combos) — must hit `default`, not `case` | [x] |
| 9 | `cleanup` | all-zero `(0,0,0,0)`; and `0` mixed with each case label | [x] |
| 10 | `cleanup` | overflow shapes: `INT_MAX` in 1..4 slots, `INT_MIN` in 1..4 slots, `INT_MAX` mixed with `INT_MIN`, `INT_MAX` mixed with case labels (wrap-around of the accumulator) | [x] |
| 11 | `cleanup` | *randomised* over the **full** `i32` range (uniform 32-bit words, so `default` dominates and overflow occurs) | [x] |
| 12 | `cleanup` | *randomised* over a biased alphabet (`{10,20,30,40}` ∪ near-case ∪ `{0,±1,INT_MIN,INT_MAX}`) so the `case`/fall-through arms are hit densely | [x] |
| 13 | `cleanup` | *randomised* small range `-64..=64` (dense `default` values incl. negatives around zero) | [x] |
| 14 | `cleanup` | repeated invocation (2 048 back-to-back calls) — no cross-call state, no allocator leak, identical stdout every time (the C `malloc`/`free`s per call) | [x] |
| 15 | `print_result` | normal ASCII label, `result = 0` | [x] |
| 16 | `print_result` | normal label × `result` ∈ `{0, 1, -1, 42, -42, INT_MAX, INT_MIN}` (axis E×F) | [x] |
| 17 | `print_result` | empty label `""` × the same `result` set | [x] |
| 18 | `print_result` | label with embedded `%` conversion specifiers (`"%s"`, `"%d"`, `"%%"`, `"%n"`, `"%1000000d"`) × `result` set | [x] |
| 19 | `print_result` | label with embedded newlines / tabs / `\r`, and label of raw non-UTF-8 bytes `0x80..=0xFF` | [x] |
| 20 | `print_result` | oversized labels: 4 KiB (stdio buffer boundary), 64 KiB, 1 MiB | [x] |
| 21 | `print_result` | *randomised* labels: random length 0..=512 of random non-NUL bytes × random `i32` result | [x] |
| 22 | `cleanup_resources` | `NULL` pointer (explicit null-check arm) | [x] |
| 23 | `cleanup_resources` | live `malloc`ed block, sizes `{0, 1, 8, 49, 50, 51, 4096, 1<<20}` (incl. the size `cleanup` itself uses) | [x] |
| 24 | `cleanup_resources` | *randomised*: 1 024 blocks of random size 0..=8192, each allocated by the test and released alternately through the C and the Rust export (proves both call the same `free`) | [x] |
| 25 | composed pipeline | `r = cleanup(a,b,c,d)` then `print_result(label, r)`, randomised `(a,b,c,d,label)` — the full end-to-end sequence a consumer runs, comparing the concatenated stdout of both calls | [x] |
| 26 | composed pipeline | interleaved: `cleanup_resources(malloc(50))`, `cleanup(...)`, `print_result(...)`, repeated, alternating which `.so` goes first — allocator/stdio state must not leak between the two implementations | [x] |
| 27 | all three | same-argument calls repeated after a `cleanup` that took the malloc-failure path (see `ERRORS.md` row 2) — state must be identical afterwards | [x] |
| 28 | all three | axis I: every row above re-run under `debug` and `release`, and under `--no-default-features` / `--all-features` (identical unit — no `[features]` declared) via `tests/run_all.sh` | [x] |
