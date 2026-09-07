# CONFIGS.md — Phase B configuration surface table

## How this table was derived

Axes were read off the C source, not guessed.

**Public entry points** — the complete set, from `c_src/include/sieve.h`:

| entry point | signature | level |
|-------------|-----------|-------|
| `sieve` | `void sieve(int start)` | lowest **and** highest — there is no convenience wrapper and no lower layer; `src/sieve.c` defines no `static` helpers |

**Runtime options / modes / flags:** none. There is no init function, no context
struct, no setter, no global, no environment lookup and no `#ifdef` in either
`sieve.c` or `sieve.h` (verified by the grep recorded in `ERRORS.md`). The
library is stateless and its only configuration axis is the single `int`
argument.

**Branches the C actually takes** (the whole control flow):

1. `while (1)` — unconditional loop.
2. `printf("%d\n", val)` — output width/format varies with the value's digit
    count and sign; this is a data-shape axis, exercised below.
3. `if (val % 10 == 9) break;` — the only conditional. With C's
    truncating `%`, the predicate is reachable-true **only** for non-negative
    `val` whose last decimal digit is 9. For negative `val` the remainder is
    `<= 0`, so the predicate is always false and the loop runs up through zero
    to `9`.
4. `val++` — signed increment; wraps at `INT_MAX` in the compiled code.

So the meaningful axes are: **sign** (negative / zero / positive) x
**residue `val % 10`** (which decides 1-iteration vs multi-iteration) x
**magnitude / digit-width shape** (single digit, digit-width transitions,
sign-boundary crossing, huge) x **call multiplicity** (statelessness: does a
second call behave like the first).

The rows below are that cross-product, pruned to the combinations the code
distinguishes, and each is driven with **many randomized inputs from a fixed
seed** (`SplitMix64`, seed `0x5EED_1E55_5EED_1E55`) rather than one hand-picked
value, so value-dependent behaviour is covered rather than sampled.

## Table

| # | entry point(s) | configuration (options set + input shape) | randomized inputs per row | [x] |
|---|----------------|-------------------------------------------|---------------------------|-----|
| C1 | `sieve` | exhaustive sweep, **every** `val` in `-256..=256` — covers all 10 non-negative residues, all 10 negative residues, zero, the sign boundary and 1/2/3-digit widths with no sampling gaps | 513 (exhaustive, not sampled) | [x] |
| C2 | `sieve` | positive, residue `== 9` (the 1-iteration break-immediately path): random `val` in `0..=100000` with `val % 10 == 9` | 300 | [x] |
| C3 | `sieve` | positive, residue `!= 9` (multi-iteration path): random `val` in `0..=100000`, `val % 10 != 9`, all 9 other residues | 400 | [x] |
| C4 | `sieve` | zero and the immediate neighbourhood of the sign boundary: `-3..=3` plus random restarts there | 20 | [x] |
| C5 | `sieve` | negative "ends in 9" values (`val % 10 == -9`), the predicate-never-true path: random `val` in `-100000..=-1` with `val % 10 == -9` | 300 | [x] |
| C6 | `sieve` | negative, all other residues `-8..=0`: random `val` in `-100000..=-1` | 400 | [x] |
| C7 | `sieve` | digit-width transition upward across a power of ten while counting (`9->10`, `99->100`, `999->1000`, `9999->10000`, …): starts just below each power of ten, so `printf` changes field width mid-run | 8 boundaries x 4 offsets = 32 | [x] |
| C8 | `sieve` | digit-width transition on the negative side (`-100 -> -99`, `-10 -> -9`, `-1 -> 0`): the minus sign disappears mid-run | 12 | [x] |
| C9 | `sieve` | large positive magnitude, long run, well clear of overflow: random `val` in `2_000_000_000..=2_147_483_600` | 200 | [x] |
| C10 | `sieve` | large negative magnitude with a long multi-million-line run: random `val` in `-2_000_000..=-1_000_000` | 8 | [x] |
| C11 | `sieve` | full-range random `int` restricted to values whose run length is bounded (`val % 10 != 9` accepted, run capped by construction): uniform over `i32` then projected into a terminating, budget-sized run | 500 | [x] |
| C12 | `sieve` | statelessness / repeat invocation: the same `val` called 3x in a row, and an interleaved sequence `a, b, a` through the **same** loaded handle — output of call *n* must not depend on calls `< n` | 100 sequences | [x] |
| C13 | `sieve` | interleaving between the two libraries on the shared `stdout` FILE buffer: alternate C-call / Rust-call without an intervening flush, to confirm the Rust side uses libc `printf` on the same buffer and does not reorder or double-buffer output | 50 alternations | [x] |
| C14 | `sieve` | huge-output shapes (`INT_MAX`, `INT_MIN`, overflow region) compared by bounded output prefix in child processes — the valid-path view of `ERRORS.md` E7–E11 | 5 starts x 64 KiB prefix | [x] |

## Binary executable

`c_src/CMakeLists.txt` declares **only** `add_library(Sieve SHARED src/sieve.c)`
— no `add_executable`. The Rust `Cargo.toml` declares `crate-type = ["cdylib"]`
and no `[[bin]]`. **The project builds no driver binary**, so the
"compare C and Rust binary stdout" gate is not applicable. (The test suite does
use a small `examples/runner.rs` loader to host the huge-output child processes,
but it is test scaffolding, not a translated C driver: it loads whichever `.so`
it is pointed at, so it is identical for both sides by construction.)

## Feature combinations

`translation/Cargo.toml` has **no `[features]` section**, so the only build
configuration is the default (empty) feature set. `cargo check`/`cargo test`
with `--no-default-features` is therefore identical to the default build; both
are run in `run_all.sh` to prove it. There are likewise no `#[cfg]`/`#ifdef`
switches in either implementation.

## Gate

- [x] Every row above passes across its randomized inputs, byte-for-byte,
      against the C `.so` loaded through the same FFI boundary.
