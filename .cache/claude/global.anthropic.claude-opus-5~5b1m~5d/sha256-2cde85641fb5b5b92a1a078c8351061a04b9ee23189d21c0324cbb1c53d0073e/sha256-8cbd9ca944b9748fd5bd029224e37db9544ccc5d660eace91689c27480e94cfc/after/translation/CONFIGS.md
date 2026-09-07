# CONFIGS.md — configuration surface table (valid inputs)

Derived mechanically from `c_src/include/driver.h` and `c_src/src/driver.c`.

## Axes the C code actually branches on

**Runtime options / modes / flags.** There are none in the classic sense: the
library has no init function, no context struct, no setter, no global state, no
`#ifdef` in either the header or the `.c` file (`grep -c '#if' src/driver.c` →
only the header guard in `driver.h`). The *only* "configuration" a caller can
apply is the **integer argument value**, which selects which branch each
function takes.

```
$ grep -n 'if\|switch\|#ifdef\|#if' c_src/src/driver.c
31:    if(line != NULL)
46:    if (data >= 0)
66:    if (data >= 0)
85:    if (data >= 0 && data < (10))
```

Four `if`s, no `switch`, no preprocessor conditionals. So the branch axes are:

- A1 `printLine`: `line == NULL` vs `line != NULL`
- A2 `bad`: `data >= 0` vs `data < 0`; and, *within* the taken branch, whether
  `data` is `< 10` (in bounds) or `>= 10` (unchecked OOB write)
- A3 `goodG2B`: constant `data = 7` — statically one path only (no axis, but it
  is unconditionally executed by every `good` / `driver` call, so its ten
  output lines prefix every `good` result)
- A4 `goodB2G`: `data >= 0 && data < 10` vs otherwise (two distinct ways to
  fail: negative, or `>= 10`)

**Input shapes.**

- S1 pointer shape for `printLine`: NULL / empty `""` / short ASCII / string
  containing `%` conversion specifiers / string containing embedded `\n` /
  string containing non-ASCII bytes (0x80–0xFF) / very long (64 KiB) /
  1-byte-at-end-of-page (no trailing readable page)
- S2 `int` shape: `0`, `1..8` (interior), `9` (max valid index), `10`, `11`
  (defined OOB), `>= 12` (UB), `-1`, `INT_MIN`, `INT_MAX`, arbitrary random
  `i32` bit patterns
- S3 buffer/element shape: fixed by the source — `int buffer[10]`, element type
  `int` (4 bytes), count 10, printed with `%d`, one value per line. Not
  caller-configurable, but the *position* of the written `1` within the ten
  printed lines is the value-dependent output shape (index 0 = first line,
  index 9 = last line, index >= 10 = no `1` at all).

**Full set of public entry points** (from `nm -D`, not just the header — the
header only declares `driver`, but four more symbols are exported and are
therefore part of the real ABI):

- `printLine(const char*)` — lowest level
- `printIntLine(int)` — lowest level
- `bad(int)` — mid level, calls both of the above
- `good(int)` — mid level, calls the two `static` helpers
- `driver(int, int)` — top level convenience wrapper, calls `good` then `bad`

Tests exercise all five directly via their `.so` exports; `driver` is *not*
used as the only entry point.

## Configuration table

Every row is a differential test: same configuration through the C `.so` and
the Rust `.so`, stdout captured at the fd level, compared byte-for-byte.
"random" rows use a fixed-seed xorshift PRNG (seed `0x2025_0905_D8_1EA7`) with
the stated iteration count, so they are reproducible.

| # | entry point(s) | configuration (options set + input shape) | test | ✔ |
|---|----------------|-------------------------------------------|------|---|
| 1 | `printLine` | NULL pointer (A1 false branch) | `cfg01` | [x] |
| 2 | `printLine` | empty string `""` (0 bytes + newline) | `cfg02` | [x] |
| 3 | `printLine` | short ASCII, no specials (`"hello"`) | `cfg03` | [x] |
| 4 | `printLine` | string containing `%` specifiers (`"%s %d %n %%"`) — must be verbatim, not formatted | `cfg04` | [x] |
| 5 | `printLine` | string containing embedded newlines and tabs | `cfg05` | [x] |
| 6 | `printLine` | string containing high bytes 0x80–0xFF (non-UTF-8) | `cfg06` | [x] |
| 7 | `printLine` | 64 KiB string (oversized, exceeds any internal buffer) | `cfg07` | [x] |
| 8 | `printLine` | 1024 randomized byte strings, random length 0..=255, random non-NUL bytes | `cfg08` | [x] |
| 9 | `printIntLine` | `0` | `cfg09` | [x] |
| 10 | `printIntLine` | each of `1`, `-1`, `9`, `10`, `INT_MAX`, `INT_MIN` (boundary sweep) | `cfg10` | [x] |
| 11 | `printIntLine` | 4096 randomized `i32` bit patterns (full domain) | `cfg11` | [x] |
| 12 | `bad` | `data == 0` — write to first element (A2 true, in bounds, boundary low) | `cfg12` | [x] |
| 13 | `bad` | `data` in `1..=8` — every interior index, exhaustive | `cfg13` | [x] |
| 14 | `bad` | `data == 9` — last valid index (boundary high) | `cfg14` | [x] |
| 15 | `bad` | `data == 10` and `11` — A2 true branch **with** unchecked OOB write, still deterministic (stack padding / loop counter slot) | `cfg15` | [x] |
| 16 | `bad` | `data < 0` — A2 false branch: `-1`, `-2`, `-10`, `INT_MIN`, plus 512 random negatives | `cfg16` | [x] |
| 17 | `good` | `data == 0` — goodG2B(7) block then goodB2G in-bounds at index 0 | `cfg17` | [x] |
| 18 | `good` | `data` in `1..=8` exhaustive — A4 true branch, every interior index | `cfg18` | [x] |
| 19 | `good` | `data == 9` — A4 true branch, boundary high | `cfg19` | [x] |
| 20 | `good` | `data == 10`, `11`, `100`, `INT_MAX` — A4 false via second conjunct | `cfg20` | [x] |
| 21 | `good` | `data == -1`, `INT_MIN` — A4 false via first conjunct | `cfg21` | [x] |
| 22 | `good` | 4096 randomized `i32` bit patterns (full domain, all A4 paths mixed) | `cfg22` | [x] |
| 23 | `driver` | `goodData` valid × `badData` valid: full cross-product `0..=9 × 0..=9` (100 combos) | `cfg23` | [x] |
| 24 | `driver` | `goodData` valid (`0..=9`) × `badData` defined-OOB (`10`, `11`) | `cfg24` | [x] |
| 25 | `driver` | `goodData` valid × `badData` negative (`-1`, `INT_MIN`) | `cfg25` | [x] |
| 26 | `driver` | `goodData` invalid (`-1`, `INT_MIN`, `10`, `INT_MAX`) × `badData` valid (`0`, `7`, `9`) | `cfg26` | [x] |
| 27 | `driver` | `goodData` invalid × `badData` invalid-but-defined (cross-product) | `cfg27` | [x] |
| 28 | `driver` | 2048 randomized `(goodData, badData)` pairs, `goodData` full `i32` domain, `badData` drawn from the defined domain `INT_MIN..=11` | `cfg28` | [x] |
| 29 | composed pipeline | `printLine`, `printIntLine`, `bad`, `good`, `driver` called in a randomized *interleaved sequence* (256 calls) against one continuous stdout capture — checks composed output/ordering/flushing, not just per-call output | `cfg29` | [x] |
| 30 | all five | repeated invocation (idempotence / no residual state): the same call issued 3× in a row must produce exactly 3 identical copies in both libraries | `cfg30` | [x] |

**Excluded, deliberately:** `bad(data)` for `data >= 12`. The C code overwrites
its saved `%rbp` (index 12) and return address (index 14) and is genuinely
undefined; see ERRORS.md row 9. No differential assertion is possible.

## Feature combinations

`translation/Cargo.toml` declares no `[features]` table and no optional
dependencies:

```
$ grep -n 'feature' translation/Cargo.toml
(no matches)
```

The only configuration is therefore the single default (empty) feature set.
`--no-default-features` is equivalent to the default here and is still run by
`run_all_features.sh` for completeness.

## Extra: exhaustive brute-force sweeps

Beyond the randomized per-row sweeps above, `tests/stress_exhaustive.rs`
(`#[ignore]`d because it is slow; run with
`cargo test --test stress_exhaustive -- --ignored`) walks the interesting
domains *exhaustively* rather than by sampling:

| sweep | domain covered | result |
|-------|----------------|--------|
| `stress_bad_exhaustive_window` | every `data` in `-2000..=11` + `INT_MIN`-region | pass |
| `stress_good_exhaustive_window` | every `data` in `-2000..=2000` + `INT_MIN`/`INT_MAX` | pass |
| `stress_driver_exhaustive_cross_product` | full `-30..=30 x -30..=11` cross-product (2 501 combos) | pass |
| `stress_print_line_exhaustive_lengths` | every length `0..=300` | pass |
| `stress_print_int_line_exhaustive_window` | every value `-5000..=5000` + every `%d` width/decade boundary | pass |

## Status

All 31 rows (cfg00–cfg30) pass, in **both** the `debug` and `release`
profiles, and under both the default and the empty feature set. Verified with
`./run_all_features.sh`:

```
PASS  cargo test  --no-default-features
PASS  cargo test  <default features>
PASS  cargo test --release --no-default-features
PASS  cargo test --release <default features>
OVERALL: ALL CONFIGURATIONS PASS
```
