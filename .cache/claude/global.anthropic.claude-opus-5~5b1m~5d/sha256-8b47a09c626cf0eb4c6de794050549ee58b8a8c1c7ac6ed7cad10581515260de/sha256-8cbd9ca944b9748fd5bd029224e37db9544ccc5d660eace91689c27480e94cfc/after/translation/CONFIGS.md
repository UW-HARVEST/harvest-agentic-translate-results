# CONFIGS.md — Phase B configuration-surface table

Derived mechanically from `c_src/src/lib.c` and `c_src/include/lib.h`.

## Axes the C code actually branches on

The library exposes **no runtime options, modes, or flags** — there is no
context/handle struct, no setter, no global, and no conditional compilation
(`grep -nE 'enum|assert|ifdef|#if ' c_src/src c_src/include` → **no matches**).
The only axes are therefore *input shape*:

* **A1 — entry point.** Three exported functions, all public:
  `cleanup` (the only header-declared one), plus the two lower-level
  externally-linked helpers `print_result` and `cleanup_resources`. All three
  are driven directly, not just through `cleanup`.
* **A2 — value class of each of `cleanup`'s four `int` parameters.** The
  `switch (numbers[i])` at `lib.c:48` distinguishes exactly five classes:
  `10` (falls through into `20` ⇒ `+30`), `20` (⇒ `+20`), `30` (falls through
  into `40` ⇒ `+70`), `40` (⇒ `+40`), and everything else (`default:` ⇒
  `+numbers[i]`).
* **A3 — position/multiplicity.** The loop runs over all four slots
  `{a,b,c,d}`, so the same value class can appear in any position and any
  number of times (0, 1, … 4 occurrences). The accumulation is order-independent
  but the per-slot dispatch is not shared, so combinations across slots matter.
* **A4 — accumulator magnitude.** `result += …` on `int`, so sums near
  `INT_MAX` / `INT_MIN` wrap. Distinguishes "small sum" from "overflowing sum".
* **A5 — `print_result` label shape.** `printf("%s: %d\n", …)`: NULL, empty,
  short ASCII, long, embedded-`%` (must be passed as an argument, never as a
  format), non-ASCII bytes.
* **A6 — `print_result` result magnitude.** `%d` of `0`, positive, negative,
  `INT_MIN`, `INT_MAX`.
* **A7 — `cleanup_resources` pointer shape.** NULL vs. a live `malloc`ed block.
* **A8 — observable channel.** Both the **return value** and the **stdout
  bytes** (the C prints `Processed numbers: numbers` via `snprintf` + `printf`;
  `TO_STRING(numbers)` stringizes the macro *argument*, so the literal text is
  `numbers`, not the array contents). Every row is checked on both channels.

## Rows (pruned cross-product of the axes the C distinguishes)

| # | entry point(s) | configuration (options set + input shape) | [x] |
|---|----------------|--------------------------------------------|-----|
| 1 | `cleanup` | all four slots in `default:` class, small random values in `[-1000,1000]` (A2=default, A3=×4, A4=small) — 512 randomized seeds | [x] |
| 2 | `cleanup` | all four slots `= 10` (pure fall-through case, `10→20`, ⇒ `120`) | [x] |
| 3 | `cleanup` | all four slots `= 20` (pure non-fall-through case ⇒ `80`) | [x] |
| 4 | `cleanup` | all four slots `= 30` (fall-through `30→40` ⇒ `280`) | [x] |
| 5 | `cleanup` | all four slots `= 40` (⇒ `160`) | [x] |
| 6 | `cleanup` | exhaustive cross-product of the five value classes over all four slots, using one representative per class (`10,20,30,40,7`) ⇒ 5⁴ = 625 configurations | [x] |
| 7 | `cleanup` | each switch label placed in exactly one slot at a time, other slots `default`-class (A3 = positional coverage, 4 positions × 4 labels) | [x] |
| 8 | `cleanup` | label values ±1 (`9,11,19,21,29,31,39,41`) and negated labels (`-10,-20,-30,-40`) — must all take `default:` | [x] |
| 9 | `cleanup` | accumulator-overflow shapes: slots drawn from `{INT_MAX, INT_MIN, INT_MAX-1, INT_MIN+1, 1, -1}` in every 4-tuple combination (6⁴ = 1296) — signed wrap must agree | [x] |
| 10 | `cleanup` | fully unconstrained random `int` over the whole domain (A2 mostly default, A4 mixed/overflowing) — 4096 randomized seeds | [x] |
| 11 | `cleanup` | mixed random: each slot independently either a random switch label or a random full-range `int`, 4096 randomized seeds (interaction of A2×A3×A4) | [x] |
| 12 | `cleanup` | stdout capture: the emitted bytes (`Processed numbers: numbers\n`) compared byte-for-byte between C and Rust, across rows 1–11 representatives | [x] |
| 13 | `print_result` | short ASCII label × result ∈ {0, 1, -1, random} (A5 short × A6 mixed), randomized, stdout compared byte-for-byte | [x] |
| 14 | `print_result` | empty label `""` × result random (A5 empty) | [x] |
| 15 | `print_result` | long label (4096 bytes, no truncation allowed) × result random (A5 long) | [x] |
| 16 | `print_result` | label containing `%d`/`%s`/`%n` (A5 embedded-`%`: must be treated as data, never as a format string) | [x] |
| 17 | `print_result` | label with non-ASCII / high bytes (UTF-8 and raw 0x80–0xFF) (A5 non-ASCII) | [x] |
| 18 | `print_result` | result at `INT_MIN` / `INT_MAX` (A6 extremes, `%d` formatting) | [x] |
| 19 | `cleanup_resources` | `NULL` pointer (A7 = NULL) — no-op, no output | [x] |
| 20 | `cleanup_resources` | live libc-`malloc`ed block of varying sizes (1, 50, 4096 bytes) (A7 = live block) — freed exactly once | [x] |
| 21 | `cleanup` + `cleanup_resources` + `print_result` | full composed pipeline, as a real consumer: `r = cleanup(a,b,c,d); print_result("cleanup", r);` repeated over randomized tuples, with interleaved `cleanup_resources` calls — return values **and** the full combined stdout stream compared byte-for-byte | [x] |

## Binary executable

`c_src/CMakeLists.txt` declares only `add_library(... SHARED src/lib.c)` — there
is **no `add_executable`**, and `translation/Cargo.toml` has no `[[bin]]`
target. The project builds no driver binary, so the "compare binary stdout"
gate is not applicable. Library stdout is nevertheless compared byte-for-byte
(rows 12, 13–18, 21) by capturing `fd 1` in forked child processes.

## Feature combinations

`translation/Cargo.toml` has no `[features]` table, so the default build is the
only configuration. Rows above are additionally re-run under
`--no-default-features` for completeness.
