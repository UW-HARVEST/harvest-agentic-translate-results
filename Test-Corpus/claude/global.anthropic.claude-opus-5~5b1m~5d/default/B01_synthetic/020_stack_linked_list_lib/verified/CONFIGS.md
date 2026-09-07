# CONFIGS.md — Phase B configuration-surface table

## Axes the C code actually branches on

Derived from `c_src/src/simplestruct.c` + `c_src/include/simplestruct.h`:

1. **Public entry points** — exactly one: `int smallestValue(struct ListNode *head)`.
   There are no convenience wrappers and no lower-level helpers; this *is* the
   lowest-level entry point.
2. **Runtime options / flags / modes** — NONE. There is no config struct, no
   flag argument, no global, and no `#ifdef` in either the header or the source
   (verified: `grep -c '#if' c_src/src/simplestruct.c` → only the header guard).
3. **Input shape axes the code distinguishes:**
   - `head == NULL` vs. non-NULL → the `if (head)` branch.
   - list length: 1 (the `while (head->next)` body never runs) vs. ≥ 2 (body runs).
   - position of the minimum: at head (seed wins, `if` never taken), in the
     middle, at the tail (last iteration takes the `if`).
   - `<` strictness: duplicate minima (ties keep the earlier node — the `if` is
     `<`, not `<=`).
   - value sign / magnitude: all-positive, all-negative, mixed, `INT_MIN`,
     `INT_MAX`, zero.
   - monotone orderings that stress the comparison: strictly ascending (the `if`
     is never taken after the seed) and strictly descending (taken every
     iteration).
   - node memory layout: contiguous array of nodes vs. individually heap-boxed
     nodes in shuffled address order (the `next` pointer is followed, not
     pointer arithmetic — both must give the same answer).

## Cross-product rows (pruned to combinations the C treats differently)

Every row is exercised through BOTH `.so` files via `libloading`, with many
randomized inputs per row (fixed seed, deterministic xorshift PRNG) except where
the row is inherently a single shape.

| # | entry point(s) | configuration (options set + input shape) | test | [ ] |
|---|----------------|--------------------------------------------|------|-----|
| 1 | `smallestValue` | NULL head (only "option" the API has) | `cfg_row01_null` | [x] |
| 2 | `smallestValue` | length 1, randomized value over full `i32` range | `cfg_row02_len1_random` | [x] |
| 3 | `smallestValue` | length 2, randomized — covers min-at-head and min-at-tail | `cfg_row03_len2_random` | [x] |
| 4 | `smallestValue` | length 3..=32, randomized values (mixed signs) | `cfg_row04_small_random` | [x] |
| 5 | `smallestValue` | length 33..=512, randomized full-range `i32` values | `cfg_row05_medium_random` | [x] |
| 6 | `smallestValue` | minimum forced at the HEAD position (`if` never taken) | `cfg_row06_min_at_head` | [x] |
| 7 | `smallestValue` | minimum forced at the TAIL position (`if` taken on last iter) | `cfg_row07_min_at_tail` | [x] |
| 8 | `smallestValue` | minimum forced at a random INTERIOR position | `cfg_row08_min_interior` | [x] |
| 9 | `smallestValue` | duplicate minima (tie) — `<` vs `<=` distinction | `cfg_row09_duplicate_minima` | [x] |
| 10 | `smallestValue` | all nodes identical value, randomized | `cfg_row10_all_equal` | [x] |
| 11 | `smallestValue` | strictly ascending values (`if` never taken) | `cfg_row11_ascending` | [x] |
| 12 | `smallestValue` | strictly descending values (`if` taken every iteration) | `cfg_row12_descending` | [x] |
| 13 | `smallestValue` | all-positive values only | `cfg_row13_all_positive` | [x] |
| 14 | `smallestValue` | all-negative values only | `cfg_row14_all_negative` | [x] |
| 15 | `smallestValue` | values restricted to `{-1, 0, 1}` (sentinel-adjacent) | `cfg_row15_sentinel_adjacent` | [x] |
| 16 | `smallestValue` | boundary values `INT_MIN` / `INT_MAX` / `0` mixed randomly | `cfg_row16_boundary_values` | [x] |
| 17 | `smallestValue` | `INT_MIN` at each position in turn (head/interior/tail) | `cfg_row17_int_min_each_position` | [x] |
| 18 | `smallestValue` | all values `INT_MAX` (maximum possible minimum) | `cfg_row18_all_int_max` | [x] |
| 19 | `smallestValue` | nodes individually heap-allocated in SHUFFLED address order (traversal must follow `next`, not layout) | `cfg_row19_shuffled_addresses` | [x] |
| 20 | `smallestValue` | long list, 10_000 nodes, randomized | `cfg_row20_long_list` | [x] |
| 21 | `smallestValue` | same node graph queried starting from a MIDDLE node (sub-list aliasing; caller-visible behaviour) | `cfg_row21_start_midlist` | [x] |
| 22 | `smallestValue` | called repeatedly on the same list (function must be pure / no hidden state mutation; also verifies list is left UNMODIFIED) | `cfg_row22_idempotent_and_nonmutating` | [x] |

## Feature combinations

`translation/Cargo.toml` declares NO `[features]` table, so the only build
configuration is the default one. Verified with:

```sh
grep -A20 '^\[features\]' translation/Cargo.toml   # → no match
```

`cargo test --no-default-features` is therefore equivalent to the default and is
run as a second configuration for completeness.

## Binary / driver

The project builds NO binary executable: `[lib] crate-type = ["cdylib"]` only,
no `src/main.rs`, no `src/bin/`, and `CMakeLists.txt` declares only
`add_library(SimpleList SHARED ...)`. The "C and Rust stdout match" gate is
therefore N/A.
