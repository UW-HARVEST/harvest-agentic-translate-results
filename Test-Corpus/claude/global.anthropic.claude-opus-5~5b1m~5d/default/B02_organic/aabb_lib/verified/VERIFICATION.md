# VERIFICATION.md — how to reproduce, and what was found

## How to run everything

```sh
cd translation
./run_all.sh          # C build + symbol diff + all tests, every feature combo x profile
```

`run_all.sh` does, for **each** feature combination (`--no-default-features`,
default, `--all-features`) × **each** profile (`dev`, `release`):

1. `cargo build` — **required**, because `cargo test` alone does *not* re-link a
   `cdylib`. (The harness enforces this: it refuses to run if
   `target/*/libaabb_lib.so` is older than `src/lib.rs`.)
2. `nm -D` diff of the two shared objects — must be empty.
3. A check that the Rust `.so` has no undefined non-libc symbols.
4. `cargo test` — 130 differential tests.
5. Three extra "soak" runs with `C2_DIFF_SEED=1..3`, which re-seeds every
   property test so the same rows are exercised with completely different random
   inputs.

Total: 24 test runs × 130 tests. All pass. On top of that the suite was soaked
with 106 further distinct `C2_DIFF_SEED` values across both profiles.

To run a single suite by hand:

```sh
cargo build --offline                       # or --release
cargo test  --offline --test phase_b_valid  # CONFIGS.md rows
cargo test  --offline --test phase_c_errors # ERRORS.md rows
cargo test  --offline --test smoke          # symbol parity at runtime
C2_DIFF_SEED=42 cargo test --offline        # fresh random inputs
```

`--offline` is only needed if crates.io is unreachable; `libloading 0.8` and its
two transitive deps are already in the local cargo registry.

## Test architecture

`tests/common/mod.rs` `dlopen`s **both** shared objects and resolves every symbol
with `dlsym`. **No Rust function is ever called directly** — every call in every
test goes through the Rust `.so`'s `#[no_mangle] extern "C"` export, exactly like
an external C consumer, so the export wrappers and the SysV AMD64 struct-passing
ABI are under test too (`c2v`/`c2r` in one `xmm`, `c2x`/`c2AABB` in two,
`c2Circle` as SSE+SSE, `c2Capsule` as MEMORY/stack).

Every comparison is **bit-exact**: `f32::to_bits`, not an epsilon. Composite
outputs (`c2Proxy`, `c2Simplex`, `c2GJKCache`) are compared field by field with
bit equality so NaN payloads and signed zeros cannot hide.

| file | purpose | tests |
|------|---------|-------|
| `tests/smoke.rs` | both `.so`s load; all 38 C symbols resolve in the Rust `.so` | 3 |
| `tests/phase_b_valid.rs` | one test per `CONFIGS.md` row (valid paths) | 67 |
| `tests/phase_c_errors.rs` | one test per `ERRORS.md` row (rejections/boundaries) | 60 (+1 `#[ignore]`d UB record) |

## Bugs found and fixed in the Rust translation

### 1. NaN payload / sign propagation (16 functions) — **real, fixed**

The C library is compiled at `-O0`, so gcc emits exactly one scalar SSE
instruction per source-level operation. On x86, `ADDSS/SUBSS/MULSS/DIVSS dst, src`
resolve a NaN operand **destination-first**: if `dst` is a NaN the result is `dst`
with the quiet bit forced on, else if `src` is a NaN the result is `src` quietened.
Which value gcc puts in the destination register is *not* always the left-hand
side of the C expression — it depends on gcc's `-O0` evaluation order.

The original translation wrote the expressions naturally (`a.x * b.x + a.y * b.y`)
and let LLVM choose the operand order, which diverged. Six of the eighteen
Phase B rows failed on the very first run, e.g.

```
c2Dot((NaN 0x7fc01234, 0x00800000), (0x00800000, NaN 0xffc01234))
  C    = NaN 0xffc01234
  Rust = NaN 0x7fc01234
```

Fix: `src/lib.rs` now models the instruction semantics explicitly with
`addss/subss/mulss/divss(dst, src)` helpers plus `fneg` (C's unary `-` on a float
is a plain `xorps` sign flip, which — unlike an arithmetic negation — never
quietens an SNaN). The destination operand for each site was read off
`objdump -d` of the reference `.so` and is documented in a doc-comment above each
function. Affected: `c2Dot`, `c2Det2`, `c2Add` (gcc puts **`b`**'s component in
the destination), `c2Sub`, `c2Mulvs`, `c2Mulrv`, `c2MulrvT`, `c2Div`, `c22`,
`c23`, `c2Witness`, `c2L`, `c2GJK`, `c2CircletoCircle` (gcc puts **`B.r`** in the
destination for `A.r + B.r`), `c2CircletoAABB`, `c2CircletoCapsule`.

This also removed the previous `black_box`-based `cneg` hack, whose behaviour
depended on the optimiser rather than being specified.

Side effect: the results are now independent of the Rust optimisation level —
verified by running the whole suite in both `dev` and `release`.

### 2. `FLT_EPSILON * FLT_EPSILON` — **hardened**

gcc constant-folds it into `.rodata` as `0x28800000`. The Rust now uses that
exact bit pattern (`C2_EPSILON_SQ`) instead of relying on a runtime multiply.
(Confirmed identical, but the constant removes the dependency.)

### 3. Memory safety on a hostile `c2GJKCache` — **hardened**

`c2GJK` indexes `pA.verts[cache->iA[i]]` with the raw cached index. The original
translation used `pA.verts[iA as usize]`, which **panics** for an out-of-range
index — and a panic across an `extern "C"` boundary aborts the whole process,
which is far worse than the C's behaviour. It now goes through `proxy_vert`,
which returns `(0,0)` for an out-of-range index and never faults or aborts. The
loop that writes the new simplex vertex likewise uses `get_mut` instead of
unchecked pointer arithmetic (the index is provably `1..=2`, but the guard makes
that enforced rather than assumed).

### 4. `c2BBVerts` output/input aliasing — **real, fixed**

```c
void c2BBVerts(c2v *out, c2AABB *bb) {
    out[0] = bb->min;
    out[1] = c2V(bb->max.x, bb->min.y);   /* may overwrite bb->min */
    out[2] = bb->max;
    out[3] = c2V(bb->min.x, bb->max.y);   /* reads the MODIFIED bb->min.x */
}
```

Every `bb->` load happens *after* the preceding store, so an `out` buffer that
overlaps `*bb` produces a cascading, partially-updated result. This is
well-defined C and it is reachable in practice — `c2MakeProxy` passes
`p->verts` as `out`. The original translation did `let bb = &*bb;` once and
worked from that snapshot, which (a) gave the copy-then-write answer instead of
the cascading one and (b) was itself Rust UB under aliasing. It now re-reads each
field with `ptr::read(addr_of!(...))` at exactly the point the C reads it, and
never creates a reference to `*bb`.

Proven by `err_bbverts_output_aliases_input`, which sweeps `bb` across every
`c2v`-slot offset of a buffer shared with `out`. Reverting the implementation
makes that test fail, so it has teeth.

### 5. `c2Witness` output/input aliasing — **real, fixed**

The C stores `*a` **before** evaluating the `*b` expression, so a caller that
points `a` into `*s` (e.g. `&s->a.sB` — legal C) changes what `*b` computes. The
original translation computed both witness points from a snapshot and then stored
them. The exported `c2Witness` now interleaves the stores exactly like the C
(reading through raw pointers only); the internal GJK call site keeps using the
snapshot helper because its two output locals are provably disjoint.

Proven by `err_witness_output_aliases_simplex`, which points `a` and `b` at all
12 × 12 `c2v`-aligned slot pairs inside the simplex for `count` 0..4. Reverting
the implementation makes that test fail.

## Known non-conformance-relevant difference

The C is built `-fPIC` as a shared object, so `c2GJK`'s calls to `c2Dot`, `c22`,
`c23`, `c2L`, `c2D`, `c2Witness` and `c2GJKSimplexMetric` go through the PLT and
are therefore *interposable* (`LD_PRELOAD`). The Rust calls private clones of
those bodies directly. The clones were diffed line-for-line against both the C
bodies and the exported Rust wrappers and are identical, so only symbol
interposition could distinguish the two — which is a linkage artifact of the C
build, not a property of the API, and is not something any caller can observe.

## Divergences that are C undefined behaviour (documented, not "fixed")

Four inputs make the C read uninitialised or out-of-bounds memory. The C result
is then whatever happens to be on its stack — not a value the C source defines,
not reproducible, and therefore not matchable. They are listed as rows U1–U6 of
`ERRORS.md`, and the boundary of each was established experimentally:

* **U3** — `cache->count == 4` makes the C read `cache->iB[3]`, which aliases the
  **float** `div` reinterpreted as an `int`. With `div == 1.0f` that is the proxy
  vertex index `1065353216`: the differential test **segfaulted the C**. Row 65
  now asserts the sub-case where every aliased index lands in `[0, 8)`.
* **U4** — a cache index `>= proxy.count` reads uninitialised proxy vertices. Made
  visible by a test that replayed a 2-vertex cache against a 1-vertex circle
  proxy: the C returned stack garbage `(-4.68e-17, 1.53e-41)` where the Rust
  returned `(0,0)`. No cache the library itself produces can reach this, because
  `c2Support` only ever yields indices `< proxy.count`.
* **U1** — an out-of-range `C2_TYPE` passed to `c2GJK` leaves `c2Proxy pA`
  uninitialised (`c2MakeProxy` has no `default:` label), so `pA.count` is stack
  garbage. Recorded as the `#[ignore]`d `err_gjk_bad_type_is_ub`.
* **U2** — `cache->count > 4` smashes the C's stack.

Note that the *defined* out-of-range-enum cases **are** asserted: `c2Collided` and
`c2MakeProxy` handle any `int` (rows 1–5), including `INT_MIN`/`INT_MAX`, and
`use_radius` is verified to be a pure truthiness test for `2`, `-1`, `INT_MAX` and
`INT_MIN` (row 16).

## Quirks of the C that are preserved verbatim

Confirmed by dedicated tests, not by reading the code:

* `c2GJKSimplexMetric`'s `default:` falls **through into `case 1:`** ⇒ any count
  outside {2,3} returns `0.0f` (row 26).
* `c2L` has `case 1`, `case 2` and `default` but **no `case 3`**, so a 3-simplex
  returns `(0,0)` (row 29).
* `c2MakeProxy` has **no `default:`** ⇒ an invalid type leaves the output proxy
  byte-for-byte unmodified (row 5, asserted against a pre-dirtied buffer).
* The cache staleness test `min_metric < max_metric*2.0f && metric < -1.0e8f` can
  never be true for a finite metric, so a non-empty cache is **always** replayed
  even when it is completely stale (row 13).
* `c2Collided` **swaps** its arguments in the `(AABB, CIRCLE)`, `(CAPSULE, CIRCLE)`
  and `(CAPSULE, AABB)` arms (rows 43, 46, 47).
* `c2Maxv`/`c2Minv` use C's `?:`, so a NaN operand yields the **second** operand —
  different from `fmaxf`/`fminf` *and* from Rust's `f32::max`/`min`. Same for
  `+0.0` vs `-0.0` (rows 52, 53).
* `c2Support`'s `dot > dmax` is strict ⇒ ties keep the **earlier** index, and
  `verts[0]` is dereferenced even when `count <= 0` (rows 44, 45).
* Negative radii are never validated and behave like positive ones because
  `r2 = (A.r + B.r)²` (row 56).
* `if (c2GJK(...)) return 0;` treats a **NaN** distance as "no collision" (row 60).
* `c2GJK` writes the candidate simplex vertex **before** the duplicate test and
  does **not** `++s.count` when the test fires (row 23).
* `cache->iA[i]`/`iB[i]` are only written back for `i < s.count`; the trailing
  slots keep the caller's values (asserted in `err_gjk_cache_count_zero_rejected`).

## Completion gate

| requirement | status |
|---|---|
| `SYMBOLS.md`: `nm -D` shows 0 missing symbols in the Rust `.so` | **PASS** — 38/38, diff empty |
| `nm -D -u`: 0 undefined non-libc symbols in the Rust `.so` | **PASS** |
| Phase B: every `CONFIGS.md` row passes across randomized inputs | **PASS** — 65/65 rows, 67 tests |
| Phase C: every `ERRORS.md` row has a passing error-path differential test | **PASS** — 71/71 rows, 60 tests |
| All of the above under every feature combination | **PASS** — 3 combos × 2 profiles (no `[features]` table exists, so the combos are `--no-default-features`, default and `--all-features`) |
| Randomized rows re-verified with fresh seeds | **PASS** — `C2_DIFF_SEED=1..3` per combo, plus 106 ad-hoc seeds (14 + 46 release + 46 debug) |
