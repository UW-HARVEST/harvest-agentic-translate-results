# CONFIGS.md — Configuration-surface table (Phase A → gates Phase B)

Mechanically derived from the C source and public header, not from assumptions.

## Axes the C code actually branches on

**Runtime options / modes / flags:** **none.** The public header
(`c_src/include/simplestruct.h`) declares exactly one function and no setters,
context struct, option bitmask, or global configuration variable. There is no
`#ifdef` in the library body other than the include guard. So the option axis is
a single point.

**Public entry points (full set, including the lowest level):**
`smallestValue` is simultaneously the highest- and lowest-level entry point —
there are no convenience wrappers layered over an internal API, and no `static`
helpers in `simplestruct.c`. It is called directly in every row below (never
through a wrapper), through the `.so` export.

**Input shapes the code special-cases** (from the three branch points):

| axis | values, from the branch it comes from |
|------|----------------------------------------|
| `head` nullness — `if (head)` at `:27` | NULL / non-NULL (NULL case is `ERRORS.md` E1) |
| list length — `while (head->next)` at `:29` | 0 (= NULL), 1 (loop body never runs), 2 (loop runs once), 3, many, very many |
| position of the minimum — `if (head->value < smallest)` at `:31` | at head (never updated), at second node, interior, at tail, everywhere-equal |
| strictness of `<` at `:31` | duplicate minima (first occurrence retained — `<` not `<=`); equal-adjacent values |
| value distribution of the `int` field | all-positive, all-negative, mixed sign, all-zero, all-equal, monotone increasing, monotone decreasing, extremes (`INT_MIN`/`INT_MAX`), full random `i32` |
| memory layout of the chain | contiguous (array-backed, ascending addresses), reversed/descending addresses, shuffled/non-contiguous individually-allocated nodes — confirms nothing depends on address order, only on `next` |

Rows below are the pruned cross-product: every combination the three C branch
points actually distinguish. Every row is run against **many randomized inputs
with a fixed seed** (deterministic SplitMix64 PRNG in the test file), except
where the shape is exact by definition.

## Configuration-surface table

| # | entry point(s) | configuration (options set + input shape) | test | [ ] |
|---|----------------|--------------------------------------------|------|-----|
| C1 | `smallestValue` | len 1, random `i32` value — loop body never executes, returns head value | `cfg_c1_single_node` | [x] |
| C2 | `smallestValue` | len 2, minimum at head (`v0 < v1`) — `<` never fires | `cfg_c2_len2_min_at_head` | [x] |
| C3 | `smallestValue` | len 2, minimum at tail (`v1 < v0`) — `<` fires once | `cfg_c3_len2_min_at_tail` | [x] |
| C4 | `smallestValue` | len 2, both values equal — `<` must *not* fire (strictness) | `cfg_c4_len2_equal` | [x] |
| C5 | `smallestValue` | len 3, minimum in interior position | `cfg_c5_len3_min_interior` | [x] |
| C6 | `smallestValue` | len 3..=8, minimum forced at head | `cfg_c6_min_at_head_various_len` | [x] |
| C7 | `smallestValue` | len 3..=8, minimum forced at tail | `cfg_c7_min_at_tail_various_len` | [x] |
| C8 | `smallestValue` | len 3..=64, minimum at a random interior index | `cfg_c8_min_random_interior` | [x] |
| C9 | `smallestValue` | duplicate minima at several indices (first-occurrence retention under `<`) | `cfg_c9_duplicate_minima` | [x] |
| C10 | `smallestValue` | all elements equal, len 1..=32 | `cfg_c10_all_equal` | [x] |
| C11 | `smallestValue` | strictly increasing values (minimum at head, `<` never fires) | `cfg_c11_monotone_increasing` | [x] |
| C12 | `smallestValue` | strictly decreasing values (`<` fires on every iteration) | `cfg_c12_monotone_decreasing` | [x] |
| C13 | `smallestValue` | all-positive values, random len 1..=64 | `cfg_c13_all_positive` | [x] |
| C14 | `smallestValue` | all-negative values, random len 1..=64 | `cfg_c14_all_negative` | [x] |
| C15 | `smallestValue` | mixed-sign values incl. zeros, random len 1..=64 | `cfg_c15_mixed_sign` | [x] |
| C16 | `smallestValue` | all zeros, random len 1..=32 | `cfg_c16_all_zero` | [x] |
| C17 | `smallestValue` | unrestricted random `i32` (full 32-bit range), random len 1..=128 — the broad property-style fuzz | `cfg_c17_full_random_i32` | [x] |
| C18 | `smallestValue` | values drawn from a boundary pool (`INT_MIN`, `INT_MIN+1`, `-1`, `0`, `1`, `INT_MAX-1`, `INT_MAX`) | `cfg_c18_boundary_value_pool` | [x] |
| C19 | `smallestValue` | `INT_MIN` present exactly once at a random index | `cfg_c19_int_min_random_index` | [x] |
| C20 | `smallestValue` | every value `INT_MAX` except one smaller value | `cfg_c20_all_int_max_but_one` | [x] |
| C21 | `smallestValue` | nodes laid out contiguously in ascending memory order | `cfg_c21_layout_ascending` | [x] |
| C22 | `smallestValue` | nodes chained in *descending* memory order (chain order ≠ address order) | `cfg_c22_layout_descending` | [x] |
| C23 | `smallestValue` | nodes individually heap-allocated and chained in shuffled address order | `cfg_c23_layout_shuffled_boxes` | [x] |
| C24 | `smallestValue` | long list, 100_000 nodes, random values (loop-count / no-stack-growth) | `cfg_c24_long_list` | [x] |
| C25 | `smallestValue` | the *same* node graph passed to C and then Rust, and again in reverse call order — confirms neither implementation mutates the caller's list | `cfg_c25_no_mutation_of_input` | [x] |

## Binary / driver executable

`c_src/CMakeLists.txt` contains only `add_library(SimpleList SHARED src/simplestruct.c)`
— no `add_executable`. `translation/Cargo.toml` has only `[lib] crate-type = ["cdylib"]`
and no `[[bin]]` / `src/main.rs`. **Neither side builds a binary**, so the
"compare stdout byte-for-byte" gate is not applicable.

## Feature combinations

No `[features]` in `Cargo.toml` → the only combination is the default, which is
identical to `--no-default-features`. Both were exercised.
