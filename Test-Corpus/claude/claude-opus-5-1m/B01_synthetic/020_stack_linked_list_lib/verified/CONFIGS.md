# CONFIGS.md — Configuration-surface table (Phase B gate)

Derived mechanically from `c_src/include/simplestruct.h` and
`c_src/src/simplestruct.c`.

## Axis 1 — runtime options / modes / flags

Mechanical grep of the public header for anything settable, and of the C source
for branches on such state:

```
$ grep -n 'enum\|#define\|extern\|static\|set[A-Z_]\|_init\|_new\|flag\|mode\|option' \
        c_src/include/simplestruct.h c_src/src/simplestruct.c
(no matches)
```

**Result: ZERO runtime options.** There is no context/handle struct, no
`init`/`create`/`destroy`, no setters, no global or `static` state, no
`#define`-selected behaviour, no `enum`, and no environment lookup. `#ifdef`s
are limited to the header's include guard. `CMakeLists.txt` defines no
`option()` and no `target_compile_definitions`, so there is exactly **one**
build configuration; `Cargo.toml` has no `[features]` section, so there is
exactly **one** Rust feature combination (the empty/default set). This axis
therefore contributes a single value and drops out of the cross-product.

## Axis 2 — full set of public entry points

```
$ grep -n '^[a-z].*(' c_src/include/simplestruct.h
34:int smallestValue (struct ListNode *date);
```

**One** entry point: `smallestValue`. It is simultaneously the lowest-level and
the only entry point — there is no convenience wrapper layered over a lower
primitive, so "test the low-level API too" is satisfied by definition. (Note the
header's parameter is named `date`, the definition's `head`; names do not affect
the ABI.)

## Axis 3 — input shapes the C actually special-cases

The three branch points in the body are the complete set of behavioural
distinctions the code makes:

| branch | line | distinction it creates |
|--------|------|------------------------|
| `if (head)` | 27 | list is empty (NULL) vs. non-empty |
| `while (head->next)` | 29 | loop body executes **0** times (length 1) vs. **n-1** times (length > 1) |
| `if (head->value < smallest)` | 31 | the running minimum **is** updated vs. **is not** updated on a given iteration |

Shape sub-axes derived from those branches:

* **length**: 0 (NULL) · 1 · 2 · 3 · many · very many
* **position of the minimum**: head (loop never updates) · second · middle ·
  last (updates on the final iteration) · tied across several positions
* **update cadence** (drives branch 31): strictly increasing (never updates) ·
  strictly decreasing (updates every iteration) · all values equal (`<` is
  strict, so never updates) · random (updates sporadically)
* **value domain / signedness**: all positive · all negative · mixed with zero ·
  `i32::MIN` / `i32::MAX` extremes · full-range random bit patterns
* **heap layout**: nodes allocated in `next` order vs. allocated in shuffled
  order so that `next` order ≠ address order (catches a translation that walks
  memory instead of the `next` pointer)

## The table

Every row is a combination the C treats differently. Each is driven with **many
randomized inputs** (fixed seed `0x5EED_1234`, deterministic xorshift64\* PRNG)
except where the shape is inherently a single value; both `.so`s are called
through `dlopen`/`dlsym` and the returned `int`s must be bit-identical.

| # | entry point(s) | configuration (options set + input shape) | test | [x] |
|---|----------------|-------------------------------------------|------|-----|
| C1 | `smallestValue` | length 0: `head = NULL` — branch 27 false | `cfg_c1_empty` | [x] |
| C2 | `smallestValue` | length 1, random full-range value — branch 29 never taken (loop body 0 iterations) | `cfg_c2_single_random` | [x] |
| C3 | `smallestValue` | length 2, min at head — branch 31 false on the only iteration | `cfg_c3_len2_min_at_head` | [x] |
| C4 | `smallestValue` | length 2, min at tail — branch 31 true on the only iteration | `cfg_c4_len2_min_at_tail` | [x] |
| C5 | `smallestValue` | length 2, both values equal — branch 31 false via strict `<` | `cfg_c5_len2_equal` | [x] |
| C6 | `smallestValue` | length 3, min in the middle | `cfg_c6_len3_min_middle` | [x] |
| C7 | `smallestValue` | length n ∈ 4..64, strictly increasing — branch 31 **never** taken | `cfg_c7_strictly_increasing` | [x] |
| C8 | `smallestValue` | length n ∈ 4..64, strictly decreasing — branch 31 taken on **every** iteration | `cfg_c8_strictly_decreasing` | [x] |
| C9 | `smallestValue` | length n ∈ 4..64, all values identical | `cfg_c9_all_equal` | [x] |
| C10 | `smallestValue` | length n ∈ 1..64, uniformly random full-range `i32` values — sporadic updates | `cfg_c10_random_fullrange` | [x] |
| C11 | `smallestValue` | length n ∈ 1..64, random values restricted to all-positive (`1..=i32::MAX`) | `cfg_c11_all_positive` | [x] |
| C12 | `smallestValue` | length n ∈ 1..64, random values restricted to all-negative (`i32::MIN..=-1`) | `cfg_c12_all_negative` | [x] |
| C13 | `smallestValue` | length n ∈ 1..64, small mixed values incl. `0`, `-1`, `+1` (dense ties/duplicates from a tiny alphabet) | `cfg_c13_small_alphabet_ties` | [x] |
| C14 | `smallestValue` | length n ∈ 2..64, minimum forced to a random **interior** index, all other values random | `cfg_c14_min_at_random_index` | [x] |
| C15 | `smallestValue` | length n ∈ 2..64, minimum value duplicated at 2+ random indices (ties at the extremum) | `cfg_c15_duplicated_minimum` | [x] |
| C16 | `smallestValue` | length n ∈ 2..64, minimum pinned at the **last** node (update on the final iteration, exiting the loop immediately after) | `cfg_c16_min_at_tail` | [x] |
| C17 | `smallestValue` | length n ∈ 2..64, `i32::MIN` and `i32::MAX` both present at random positions | `cfg_c17_extremes_present` | [x] |
| C18 | `smallestValue` | length n ∈ 2..64, nodes allocated in **shuffled** address order so `next` order ≠ memory order | `cfg_c18_shuffled_layout` | [x] |
| C19 | `smallestValue` | length 1000 and length 100 000 (large / "very many"), random values | `cfg_c19_long_lists` | [x] |
| C20 | `smallestValue` | interleaving: NULL, then a non-empty list, then NULL again, repeatedly — proves the function is stateless/re-entrant across configurations | `cfg_c20_interleaved_stateless` | [x] |
| C21 | `smallestValue` | same list traversed twice in a row — proves the C's pointer walk does not mutate the caller's nodes (node values compared before/after in both impls) | `cfg_c21_no_mutation` | [x] |

## Verdict

- [x] All 21 rows pass with C and Rust returning bit-identical results across
      their randomized inputs.
- [x] Only one feature combination exists (no `[features]` in `Cargo.toml`, no
      `option()` in `CMakeLists.txt`), and the whole table passes under it.
