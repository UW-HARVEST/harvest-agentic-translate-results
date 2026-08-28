# CONFIGS.md — Phase A: configuration surface table (valid inputs)

Derived mechanically from `c_src/src/lib.c` + `c_src/include/lib.h`.

## Axes the C code actually branches on

**Runtime options / modes / flags: NONE.** `lib.h` exposes one function and one
struct; there is no flag, mode, context object, global state, `#ifdef`, or
`switch` anywhere in the C (`grep -nE '#if|switch|case' src/lib.c` → no matches).
So the configuration surface is entirely made of **input shapes**.

**Public entry points** (`nm -D` → exactly one; `lib.h` declares exactly one):

* `merge_sort(spritebatch_sprite_t *a, spritebatch_sprite_t *b, int size)`

**Lowest-level entry points** `spritebatch_internal_sprite_less_than_or_equal`,
`spritebatch_internal_merge_sort_iteration` and
`spritebatch_internal_merge_sort_recurse` are `static`: they are **not** in the C
`.so`'s dynamic symbol table, so no direct differential call is possible for
either side. They are driven *through* `merge_sort`, and the axes below are
chosen specifically to reach every branch of each (see the "reaches" column).

### Axis S — `size` (drives `hi - lo <= 1` and the `(lo+hi)/2` split shape)

`S0`=0, `S1`=1, `S2`=2, `S3`=3, `S4`=4, `S5`=5, `S7`=7, `S8`=8, `S9`=9,
`S15`=15, `S16`=16, `S17`=17, `S31`=31, `S32`=32, `S33`=33, `S100`=100,
`S255`=255, `S256`=256, `S257`=257, `S1000`=1000, `S4096`=4096, `S4097`=4097.
(Powers of two split evenly; odd sizes force uneven splits and `hi-lo==1` leaves
on one side only.)

### Axis K — `sort_bits` value pattern (drives `_less_than_or_equal`)

* `K-EQ` all identical → every compare hits line 7 tie path (stability)
* `K-ASC` strictly ascending → `_iteration` always takes the left run
* `K-DESC` strictly descending → `_iteration` always takes the right run
* `K-RAND` uniform random `i32` (full range, both signs)
* `K-FEW` random from a 2–4 value alphabet → many ties interleaved
* `K-ALT` alternating high/low
* `K-NEG` all negative → signed-comparison correctness
* `K-EXT` only `INT_MIN` / `INT_MAX` / `0` / `-1` → signed boundary values
* `K-ONE` sorted except a single displaced element
* `K-SORTED-DUPS` non-decreasing with runs of duplicates

### Axis T — `texture_id` value pattern (drives the DEAD line-9 branch)

* `T-ZERO` all zero
* `T-RAND` uniform random `u64`
* `T-EXT` only `0` / `u64::MAX` / `1` / `u64::MAX-1` → tests that the *unsigned*
  `<=` on line 9 is never reached and never influences order
* `T-ANTI` `texture_id` descending while `sort_bits` ties → the input that
  *would* differ if line 9 were reachable; pins the dead-code behaviour

### Axis P — struct padding bytes (offsets 12..16)

* `P-ZERO` padding all `0x00`
* `P-GARBAGE` padding filled with distinct non-zero bytes → verifies the
  `memcpy` **and** the `b[k]=a[i]` struct assignment propagate padding
  byte-identically (C emits two 8-byte `mov`s, confirmed by `objdump`)

### Axis F — scratch buffer `b` pre-fill

* `F-ZERO` `b` zeroed before the call
* `F-SENTINEL` `b` pre-filled with `0xAA` → detects any region of `b` that C
  leaves untouched but Rust writes (or vice-versa)

### Axis A — buffer aliasing / geometry

* `A-DISJOINT` `a` and `b` are separate allocations (normal use)
* `A-SAME` `a == b` (documented in `ERRORS.md` #18)

### Axis O — output surface compared

Both buffers are compared on **every** row, as all 16 bytes of both:
`a` (which is where the final result lands for `size >= 2`) **and** `b` (the
scratch buffer, which is only *partially* written).

## Configuration table

Every row is exercised with **many randomized inputs** (fixed seed
`0x5EED_5P12_3ABC_DEF0`, splitmix64) across the whole `size` list of the row, and
both `a` and `b` are compared byte-for-byte between the C and Rust `.so`.

| # | entry point(s) | configuration (options set + input shape) | reaches | [x] |
|---|----------------|-------------------------------------------|---------|-----|
| 1 | `merge_sort` | `S0`, `K-RAND`, `T-RAND`, `P-ZERO`, `F-SENTINEL`, `A-DISJOINT` | `recurse` `hi-lo==0` guard; zero-length memcpy | [x] |
| 2 | `merge_sort` | `S1`, `K-RAND`, `T-RAND`, `P-ZERO`, `F-SENTINEL`, `A-DISJOINT` | `recurse` `hi-lo==1` guard; 16-byte memcpy | [x] |
| 3 | `merge_sort` | `S2`, `K-RAND`, `T-RAND`, `P-ZERO`, `F-ZERO`, `A-DISJOINT` | first `_iteration` call; both compare outcomes | [x] |
| 4 | `merge_sort` | `S2`, `K-EQ`, `T-ANTI`, `P-ZERO`, `F-ZERO`, `A-DISJOINT` | tie → line 7; proves line 9 dead at minimal size | [x] |
| 5 | `merge_sort` | `S3`, `K-RAND`, `T-RAND`, `P-ZERO`, `F-SENTINEL`, `A-DISJOINT` | odd split `(0+3)/2==1`; one leaf, one 2-elem subtree | [x] |
| 6 | `merge_sort` | `S4`, `K-RAND`, `T-RAND`, `P-ZERO`, `F-SENTINEL`, `A-DISJOINT` | balanced 2-level ping-pong (result parity) | [x] |
| 7 | `merge_sort` | `S5`,`S7`,`S9` (odd), `K-RAND`, `T-RAND`, `P-ZERO`, `F-SENTINEL`, `A-DISJOINT` | uneven splits at several depths | [x] |
| 8 | `merge_sort` | `S8`,`S16`,`S32`,`S256` (powers of 2), `K-RAND`, `T-RAND`, `P-ZERO`, `F-SENTINEL`, `A-DISJOINT` | perfectly balanced recursion; deepest ping-pong | [x] |
| 9 | `merge_sort` | `S15`,`S17`,`S31`,`S33`,`S255`,`S257` (2^n±1), `K-RAND`, `T-RAND`, `P-ZERO`, `F-SENTINEL`, `A-DISJOINT` | most irregular split trees | [x] |
| 10 | `merge_sort` | `S100`,`S1000`, `K-RAND`, `T-RAND`, `P-ZERO`, `F-SENTINEL`, `A-DISJOINT` | bulk random, mixed parity | [x] |
| 11 | `merge_sort` | `S4096`,`S4097`, `K-RAND`, `T-RAND`, `P-ZERO`, `F-SENTINEL`, `A-DISJOINT` | large input, deep recursion | [x] |
| 12 | `merge_sort` | ALL sizes, `K-EQ`, `T-RAND`, `P-ZERO`, `F-SENTINEL`, `A-DISJOINT` | every compare is a tie → left-run-always path | [x] |
| 13 | `merge_sort` | ALL sizes, `K-EQ`, `T-ANTI`, `P-ZERO`, `F-SENTINEL`, `A-DISJOINT` | pins dead line 9 across all split shapes | [x] |
| 14 | `merge_sort` | ALL sizes, `K-ASC`, `T-RAND`, `P-ZERO`, `F-SENTINEL`, `A-DISJOINT` | `_iteration` right-run-exhausted (`j>=hi`) path | [x] |
| 15 | `merge_sort` | ALL sizes, `K-DESC`, `T-RAND`, `P-ZERO`, `F-SENTINEL`, `A-DISJOINT` | `_iteration` left-run-exhausted (`i>=split`) path | [x] |
| 16 | `merge_sort` | ALL sizes, `K-FEW`, `T-RAND`, `P-ZERO`, `F-SENTINEL`, `A-DISJOINT` | ties interleaved with strict compares (stability) | [x] |
| 17 | `merge_sort` | ALL sizes, `K-ALT`, `T-RAND`, `P-ZERO`, `F-SENTINEL`, `A-DISJOINT` | maximal alternation between the two merge branches | [x] |
| 18 | `merge_sort` | ALL sizes, `K-NEG`, `T-RAND`, `P-ZERO`, `F-SENTINEL`, `A-DISJOINT` | signed `<=` on negative `sort_bits` | [x] |
| 19 | `merge_sort` | ALL sizes, `K-EXT`, `T-EXT`, `P-ZERO`, `F-SENTINEL`, `A-DISJOINT` | `INT_MIN`/`INT_MAX` × `0`/`u64::MAX` boundaries | [x] |
| 20 | `merge_sort` | ALL sizes, `K-ONE`, `T-RAND`, `P-ZERO`, `F-SENTINEL`, `A-DISJOINT` | nearly-sorted (single displaced element) | [x] |
| 21 | `merge_sort` | ALL sizes, `K-SORTED-DUPS`, `T-RAND`, `P-ZERO`, `F-SENTINEL`, `A-DISJOINT` | non-decreasing runs of duplicates | [x] |
| 22 | `merge_sort` | ALL sizes, `K-RAND`, `T-ZERO`, `P-ZERO`, `F-SENTINEL`, `A-DISJOINT` | `texture_id` constant → order determined by `sort_bits` only | [x] |
| 23 | `merge_sort` | ALL sizes, `K-RAND`, `T-RAND`, **`P-GARBAGE`**, `F-SENTINEL`, `A-DISJOINT` | padding propagation through memcpy + struct assign | [x] |
| 24 | `merge_sort` | ALL sizes, `K-EQ`, `T-RAND`, **`P-GARBAGE`**, `F-SENTINEL`, `A-DISJOINT` | padding propagation on the all-ties (left-run) path | [x] |
| 25 | `merge_sort` | ALL sizes, `K-DESC`, `T-RAND`, **`P-GARBAGE`**, `F-ZERO`, `A-DISJOINT` | padding propagation on the right-run path | [x] |
| 26 | `merge_sort` | ALL sizes, `K-RAND`, `T-RAND`, `P-ZERO`, **`F-ZERO`**, `A-DISJOINT` | scratch buffer starting zeroed (untouched regions) | [x] |
| 27 | `merge_sort` | `S0..S17` + `S100`, `K-RAND`, `T-RAND`, `P-GARBAGE`, `F-SENTINEL`, **`A-SAME`** | aliased `a == b` ping-pong (see `ERRORS.md` #18) | [x] |
| 28 | `merge_sort` | ALL sizes, `K-RAND`, `T-RAND`, `P-ZERO`, `F-SENTINEL`, `A-DISJOINT`, **called twice in a row on the same buffers** | idempotency / no hidden state between calls | [x] |
| 29 | `merge_sort` | random `size` in `1..=600` + random `K`/`T`/`P`/`F` combo, 4000 iterations (property-style fuzz over the full cross-product) | catch-all for unpruned axis interactions | [x] |
| 30 | `merge_sort` | buffer `b` **larger** than `size` (slack tail), ALL sizes, `K-RAND`, `F-SENTINEL` | proves C writes only `[0,size)` of `b` and Rust does not over-write | [x] |
