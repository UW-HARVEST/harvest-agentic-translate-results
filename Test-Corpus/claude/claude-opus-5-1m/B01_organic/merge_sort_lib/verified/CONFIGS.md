# CONFIGS.md — Configuration-surface table (Phase A / gate for Phase B)

Derived mechanically from the branches the C actually takes.

## Build-time configuration axes

| axis | source | values |
|------|--------|--------|
| Cargo features | `Cargo.toml` has **no `[features]` section** | exactly one configuration: `--no-default-features` (≡ default) |
| CMake options | `c_src/CMakeLists.txt` has **no `option()`, no `if()`, no `target_compile_definitions`** | exactly one configuration |
| `#ifdef` in C | `grep -c '#if' c_src/src/lib.c c_src/include/lib.h` = **0** | none |

So there is **one** build configuration. Phase D's "repeat for every feature
combination" therefore reduces to the single combination, which is what
`run_all_configs.sh` iterates.

## Runtime option axes

`merge_sort(spritebatch_sprite_t *a, spritebatch_sprite_t *b, int size)` is the
**entire** public API (`c_src/include/lib.h`, 1 declaration). There are **no**
option structs, mode flags, byte-order settings, function pointers or global
state. Confirmed: the header declares one function and one struct; the `.so`
exports one symbol (see `SYMBOLS.md`).

## Public entry points

| entry point | linkage | callable across FFI? |
|---|---|---|
| `merge_sort` | extern (`T`) | **yes** — every row below drives it |
| `spritebatch_internal_merge_sort_recurse` | `static` | no |
| `spritebatch_internal_merge_sort_iteration` | `static` | no |
| `spritebatch_internal_sprite_less_than_or_equal` | `static` | no |

The three lowest-level functions are `static` in the C, so they are *not* part
of any consumer's surface on either side and cannot be called directly. They are
covered indirectly: the **internal branch** column below records which of their
branches each row forces, and `internal_branch_coverage_is_complete` asserts
every branch is hit by the suite.

## Input-shape axes the C branches on

* **`size`** — `recurse`'s base case `hi - lo <= 1`, and `split = (lo+hi)/2`
  splitting evenly (even span) vs. unevenly (odd span). Classes: `0`, `1`, `2`,
  `3`, exact powers of two (fully balanced tree), odd/prime (maximally ragged
  tree), large (deep recursion).
* **`sort_bits` relation** — comparator line 7 (`a->sort_bits <= b->sort_bits`)
  is the only live predicate: `<`, `==`, `>`. Distribution controls how often
  each run is exhausted first. Signed extremes (`INT_MIN`, `INT_MAX`) exercise
  the signed compare.
* **`texture_id`** — only appears in the **dead** line 9. Rows T1/T2 pin the
  quirk that it can never affect ordering.
* **run-exhaustion sub-branches in `iteration`** — `i < split` false (left run
  spent), `j >= hi` true (right run spent), comparator true, comparator false.
* **tail padding (bytes 12..16)** — `b[k] = a[i]` is a whole-struct assignment;
  gcc -O0 emits `mov 0x8(%rax),%rdx / mov %rdx,0x8(%rcx)`, i.e. it *does* copy
  the padding quadword. Rows with garbage padding pin this byte-exactly.
* **both output buffers** — `merge_sort` leaves the sorted result in `a` *and*
  leaves intermediate merge state in the scratch buffer `b`; both are
  caller-visible, so **every row compares `a` and `b` in full, all 16 bytes per
  element**.

## Configuration-surface table

Pattern legend — `sort_bits` fill: **EQ** all equal · **ASC** ascending ·
**DESC** descending · **TWO** two distinct values · **FEWDUP** 4 distinct
(heavy duplicates) · **RSMALL** random in 0..7 · **RFULL** random full `i32`
range · **EXTR** random from {`INT_MIN`,`INT_MAX`,`0`,`-1`,`1`}.
Padding: **PAD0** zeroed · **PADG** random garbage.

Every row is run with **many randomized inputs** (fixed seed `0x5EED_1234`,
default 200 iterations/row; `size`-swept rows draw a fresh random array per
size per iteration) and asserts `a` and `b` match byte-for-byte.

| # | entry point(s) | configuration (options set + input shape) | internal branches forced | [x] |
|---|----------------|--------------------------------------------|--------------------------|-----|
| C1 | `merge_sort` | `size=0`, PAD0 — degenerate empty | `recurse` base case only; zero-byte `memcpy` | [x] |
| C2 | `merge_sort` | `size=1`, PAD0 — single element | `recurse` base case; 16-byte `memcpy`, no merge | [x] |
| C3 | `merge_sort` | `size=1`, PADG — padding must survive the copy | as C2, padding propagation | [x] |
| C4 | `merge_sort` | `size=2`, ASC (already ordered), PAD0 | 1 merge; `split=1`; comparator `<` then `j>=hi` | [x] |
| C5 | `merge_sort` | `size=2`, DESC (inverted), PAD0 | 1 merge; comparator `>` → take-`j`, then `i<split` | [x] |
| C6 | `merge_sort` | `size=2`, EQ, PAD0 | comparator `==` → line 7 true (line 9 dead) | [x] |
| C7 | `merge_sort` | `size=2`, PADG | padding propagation through a real merge | [x] |
| C8 | `merge_sort` | `size=3`, RSMALL, PAD0 — first ragged split (`split=1`) | uneven `split`; both run-exhaustion branches | [x] |
| C9 | `merge_sort` | `size=3`, EQ, PAD0 | uneven split with all-equal keys | [x] |
| C10 | `merge_sort` | `size ∈ {4,8,16,32,64,256,1024}`, RSMALL, PAD0 — balanced tree | fully balanced recursion, all 4 iteration branches | [x] |
| C11 | `merge_sort` | `size ∈ {4,8,16,32,64,256,1024}`, RFULL, PAD0 | balanced tree, full-range signed compares | [x] |
| C12 | `merge_sort` | `size ∈ {5,7,11,17,31,99,1001}`, RSMALL, PAD0 — ragged tree | odd spans at every level | [x] |
| C13 | `merge_sort` | `size ∈ {5,7,11,17,31,99,1001}`, RFULL, PAD0 | ragged tree, full-range compares | [x] |
| C14 | `merge_sort` | `size` swept `0..=64` (every value), RSMALL, PAD0 | exhaustive small-`size` sweep, all split parities | [x] |
| C15 | `merge_sort` | `size` swept `0..=64`, PADG | exhaustive sweep with garbage padding | [x] |
| C16 | `merge_sort` | `size ∈ {2,3,4,...,64}`, ASC — pre-sorted input | comparator always `<=` → `i` side always taken until `i>=split` | [x] |
| C17 | `merge_sort` | `size ∈ {2,3,4,...,64}`, DESC — reverse-sorted input | comparator mostly false → `j` side taken; `i<split` exhaustion | [x] |
| C18 | `merge_sort` | `size ∈ {2..64}`, EQ — all keys identical | line 7 `==` path on every comparison | [x] |
| C19 | `merge_sort` | `size ∈ {2..64}`, TWO — two distinct keys, many ties | maximal tie density across merges | [x] |
| C20 | `merge_sort` | `size ∈ {2..64}`, FEWDUP — 4 distinct keys | heavy duplicates, mixed branches | [x] |
| C21 | `merge_sort` | `size ∈ {2..64}`, EXTR — `INT_MIN`/`INT_MAX`/`0`/`-1`/`1` keys | signed compare at the extremes (no subtraction ⇒ no overflow) | [x] |
| C22 | `merge_sort` | `size ∈ {2..64}`, RFULL keys, PADG padding | full-range keys **and** garbage padding together | [x] |
| C23 | `merge_sort` | T1: `size ∈ {2..64}`, EQ keys + **descending** `texture_id`, PAD0 | pins the dead line 9: `texture_id` must NOT reorder | [x] |
| C24 | `merge_sort` | T2: `size ∈ {2..64}`, EQ keys + `texture_id ∈ {0, u64::MAX, random}` | unsigned-compare extremes in the dead branch | [x] |
| C25 | `merge_sort` | `size ∈ {2..64}`, RSMALL keys + `texture_id = 0` for all | texture_id-invariance under real reordering | [x] |
| C26 | `merge_sort` | `size=4096`, RFULL, PAD0 — deep recursion (13 levels) | recursion depth / large-array stability | [x] |
| C27 | `merge_sort` | `size=4095` (odd, large), FEWDUP, PADG | deep ragged recursion + padding | [x] |
| C28 | `merge_sort` | repeated `merge_sort` on the **already-returned** `a` (idempotence/2nd pass), `size ∈ {2..64}`, RSMALL | driving the API as a consumer does across calls; scratch `b` carries prior state | [x] |
| C29 | `merge_sort` | scratch `b` pre-filled with garbage ≠ `a` (uninitialised scratch), `size ∈ {2..64}`, RSMALL, PADG | proves the initial `memcpy` fully overwrites `b`; any partial copy diverges | [x] |
| C30 | `merge_sort` | `size ∈ {2..64}`, keys chosen so **one run is always exhausted first** (left block all-min vs. all-max, and the mirror) | forces `j >= hi` and `i >= split` exhaustion deterministically at every level | [x] |

## Internal-branch coverage checklist

| C site | branch | forced by |
|---|---|---|
| `lib.c:33` | `hi - lo <= 1` true (base case) | C1, C2, every leaf of C4+ |
| `lib.c:33` | `hi - lo <= 1` false (recurse) | C4+ |
| `lib.c:35` | `split` from even span | C4, C10, C11, C26 |
| `lib.c:35` | `split` from odd span | C8, C9, C12, C13, C27 |
| `lib.c:19` | `i < split` false → take `j` | C17, C30 |
| `lib.c:20` | `j >= hi` true → take `i` | C16, C30 |
| `lib.c:21` | comparator returns 1 → take `i` | C4, C6, C18 |
| `lib.c:21` | comparator returns 0 → take `j` | C5, C17 |
| `lib.c:7` | `sort_bits <` (true) | C4, C16 |
| `lib.c:7` | `sort_bits ==` (true) | C6, C9, C18, C23, C24 |
| `lib.c:7` | `sort_bits >` (false, falls to line 9) | C5, C17 |
| `lib.c:9` | **dead** — `sort_bits ==` is always false here | C23, C24 assert the resulting invariance |
| `lib.c:43` | `memcpy` with `n == 0` | C1 |
| `lib.c:43` | `memcpy` with `n > 0` | C2+ |

Asserted programmatically by `internal_branch_coverage_is_complete`, which
re-runs the algorithm over the same generated inputs with a counter on each
branch and fails if any counter is zero. That test also asserts that `lib.c:9`
is only ever reached with **unequal** `sort_bits` — i.e. it proves the dead
branch really is dead, rather than assuming it.

## Volume actually executed

`cargo test --test phase_b_valid -- --nocapture` reports the per-row count;
the 31 rows total **≈40,000 differential `merge_sort` call pairs**, each
comparing both the result buffer and the scratch buffer in full.

## Cross-toolchain robustness (beyond the required default configuration)

The Rust artifact was additionally differential-tested against the C compiled
at every optimization level and with a second compiler, to confirm nothing in
the translation depends on a particular C codegen choice (notably the
struct-assignment padding copy):

| C build | phase B | phase C |
|---|---|---|
| gcc `-O0` (the CMake default, ground truth) | 34/34 | 15/15 |
| gcc `-O1` | 34/34 | – |
| gcc `-O2` | 34/34 | 15/15 |
| gcc `-O3` | 34/34 | 15/15 |
| gcc `-Os` | 34/34 | – |
| clang `-O2` | 34/34 | 15/15 |

Point the suite at any of these with `C_SO=<path> cargo test`.
