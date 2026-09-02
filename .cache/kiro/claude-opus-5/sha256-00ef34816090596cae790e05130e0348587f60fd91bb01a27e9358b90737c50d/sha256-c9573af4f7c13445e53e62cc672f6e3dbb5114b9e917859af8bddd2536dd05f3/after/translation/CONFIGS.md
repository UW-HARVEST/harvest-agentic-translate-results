# CONFIGS.md — Phase B configuration-surface table

## Entry points (complete, from the public header)

`grep -n "^int\|^[a-z].*(" c_src/include/simplestruct.h` gives exactly one
declaration:

```c
int smallestValue (struct ListNode *date);
```

There are no convenience wrappers and no simplified one-shot API layered over a
lower level: `smallestValue` **is** the lowest-level public entry point, and the
full set of public entry points is `{ smallestValue }`. It is exercised directly
in every row below.

## Runtime options / modes / flags

**None.** The API takes no mode, flag, format, byte-order, or element-type
parameter; there is no init/config struct, no global state, no setter, and no
`#ifdef` in the source or header (`grep -nE '#if|#ifdef|#define'` finds only the
`SIMPLESTRUCT_H_` include guard). The Rust `Cargo.toml` declares no `[features]`
table, so the feature cross-product is the single default configuration.

Consequently the configuration surface is driven entirely by **input shape**.

## Axes the C actually branches on

| axis | source line | values the code distinguishes |
|------|-------------|-------------------------------|
| A — head nullness | `if (head)` @ 27 | NULL (→ ERRORS.md E1) / non-NULL |
| B — list length | `while (head->next)` @ 29 | 1 (loop body never runs) / 2 / 3+ / very large |
| C — position of the minimum | `if (...< smallest)` @ 31 | at head (branch never taken) / in the middle / at the tail (branch taken on the last iteration) |
| D — multiplicity of the minimum | strict `<` @ 31 | unique / duplicated (tie ⇒ earliest occurrence kept, later equal values do **not** re-trigger) / all elements equal |
| E — value domain | `int value` @ header 28; the `<` comparison @ 31 | small non-negative / all negative / mixed sign / contains `-1` / contains `INT_MIN` / contains `INT_MAX` / all zero |
| F — ordering | governs how often the @31 branch fires | strictly ascending (branch never fires after seeding) / strictly descending (fires every iteration) / random |

Axis A's NULL value belongs to the error surface and is tabulated in
`ERRORS.md`; rows below are the **valid** (non-NULL) cross-product.

## Configuration table

One row per meaningful combination the C treats differently. Every row is run
with many randomized inputs under a fixed seed (`SEED = 0x5EED_1EAF_D00D_F00D`,
LCG in `tests/differential.rs`), not a single hand-picked value, and asserts the
C and Rust `.so` exports return byte-identical `int`s.

| # | entry point(s) | configuration (options set + input shape) | [ ] |
|---|----------------|--------------------------------------------|-----|
| C01 | `smallestValue` | len 1, random value over full `i32` range (loop never entered) | [x] |
| C02 | `smallestValue` | len 2, min at head (branch @31 never taken) | [x] |
| C03 | `smallestValue` | len 2, min at tail (branch @31 taken once) | [x] |
| C04 | `smallestValue` | len 2, both values equal (tie; strict `<` keeps head) | [x] |
| C05 | `smallestValue` | len 3–8, unique min at head | [x] |
| C06 | `smallestValue` | len 3–8, unique min strictly in the middle | [x] |
| C07 | `smallestValue` | len 3–8, unique min at tail | [x] |
| C08 | `smallestValue` | len 3–8, min duplicated at two positions (tie across a distance) | [x] |
| C09 | `smallestValue` | len 3–8, all elements equal | [x] |
| C10 | `smallestValue` | len 3–64, strictly ascending (branch @31 never fires) | [x] |
| C11 | `smallestValue` | len 3–64, strictly descending (branch @31 fires every iteration) | [x] |
| C12 | `smallestValue` | len 1–64, uniform random values, all non-negative (`0..=i32::MAX`) | [x] |
| C13 | `smallestValue` | len 1–64, uniform random values, all negative (`i32::MIN..0`) | [x] |
| C14 | `smallestValue` | len 1–64, uniform random over the **full** `i32` range (mixed sign) | [x] |
| C15 | `smallestValue` | len 1–64, values drawn from the narrow set `{-1, 0, 1}` (dense ties, and `-1` aliasing the E1 sentinel) | [x] |
| C16 | `smallestValue` | len 2–64, `i32::MIN` planted at a random position (min is `INT_MIN`) | [x] |
| C17 | `smallestValue` | len 2–64, all values `i32::MAX` except one smaller (no overflow in `<`) | [x] |
| C18 | `smallestValue` | len 2–64, values from `{i32::MIN, i32::MAX}` only (extreme-only comparisons) | [x] |
| C19 | `smallestValue` | len 1–64, all values `0` | [x] |
| C20 | `smallestValue` | len 1–64, exactly one `-1` and the rest `> -1` (result `-1` must equal the NULL result of E1) | [x] |
| C21 | `smallestValue` | len 100–1000, uniform random full-range (long-list traversal) | [x] |
| C22 | `smallestValue` | len 100_000, uniform random full-range (oversized; iterative traversal, no stack growth) | [x] |
| C23 | `smallestValue` | len 1–64, nodes allocated **non-contiguously** in shuffled memory order (traversal must follow `next`, not address order) | [x] |
| C24 | `smallestValue` | len 3–64, traversal started from a **mid-list node** (caller passes an interior pointer — a valid sub-list, so only the tail suffix is scanned) | [x] |
