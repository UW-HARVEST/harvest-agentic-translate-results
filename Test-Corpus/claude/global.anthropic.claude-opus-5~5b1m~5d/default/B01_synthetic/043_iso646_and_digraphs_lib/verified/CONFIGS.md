# CONFIGS.md — Configuration-surface table (Phase A / gate for Phase B)

## Mechanical derivation of the axes

Public API, from `c_src/include/driver.h` (the *only* header, one declaration):

```c
void driver(int x, int y);
```

Body, after digraph (`%:`→`#`, `<%`→`{`, `%>`→`}`) and `<iso646.h>`
(`bitor`→`|`, `compl`→`~`) expansion — confirmed with `gcc -E`:

```c
void driver(int x, int y) {
    int result = x | ~y;
    printf("%d", result);
    puts("");
}
```

Axis enumeration, derived strictly from what the C code above can branch on:

| axis | values the C actually distinguishes | source of the distinction |
|------|--------------------------------------|---------------------------|
| **A1** runtime options / modes / flags | **none** — there is no setter, no global, no context struct, no flag argument, no `#ifdef` in the library | grep of the tree found 0 `if`/`switch`/`?:`/`#if` other than the header include guard |
| **A2** entry points | **one**: `driver`. It is simultaneously the lowest-level and the only entry point — there is no convenience wrapper to hide behind | `nm -D` exports exactly `driver` |
| **A3** value class of `x` | `0`, `+1`, `-1`, `INT_MAX`, `INT_MIN`, random positive, random negative, single-bit, all-bits | `x` feeds `\|` (bit-level: every bit position is a distinct path) and then `%d` |
| **A4** value class of `y` | same set; additionally `y = 0` (`~0 = -1` ⇒ result always `-1`) and `y = -1` (`~-1 = 0` ⇒ result `= x`) are the two absorbing/identity cases of `\| ~y` | `~y` then `\|` |
| **A5** value class of `result = x \| ~y` | `0`, `-1`, `INT_MIN`, `INT_MAX`, positive, negative | `printf("%d")` formatting is value-dependent (sign, digit count, and `INT_MIN` has no positive magnitude) |
| **A6** output width | 1..10 decimal digits, with and without a leading `-` (1..11 bytes) | `%d` conversion |
| **A7** call count / sequencing in one stream | 1 call, few calls, many calls (crossing the 4096-byte `stdio` buffer), C-and-Rust calls interleaved into the *same* `stdout` `FILE` | `printf` + `puts` write into the shared, buffered `stdout`; interleaving/buffering is observable |
| **A8** `stdout` sink shape | regular file (fully buffered), pipe (fully buffered), character device / non-seekable | glibc picks the buffering mode from `fstat` on fd 1; affects *when* bytes appear, so it must be compared under each shape |

There is no byte-order axis (no multi-byte buffer is produced), no element-type
axis (both parameters are `int`), and no format axis (the format string is the
literal `"%d"`).

Rows below are the pruned cross-product of A3–A8 — pruned to the combinations
the code above actually treats differently. Every row is driven through the
`.so` exports of **both** libraries and compared byte-for-byte. Rows marked
"randomized" use ≥256 pseudo-random inputs from a fixed-seed SplitMix64
generator (seed `0x2545F4914F6CDD1D`), so runs are reproducible.

## Table

| # | entry point(s) | configuration (options set + input shape) | [ ] |
|---|----------------|-------------------------------------------|-----|
| C1 | `driver` | `(0, 0)` — both identity-ish zero; result `-1` | [x] |
| C2 | `driver` | `(0, -1)` — the only shape producing result `0` (shortest positive output) | [x] |
| C3 | `driver` | `(-1, 0)` — all-bits `x`, absorbing `y` | [x] |
| C4 | `driver` | `y = 0` fixed, `x` randomized over all 32 bits — `~0 = -1` absorbs, result must always be `-1` | [x] |
| C5 | `driver` | `x = 0` fixed, `y` randomized over all 32 bits — result must be exactly `~y` | [x] |
| C6 | `driver` | `y = -1` fixed, `x` randomized — `~-1 = 0`, identity: result must be exactly `x` | [x] |
| C7 | `driver` | `x = -1` fixed, `y` randomized — result must always be `-1` | [x] |
| C8 | `driver` | `(INT_MAX, INT_MIN)` — result `INT_MAX`, widest positive output (10 digits) | [x] |
| C9 | `driver` | `(INT_MIN, INT_MAX)` — result `INT_MIN`, widest negative output (`-2147483648`) | [x] |
| C10 | `driver` | `(INT_MAX, INT_MAX)` and `(INT_MIN, INT_MIN)` — result `-1` in both | [x] |
| C11 | `driver` | both operands randomized, **`x > 0`, `y > 0`** quadrant | [x] |
| C12 | `driver` | both operands randomized, **`x > 0`, `y < 0`** quadrant | [x] |
| C13 | `driver` | both operands randomized, **`x < 0`, `y > 0`** quadrant | [x] |
| C14 | `driver` | both operands randomized, **`x < 0`, `y < 0`** quadrant | [x] |
| C15 | `driver` | result forced to each **positive** decimal width 1..10 (`x = 0`, `y = ~v` for `v` at each power-of-ten boundary and boundary−1) | [x] |
| C16 | `driver` | result forced to each **negative** decimal width 1..10 (`x = 0`, `y = ~(-v)`) | [x] |
| C17 | `driver` | single-bit `x` (`1 << b`, `b = 0..31`) × `y = 0, -1, INT_MIN, INT_MAX` — every bit position of the `\|` | [x] |
| C18 | `driver` | single-bit `y` (`1 << b`, `b = 0..31`) × `x = 0, -1, 1, INT_MIN` — every bit position of the `~` | [x] |
| C19 | `driver` | complementary / equal pairs: `y = x`, `y = !x` (`~x`), `y = -x`, over randomized `x` | [x] |
| C20 | `driver` | full boundary cross-product: all 81 pairs from `{INT_MIN, INT_MIN+1, -2, -1, 0, 1, 2, INT_MAX-1, INT_MAX}` | [x] |
| C21 | `driver` | one-past-narrow-width values: all pairs from `{-32769, -32768, -129, -128, -1, 0, 127, 128, 255, 256, 32767, 32768, 65535, 65536}` (all 196 combinations of `int` values that a narrower port would treat differently) | [x] |
| C22 | `driver` | unconstrained randomized sweep: 4096 fully random `(x, y)` pairs over the whole `int × int` domain, single capture per pair | [x] |
| C23 | `driver` | **A7 sequencing:** 200 randomized calls inside **one** capture — exercises `stdout` buffer accumulation and `printf`/`puts` ordering across calls | [x] |
| C24 | `driver` | **A7 sequencing:** ~2000 randomized calls in one capture, > 4096 bytes total, forcing intermediate `stdio` buffer flushes mid-stream | [x] |
| C25 | `driver` | **A7 interleaving:** C and Rust `driver` called alternately into the *same* `stdout` `FILE` in one capture; compared against the same alternation with the roles swapped — proves both share and advance the identical stream state | [x] |
| C26 | `driver` | **A8 sink shape:** `stdout` redirected to a **regular file** (fully buffered, seekable) — the baseline used by all rows above | [x] |
| C27 | `driver` | **A8 sink shape:** `stdout` redirected to a **pipe** (fully buffered, non-seekable), randomized inputs | [x] |
| C28 | `driver` | **A8 sink shape:** `stdout` redirected to `/dev/null` (character device), randomized inputs — both must complete without output and without error | [x] |
| C29 | `driver` | **A1 verification:** no runtime option exists — asserted structurally by checking that the C `.so` exports exactly one symbol and that repeated identical calls are stateless (same input ⇒ same output, 100 repeats, no drift) | [x] |

## Result

All 29 rows pass. `cargo test` → 46 tests across 5 binaries, 0 failures, under
both `default` and `--no-default-features` (the crate declares no cargo
features, so those are the only two configurations — see `scripts/verify.sh`
step 4, which enumerates them from `cargo metadata` rather than by assumption).

### Sensitivity evidence

The rows are only meaningful if they can fail. Mutating the Rust source and
re-running (via `scripts/mutate.py`) produces:

| mutation to `src/lib.rs` | rows that caught it |
|--------------------------|---------------------|
| `x \| !y` → `x ^ !y` | 21 of 29 (the 8 that survive are the rows where `\|` and `^` genuinely agree: `x` or `~y` is `0`, or the operands are bit-disjoint) |
| `printf("%d")` → `printf("%u")` | 23 of 29 |
| dropped the `puts("")` newline | **29 of 29** |
| spurious `if x == INT_MIN \|\| y == INT_MIN { return; }` rejection | 9 of 29 Phase B rows + 6 of 10 Phase C rows + both isolated Phase C binaries |

Each mutation was reverted immediately; `src/lib.rs` is byte-identical to its
pre-testing state.
